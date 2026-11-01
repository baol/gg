//! Shared private rewrite-backup records.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Backup {
    /// Private reference such as `refs/gg/backup/rebase/...`.
    pub reference: String,
    /// Rewrite kind recorded in the reference path.
    pub label: String,
    /// Commit saved before the rewrite.
    pub commit: String,
    /// Commit date reported by Git.
    pub time: String,
    pub subject: String,
    /// Branch that was rewritten, recorded after the creation time in the
    /// reference name; `None` for older backups and a detached HEAD.
    #[serde(default)]
    pub branch: Option<String>,
    /// Unix seconds when gg saved the backup, from the reference name.
    #[serde(default)]
    pub saved: i64,
}

/// What restoring a backup did: the branch moved to `commit` and the tip it
/// left is kept as the new backup `saved`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Restore {
    pub branch: String,
    pub previous: String,
    pub commit: String,
    pub saved: String,
}
