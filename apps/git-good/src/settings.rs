//! Preferences kept across launches in `gg/settings.json` under the OS
//! configuration directory.
//!
//! Each preference is its own top-level key and is read on its own, so a
//! malformed or unknown value resets only that preference. Writing one key
//! keeps every other key in the file, including ones a newer gg wrote.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};

/// Where gg looks for an author's avatar, tried in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvatarSource {
    Github,
    Gitlab,
    Gravatar,
    Libravatar,
}

impl AvatarSource {
    pub const ALL: [Self; 4] = [Self::Github, Self::Gitlab, Self::Gravatar, Self::Libravatar];

    pub fn label(self) -> &'static str {
        match self {
            Self::Github => "GitHub",
            Self::Gitlab => "GitLab",
            Self::Gravatar => "Gravatar",
            Self::Libravatar => "Libravatar",
        }
    }

    /// What each source learns, for Preferences.
    pub fn note(self) -> &'static str {
        match self {
            Self::Github => "users.noreply.github.com addresses only; sends the GitHub user name",
            Self::Gitlab => "sends the email address to gitlab.com",
            Self::Gravatar => "sends a hash of the email address",
            Self::Libravatar => "sends a hash of the email address",
        }
    }
}

/// What Pull does after fetching the upstream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullStrategy {
    #[default]
    Rebase,
    Merge,
    FastForward,
}

impl PullStrategy {
    pub const ALL: [Self; 3] = [Self::Rebase, Self::Merge, Self::FastForward];

    pub fn label(self) -> &'static str {
        match self {
            Self::Rebase => "Rebase",
            Self::Merge => "Merge",
            Self::FastForward => "Fast-forward only",
        }
    }

    /// The menu and toolbar label: `Pull (rebase)`.
    pub fn menu_label(self) -> &'static str {
        match self {
            Self::Rebase => "rebase",
            Self::Merge => "merge",
            Self::FastForward => "fast-forward",
        }
    }
}

/// Backup retention choices in days; 0 keeps them forever.
pub const BACKUP_DAYS: [u16; 3] = [30, 90, 0];

/// Interface zoom choices, in percent.
pub const ZOOMS: [u16; 3] = [100, 110, 125];
/// Code font sizes, in tenths of a pixel.
pub const CODE_FONTS: [u16; 3] = [125, 130, 140];
/// Subject length guides, in characters; 0 is off.
pub const SUBJECT_GUIDES: [u16; 3] = [72, 50, 0];
/// Unchanged lines around each change in Diff mode.
pub const DIFF_CONTEXTS: [u16; 3] = [3, 5, 10];
pub use gg_engine::diff::Whitespace;

/// Whitespace choices in the mockup's order, with their labels.
pub const WHITESPACE: [(Whitespace, &str); 3] = [
    (Whitespace::All, "Show all changes"),
    (Whitespace::IgnoreAtEol, "Ignore at line end"),
    (Whitespace::IgnoreAll, "Ignore all"),
];

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

type Map = serde_json::Map<String, serde_json::Value>;

fn parse(bytes: &[u8]) -> Option<Map> {
    match serde_json::from_slice::<serde_json::Value>(bytes).ok()? {
        serde_json::Value::Object(map) => Some(map),
        _ => None,
    }
}

fn object(path: &Path) -> Map {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| parse(&bytes))
        .unwrap_or_default()
}

/// Change the settings file as it is on disk now, keeping every key `change`
/// leaves alone, and replace it in one step: written to a temporary file in
/// the same folder, then renamed over it, so another gg window or a crash
/// never sees half a file. A file that cannot be read is kept beside it as
/// `settings.json.unreadable` instead of being overwritten.
fn update(path: &Path, change: impl FnOnce(&mut Map)) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Create {}: {error}", parent.display()))?;
    }
    let mut map = match std::fs::read(path) {
        Ok(bytes) => match parse(&bytes) {
            Some(map) => map,
            None => {
                let aside = path.with_extension("json.unreadable");
                std::fs::write(&aside, &bytes)
                    .map_err(|error| format!("Keep {}: {error}", aside.display()))?;
                Map::new()
            }
        },
        Err(_) => Map::new(),
    };
    change(&mut map);
    let bytes = serde_json::to_vec_pretty(&serde_json::Value::Object(map))
        .map_err(|error| format!("Encode settings: {error}"))?;
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    std::fs::write(&temporary, bytes)
        .and_then(|()| std::fs::rename(&temporary, path))
        .map_err(|error| {
            let _ = std::fs::remove_file(&temporary);
            format!("Write settings to {}: {error}", path.display())
        })
}

