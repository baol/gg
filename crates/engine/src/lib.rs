//! Toolkit-independent history command and snapshots.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const HISTORY_COMMAND: &str = "repository.history";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryRequest {
    pub path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    pub time: String,
    pub message: String,
}

impl Commit {
    pub fn subject(&self) -> &str {
        self.message.lines().next().unwrap_or("(empty message)")
    }

    pub fn matches(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        [&self.id, &self.author, &self.email, &self.message]
            .iter()
            .any(|value| value.to_lowercase().contains(&query))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct History {
    pub path: PathBuf,
    pub head: String,
    pub commits: Vec<Commit>,
    pub truncated: bool,
    #[serde(default)]
    pub shallow: bool,
}

/// Incremental traversal owned by a background session or a terminal consumer.
pub trait HistoryReader {
    fn identity(&self) -> &History;
    fn next_commit(&mut self) -> Result<Option<Commit>, String>;
}

pub trait HistorySource {
    fn history(&self, request: &HistoryRequest) -> Result<History, String>;
}

pub fn dispatch(source: &impl HistorySource, request: &HistoryRequest) -> Result<History, String> {
    source.history(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_covers_identity_and_full_message() {
        let commit = Commit {
            id: "abc123".into(),
            parents: vec![],
            author: "Ada".into(),
            email: "ada@example.org".into(),
            time: String::new(),
            message: "Add history\n\nUnicode café".into(),
        };
        for query in ["ADA", "abc", " CAFÉ ", "", "example"] {
            assert!(commit.matches(query));
        }
        assert!(!commit.matches("missing"));
        assert_eq!(commit.subject(), "Add history");
    }
}

pub mod blame;
pub mod conflict;
pub mod diff;
pub mod file;
pub mod file_history;
pub mod lfs;
pub mod merge;
pub mod mutation;
pub mod rebase;
pub mod recovery;
pub mod reflog;
pub mod refs;
pub mod reset;
pub mod review;
pub mod rewrite;
pub mod stash;
pub mod status;
pub mod submodule;
pub mod sync;
pub mod worktree;
