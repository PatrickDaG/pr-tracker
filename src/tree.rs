// SPDX-License-Identifier: AGPL-3.0-or-later WITH GPL-3.0-linking-exception
// SPDX-FileCopyrightText: 2021 Alyssa Ross <hi@alyssa.is>
// SPDX-FileCopyrightText: 2022 Arnout Engelen <arnout@bzzt.net>

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};

use askama::Template;

use crate::branches::branch_hydra_link;
use crate::branches::next_branches;
use crate::github;
use crate::nixpkgs::Nixpkgs;

#[derive(Debug, Template)]
#[template(path = "tree.html")]
pub struct Tree {
    branch_name: String,
    accepted: Option<bool>,
    hydra_link: Option<String>,
    children: Vec<Tree>,
}

impl Tree {
    fn generate(branch: String, found_branches: &mut BTreeSet<OsString>) -> Tree {
        found_branches.insert((&branch).into());

        let nexts = next_branches(&branch)
            .into_iter()
            .map(|b| Self::generate(b.to_string(), found_branches))
            .collect();

        let link = branch_hydra_link(&branch);

        Tree {
            accepted: None,
            branch_name: branch,
            hydra_link: link,
            children: nexts,
        }
    }
    pub fn collect_branches(&self, vec: &mut Vec<String>) -> bool {
        let mut res = false;
        if let Some(true) = self.accepted {
            vec.push(self.branch_name.clone());
        } else {
            res = true
        }
        for c in &self.children {
            res |= c.collect_branches(vec);
        }
        res
    }

    fn fill_accepted(&mut self, branches: &BTreeSet<OsString>, missing_means_absent: bool) {
        self.accepted = match branches.contains(OsStr::new(&self.branch_name)) {
            true => Some(true),
            false if missing_means_absent => Some(false),
            false => None,
        };

        for child in self.children.iter_mut() {
            child.fill_accepted(branches, missing_means_absent);
        }
    }

    pub async fn make(
        base_branch: String,
        merge_status: &github::PullRequestStatus,
        nixpkgs: &Nixpkgs<'_>,
    ) -> Tree {
        let mut missing_means_absent = true;
        let mut branches = BTreeSet::new();

        let mut tree = Self::generate(base_branch.clone(), &mut branches);

        if let github::PullRequestStatus::Merged {
            merge_commit_oid, ..
        } = merge_status
        {
            if let Some(merge_commit) = merge_commit_oid {
                let mut containing_commits = BTreeSet::new();

                if let Err(e) = nixpkgs
                    .branches_containing_commit(merge_commit, &mut containing_commits)
                    .await
                {
                    eprintln!("pr-tracker: branches_containing_commit: {}", e);
                    missing_means_absent = false;
                }

                branches = branches
                    .intersection(&containing_commits)
                    .cloned()
                    .collect();
            } else {
                branches.clear();
                missing_means_absent = false;
            }

            // Even if something goes wrong with our local Git repo,
            // or GitHub didn't tell us the merge commit, we know that
            // the base branch of the PR must contain the commit,
            // because GitHub told us it was merged into it.
            branches.insert(base_branch.into());
        } else {
            branches.clear();
        }

        tree.fill_accepted(&branches, missing_means_absent);
        tree
    }
}