/// One key of the settings file, or `None` when it is absent or malformed.
pub fn read<T: DeserializeOwned>(path: &Path, key: &str) -> Option<T> {
    object(path)
        .remove(key)
        .and_then(|value| serde_json::from_value(value).ok())
}

/// Replace one key of the settings file, keeping the others.
pub fn write<T: Serialize>(path: &Path, key: &str, value: &T) -> Result<(), String> {
    let value = serde_json::to_value(value).map_err(|error| format!("Encode {key}: {error}"))?;
    update(path, |map| {
        map.insert(key.to_string(), value);
    })
}

/// Most recent repositories kept: the welcome list, Repository › Open
/// recent, and the Open… dialog all show up to this many.
pub const RECENT_LIMIT: usize = 10;

/// A repository opened before and when it was last opened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Recent {
    pub path: PathBuf,
    /// Seconds since the Unix epoch; `None` for entries saved before gg
    /// recorded the time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opened: Option<u64>,
}

impl Recent {
    pub fn new(path: PathBuf) -> Self {
        Self { path, opened: None }
    }
}

/// `recent_repositories` entries: `{"path", "opened"}` objects, or the
/// plain path strings older versions wrote.
impl<'de> Deserialize<'de> for Recent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Stored {
            Path(PathBuf),
            Entry {
                path: PathBuf,
                #[serde(default)]
                opened: Option<u64>,
            },
        }
        Ok(match Stored::deserialize(deserializer)? {
            Stored::Path(path) => Recent::new(path),
            Stored::Entry { path, opened } => Recent { path, opened },
        })
    }
}

/// Seconds since the Unix epoch, now.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// Repositories opened before, newest first.
pub fn recent() -> Vec<Recent> {
    path().map(|path| recent_from(&path)).unwrap_or_default()
}

pub fn recent_from(path: &Path) -> Vec<Recent> {
    read::<Vec<Recent>>(path, "recent_repositories").unwrap_or_default()
}

/// Put `repository` first in `list`, opened at `opened`, without
/// duplicates, bounded.
pub fn push_recent(list: &mut Vec<Recent>, repository: PathBuf, opened: u64) {
    list.retain(|entry| entry.path != repository);
    list.insert(
        0,
        Recent {
            path: repository,
            opened: Some(opened),
        },
    );
    list.truncate(RECENT_LIMIT);
}

/// Save the recent list; tests never write the user's file.
/// Save this window's recent list over the one on disk, which other gg
/// windows may have changed since `base` was read: their additions and
/// removals are kept along with this window's. `list` and `base` both
/// become the merged list.
pub fn save_recent(list: &mut Vec<Recent>, base: &mut Vec<Recent>) -> Result<(), String> {
    if cfg!(test) {
        *base = list.clone();
        return Ok(());
    }
    let path = path().ok_or("No configuration folder is available for preferences.")?;
    save_recent_to(&path, list, base)
}

pub fn save_recent_to(
    path: &Path,
    list: &mut Vec<Recent>,
    base: &mut Vec<Recent>,
) -> Result<(), String> {
    let mut merged = Vec::new();
    update(path, |map| {
        let disk: Vec<Recent> = map
            .get("recent_repositories")
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or_default();
        merged = merge_recent(&disk, base, list);
        if let Ok(value) = serde_json::to_value(&merged) {
            map.insert("recent_repositories".into(), value);
        }
    })?;
    *list = merged.clone();
    *base = merged;
    Ok(())
}

