//! Read-only submodule identity and explicit parent-recorded update receipts.
use serde::{Deserialize, Serialize};

/// One gitlink's parent-recorded and nested state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Details {
    pub path: String,
    pub path_bytes: Vec<u8>,
    /// Gitlink commit on the selected After side, if any.
    pub recorded: Option<String>,
    /// Gitlink commit on the selected Before side, if any.
    pub before: Option<String>,
    /// Gitlink commit in the parent index.
    pub index: Option<String>,
    /// Nested HEAD, when the submodule is initialized.
    pub checked_out: Option<String>,
    pub initialized: bool,
    /// `None` when the nested status could not be read.
    pub dirty: Option<bool>,
    pub url: Option<String>,
    pub branch: Option<String>,
    /// URL currently configured in the parent repository, when it differs.
    pub configured_url: Option<String>,
    /// Whether the recorded commit exists in the nested repository.
    pub recorded_available: Option<bool>,
    /// Git's one-character submodule status prefix, when reported.
    pub status: Option<String>,
    pub note: Option<String>,
}

impl Details {
    pub fn target(&self) -> Option<&str> {
        self.recorded.as_deref().or(self.index.as_deref())
    }

    pub fn status_label(&self) -> &'static str {
        if !self.initialized {
            "Uninitialized"
        } else if self.dirty == Some(true) {
            "Modified"
        } else if self.recorded_available == Some(false) {
            "Recorded commit missing"
        } else if self.status.as_deref() == Some("+") {
            "Different commit"
        } else if self.status.as_deref() == Some("U") {
            "Conflict"
        } else if self.dirty == Some(false) {
            "Clean"
        } else {
            "Unknown"
        }
    }

    pub fn can_update(&self) -> Result<(), String> {
        let Some(target) = self.target() else {
            return Err("No parent-recorded commit is selected.".into());
        };
        if self.index.as_deref().is_some_and(|index| index != target) {
            return Err(format!(
                "Parent index records {}; stage the parent gitlink before updating.",
                self.index.as_deref().unwrap_or_default()
            ));
        }
        if self.dirty == Some(true) {
            return Err("Submodule has local changes; open it to inspect or clean it.".into());
        }
        if self.initialized && self.recorded_available == Some(false) {
            return Err(
                "Recorded commit is not available locally; fetch it explicitly first.".into(),
            );
        }
        Ok(())
    }

    pub fn network_hint(&self) -> Option<&str> {
        self.url.as_deref()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub path: String,
    pub path_bytes: Vec<u8>,
    pub target: Option<String>,
    pub checked_out: Option<String>,
    pub initialized: bool,
    pub dirty: Option<bool>,
}

impl Receipt {
    pub fn from_details(details: &Details) -> Self {
        Self {
            path: details.path.clone(),
            path_bytes: details.path_bytes.clone(),
            target: details.target().map(str::to_owned),
            checked_out: details.checked_out.clone(),
            initialized: details.initialized,
            dirty: details.dirty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details() -> Details {
        Details {
            path: "vendor/lib".into(),
            path_bytes: b"vendor/lib".to_vec(),
            recorded: Some("a".repeat(40)),
            before: Some("b".repeat(40)),
            index: Some("a".repeat(40)),
            checked_out: Some("a".repeat(40)),
            initialized: true,
            dirty: Some(false),
            url: Some("https://example.org/lib".into()),
            branch: Some("main".into()),
            configured_url: None,
            recorded_available: Some(true),
            status: Some(" ".into()),
            note: None,
        }
    }

    #[test]
    fn updates_require_index_agreement_and_a_clean_nested_tree() {
        let clean = details();
        assert!(clean.can_update().is_ok());
        let mut dirty = clean.clone();
        dirty.dirty = Some(true);
        assert!(dirty.can_update().is_err());
        let mut staged = clean.clone();
        staged.index = Some("c".repeat(40));
        assert!(staged.can_update().is_err());
        let mut missing = clean;
        missing.recorded_available = Some(false);
        assert!(missing.can_update().is_err());
    }

    #[test]
    fn status_names_uninitialized_dirty_and_missing_states() {
        let mut item = details();
        item.initialized = false;
        assert_eq!(item.status_label(), "Uninitialized");
        item.initialized = true;
        item.dirty = Some(true);
        assert_eq!(item.status_label(), "Modified");
        item.dirty = Some(false);
        item.recorded_available = Some(false);
        assert_eq!(item.status_label(), "Recorded commit missing");
    }
}
