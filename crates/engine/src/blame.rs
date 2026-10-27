//! Bounded, read-only line attribution for committed file content.
use serde::{Deserialize, Serialize};

/// Maximum source lines attributed in one request.
pub const MAX_LINES: usize = 100_000;
/// Maximum source bytes attributed in one request.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;

/// Attribution for one source line in the selected revision.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Line {
    /// One-based line in the selected revision.
    pub line: usize,
    /// One-based line in the origin commit's version of the path.
    pub origin_line: usize,
    pub commit: String,
    pub author: String,
    pub email: String,
    /// Author time in Unix seconds.
    pub time: i64,
    pub summary: String,
}

impl Line {
    pub fn short_commit(&self) -> String {
        self.commit.chars().take(8).collect()
    }
}

/// Attribution for one bounded line range of a committed path.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Blame {
    pub revision: String,
    pub path: String,
    pub path_bytes: Vec<u8>,
    pub start_line: usize,
    pub end_line: usize,
    pub lines: Vec<Line>,
    pub complete: bool,
    pub note: Option<String>,
}

impl Blame {
    pub fn line(&self, line: usize) -> Option<&Line> {
        self.lines.iter().find(|entry| entry.line == line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_lookup_and_short_commit_are_exact() {
        let blame = Blame {
            revision: "HEAD".into(),
            path: "a.txt".into(),
            path_bytes: b"a.txt".to_vec(),
            start_line: 1,
            end_line: 2,
            lines: vec![Line {
                line: 1,
                origin_line: 4,
                commit: "a".repeat(40),
                author: "Test".into(),
                email: "test@example.org".into(),
                time: 1,
                summary: "first".into(),
            }],
            complete: true,
            note: None,
        };
        assert_eq!(blame.line(1).map(|line| line.origin_line), Some(4));
        assert!(blame.line(2).is_none());
        assert_eq!(blame.lines[0].short_commit(), "aaaaaaaa");
    }
}
