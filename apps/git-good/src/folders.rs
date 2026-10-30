//! Persisted development base folders offered as clone destinations.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
struct Stored {
    #[serde(default)]
    roots: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    preferred: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Roots {
    entries: Vec<PathBuf>,
    preferred: Option<PathBuf>,
    path: Option<PathBuf>,
}

impl Roots {
    pub fn load() -> Self {
        settings_path()
            .map(|path| Self::load_from(&path))
            .unwrap_or_default()
    }

    pub fn load_from(path: &Path) -> Self {
        let stored = std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Stored>(&bytes).ok())
            .unwrap_or_default();
        let preferred = stored
            .preferred
            .filter(|folder| stored.roots.contains(folder));
        Self {
            entries: stored.roots,
            preferred,
            path: Some(path.to_path_buf()),
        }
    }

    /// Save to the file this list was loaded from.
    pub fn save(&self) -> Result<(), String> {
        let path = self
            .path
            .as_deref()
            .ok_or("No configuration folder is available for development base folders.")?;
        self.save_to(path)
    }

    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Create {}: {e}", parent.display()))?;
        }
        let stored = Stored {
            roots: self.entries.clone(),
            preferred: self.preferred.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&stored)
            .map_err(|e| format!("Encode development base folders: {e}"))?;
        std::fs::write(path, bytes)
            .map_err(|e| format!("Write development base folders to {}: {e}", path.display()))
    }

    pub fn entries(&self) -> &[PathBuf] {
        &self.entries
    }

    /// The saved folder the user marked as the clone default.
    #[cfg(test)]
    pub fn preferred(&self) -> Option<&Path> {
        self.preferred.as_deref()
    }

    /// Mark a saved folder as the clone default. Returns whether it changed.
    pub fn set_preferred(&mut self, index: usize) -> bool {
        let Some(folder) = self.entries.get(index) else {
            return false;
        };
        if self.preferred.as_ref() == Some(folder) {
            return false;
        }
        self.preferred = Some(folder.clone());
        true
    }

    /// The startup clone default: the marked folder, else the first saved
    /// folder, else the launch folder.
    pub fn default_folder(&self) -> PathBuf {
        self.preferred
            .clone()
            .or_else(|| self.entries.first().cloned())
            .unwrap_or_else(starting_folder)
    }

    /// Destination choices: saved folders in order, then the launch folder.
    pub fn candidates(&self) -> Vec<PathBuf> {
        let mut candidates = self.entries.clone();
        let launch = starting_folder();
        if !candidates.contains(&launch) {
            candidates.push(launch);
        }
        candidates
    }

    /// Add one folder. Returns whether the list changed.
    pub fn add(&mut self, value: &str) -> Result<bool, String> {
        let path = normalize_folder(value)?;
        if self.entries.iter().any(|entry| entry == &path) {
            return Ok(false);
        }
        self.entries.push(path);
        Ok(true)
    }

    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.entries.len() {
            let removed = self.entries.remove(index);
            if self.preferred.as_ref() == Some(&removed) {
                self.preferred = None;
            }
            true
        } else {
            false
        }
    }
}

/// The starting folder for a clone: the launch directory, else the home folder.
pub fn starting_folder() -> PathBuf {
    std::env::current_dir()
        .ok()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_default()
}

pub fn normalize_folder(value: &str) -> Result<PathBuf, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("Folder is empty.".into());
    }
    let expanded = match value {
        "~" => std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or("Home folder is unavailable.")?,
        value if value.starts_with("~/") || value.starts_with("~\\") => std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(&value[2..]))
            .ok_or("Home folder is unavailable.")?,
        value => PathBuf::from(value),
    };
    let path = if expanded.is_absolute() {
        expanded
    } else {
        starting_folder().join(expanded)
    };
    if !path.is_absolute() {
        return Err("Folder must be an absolute path.".into());
    }
    Ok(path)
}

pub fn settings_path() -> Option<PathBuf> {
    if let Some(folder) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(folder).join("gg").join("roots.json"));
    }
    if cfg!(windows) {
        return std::env::var_os("APPDATA")
            .map(|folder| PathBuf::from(folder).join("gg").join("roots.json"));
    }
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join("gg")
            .join("roots.json")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_deduplicates_removes_and_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("nested/roots.json");
        let mut roots = Roots::load_from(&path);
        assert!(roots.entries().is_empty());
        let first = dir.path().join("projects");
        assert!(roots.add(first.to_string_lossy().as_ref())?);
        assert!(!roots.add(first.to_string_lossy().as_ref())?);
        let second = dir.path().join("work");
        assert!(roots.add(second.to_string_lossy().as_ref())?);
        roots.save_to(&path)?;

        let loaded = Roots::load_from(&path);
        assert_eq!(loaded.entries(), &[first.clone(), second.clone()]);
        let mut loaded = loaded;
        assert!(loaded.remove(0));
        assert!(!loaded.remove(9));
        loaded.save_to(&path)?;
        assert_eq!(Roots::load_from(&path).entries(), &[second]);
        assert!(normalize_folder("relative/path").is_ok());
        assert!(normalize_folder("").is_err());
        Ok(())
    }

    #[test]
    fn defaults_to_the_marked_then_first_saved_folder_and_lists_the_launch_folder_last()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("roots.json");
        let mut roots = Roots::load_from(&path);
        assert_eq!(roots.default_folder(), starting_folder());
        assert_eq!(roots.candidates(), vec![starting_folder()]);

        let first = dir.path().join("projects");
        let second = dir.path().join("work");
        roots.add(first.to_string_lossy().as_ref())?;
        roots.add(second.to_string_lossy().as_ref())?;
        assert_eq!(roots.default_folder(), first);
        assert_eq!(
            roots.candidates(),
            vec![first.clone(), second.clone(), starting_folder()]
        );

        assert!(roots.set_preferred(1));
        assert!(!roots.set_preferred(1));
        assert!(!roots.set_preferred(9));
        roots.save()?;
        let mut loaded = Roots::load_from(&path);
        assert_eq!(loaded.preferred(), Some(second.as_path()));
        assert_eq!(loaded.default_folder(), second);

        assert!(loaded.remove(1));
        assert_eq!(loaded.preferred(), None);
        assert_eq!(loaded.default_folder(), first);
        Ok(())
    }

    #[test]
    fn reports_malformed_configuration_as_an_empty_list() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("roots.json");
        std::fs::write(&path, b"{not json")?;
        assert!(Roots::load_from(&path).entries().is_empty());
        Ok(())
    }
}
