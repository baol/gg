//! Immutable comparison identities and incremental changed-file summaries.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ReviewRequest {
    pub path: PathBuf,
    pub revision: String,
    pub parent: Option<usize>,
    /// Compare `revision` against this commit instead of one of its parents.
    pub against: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Comparison {
    pub path: PathBuf,
    pub commit: String,
    pub parent: Option<String>,
    pub parents: Vec<String>,
    pub subject: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSide {
    pub id: String,
    pub mode: u16,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Deleted,
    Modified,
    TypeChanged,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileChange {
    pub path_bytes: Vec<u8>,
    pub path: String,
    pub kind: ChangeKind,
    pub old: Option<FileSide>,
    pub new: Option<FileSide>,
}
pub enum ReviewStep {
    Change(FileChange),
    Pending,
    Complete,
}
pub trait ReviewReader {
    fn comparison(&self) -> &Comparison;
    /// One traversal step; callers can cancel between directory visits.
    fn step(&mut self) -> Result<ReviewStep, String>;
}

/// One-line, unambiguous display; path_bytes remains the native lookup identity.
pub fn display_path(bytes: &[u8]) -> String {
    let mut result = String::new();
    for chunk in bytes.utf8_chunks() {
        for ch in chunk.valid().chars() {
            match ch {
                '\\' => result.push_str("\\\\"),
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                ch if ch.is_control() => result.extend(ch.escape_unicode()),
                ch => result.push(ch),
            }
        }
        for byte in chunk.invalid() {
            result.push_str(&format!("\\x{byte:02x}"));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_display_preserves_unicode_and_escapes_ambiguous_bytes() {
        assert_eq!(display_path("café/界.rs".as_bytes()), "café/界.rs");
        assert_eq!(
            display_path(b"a\n\r\t\x1b\\\xff"),
            "a\\n\\r\\t\\u{1b}\\\\\\xff"
        );
        assert_ne!(display_path(b"\xff"), display_path(b"\\xff"));
    }
}
