//! Preferences kept across launches in `gg/settings.json` under the OS
//! configuration directory.
//!
//! Each preference is its own top-level key and is read on its own, so a
//! malformed or unknown value resets only that preference. Writing one key
//! keeps every other key in the file, including ones a newer gg wrote.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};

/// Interface zoom choices, in percent.
pub const ZOOMS: [u16; 3] = [100, 110, 125];
/// Code font sizes, in tenths of a pixel.
pub const CODE_FONTS: [u16; 3] = [125, 130, 140];
/// Subject length guides, in characters; 0 is off.
pub const SUBJECT_GUIDES: [u16; 3] = [72, 50, 0];

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

/// Most repositories listed under Repository › Open recent.
pub const RECENT_LIMIT: usize = 10;

/// Repositories opened before, newest first.
pub fn recent() -> Vec<PathBuf> {
    path()
        .and_then(|path| read::<Vec<PathBuf>>(&path, "recent_repositories"))
        .unwrap_or_default()
}

/// Put `repository` first in `list`, without duplicates, bounded.
pub fn push_recent(list: &mut Vec<PathBuf>, repository: PathBuf) {
    list.retain(|entry| entry != &repository);
    list.insert(0, repository);
    list.truncate(RECENT_LIMIT);
}

/// Save the recent list; tests never write the user's file.
pub fn save_recent(list: &[PathBuf]) -> Result<(), String> {
    if cfg!(test) {
        return Ok(());
    }
    let path = path().ok_or("No configuration folder is available for preferences.")?;
    write(&path, "recent_repositories", &list)
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
    /// In the mockup's order: System, Dark, Light.
    pub const ALL: [Self; 3] = [Self::System, Self::Dark, Self::Light];

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

/// How Diff mode lays out a text comparison.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffLayout {
    /// One column with both line numbers.
    #[default]
    Inline,
    /// Before on the left and After on the right.
    SideBySide,
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

/// The commit subject length guide (0 is off); 72 until Preferences sets it.
pub fn subject_guide(ctx: &eframe::egui::Context) -> u16 {
    ctx.data(|data| data.get_temp(subject_guide_id()))
        .unwrap_or(72)
}

pub fn set_subject_guide(ctx: &eframe::egui::Context, guide: u16) {
    ctx.data_mut(|data| data.insert_temp(subject_guide_id(), guide));
}

fn subject_guide_id() -> eframe::egui::Id {
    eframe::egui::Id::unique("gg-subject-guide")
}

/// The diff layout every reader in the window follows.
pub fn diff_layout(ctx: &eframe::egui::Context) -> DiffLayout {
    ctx.data(|data| data.get_temp(diff_layout_id()))
        .unwrap_or_default()
}

pub fn set_diff_layout(ctx: &eframe::egui::Context, layout: DiffLayout) {
    ctx.data_mut(|data| data.insert_temp(diff_layout_id(), layout));
}

fn diff_layout_id() -> eframe::egui::Id {
    eframe::egui::Id::unique("gg-diff-layout")
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
    /// Inline or side-by-side Diff; remembered from the last choice.
    pub diff_layout: DiffLayout,
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
    /// Command for Open in editor, such as `code --wait {file}`; empty opens
    /// the file with the system's default app.
    pub editor: String,
    /// Interface zoom in percent: 100, 110, or 125.
    pub zoom: u16,
    /// Code font size in the reader, in tenths of a pixel: 125, 130, or 140.
    pub code_font: u16,
    /// Create tag starts with Sign checked.
    pub sign_tags: bool,
    /// Commit subject length guide in characters; 0 is off.
    pub subject_guide: u16,
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
            diff_layout: DiffLayout::default(),
            image_layout: ImageLayout::default(),
            sign_commits: SignCommits::default(),
            details_visible: true,
            stash_include_untracked: false,
            stash_keep_index: false,
            merge_no_ff: false,
            branch_switch: true,
            fetch_prune: false,
            editor: String::new(),
            zoom: 100,
            code_font: 125,
            sign_tags: false,
            subject_guide: 72,
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
            diff_layout: read(path, "diff_layout").unwrap_or(defaults.diff_layout),
            image_layout: read(path, "image_layout").unwrap_or(defaults.image_layout),
            sign_commits: read(path, "sign_commits").unwrap_or(defaults.sign_commits),
            details_visible: read(path, "details_visible").unwrap_or(defaults.details_visible),
            stash_include_untracked: read(path, "stash_include_untracked")
                .unwrap_or(defaults.stash_include_untracked),
            stash_keep_index: read(path, "stash_keep_index").unwrap_or(defaults.stash_keep_index),
            merge_no_ff: read(path, "merge_no_ff").unwrap_or(defaults.merge_no_ff),
            branch_switch: read(path, "branch_switch").unwrap_or(defaults.branch_switch),
            fetch_prune: read(path, "fetch_prune").unwrap_or(defaults.fetch_prune),
            editor: read(path, "editor").unwrap_or(defaults.editor),
            zoom: read(path, "zoom")
                .filter(|zoom| ZOOMS.contains(zoom))
                .unwrap_or(defaults.zoom),
            code_font: read(path, "code_font")
                .filter(|size| CODE_FONTS.contains(size))
                .unwrap_or(defaults.code_font),
            sign_tags: read(path, "sign_tags").unwrap_or(defaults.sign_tags),
            subject_guide: read(path, "subject_guide")
                .filter(|guide| SUBJECT_GUIDES.contains(guide))
                .unwrap_or(defaults.subject_guide),
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
        write(path, "diff_layout", &self.diff_layout)?;
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
        write(path, "fetch_prune", &self.fetch_prune)?;
        write(path, "editor", &self.editor)?;
        write(path, "zoom", &self.zoom)?;
        write(path, "code_font", &self.code_font)?;
        write(path, "sign_tags", &self.sign_tags)?;
        write(path, "subject_guide", &self.subject_guide)
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
    fn recent_repositories_move_to_the_front_without_duplicates() {
        let mut list = Vec::new();
        for name in ["a", "b", "a"] {
            push_recent(&mut list, PathBuf::from(name));
        }
        assert_eq!(list, [PathBuf::from("a"), PathBuf::from("b")]);
        for index in 0..20 {
            push_recent(&mut list, PathBuf::from(format!("r{index}")));
        }
        assert_eq!(list.len(), RECENT_LIMIT);
    }

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
        prefs.diff_layout = DiffLayout::SideBySide;
        prefs.image_layout = ImageLayout::Onion;
        prefs.sign_commits = SignCommits::Never;
        prefs.details_visible = false;
        prefs.stash_include_untracked = true;
        prefs.stash_keep_index = true;
        prefs.merge_no_ff = true;
        prefs.branch_switch = false;
        prefs.fetch_prune = true;
        prefs.editor = "code --wait {file}".into();
        prefs.zoom = 125;
        prefs.code_font = 140;
        prefs.sign_tags = true;
        prefs.subject_guide = 50;
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
        assert_eq!(
            prefs.diff_layout,
            DiffLayout::Inline,
            "inline is the default"
        );
        std::fs::write(&path, b"{not json")?;
        assert_eq!(Prefs::load_from(&path).density, Density::Compact);
        Ok(())
    }
}
