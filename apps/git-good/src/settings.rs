//! Preferences kept across launches in `gg/settings.json` under the OS
//! configuration directory.
//!
//! Each preference is its own top-level key and is read on its own, so a
//! malformed or unknown value resets only that preference. Writing one key
//! keeps every other key in the file, including ones a newer gg wrote.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};

/// `gg/settings.json` under the OS configuration directory.
pub fn path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("settings.json"))
}

/// The configuration folder for gg, such as `~/.config/gg`.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(folder) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(folder).join("gg"));
    }
    if cfg!(windows) {
        return std::env::var_os("APPDATA").map(|folder| PathBuf::from(folder).join("gg"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config").join("gg"))
}

fn object(path: &Path) -> serde_json::Map<String, serde_json::Value> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|value| match value {
            serde_json::Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default()
}

/// One key of the settings file, or `None` when it is absent or malformed.
pub fn read<T: DeserializeOwned>(path: &Path, key: &str) -> Option<T> {
    object(path)
        .remove(key)
        .and_then(|value| serde_json::from_value(value).ok())
}

/// Replace one key of the settings file, keeping the others.
pub fn write<T: Serialize>(path: &Path, key: &str, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Create {}: {error}", parent.display()))?;
    }
    let mut map = object(path);
    map.insert(
        key.to_string(),
        serde_json::to_value(value).map_err(|error| format!("Encode {key}: {error}"))?,
    );
    let bytes = serde_json::to_vec_pretty(&serde_json::Value::Object(map))
        .map_err(|error| format!("Encode settings: {error}"))?;
    std::fs::write(path, bytes)
        .map_err(|error| format!("Write settings to {}: {error}", path.display()))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Follow the operating system.
    System,
    Light,
    #[default]
    Dark,
}

impl Theme {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    #[default]
    Compact,
    Comfortable,
}

/// Preferences other than the clone folders, which `folders::Roots` keeps
/// under its own key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prefs {
    pub theme: Theme,
    pub density: Density,
    /// Fetch avatars from the network; off uses cached avatars and initials.
    pub network_avatars: bool,
    /// Rebase dialogs start with "stash working changes first" checked.
    pub rebase_autostash: bool,
    path: Option<PathBuf>,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            density: Density::default(),
            network_avatars: true,
            rebase_autostash: false,
            path: None,
        }
    }
}

impl Prefs {
    pub fn load() -> Self {
        path()
            .map(|path| Self::load_from(&path))
            .unwrap_or_default()
    }

    pub fn load_from(path: &Path) -> Self {
        let defaults = Self::default();
        Self {
            theme: read(path, "theme").unwrap_or(defaults.theme),
            density: read(path, "density").unwrap_or(defaults.density),
            network_avatars: read(path, "network_avatars").unwrap_or(defaults.network_avatars),
            rebase_autostash: read(path, "rebase_autostash").unwrap_or(defaults.rebase_autostash),
            path: Some(path.to_path_buf()),
        }
    }

    /// Save every preference to the file it was loaded from.
    pub fn save(&self) -> Result<(), String> {
        let Some(path) = self.path.as_deref() else {
            return Err("No configuration folder is available for preferences.".into());
        };
        write(path, "theme", &self.theme)?;
        write(path, "density", &self.density)?;
        write(path, "network_avatars", &self.network_avatars)?;
        write(path, "rebase_autostash", &self.rebase_autostash)
    }

    /// The theme to show: `GG_THEME` (debug builds) wins over the saved one
    /// without rewriting it.
    pub fn effective_theme(&self) -> Theme {
        match std::env::var("GG_THEME").ok().as_deref() {
            Some("light") if cfg!(debug_assertions) => Theme::Light,
            Some("dark") if cfg!(debug_assertions) => Theme::Dark,
            _ => self.theme,
        }
    }

    /// Whether avatars may use the network: `GG_OFFLINE` wins.
    pub fn effective_network_avatars(&self) -> bool {
        self.network_avatars && std::env::var_os("GG_OFFLINE").is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_keeps_unknown_keys() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("gg/settings.json");
        std::fs::create_dir_all(dir.path().join("gg"))?;
        std::fs::write(&path, br#"{"future_option": 7}"#)?;
        let mut prefs = Prefs::load_from(&path);
        assert_eq!(
            prefs,
            Prefs {
                path: Some(path.clone()),
                ..Prefs::default()
            }
        );
        prefs.theme = Theme::Light;
        prefs.density = Density::Comfortable;
        prefs.network_avatars = false;
        prefs.rebase_autostash = true;
        prefs.save()?;
        assert_eq!(Prefs::load_from(&path), prefs);
        assert_eq!(read::<u32>(&path, "future_option"), Some(7));
        Ok(())
    }

    #[test]
    fn a_malformed_value_resets_only_that_preference() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            br#"{"theme": "purple", "density": "comfortable", "network_avatars": "yes"}"#,
        )?;
        let prefs = Prefs::load_from(&path);
        assert_eq!(prefs.theme, Theme::Dark);
        assert_eq!(prefs.density, Density::Comfortable);
        assert!(prefs.network_avatars);
        std::fs::write(&path, b"{not json")?;
        assert_eq!(Prefs::load_from(&path).density, Density::Compact);
        Ok(())
    }
}
