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
}
