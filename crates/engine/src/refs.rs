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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub head: Option<Head>,
    pub items: Vec<RefItem>,
}

impl Snapshot {
    pub fn for_commit<'a>(&'a self, commit: &'a str) -> impl Iterator<Item = &'a RefItem> + 'a {
        self.items.iter().filter(move |item| item.commit == commit)
    }
}