/// Three-way merge of recent lists by path: an entry this window removed
/// stays removed unless another window opened it since; one another window
/// removed stays removed unless this window opened it since. Newest first.
fn merge_recent(disk: &[Recent], base: &[Recent], ours: &[Recent]) -> Vec<Recent> {
    let find = |list: &[Recent], path: &Path| list.iter().find(|entry| entry.path == path).cloned();
    let mut merged: Vec<Recent> = Vec::new();
    for entry in ours.iter().chain(disk) {
        if merged.iter().any(|kept| kept.path == entry.path) {
            continue;
        }
        let (mine, theirs, was) = (
            find(ours, &entry.path),
            find(disk, &entry.path),
            find(base, &entry.path),
        );
        let keep = match (mine, theirs) {
            (Some(mine), None) if was.as_ref() == Some(&mine) => None,
            (Some(mine), Some(theirs)) => Some(if theirs.opened > mine.opened {
                theirs
            } else {
                mine
            }),
            (Some(mine), None) => Some(mine),
            (None, Some(theirs)) => match was {
                Some(was) if theirs.opened <= was.opened => None,
                _ => Some(theirs),
            },
            (None, None) => None,
        };
        merged.extend(keep);
    }
    // Stable: entries without a time keep their order at the end.
    merged.sort_by_key(|entry| std::cmp::Reverse(entry.opened));
    merged.truncate(RECENT_LIMIT);
    merged
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
    /// Hexadecimal rows of both sides.
    Hex,
}

impl ImageLayout {
    pub const ALL: [Self; 4] = [Self::SideBySide, Self::Swipe, Self::Onion, Self::Hex];

