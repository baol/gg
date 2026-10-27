//! Read-only worktree inventory and explicit add/remove receipts.
use serde::{Deserialize, Serialize};

/// One worktree reported by `git worktree list --porcelain`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Worktree {
    pub path: String,
    pub path_bytes: Vec<u8>,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub detached: bool,
    pub bare: bool,
    pub locked: Option<String>,
    pub prunable: Option<String>,
    pub current: bool,
    pub main: bool,
    /// `None` when the path is missing or its status could not be read.
    pub dirty: Option<bool>,
}

impl Worktree {
    pub fn branch_label(&self) -> &str {
        self.branch.as_deref().unwrap_or(if self.detached {
            "detached HEAD"
        } else if self.bare {
            "bare"
        } else {
            "unknown"
        })
    }

    pub fn status_label(&self) -> &'static str {
        if self.current {
            "Current"
        } else if self.prunable.is_some() {
            "Missing"
        } else if self.locked.is_some() {
            "Locked"
        } else if self.dirty == Some(true) {
            "Modified"
        } else if self.dirty == Some(false) {
            "Clean"
        } else {
            "Unavailable"
        }
    }

    pub fn can_remove(&self) -> Result<(), String> {
        if self.main {
            Err("The main worktree cannot be removed.".into())
        } else if self.current {
            Err("Open another worktree before removing this one.".into())
        } else if let Some(reason) = &self.locked {
            Err(if reason.is_empty() {
                "Worktree is locked.".into()
            } else {
                format!("Worktree is locked: {reason}")
            })
        } else if let Some(reason) = &self.prunable {
            Err(if reason.is_empty() {
                "Worktree path is missing; prune is not offered here.".into()
            } else {
                format!("Worktree is prunable: {reason}")
            })
        } else if self.dirty == Some(true) {
            Err("Worktree has local changes; open it to inspect or clean it.".into())
        } else if self.dirty.is_none() {
            Err("Worktree status is unavailable; open it before removing.".into())
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub worktrees: Vec<Worktree>,
}

impl Snapshot {
    pub fn find(&self, path_bytes: &[u8]) -> Option<&Worktree> {
        self.worktrees
            .iter()
            .find(|worktree| worktree.path_bytes == path_bytes)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddRequest {
    pub destination: std::path::PathBuf,
    pub branch: Option<String>,
    pub create: Option<String>,
    pub at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub path: String,
    pub path_bytes: Vec<u8>,
    pub branch: Option<String>,
    pub head: Option<String>,
}

impl Receipt {
    pub fn from_worktree(worktree: &Worktree) -> Self {
        Self {
            path: worktree.path.clone(),
            path_bytes: worktree.path_bytes.clone(),
            branch: worktree.branch.clone(),
            head: worktree.head.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worktree() -> Worktree {
        Worktree {
            path: "/repo".into(),
            path_bytes: b"/repo".to_vec(),
            head: Some("a".repeat(40)),
            branch: Some("main".into()),
            detached: false,
            bare: false,
            locked: None,
            prunable: None,
            current: true,
            main: true,
            dirty: Some(false),
        }
    }

    #[test]
    fn main_and_current_worktrees_are_not_removable() {
        let main = worktree();
        assert!(main.can_remove().is_err());
        let mut linked = main.clone();
        linked.main = false;
        linked.current = true;
        assert!(linked.can_remove().is_err());
        linked.current = false;
        linked.dirty = Some(true);
        assert!(linked.can_remove().is_err());
        linked.dirty = Some(false);
        assert!(linked.can_remove().is_ok());
    }

    #[test]
    fn labels_name_detached_bare_locked_and_missing_states() {
        let mut item = worktree();
        item.branch = None;
        item.detached = true;
        assert_eq!(item.branch_label(), "detached HEAD");
        item.detached = false;
        item.bare = true;
        assert_eq!(item.branch_label(), "bare");
        item.bare = false;
        item.current = false;
        item.locked = Some("maintenance".into());
        assert_eq!(item.status_label(), "Locked");
        item.locked = None;
        item.prunable = Some("gone".into());
        assert_eq!(item.status_label(), "Missing");
    }
}
