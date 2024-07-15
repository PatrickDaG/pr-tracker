// SPDX-License-Identifier: AGPL-3.0-or-later WITH GPL-3.0-linking-exception
// SPDX-FileCopyrightText: 2021 Alyssa Ross <hi@alyssa.is>
// SPDX-FileCopyrightText: 2021 Sumner Evans <me@sumnerevans.com>

mod branches;
mod github;
mod mail;
mod nixpkgs;
mod systemd;
mod tree;

use std::collections::HashSet;
use std::fs::{remove_dir_all, File};
use std::io::BufReader;
use std::path::PathBuf;
use std::{ffi::OsString, fs::read_dir};

use askama::Template;
use async_std::io::{self};
use async_std::net::TcpListener;
use async_std::os::unix::io::FromRawFd;
use async_std::os::unix::net::UnixListener;
use async_std::pin::Pin;
use async_std::prelude::*;
use async_std::process::exit;
use futures_util::future::join_all;
use http_types::mime;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use serde_json::json;
use structopt::StructOpt;
use tide::{Request, Response};

use github::{GitHub, PullRequestStatus};
use mail::send_notification;
use nixpkgs::Nixpkgs;
use systemd::{is_socket_inet, is_socket_unix, listen_fds};
use tree::Tree;

#[derive(StructOpt, Debug)]
struct Config {
    #[structopt(long, parse(from_os_str))]
    path: PathBuf,

    #[structopt(long, parse(from_os_str))]
    remote: PathBuf,

    #[structopt(long, parse(from_os_str))]
    user_agent: OsString,

    #[structopt(long)]
    source_url: String,

    #[structopt(long, default_value = "/")]
    mount: String,

    #[structopt(long, default_value = "data")]
    data_folder: String,
}

static CONFIG: Lazy<Config> = Lazy::new(Config::from_args);

static GITHUB_TOKEN: Lazy<OsString> = Lazy::new(|| {
    use std::env;
    use std::io::{stdin, BufRead, BufReader};
    use std::os::unix::prelude::*;

    match env::var_os("PR_TRACKER_GITHUB_TOKEN") {
        Some(token) => token,
        None => {
            let mut bytes = Vec::with_capacity(41);
            if let Err(e) = BufReader::new(stdin()).read_until(b'\n', &mut bytes) {
                eprintln!("pr-tracker: read: {}", e);
                exit(74)
            }
            if bytes.last() == Some(&b'\n') {
                bytes.pop();
            }
            OsString::from_vec(bytes)
        }
    }
});

#[derive(Debug, Default, Template)]
#[template(path = "page.html")]
struct PageTemplate {
    error: Option<String>,
    pr_number: Option<String>,
    email: Option<String>,
    pr_title: Option<String>,
    closed: bool,
    subscribed: bool,
    tree: Option<Tree>,
    source_url: String,
}

#[derive(Debug, Deserialize)]
struct Query {
    pr: Option<String>,
    email: Option<String>,
}

async fn track_pr(pr_number: String, status: &mut u16, page: &mut PageTemplate) {
    let pr_number_i64 = match pr_number.parse() {
        Ok(n) => n,
        Err(_) => {
            *status = 400;
            page.error = Some(format!("Invalid PR number: {}", pr_number));
            return;
        }
    };

    let github = GitHub::new(&GITHUB_TOKEN, &CONFIG.user_agent);

    let pr_info = match github.pr_info_for_nixpkgs_pr(pr_number_i64).await {
        Err(github::Error::NotFound) => {
            *status = 404;
            page.error = Some(format!("No such nixpkgs PR #{}.", pr_number_i64));
            return;
        }

        Err(e) => {
            *status = 500;
            page.error = Some(e.to_string());
            return;
        }

        Ok(info) => info,
    };

    page.pr_number = Some(pr_number);
    page.pr_title = Some(pr_info.title);

    if matches!(pr_info.status, PullRequestStatus::Closed) {
        page.closed = true;
        return;
    }

    let nixpkgs = Nixpkgs::new(&CONFIG.path, &CONFIG.remote);
    let tree = Tree::make(pr_info.branch.to_string(), &pr_info.status, &nixpkgs).await;

    if let github::PullRequestStatus::Merged {
        merge_commit_oid, ..
    } = pr_info.status
    {
        if merge_commit_oid.is_none() {
            page.error = Some("For older PRs, GitHub doesn't tell us the merge commit, so we're unable to track this PR past being merged.".to_string());
        }
    }

    page.tree = Some(tree);
}
async fn update_subscribers<S>(_request: Request<S>) -> http_types::Result<Response> {
    let mut status = 200;
    let mut page = PageTemplate {
        source_url: CONFIG.source_url.clone(),
        ..Default::default()
    };

    let re_pull = Regex::new(r"^[0-9]*$")?;
    let re_mail = Regex::new(
        r#"^(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])$"#,
    )?;
    for f in read_dir(CONFIG.data_folder.clone())? {
        let dir_path = f?.path();
        let dir_name = dir_path.file_name().and_then(|x| x.to_str()).unwrap();
        if dir_path.is_dir() && re_pull.is_match(dir_name) {
            track_pr(dir_name.to_string(), &mut status, &mut page).await;
            println!("Pruning pr number {dir_name}");
            if let Some(ref tree) = page.tree {
                let mut v = Vec::new();
                let remaining = tree.collect_branches(&mut v);
                let current: HashSet<String> = v.into_iter().collect();
                println!("the pr is merged in: {:#?}", current);
                for f in read_dir(dir_path.clone())? {
                    let file_path = f?.path();
                    let file_name = file_path
                        .file_name()
                        .and_then(|x| x.to_str())
                        .unwrap()
                        .to_owned();
                    if file_path.is_file() && re_mail.is_match(&file_name) {
                        println!("{} has received notifications for:", file_name);
                        let file = File::open(file_path)?;
                        let reader = BufReader::new(file);
                        let val: HashSet<String> = serde_json::from_reader(reader)?;
                        println!("{:#?}", val);
                        let to_do = &current - &val;
                        println!("You will be notified for: {:#?}", to_do);
                        let _ = send_notification(
                            &file_name,
                            &to_do,
                            page.pr_number.as_ref().unwrap(),
                            page.pr_title.as_ref().unwrap(),
                            !remaining,
                        );
                    }
                }
                if !remaining {
                    println!("Removing {}", dir_name);
                    remove_dir_all(dir_path)?;
                }
            }
        }
    }
    Ok(Response::builder(200)
        .content_type(mime::HTML)
        .body("Sucess")
        .build())
}

