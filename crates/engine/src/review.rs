//! Immutable comparison identities and incremental changed-file summaries.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ReviewRequest {
    pub path: PathBuf,
    pub revision: String,
    pub parent: Option<usize>,
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
