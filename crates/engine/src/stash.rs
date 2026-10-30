//! Shared stash observations and mutation receipts.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StashEntry {
    /// Position in the stash reflog, where zero is the newest entry.
    pub index: u32,
    /// Full reference such as `stash@{0}`.
    pub reference: String,
    /// Commit recorded by the stash reflog.
    pub commit: String,
    /// Reflog subject, such as `WIP on main: abc123 subject`.
    pub message: String,
    /// Branch the stash was made on; `None` for a detached HEAD or a subject
    /// that names no branch.
    #[serde(default)]
    pub branch: Option<String>,
    /// Relative age of the stash commit, such as `3 days ago`.
    #[serde(default)]
    pub age: String,
    /// Changed paths, counting untracked files stored with the entry; `None`
    /// when not counted.
    #[serde(default)]
    pub files: Option<usize>,
}

/// The branch named by a stash reflog subject: `WIP on <branch>: …` or
/// `On <branch>: …`. A detached HEAD (`(no branch)`) has none.
pub fn subject_branch(subject: &str) -> Option<String> {
    let rest = subject
        .strip_prefix("WIP on ")
        .or_else(|| subject.strip_prefix("On "))?;
    let (branch, _) = rest.split_once(": ")?;
    (!branch.is_empty() && branch != "(no branch)").then(|| branch.to_string())
}

#[cfg(test)]
mod tests {
    use super::subject_branch;

    #[test]
    fn subject_names_the_branch() {
        assert_eq!(
            subject_branch("WIP on main: abc123 subject").as_deref(),
            Some("main")
        );
        assert_eq!(
            subject_branch("On feature/x: my message").as_deref(),
            Some("feature/x")
        );
        assert_eq!(subject_branch("WIP on (no branch): abc123 subject"), None);
        assert_eq!(subject_branch("custom subject"), None);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StashList {
    pub entries: Vec<StashEntry>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StashReceipt {
    /// Reference created, applied, popped, or dropped.
    pub reference: Option<String>,
    /// Message recorded by a push, or the listed entry message.
    pub message: Option<String>,
    /// A new stash entry was recorded.
    pub created: bool,
    /// Working changes were restored.
    pub applied: bool,
    /// The stash entry was removed after a successful pop or drop.
    pub dropped: bool,
    /// Paths left with unresolved conflicts by an apply or pop.
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
}
