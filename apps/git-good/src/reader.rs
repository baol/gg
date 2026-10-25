//! Shared reader identity and saved reading place.
//!
//! The reader is `diff_view::DiffView`. This module carries the values that
//! consumers (review, staging, and history) exchange without owning another
//! renderer. A saved context is exact: comparison identity, both endpoint
//! labels, presentation, inspection side, cursor, anchor, and pixel offset.

use crate::diff_worker::Mode;
use gg_engine::{file::ReadingSide, review::FileChange};
use std::path::PathBuf;

/// One endpoint label for the exact comparison side it names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint {
    /// Exact display label, such as `main (a1b2c3d)`, `Empty tree`, or `Absent`.
    pub label: String,
}

impl Endpoint {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

/// Presentation and place restored without reinterpreting the comparison.
#[derive(Clone, Debug, PartialEq)]
pub struct ReaderPlace {
    pub mode: Mode,
    pub side: ReadingSide,
    pub cursor: Option<usize>,
    pub anchor: Option<usize>,
    pub offset: f32,
}

/// Capabilities the current comparison actually supports.
///
/// Consumers use these flags to enable or explain actions. A label alone never
/// grants a capability.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub diff: bool,
    pub full_before: bool,
    pub full_after: bool,
    pub export_before: bool,
    pub export_after: bool,
}

/// A complete reader context, including both exact endpoints and the place.
#[derive(Clone, Debug)]
pub struct ReaderContext {
    /// Repository root used by the selected-file worker.
    pub repository: PathBuf,
    pub change: FileChange,
    pub before: Endpoint,
    pub after: Endpoint,
    pub place: ReaderPlace,
}

impl ReaderContext {
    pub fn new(
        repository: PathBuf,
        change: FileChange,
        before: impl Into<String>,
        after: impl Into<String>,
        place: ReaderPlace,
    ) -> Self {
        Self {
            before: Endpoint::new(before),
            after: Endpoint::new(after),
            repository,
            change,
            place,
        }
    }
}
