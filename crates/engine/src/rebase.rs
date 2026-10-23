//! Shared rebase observations and receipts.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// Reference requested by the user.
    pub reference: String,
    /// Resolved commit used as the new base.
    pub commit: Option<String>,
    /// HEAD before the rewrite.
    pub previous: Option<String>,
    /// Private recovery reference for the pre-rewrite tip.
    pub backup: Option<String>,
    /// Git completed the rebase without leaving an operation behind.
    pub applied: bool,
    /// Git left a rebase that can continue, skip, or abort.
    pub in_progress: bool,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
}
