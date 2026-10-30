//! Exact caller state restored when a repository tool closes.
//!
//! Presentation changes inside a tool never create a location. Recovery adds
//! its own source variant here when it is implemented; do not invent one.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoverySource {
    Backups,
    Reflog,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HistoryLocation {
    pub selected: Option<String>,
    pub query: String,
    pub offset: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReturnLocation {
    History(HistoryLocation),
    Details(HistoryLocation),
    Recovery {
        source: RecoverySource,
        /// Reflog filter that owned the entry; absent for Backups.
        reference: Option<String>,
        /// Source-specific entry identity: reflog selector or backup reference.
        entry: String,
    },
    /// The interactive rebase dialog, which keeps its plan while held.
    Rebase,
}

impl ReturnLocation {
    pub fn location(&self) -> Option<&HistoryLocation> {
        match self {
            Self::History(location) | Self::Details(location) => Some(location),
            Self::Recovery { .. } | Self::Rebase => None,
        }
    }

    pub fn shows_details(&self) -> bool {
        matches!(self, Self::Details(_))
    }
}