    pub fn label(self) -> &'static str {
        match self {
            Self::SideBySide => "Side by side",
            Self::Swipe => "Swipe",
            Self::Onion => "Onion skin",
            Self::Hex => "Hex",
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

/// Whether commits run the pre-commit and commit-msg hooks; on until
/// Preferences sets it.
pub fn run_hooks(ctx: &eframe::egui::Context) -> bool {
    ctx.data(|data| data.get_temp(run_hooks_id()))
        .unwrap_or(true)
}

pub fn set_run_hooks(ctx: &eframe::egui::Context, run: bool) {
    ctx.data_mut(|data| data.insert_temp(run_hooks_id(), run));
}

fn run_hooks_id() -> eframe::egui::Id {
    eframe::egui::Id::unique("gg-run-hooks")
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

/// Windows narrower than this use the narrow layouts (narrow proposal).
pub const NARROW_WINDOW: f32 = 1040.0;

/// The layout readers show: the preference in wide windows; below
/// [`NARROW_WINDOW`] Inline unless Side by side was chosen there this
/// session (narrow proposal).
pub fn reader_layout(ctx: &eframe::egui::Context) -> DiffLayout {
    if ctx.content_rect().width() < NARROW_WINDOW {
        ctx.data(|data| data.get_temp(narrow_layout_id()))
            .unwrap_or(DiffLayout::Inline)
    } else {
        diff_layout(ctx)
    }
}

/// A layout chosen in a reader: the preference in wide windows, the
/// session's narrow layout below [`NARROW_WINDOW`].
pub fn choose_reader_layout(ctx: &eframe::egui::Context, layout: DiffLayout) {
    if ctx.content_rect().width() < NARROW_WINDOW {
        ctx.data_mut(|data| data.insert_temp(narrow_layout_id(), layout));
    } else {
        set_diff_layout(ctx, layout);
    }
}

/// The session's narrow layout, as if chosen in a narrow reader.
#[cfg(test)]
pub fn set_narrow_layout(ctx: &eframe::egui::Context, layout: DiffLayout) {
    ctx.data_mut(|data| data.insert_temp(narrow_layout_id(), layout));
}

fn narrow_layout_id() -> eframe::egui::Id {
    eframe::egui::Id::unique("gg-narrow-diff-layout")
}

/// Context lines and whitespace handling every Diff in the window uses.
pub fn diff_options(ctx: &eframe::egui::Context) -> gg_engine::diff::Options {
    ctx.data(|data| data.get_temp(diff_options_id()))
        .unwrap_or_default()
}

pub fn set_diff_options(ctx: &eframe::egui::Context, options: gg_engine::diff::Options) {
    ctx.data_mut(|data| data.insert_temp(diff_options_id(), options));
}

fn diff_options_id() -> eframe::egui::Id {
    eframe::egui::Id::unique("gg-diff-options")
}

/// Whether changed Diff lines show tabs as `→` and trailing spaces as `·`;
/// on until Preferences sets it.
pub fn diff_invisibles(ctx: &eframe::egui::Context) -> bool {
    ctx.data(|data| data.get_temp(diff_invisibles_id()))
        .unwrap_or(true)
}

pub fn set_diff_invisibles(ctx: &eframe::egui::Context, show: bool) {
    ctx.data_mut(|data| data.insert_temp(diff_invisibles_id(), show));
}

fn diff_invisibles_id() -> eframe::egui::Id {
    eframe::egui::Id::unique("gg-diff-invisibles")
}

/// Whether the reader wraps long lines instead of scrolling sideways.
pub fn diff_wrap(ctx: &eframe::egui::Context) -> bool {
    ctx.data(|data| data.get_temp(diff_wrap_id()))
        .unwrap_or(false)
}

pub fn set_diff_wrap(ctx: &eframe::egui::Context, wrap: bool) {
    ctx.data_mut(|data| data.insert_temp(diff_wrap_id(), wrap));
}

fn diff_wrap_id() -> eframe::egui::Id {
    eframe::egui::Id::unique("gg-diff-wrap")
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
    /// Commits run the pre-commit and commit-msg hooks.
    pub run_hooks: bool,
    /// Create tag starts with Sign checked.
    pub sign_tags: bool,
    /// Commit subject length guide in characters; 0 is off.
    pub subject_guide: u16,
    pub pull_strategy: PullStrategy,
    /// Clone fills the folder name from the URL (`~/dev/gg`).
    pub clone_named_after_repo: bool,
    /// Drop… and Reset current branch to here… ask first; off runs them at
    /// once (Soft and Mixed reset as chosen; Hard still asks), still saving
    /// a backup.
    pub confirm_destructive: bool,
    /// Delete gg's private backups older than this many days; 0 keeps
    /// them forever (the default, so upgrading never deletes backups).
    pub backup_days: u16,
    /// After rewriting a branch that has an upstream, offer Push with
    /// force-with-lease.
    pub offer_force_lease: bool,
    /// Avatar sources that are on; tried in [`AvatarSource::ALL`] order.
    pub avatar_sources: Vec<AvatarSource>,
    /// History rows: author avatars, tag pills, and relative dates.
    pub history_avatars: bool,
    pub history_tags: bool,
    pub relative_dates: bool,
    /// Unchanged lines around each change in Diff mode: 3, 5, or 10.
    pub diff_context: u16,
    /// Which whitespace differences Diff mode shows.
    pub diff_whitespace: Whitespace,
    /// Changed lines show tabs and trailing spaces.
    pub show_invisibles: bool,
    /// The reader wraps long lines instead of scrolling sideways.
    pub wrap_lines: bool,
    /// Windows and Linux: keep the system's title bar instead of gg's menu
    /// bar being the title bar; read at launch. `None` follows the desktop:
    /// on for Windows and KDE Plasma, off elsewhere (`title_bar_default`).
    pub use_system_title_bar: Option<bool>,
    /// Selection, links and primary buttons in the desktop's accent color.
    pub desktop_accent: bool,
    /// The desktop's interface and monospace fonts (at launch) and its text
    /// size.
    pub desktop_fonts: bool,
    /// Keys changed in Preferences › Keyboard: command id → chord
    /// (`keymap.rs`); the defaults are not stored.
    pub keys: std::collections::BTreeMap<String, String>,
    path: Option<PathBuf>,
    /// The values as last loaded or saved, to tell this window's changes
    /// from another window's.
    saved: Saved,
}

/// Compares equal to any other, so it never makes two `Prefs` differ.
#[derive(Clone, Debug, Default)]
struct Saved(Map);

impl PartialEq for Saved {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Saved {}

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
            // On by default: "stash my changes" usually means new files too.
            stash_include_untracked: true,
            stash_keep_index: false,
            merge_no_ff: false,
            branch_switch: true,
            fetch_prune: false,
            editor: String::new(),
            zoom: 100,
            code_font: 125,
            sign_tags: false,
            run_hooks: true,
            subject_guide: 72,
            pull_strategy: PullStrategy::default(),
            clone_named_after_repo: true,
            offer_force_lease: true,
            backup_days: 0,
            confirm_destructive: true,
            avatar_sources: vec![AvatarSource::Github, AvatarSource::Gravatar],
            // Off by default, as in the History mockup (plain author names).
            history_avatars: false,
            history_tags: true,
            relative_dates: false,
            diff_context: 3,
            diff_whitespace: Whitespace::All,
            show_invisibles: true,
            wrap_lines: false,
            use_system_title_bar: None,
            desktop_accent: true,
            desktop_fonts: true,
            keys: Default::default(),
            path: None,
            saved: Saved::default(),
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
        let mut prefs = Self {
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
            run_hooks: read(path, "run_hooks").unwrap_or(defaults.run_hooks),
            pull_strategy: read(path, "pull_strategy").unwrap_or(defaults.pull_strategy),
            clone_named_after_repo: read(path, "clone_named_after_repo")
                .unwrap_or(defaults.clone_named_after_repo),
            offer_force_lease: read(path, "offer_force_lease")
                .unwrap_or(defaults.offer_force_lease),
            backup_days: read(path, "backup_days")
                .filter(|days| BACKUP_DAYS.contains(days))
                .unwrap_or(defaults.backup_days),
            confirm_destructive: read(path, "confirm_destructive")
                .unwrap_or(defaults.confirm_destructive),
            avatar_sources: read(path, "avatar_sources").unwrap_or(defaults.avatar_sources),
            history_avatars: read(path, "history_avatars").unwrap_or(defaults.history_avatars),
            history_tags: read(path, "history_tags").unwrap_or(defaults.history_tags),
            relative_dates: read(path, "relative_dates").unwrap_or(defaults.relative_dates),
            subject_guide: read(path, "subject_guide")
                .filter(|guide| SUBJECT_GUIDES.contains(guide))
                .unwrap_or(defaults.subject_guide),
            diff_context: read(path, "diff_context")
                .filter(|lines| DIFF_CONTEXTS.contains(lines))
                .unwrap_or(defaults.diff_context),
            diff_whitespace: read(path, "diff_whitespace").unwrap_or(defaults.diff_whitespace),
            show_invisibles: read(path, "show_invisibles").unwrap_or(defaults.show_invisibles),
            wrap_lines: read(path, "wrap_lines").unwrap_or(defaults.wrap_lines),
            // Earlier versions saved `system_title_bar: false` whether or not it
            // was chosen, so only an earlier `true` is a choice.
            use_system_title_bar: read(path, "use_system_title_bar")
                .or_else(|| read::<bool>(path, "system_title_bar").filter(|&on| on))
                .or(defaults.use_system_title_bar),
            desktop_accent: read(path, "desktop_accent").unwrap_or(defaults.desktop_accent),
            desktop_fonts: read(path, "desktop_fonts").unwrap_or(defaults.desktop_fonts),
            keys: read(path, "keys").unwrap_or(defaults.keys),
            path: Some(path.to_path_buf()),
            saved: Saved::default(),
        };
        prefs.saved = Saved(prefs.entries());
        prefs
    }

    /// Every preference as stored in the settings file: one key per field.
    fn entries(&self) -> Map {
        macro_rules! entries {
            ($($field:ident),* $(,)?) => {
                [$((stringify!($field), serde_json::to_value(&self.$field).ok())),*]
            };
        }
        entries!(
            theme,
            density,
            network_avatars,
            rebase_autostash,
            reader_mode,
            diff_layout,
            image_layout,
            sign_commits,
            details_visible,
            stash_include_untracked,
            stash_keep_index,
            merge_no_ff,
            branch_switch,
            fetch_prune,
            editor,
            zoom,
            code_font,
            sign_tags,
            run_hooks,
            subject_guide,
            pull_strategy,
            clone_named_after_repo,
            offer_force_lease,
            backup_days,
            confirm_destructive,
            avatar_sources,
            history_avatars,
            history_tags,
            relative_dates,
            diff_context,
            diff_whitespace,
            show_invisibles,
            wrap_lines,
            use_system_title_bar,
            desktop_accent,
            desktop_fonts,
            keys
        )
        .into_iter()
        .filter_map(|(key, value)| Some((key.to_string(), value?)))
        .collect()
    }

    /// Save the preferences this window changed since it loaded or last
    /// saved them, over the file as it is now: another gg window's changes
    /// to other keys are kept.
    pub fn save(&mut self) -> Result<(), String> {
        let Some(path) = self.path.as_deref() else {
            return Err("No configuration folder is available for preferences.".into());
        };
        let current = self.entries();
        let saved = &self.saved.0;
        update(path, |map| {
            for (key, value) in &current {
                if saved.get(key) != Some(value) {
                    map.insert(key.clone(), value.clone());
                }
            }
        })?;
        self.saved.0 = current;
        Ok(())
    }

    /// The comparison options Diff mode reads with.
    pub fn diff_options(&self) -> gg_engine::diff::Options {
        gg_engine::diff::Options {
            context: usize::from(self.diff_context),
            whitespace: self.diff_whitespace,
        }
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
        for (time, name) in ["a", "b", "a"].into_iter().enumerate() {
            push_recent(&mut list, PathBuf::from(name), time as u64);
        }
        assert_eq!(
            list,
            [
                Recent {
                    path: PathBuf::from("a"),
                    opened: Some(2)
                },
                Recent {
                    path: PathBuf::from("b"),
                    opened: Some(1)
                }
            ]
        );
        for index in 0..20 {
            push_recent(&mut list, PathBuf::from(format!("r{index}")), 9);
        }
        assert_eq!(list.len(), RECENT_LIMIT);
    }

    #[test]
    fn recent_times_round_trip_and_old_path_lists_still_load()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            br#"{"recent_repositories": ["/old/one", {"path": "/new/two", "opened": 1700000000}]}"#,
        )?;
        assert_eq!(
            recent_from(&path),
            [
                Recent::new("/old/one".into()),
                Recent {
                    path: "/new/two".into(),
                    opened: Some(1_700_000_000)
                }
            ]
        );
        let mut list = recent_from(&path);
        push_recent(&mut list, "/old/one".into(), 1_800_000_000);
        write(&path, "recent_repositories", &list)?;
        assert_eq!(recent_from(&path), list);
        assert_eq!(
            read::<serde_json::Value>(&path, "recent_repositories"),
            Some(serde_json::json!([
                {"path": "/old/one", "opened": 1_800_000_000u64},
                {"path": "/new/two", "opened": 1_700_000_000u64}
            ]))
        );
        Ok(())
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
        prefs.stash_include_untracked = false;
        prefs.stash_keep_index = true;
        prefs.merge_no_ff = true;
        prefs.branch_switch = false;
        prefs.fetch_prune = true;
        prefs.editor = "code --wait {file}".into();
        prefs.zoom = 125;
        prefs.code_font = 140;
        prefs.sign_tags = true;
        prefs.run_hooks = false;
        prefs.subject_guide = 50;
        prefs.history_avatars = true;
        prefs.avatar_sources = vec![AvatarSource::Gitlab, AvatarSource::Libravatar];
        prefs.pull_strategy = PullStrategy::FastForward;
        prefs.clone_named_after_repo = false;
        prefs.offer_force_lease = false;
        prefs.backup_days = 30;
        prefs.confirm_destructive = false;
        prefs.history_tags = false;
        prefs.relative_dates = true;
        prefs.diff_context = 10;
        prefs.diff_whitespace = Whitespace::IgnoreAtEol;
        prefs.show_invisibles = false;
        prefs.wrap_lines = true;
        prefs.use_system_title_bar = Some(true);
        prefs.desktop_accent = false;
        prefs.desktop_fonts = false;
        prefs.save()?;
        assert_eq!(Prefs::load_from(&path), prefs);
        assert_eq!(read::<u32>(&path, "future_option"), Some(7));
        Ok(())
    }