async fn handle_request<S>(request: Request<S>) -> http_types::Result<Response> {
    let mut status = 200;
    let mut page = PageTemplate {
        source_url: CONFIG.source_url.clone(),
        ..Default::default()
    };

    let pr_number = request.query::<Query>()?.pr;
    let email = request.query::<Query>()?.email;
    page.email = email.clone();

    match pr_number.clone() {
        Some(pr_number) => track_pr(pr_number, &mut status, &mut page).await,
        None => {}
    };
    match email {
        Some(email) => {
            if let Some(ref tree) = page.tree {
                let mut v = Vec::new();
                let remaining = tree.collect_branches(&mut v);
                if !remaining {
                    page.error = Some("There are no branches remaining to be tracked".to_string())
                } else {
                    page.subscribed = true;
                    let folder = format!("{}/{}", CONFIG.data_folder, pr_number.unwrap());
                    std::fs::create_dir_all(folder.clone())?;
                    std::fs::write(format!("{folder}/{email}"), json!(v).to_string())?;
                }
            }
        }
        None => {}
    }

    Ok(Response::builder(status)
        .content_type(mime::HTML)
        .body(page.render()?)
        .build())
}

#[async_std::main]
async fn main() {
    fn handle_error<T, E>(result: Result<T, E>, code: i32, message: impl AsRef<str>) -> T
    where
        E: std::error::Error,
    {
        match result {
            Ok(v) => v,
            Err(e) => {
                eprintln!("pr-tracker: {}: {}", message.as_ref(), e);
                exit(code);
            }
        }
    }

    // Make sure arguments are parsed before starting server.
    let _ = *CONFIG;
    let _ = *GITHUB_TOKEN;

    let mut server = tide::new();
    let mut root = server.at(&CONFIG.mount);

    root.at("/").get(handle_request);
    root.at("update").get(update_subscribers);

    let fd_count = handle_error(listen_fds(true), 71, "sd_listen_fds");

    if fd_count == 0 {
        eprintln!("pr-tracker: No listen file descriptors given");
        exit(64);
    }

    let mut listeners: Vec<Pin<Box<dyn Future<Output = _>>>> = Vec::new();

    for fd in (3..).take(fd_count as usize) {
        let s = server.clone();
        if handle_error(is_socket_inet(fd), 74, "sd_is_socket_inet") {
            listeners.push(Box::pin(s.listen(unsafe { TcpListener::from_raw_fd(fd) })));
        } else if handle_error(is_socket_unix(fd), 74, "sd_is_socket_unix") {
            listeners.push(Box::pin(s.listen(unsafe { UnixListener::from_raw_fd(fd) })));
        } else {
            eprintln!("pr-tracker: file descriptor {} is not a socket", fd);
            exit(64);
        }
    }

    let errors: Vec<_> = join_all(listeners)
        .await
        .into_iter()
        .filter_map(io::Result::err)
        .collect();
    for error in errors.iter() {
        eprintln!("pr-tracker: listen: {}", error);
    }
    if !errors.is_empty() {
        exit(74);
    }
}
