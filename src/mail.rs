use std::collections::HashSet;

use anyhow::Result;
use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::env;
use urlencoding::encode;

use crate::CONFIG;

pub fn send_notification(
    recipient: &str,
    branches: &HashSet<String>,
    pr_number: &str,
    pr_title: &str,
    last: bool,
) -> Result<()> {
    let mut body = format!(
        "This is your friendly neighbourhood pr-tracker.<br>
        PR <a href=\"https://github.com/NixOS/nixpkgs/pull/{pr_number}\">#{pr_number}</a>\
        (\"{pr_title}\") has reached:<br>
        {:#?}<br>",
        branches
    );
    if last {
        body += "This is the last update you will get for this pr.<br>\
        Thx for using this service<br>\
        Goodbye";
    } else {
        body += &format!(
            "<a href=\"{}/unsubscribe?pr={pr_number}&email={}\">Unsubscribe from this PR</a><br>",
            &CONFIG.url,
            encode(recipient)
        );
        body += &format!(
            "<a href=\"{}/unsubscribe?email={}\">Unsubscribe from all PRs</a>",
            &CONFIG.url,
            encode(recipient)
        );
    }
    let sending_address = &CONFIG.email_address;
    let sending_user = match &CONFIG.email_user {
        Some(address) => address,
        _ => &sending_address,
    };
    let sending_passwd = env::var("PR_TRACKER_MAIL_PASSWD")?;

    let sending_server = CONFIG.email_server.as_ref();

    let email = Message::builder()
        .from(format!("PR-Tracker <{}>", sending_address).parse().unwrap())
        .to(Mailbox::new(None, recipient.parse().unwrap()))
        .subject(format!(
            "PR-tracker: {pr_number}: {pr_title} has reached {:?}",
            branches
        ))
        .header(ContentType::TEXT_HTML)
        .body(body)
        .unwrap();

    let creds = Credentials::new(sending_user.to_string(), sending_passwd.to_string());

    // Open a remote connection to gmail
    let mailer = SmtpTransport::relay(&sending_server)
        .unwrap()
        .credentials(creds)
        .build();

    // Send the email
    mailer.send(&email).unwrap();

    println!("Email sent successfully!");
    Ok(())
}
