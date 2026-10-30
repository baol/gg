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

/// How image and binary files are compared.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageLayout {
    #[default]
    SideBySide,
    Swipe,
    Onion,
}

impl ImageLayout {
    pub const ALL: [Self; 3] = [Self::SideBySide, Self::Swipe, Self::Onion];

    pub fn label(self) -> &'static str {
        match self {
            Self::SideBySide => "Side by side",
            Self::Swipe => "Swipe",
            Self::Onion => "Onion skin",
        }
    }
}

/// The presentation a newly opened reader starts in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaderMode {
    #[default]
    Diff,
    Full,
}

/// Whether Local Changes starts with **Sign** checked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignCommits {
    /// Follow `commit.gpgSign`.
    #[default]
    Git,
    /// Checked whenever signing is configured and ready.
    Always,
    Never,
}

impl SignCommits {
    pub const ALL: [Self; 3] = [Self::Git, Self::Always, Self::Never];

    pub fn label(self) -> &'static str {
        match self {
            Self::Git => "Follow Git config (commit.gpgSign)",
            Self::Always => "Always, when signing is configured",
            Self::Never => "Never",
        }
    }
}

/// Starting values readers take from the window: stored per egui context
/// (so each window and each test has its own), seeded from the preferences.
pub fn image_layout(ctx: &eframe::egui::Context) -> ImageLayout {
    ctx.data(|data| data.get_temp(image_layout_id()))
        .unwrap_or_default()
}

pub fn set_image_layout(ctx: &eframe::egui::Context, layout: ImageLayout) {
    ctx.data_mut(|data| data.insert_temp(image_layout_id(), layout));
}

fn image_layout_id() -> eframe::egui::Id {
    eframe::egui::Id::unique("gg-image-layout")
}

pub fn reader_mode(ctx: &eframe::egui::Context) -> ReaderMode {
    ctx.data(|data| data.get_temp(reader_mode_id()))
        .unwrap_or_default()
}

pub fn set_reader_mode(ctx: &eframe::egui::Context, mode: ReaderMode) {
    ctx.data_mut(|data| data.insert_temp(reader_mode_id(), mode));
}

fn reader_mode_id() -> eframe::egui::Id {
    eframe::egui::Id::unique("gg-reader-mode")
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
    /// Reader presentation for newly opened files.
    pub reader_mode: ReaderMode,
    /// Image comparison layout; remembered from the last choice.
    pub image_layout: ImageLayout,
    pub sign_commits: SignCommits,
    /// Commit details pane in wide History; remembered.
    pub details_visible: bool,
    /// Remembered from the last submitted stash form.
    pub stash_include_untracked: bool,
    pub stash_keep_index: bool,
    /// Remembered from the last submitted merge.
    pub merge_no_ff: bool,
    /// Remembered from the last new branch created at a commit.
    pub branch_switch: bool,
    /// Fetch also prunes remote-tracking branches deleted on the remote.
    pub fetch_prune: bool,
    path: Option<PathBuf>,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            density: Density::default(),
            network_avatars: true,
            rebase_autostash: false,
            reader_mode: ReaderMode::default(),
            image_layout: ImageLayout::default(),
            sign_commits: SignCommits::default(),
            details_visible: true,
            stash_include_untracked: false,
            stash_keep_index: false,
            merge_no_ff: false,
            branch_switch: true,
            fetch_prune: false,
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
            reader_mode: read(path, "reader_mode").unwrap_or(defaults.reader_mode),
            image_layout: read(path, "image_layout").unwrap_or(defaults.image_layout),
            sign_commits: read(path, "sign_commits").unwrap_or(defaults.sign_commits),
            details_visible: read(path, "details_visible").unwrap_or(defaults.details_visible),
            stash_include_untracked: read(path, "stash_include_untracked")
                .unwrap_or(defaults.stash_include_untracked),
            stash_keep_index: read(path, "stash_keep_index").unwrap_or(defaults.stash_keep_index),
            merge_no_ff: read(path, "merge_no_ff").unwrap_or(defaults.merge_no_ff),
            branch_switch: read(path, "branch_switch").unwrap_or(defaults.branch_switch),
            fetch_prune: read(path, "fetch_prune").unwrap_or(defaults.fetch_prune),
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
        write(path, "rebase_autostash", &self.rebase_autostash)?;
        write(path, "reader_mode", &self.reader_mode)?;
        write(path, "image_layout", &self.image_layout)?;
        write(path, "sign_commits", &self.sign_commits)?;
        write(path, "details_visible", &self.details_visible)?;
        write(
            path,
            "stash_include_untracked",
            &self.stash_include_untracked,
        )?;
        write(path, "stash_keep_index", &self.stash_keep_index)?;
        write(path, "merge_no_ff", &self.merge_no_ff)?;
        write(path, "branch_switch", &self.branch_switch)?;
        write(path, "fetch_prune", &self.fetch_prune)
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
        prefs.reader_mode = ReaderMode::Full;
        prefs.image_layout = ImageLayout::Onion;
        prefs.sign_commits = SignCommits::Never;
        prefs.details_visible = false;
        prefs.stash_include_untracked = true;
        prefs.stash_keep_index = true;
        prefs.merge_no_ff = true;
        prefs.branch_switch = false;
        prefs.fetch_prune = true;
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
        assert!(prefs.branch_switch, "absent keys keep their defaults");
        assert!(prefs.details_visible);
        assert_eq!(prefs.sign_commits, SignCommits::Git);
        std::fs::write(&path, b"{not json")?;
        assert_eq!(Prefs::load_from(&path).density, Density::Compact);
        Ok(())
    }
}