    #[test]
    fn a_title_bar_setting_saved_by_an_earlier_git_good_keeps_the_desktops_frame()
    -> Result<(), Box<dyn std::error::Error>> {
        // Earlier versions saved `system_title_bar: false` with every other
        // preference, chosen or not, so it cannot hide KWin's or Windows'
        // frame (FRM-87). An earlier `true` was always a choice.
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"theme": "dark", "system_title_bar": false}"#)?;
        assert_eq!(Prefs::load_from(&path).use_system_title_bar, None);
        std::fs::write(&path, br#"{"system_title_bar": true}"#)?;
        assert_eq!(Prefs::load_from(&path).use_system_title_bar, Some(true));
        // A choice made now is kept, off as well as on.
        let mut prefs = Prefs::load_from(&path);
        prefs.use_system_title_bar = Some(false);
        prefs.save()?;
        assert_eq!(Prefs::load_from(&path).use_system_title_bar, Some(false));
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
        assert_eq!(prefs.use_system_title_bar, None, "the desktop's default");
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

    #[test]
    fn two_windows_keep_each_others_preferences() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("gg/settings.json");
        // Each window is its own process and loads the file once.
        let mut first = Prefs::load_from(&path);
        let mut second = Prefs::load_from(&path);
        first.theme = Theme::Light;
        first.save()?;
        second.density = Density::Comfortable;
        second.save()?;
        let reopened = Prefs::load_from(&path);
        assert_eq!(reopened.theme, Theme::Light);
        assert_eq!(reopened.density, Density::Comfortable);
        // A window that changes a key back still writes it.
        first.theme = Theme::System;
        first.save()?;
        assert_eq!(Prefs::load_from(&path).theme, Theme::System);
        assert_eq!(Prefs::load_from(&path).density, Density::Comfortable);
        Ok(())
    }

