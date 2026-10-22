//! Read-only branch, remote, tag, and HEAD labels shared by CLI and native views.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    Branch,
    Remote,
    Tag,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefItem {
    /// Short display name, such as `main`, `origin/main`, or `v1.0`.
    pub name: String,
    /// Full Git reference name, such as `refs/heads/main`.
    pub full_name: String,
    pub kind: RefKind,
    /// Peeled commit ID when the reference points at a commit.
    pub commit: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Head {
    /// Full symbolic reference name, or `None` for a detached HEAD.
    pub name: Option<String>,
    pub detached: bool,
    pub commit: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Remote {
    /// Configured remote name, such as `origin`.
    pub name: String,
    /// Display-safe fetch URL with any password removed.
    pub fetch_url: Option<String>,
    /// Display-safe push URL when it differs from the fetch URL.
    pub push_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tracking {
    /// Short local branch name, such as `main`.
    pub branch: String,
    /// Full local reference name, such as `refs/heads/main`.
    pub full_name: String,
    /// Short upstream label, such as `origin/main`.
    pub upstream: String,
    /// Full remote-tracking reference name, such as `refs/remotes/origin/main`.
    pub upstream_full_name: String,
    pub remote: String,
    /// Branch name on the remote, such as `main`.
    pub remote_branch: String,
    /// Commits reachable from the local branch and not the upstream.
    pub ahead: Option<u64>,
    /// Commits reachable from the upstream and not the local branch.
    pub behind: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub head: Option<Head>,
    pub items: Vec<RefItem>,
    pub remotes: Vec<Remote>,
    pub tracking: Vec<Tracking>,
}

impl Snapshot {
    pub fn for_commit<'a>(&'a self, commit: &'a str) -> impl Iterator<Item = &'a RefItem> + 'a {
        self.items.iter().filter(move |item| item.commit == commit)
    }

    pub fn current_tracking(&self) -> Option<&Tracking> {
        let head = self.head.as_ref()?.name.as_deref()?;
        self.tracking.iter().find(|item| item.full_name == head)
    }
}
