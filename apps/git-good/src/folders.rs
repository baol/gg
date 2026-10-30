//! Development base folders offered as clone destinations, kept under the
//! `clone_folders` key of the settings file. A legacy `gg/roots.json` next to
//! it is read once when the key is absent.
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
        let stored = crate::settings::read::<Stored>(path, KEY)
            .or_else(|| {
                let legacy = path.with_file_name("roots.json");
                std::fs::read(legacy)
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<Stored>(&bytes).ok())
            })
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
        let stored = Stored {
            roots: self.entries.clone(),
            preferred: self.preferred.clone(),
        };
        crate::settings::write(path, KEY, &stored)
    }

    /// Move the folder at `index` one place up (`true`) or down.
    pub fn move_entry(&mut self, index: usize, up: bool) -> bool {
        let other = if up {
            index.checked_sub(1)
        } else {
            (index + 1 < self.entries.len()).then_some(index + 1)
        };
        match other {
            Some(other) if index < self.entries.len() => {
                self.entries.swap(index, other);
                true
            }
            _ => false,
        }
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
            // Removing the preferred folder makes the first one preferred.
            if self.preferred.as_ref() == Some(&removed) {
                self.preferred = self.entries.first().cloned();
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
    if !expanded.is_absolute() {
        return Err("Folder must be an absolute path.".into());
    }
    Ok(expanded)
}

const KEY: &str = "clone_folders";

pub fn settings_path() -> Option<PathBuf> {
    crate::settings::path()
}

/// Repositories directly inside `folder`, or `None` when it is missing.
pub fn repository_count(folder: &Path) -> Option<usize> {
    let entries = std::fs::read_dir(folder).ok()?;
    Some(
        entries
            .flatten()
            .filter(|entry| entry.path().join(".git").exists())
            .count(),
    )
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
        assert_eq!(
            normalize_folder("relative/path"),
            Err("Folder must be an absolute path.".into())
        );
        assert_eq!(normalize_folder("  "), Err("Folder is empty.".into()));
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
        assert_eq!(
            loaded.preferred(),
            Some(first.as_path()),
            "removing the preferred folder prefers the first"
        );
        assert_eq!(loaded.default_folder(), first);
        Ok(())
    }

    #[test]
    fn migrates_roots_json_and_keeps_other_settings() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let first = dir.path().join("projects");
        let second = dir.path().join("work");
        std::fs::write(
            dir.path().join("roots.json"),
            serde_json::to_vec(&serde_json::json!({
                "roots": [first, second],
                "preferred": second,
            }))?,
        )?;
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"theme": "light"}"#)?;
        let mut roots = Roots::load_from(&path);
        assert_eq!(roots.entries(), &[first.clone(), second.clone()]);
        assert_eq!(
            roots.preferred(),
            Some(second.as_path()),
            "the default survives"
        );
        assert!(roots.move_entry(1, true));
        assert!(!roots.move_entry(0, true));
        roots.save_to(&path)?;
        // The settings file now wins over the legacy file.
        std::fs::write(dir.path().join("roots.json"), b"{}")?;
        let loaded = Roots::load_from(&path);
        assert_eq!(loaded.entries(), &[second.clone(), first.clone()]);
        assert_eq!(
            crate::settings::read::<crate::settings::Theme>(&path, "theme"),
            Some(crate::settings::Theme::Light)
        );
        Ok(())
    }

    #[test]
    fn counts_repositories_and_reports_missing_folders() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir_all(dir.path().join("one/.git"))?;
        std::fs::create_dir_all(dir.path().join("two/.git"))?;
        std::fs::create_dir_all(dir.path().join("plain"))?;
        assert_eq!(repository_count(dir.path()), Some(2));
        assert_eq!(repository_count(&dir.path().join("missing")), None);
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