    #[test]
    fn an_unreadable_file_is_set_aside_not_overwritten() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("gg/settings.json");
        std::fs::create_dir_all(dir.path().join("gg"))?;
        std::fs::write(&path, br#"{"theme": "light", "future_op"#)?;
        let mut prefs = Prefs::load_from(&path);
        prefs.density = Density::Comfortable;
        prefs.save()?;
        assert_eq!(Prefs::load_from(&path).density, Density::Comfortable);
        assert_eq!(
            std::fs::read(dir.path().join("gg/settings.json.unreadable"))?,
            br#"{"theme": "light", "future_op"#
        );
        // No temporary file is left behind.
        let names: Vec<_> = std::fs::read_dir(dir.path().join("gg"))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name())
            .collect();
        assert_eq!(names.len(), 2, "{names:?}");
        Ok(())
    }

    #[test]
    fn two_windows_merge_their_recent_repositories() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("gg/settings.json");
        let entry = |name: &str, opened| Recent {
            path: name.into(),
            opened: Some(opened),
        };
        let mut first = vec![entry("/old", 1), entry("/gone", 0)];
        let mut first_base = Vec::new();
        save_recent_to(&path, &mut first, &mut first_base)?;
        let mut second = recent_from(&path);
        let mut second_base = second.clone();
        // The first window opens a repository and forgets another.
        push_recent(&mut first, "/one".into(), 5);
        first.retain(|recent| recent.path != Path::new("/gone"));
        save_recent_to(&path, &mut first, &mut first_base)?;
        // The second, loaded earlier, opens a different one.
        push_recent(&mut second, "/two".into(), 6);
        save_recent_to(&path, &mut second, &mut second_base)?;
        let expected = vec![entry("/two", 6), entry("/one", 5), entry("/old", 1)];
        assert_eq!(recent_from(&path), expected);
        assert_eq!(second, expected);
        Ok(())
    }
}
