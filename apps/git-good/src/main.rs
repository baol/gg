mod avatars;
mod binary_preview;
mod blame_cli;
mod blame_view;
mod blame_worker;
mod bounded_text;
mod branch_cli;
mod branding;
mod caption_style;
mod clipboard;
mod colors;
mod commit_cli;
mod commit_stats;
mod conflict_blocks;
mod conflict_cli;
mod conflict_result;
mod conflict_workspace;
mod consequence;
mod density;
mod details_view;
mod diff_view;
mod diff_worker;
mod editor;
mod export;
mod file_history_view;
mod file_history_worker;
mod folder_picker;
mod folders;
mod footer;
mod graph;
mod history_view;
mod init_cli;
mod inspect_ui;
#[cfg(test)]
mod interaction_tests;
mod lfs_cli;
mod lfs_view;
mod lfs_worker;
mod line_map_worker;
#[cfg(target_os = "macos")]
mod mac_menu;
mod menu_keys;
mod menu_nav;
#[cfg(test)]
mod menu_tests;
mod menus;
mod merge_cli;
mod mutation_cli;
mod native_menu;
mod navigation;
mod network;
mod new_repository;
mod operation_band;
mod outcome_details;
mod phosphor;
mod reader;
mod rebase_cli;
mod rebase_dialog;
mod rebase_plan;
mod rebase_stop;
mod records;
mod recovery_view;
mod reflog_cli;
mod refs_cli;
mod refs_view;
mod reset_cli;
mod return_location;
mod review_cli;
mod review_view;
mod review_worker;
mod rewrite_cli;
mod search;
mod settings;
mod shortcuts;
mod signature_cli;
mod signature_view;
mod signature_worker;
mod split_rows;
mod stage_view;
mod stash_cli;
mod state;
mod status_cli;
mod status_view;
mod status_worker;
mod stream;
mod submodule_cli;
mod submodule_worker;
mod submodules_view;
mod sync_cli;
mod tag_cli;
mod target_guard;
mod thumbnails;
mod titlebar;
mod watcher;
mod welcome_view;
mod worktree_cli;
mod worktree_worker;
mod wrap_rows;
use crate::navigation::ModalWindow;
use eframe::egui;
#[cfg(test)]
use gg_engine::History;
use gg_engine::HistoryRequest;
use std::{
    ops::Range,
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let first = args.next();
    if first.as_deref() == Some(std::ffi::OsStr::new("--version")) {
        println!("git-good {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("--help")) {
        println!(
            "Git Good\n\ngit-good [PATH]\ngit-good --version        Print version\ngit-good history [PATH]   Print history as JSON\ngit-good refs [PATH]      Print branches, remotes, tags, and HEAD as JSON\ngit-good status [PATH] [--file PATH --area AREA]   Print working status or diff as JSON\ngit-good stage --file PATH [--hunk HEADER]... [--repo PATH]   Stage one file or selected hunks\ngit-good unstage --file PATH [--repo PATH]   Unstage one file\ngit-good commit [--amend] [--sign|--no-sign] -m MESSAGE [--repo PATH]   Commit the index or amend HEAD after Git hooks\ngit-good reflog [REF] [--repo PATH]   Print reflog entries as JSON\ngit-good blame PATH [--revision REF] [--lines START,END] [--repo PATH]   Print line attribution as JSON\ngit-good worktree <list|add|remove> [OPTIONS] [--repo PATH]   List, add, or remove worktrees\ngit-good submodule <status|update> PATH [OPTIONS] [--repo PATH]   Inspect or update a gitlink submodule\ngit-good lfs <status|fetch> PATH [--area staged|unstaged|untracked|conflict] [--revision REF] [--before REF] [--repo PATH]   Inspect or fetch an LFS pointer\ngit-good signature <config|verify> [REF] [--repo PATH]   Read signing config or verify a commit/tag\ngit-good switch [--create] [--stash [--include-untracked]] BRANCH [--repo PATH]   Switch to or create a local branch, optionally stashing changes first\ngit-good fetch [REMOTE] [--prune] [--repo PATH]   Fetch and update remote-tracking refs\ngit-good push [REMOTE] [BRANCH] [--set-upstream] [--force-with-lease] [--repo PATH]   Push a local branch, with an explicit lease when rewriting\ngit-good clone URL [PATH]   Clone a repository and open it\ngit-good init [--branch NAME] [--commit] [PATH]   Create a repository, optionally committing the files already there\ngit-good stash <list|push|apply|pop|drop> [OPTIONS] [--repo PATH]   List or mutate stashes\ngit-good cherry-pick COMMIT [--repo PATH]   Apply a commit\ngit-good revert COMMIT [--repo PATH]   Invert a commit\ngit-good merge REF [--no-ff] [--repo PATH]   Merge a branch, remote, or tag\ngit-good rebase REF [--autostash] [--repo PATH]   Rebase the current branch onto a branch, remote, or tag\ngit-good rebase --continue|--skip|--abort [--repo PATH]   Recover an in-progress rebase\ngit-good rewrite --from COMMIT STEP... [--autostash] [--repo PATH]   Pick, reword, edit, squash, fixup, drop, or reorder a linear range (STEP: --pick|--edit|--squash|--fixup|--drop COMMIT, --reword COMMIT MESSAGE)\ngit-good rewrite --continue|--skip|--abort [--repo PATH]   Recover an in-progress rewrite\ngit-good conflicts [--repo PATH]   List unmerged paths\ngit-good conflict --file PATH [--use ours|theirs|deletion|--stage] [--repo PATH]   Read or resolve a conflict\ngit-good reset --soft|--mixed|--hard COMMIT [--repo PATH]   Reset the current branch\ngit-good branch --delete BRANCH [--force] [--repo PATH]   Delete a local branch\ngit-good branch --create NAME [--at REF] [--repo PATH]   Create a local branch without switching\ngit-good tag --delete TAG [--repo PATH]   Delete a local tag
git-good tag --create TAG --at REF -m MESSAGE [--sign] [--repo PATH]   Create an annotated or signed tag\ngit-good show <commit> [--repo PATH] [--parent N] [--file PATH] [--export before|after --output PATH]   Review or export\n\nBrowse commits reachable from HEAD and all local, remote, and tag refs."
        );
        return Ok(());
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("show")) {
        return review_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("status")) {
        return status_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("unstage")) {
        return status_cli::unstage(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("stage")) {
        return status_cli::stage(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("commit")) {
        return commit_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("reflog")) {
        return reflog_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("blame")) {
        return blame_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("signature")) {
        let sub = args
            .next()
            .ok_or("Use git-good signature <config|verify> --help for usage")?;
        return match sub.to_str() {
            Some("config") => signature_cli::config(args),
            Some("verify") => signature_cli::verify(args),
            _ => Err("Use git-good signature <config|verify>".into()),
        };
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("lfs")) {
        let sub = args
            .next()
            .ok_or("Use git-good lfs <status|fetch> --help for usage")?;
        return match sub.to_str() {
            Some("status") => lfs_cli::status(args),
            Some("fetch") => lfs_cli::fetch(args),
            _ => Err("Use git-good lfs <status|fetch>".into()),
        };
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("submodule")) {
        let sub = args
            .next()
            .ok_or("Use git-good submodule <status|update> --help for usage")?;
        return match sub.to_str() {
            Some("status") => submodule_cli::status(args),
            Some("update") => submodule_cli::update(args),
            _ => Err("Use git-good submodule <status|update>".into()),
        };
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("worktree")) {
        let sub = args
            .next()
            .ok_or("Use git-good worktree <list|add|remove> --help for usage")?;
        return match sub.to_str() {
            Some("list") => worktree_cli::list(args),
            Some("add") => worktree_cli::add(args),
            Some("remove") => worktree_cli::remove(args),
            _ => Err("Use git-good worktree <list|add|remove>".into()),
        };
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("refs")) {
        return refs_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("switch")) {
        return branch_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("stash")) {
        return stash_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("cherry-pick")) {
        return mutation_cli::cherry_pick(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("revert")) {
        return mutation_cli::revert(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("merge")) {
        return merge_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("rebase")) {
        return rebase_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("rewrite")) {
        return rewrite_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("conflicts")) {
        return conflict_cli::list(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("conflict")) {
        return conflict_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("reset")) {
        return reset_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("branch")) {
        return branch_cli::manage(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("tag")) {
        return tag_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("fetch")) {
        return sync_cli::fetch(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("push")) {
        return sync_cli::push(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("clone")) {
        return sync_cli::clone(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("init")) {
        return init_cli::run(args);
    }
    let headless = first.as_deref() == Some(std::ffi::OsStr::new("history"));
    let given = if headless { args.next() } else { first }.map(PathBuf::from);
    let path = match given {
        Some(path) => path,
        None if headless => std::env::current_dir()?,
        None => launch_folder(
            std::env::current_dir()?,
            settings::Prefs::load().reopen_last,
            &settings::recent()
                .into_iter()
                .map(|entry| entry.path)
                .collect::<Vec<_>>(),
            is_repository,
        ),
    };
    if args.next().is_some() {
        return Err("Use git-good --help for usage".into());
    }
    if headless {
        write_history_json(
            &path,
            &mut std::io::BufWriter::new(std::io::stdout().lock()),
        )?;
        return Ok(());
    }
    let mut window_size = egui::vec2(1120.0, 740.0);
    if cfg!(debug_assertions)
        && let Ok(size) = std::env::var("GG_WINDOW_SIZE")
        && let Some((width, height)) = size.split_once('x')
        && let (Ok(width), Ok(height)) = (width.parse::<f32>(), height.parse::<f32>())
        && width.is_finite()
        && height.is_finite()
    {
        window_size = egui::vec2(width.clamp(720.0, 3840.0), height.clamp(480.0, 2160.0));
    }
    let custom_titlebar = custom_titlebar(&settings::Prefs::load());
    eframe::run_native(
        "Git Good",
        native_options(custom_titlebar, window_size, branding::icon()?.clone()),
        Box::new(move |cc| {
            phosphor::install(&cc.egui_ctx);
            let mut app = Browser::new(path);
            app.chrome.custom = custom_titlebar;
            // Linux: the desktop's caption buttons on the side its layout puts them.
            if custom_titlebar {
                app.caption_style = caption_style::service();
            }
            // The macOS menu bar, installed now so winit's default menu doesn't stay up.
            #[cfg(target_os = "macos")]
            {
                app.native_menu = mac_menu::install(&cc.egui_ctx);
            }
            app.apply_prefs(&cc.egui_ctx);
            app.details_visible = app.prefs.details_visible;
            app.load(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )?;
    Ok(())
}

/// Windows and Linux: the menu bar is the window's title bar (no OS
/// decorations), unless Preferences › Appearance › Use the system title bar
/// is on. macOS keeps its own title bar; its menus are in the menu bar at
/// the top of the screen.
fn custom_titlebar(prefs: &settings::Prefs) -> bool {
    !cfg!(target_os = "macos") && !prefs.system_title_bar
}

/// The main window: `size`, at least 720 × 480, decorated unless gg draws
/// the title bar.
fn native_options(
    custom_titlebar: bool,
    size: egui::Vec2,
    icon: egui::IconData,
) -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // The name docks and taskbars show (Wayland app ID, X11 class).
            .with_app_id(native_menu::APP_NAME)
            .with_icon(icon)
            .with_inner_size(size)
            .with_min_inner_size([720.0, 480.0])
            .with_decorations(!custom_titlebar),
        ..Default::default()
    }
}

fn write_history_json(
    path: &std::path::Path,
    writer: &mut impl std::io::Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut session = gg_git::Session::open(&HistoryRequest { path: path.into() })?;
    write!(
        writer,
        "{{\"path\":{},\"head\":{},\"commits\":[",
        serde_json::to_string(&session.identity.path)?,
        serde_json::to_string(&session.identity.head)?
    )?;
    let mut first = true;
    while let Some(commit) = session.next_commit()? {
        if !first {
            write!(writer, ",")?;
        }
        serde_json::to_writer(&mut *writer, &commit)?;
        first = false;
    }
    writeln!(
        writer,
        "],\"truncated\":false,\"shallow\":{}}}",
        session.identity.shallow
    )?;
    writer.flush()?;
    Ok(())
}

struct CloneDialog {
    url: String,
    folder: String,
    name: String,
    name_from_url: bool,
}

impl CloneDialog {
    fn from_url(url: Option<String>, folder: PathBuf) -> Self {
        let url = url.unwrap_or_default();
        let name = gg_engine::sync::repository_name(&url).unwrap_or_default();
        Self {
            url,
            folder: folder.to_string_lossy().into_owned(),
            name,
            name_from_url: true,
        }
    }

    fn sync_name(&mut self) {
        if self.name_from_url
            && let Some(name) = gg_engine::sync::repository_name(&self.url)
        {
            self.name = name;
        }
    }

    fn destination(&self) -> Option<PathBuf> {
        let folder = self.folder.trim();
        let name = self.name.trim();
        if folder.is_empty()
            || !std::path::Path::new(folder).is_absolute()
            || !gg_engine::sync::repository_folder_name(name)
        {
            return None;
        }
        Some(PathBuf::from(folder).join(name))
    }
}

/// Mark the session default clone folder in the destination list.
fn clone_folder_label(path: &Path, default: &Path) -> String {
    let label = path.to_string_lossy();
    if path == default {
        format!("{} {label}", phosphor::CHECK)
    } else {
        label.into_owned()
    }
}

/// Sections of the Preferences window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PrefSection {
    General,
    Appearance,
    Diff,
    Commit,
    Rebase,
    Keyboard,
}

impl PrefSection {
    /// Words the Preferences search matches for this section.
    fn keywords(self) -> &'static str {
        match self {
            Self::General => {
                "general clone folders avatars network fetch prune editor open dialog defaults stash merge branch"
            }
            Self::Appearance => {
                "appearance theme dark light system density compact comfortable zoom font size code title bar window frame decorations caption buttons"
            }
            Self::Diff => {
                "diff open files full file layout inline side by side images compare context lines whitespace ignore line end display wrap long lines invisible characters tabs trailing spaces"
            }
            Self::Commit => {
                "commit identity name email signing sign tags subject length guide hooks verify signing key format allowed signers gpg"
            }
            Self::Rebase => {
                "rebase recovery stash autostash backup pull strategy merge fast-forward push force lease keep backups days retention confirm drop reset"
            }
            Self::Keyboard => "keyboard shortcuts keys",
        }
    }

    fn matches(self, query: &str) -> bool {
        let haystack = format!("{} {}", self.label(), self.keywords()).to_lowercase();
        query
            .to_lowercase()
            .split_whitespace()
            .all(|word| haystack.contains(word))
    }

    const ALL: [Self; 6] = [
        Self::General,
        Self::Appearance,
        Self::Diff,
        Self::Commit,
        Self::Rebase,
        Self::Keyboard,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Appearance => "Appearance",
            Self::Diff => "Diff",
            Self::Commit => "Commit",
            Self::Rebase => "Rebase & recovery",
            Self::Keyboard => "Keyboard",
        }
    }
}

/// The Preferences window (Repository › Preferences…), non-modal.
struct FoldersDialog {
    section: PrefSection,
    /// Search preferences: sections without a match are hidden.
    search: String,
    /// `user.name <user.email>` from Git config, read when the window opens.
    identity: Option<String>,
    /// Signing configuration and the repository config file, read in the
    /// background when the window opens (Commit › Signing rows).
    signing: Option<SigningConfig>,
    signing_read: Option<std::sync::mpsc::Receiver<SigningConfig>>,
    new_root: String,
    error: Option<String>,
    /// Repository count per clone folder; `None` for a missing folder.
    counts: Vec<(PathBuf, Option<usize>)>,
}

impl FoldersDialog {
    fn new(section: PrefSection) -> Self {
        Self {
            section,
            search: String::new(),
            identity: None,
            signing: None,
            signing_read: None,
            new_root: String::new(),
            error: None,
            counts: Vec::new(),
        }
    }
}

/// What Preferences › Commit shows under Signing, read from Git config.
#[derive(Clone, Debug)]
struct SigningConfig {
    config: gg_engine::signature::Config,
    /// The repository's own config file, for **Edit in repository config…**.
    file: Option<PathBuf>,
}

fn read_signing(repo: &Path) -> SigningConfig {
    let file = std::process::Command::new("git")
        .current_dir(repo)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()).join("config"));
    SigningConfig {
        config: gg_git::signature::config(repo),
        file,
    }
}

struct StashSwitchDialog {
    target: String,
    include_untracked: bool,
    error: Option<String>,
}

struct PullRebase {
    path: PathBuf,
    /// The local branch's full ref name.
    branch: String,
    remote: String,
    /// The upstream's full remote-tracking ref name.
    upstream: String,
    /// The upstream's short name for messages, such as `origin/main`.
    upstream_label: String,
}

struct ForcePushDialog {
    branch: String,
    remote: String,
    remote_branch: String,
    upstream: String,
    lease: String,
}

struct StashDialog {
    message: String,
    include_untracked: bool,
    keep_index: bool,
    error: Option<String>,
}

/// Repository facts the window frame needs without opening Local Changes,
/// read on a background thread.
#[derive(Clone, Debug, Default)]
struct LocalSummary {
    /// Changed paths: staged, unstaged, untracked, or conflicted, each once.
    changes: Option<usize>,
    stashes: Vec<gg_engine::stash::StashEntry>,
    /// First commit of the current branch's own work and its commit count.
    branch_start: Option<(String, usize)>,
    /// Steps and stop of an interactive rebase in progress.
    rebase: Option<gg_engine::mutation::RebaseProgress>,
    /// Tracked paths for the search Files group, bounded.
    files: Vec<String>,
    /// This repository's worktrees, for the refs sidebar.
    worktrees: Vec<gg_engine::worktree::Worktree>,
}

impl LocalSummary {
    fn read(path: PathBuf) -> Self {
        Self {
            changes: gg_git::stash::change_count(&path).ok(),
            stashes: gg_git::stash::list_with_files(&path)
                .map(|list| list.entries)
                .unwrap_or_default(),
            branch_start: gg_git::rewrite::branch_start(&path).ok().flatten(),
            rebase: gg_git::mutation::rebase_progress(&path).ok().flatten(),
            files: gg_git::files::tracked(&path, SEARCH_FILE_LIMIT).unwrap_or_default(),
            worktrees: gg_git::worktree::list(&path)
                .map(|snapshot| snapshot.worktrees)
                .unwrap_or_default(),
        }
    }
}

/// A choice made in the toolbar branch switcher.
#[derive(Clone, Debug, PartialEq, Eq)]
enum BranchPick {
    Switch(String),
    /// Track a remote branch that has no local branch yet.
    Track(String),
    New,
}

/// Remote branches without a local branch of the same name, for the
/// switcher's second section.
fn switcher_remotes(refs: &gg_engine::refs::Snapshot, filter: &str) -> Vec<String> {
    let locals = refs
        .items
        .iter()
        .filter(|item| item.kind == gg_engine::refs::RefKind::Branch)
        .map(|item| item.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    refs.items
        .iter()
        .filter(|item| item.kind == gg_engine::refs::RefKind::Remote)
        .filter(|item| !item.name.ends_with("/HEAD"))
        .filter(|item| {
            item.name
                .split_once('/')
                .is_none_or(|(_, short)| !locals.contains(short))
        })
        .filter(|item| crate::search::matches(filter, &item.name))
        .map(|item| item.name.clone())
        .collect()
}

fn branch_switcher_popup_id() -> egui::Id {
    egui::Id::unique("toolbar-branch-switcher")
}

/// Local branches for the switcher: short name and whether it is checked
/// out, filtered by every word of `filter`, current branch first.
fn switcher_branches(refs: &gg_engine::refs::Snapshot, filter: &str) -> Vec<(String, bool)> {
    let head = refs.head.as_ref().and_then(|head| head.name.as_deref());
    let mut branches = refs
        .items
        .iter()
        .filter(|item| item.kind == gg_engine::refs::RefKind::Branch)
        .filter(|item| crate::search::matches(filter, &item.name))
        .map(|item| (item.name.clone(), Some(item.full_name.as_str()) == head))
        .collect::<Vec<_>>();
    branches.sort_by_key(|(_, current)| !*current);
    branches
}

/// The Stashes picker popup, opened from Changes › Stashes… and the sidebar.
fn stash_picker_popup_id() -> egui::Id {
    egui::Id::unique("stash-picker")
}

/// A command chosen in the Stashes picker.
#[derive(Clone, Debug, PartialEq, Eq)]
enum StashPick {
    Apply(String),
    Pop(String),
    Show { commit: String, message: String },
    Drop(String),
}

/// Stashes picker: selected entry and pending drop confirmation.
#[derive(Clone, Debug, Default)]
struct StashPicker {
    selected: usize,
    confirm_drop: bool,
    error: Option<String>,
    /// Apply and Pop also restore what was staged (remembered for the
    /// session); replaces the old Restore… dialog.
    restore_index: bool,
}

use recovery_view::RecoveryView;

/// The reason Git gave for refusing, from its last `fatal:` or `error:`
/// line, such as *'feature' is already used by worktree at '/src/x'*.
fn git_reason(output: &str) -> Option<&str> {
    output
        .lines()
        .rev()
        .find_map(|line| {
            let line = line.trim();
            line.strip_prefix("fatal:")
                .or_else(|| line.strip_prefix("error:"))
        })
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
}

/// What switching to a remote branch does.
#[derive(Debug, PartialEq, Eq)]
enum RemoteSwitch {
    /// A local branch of the same name exists: switch to it.
    Local(String),
    /// Create a local branch tracking the remote one and switch to it.
    Track(String),
}

fn remote_switch(remote_branch: &str, refs: &[gg_engine::refs::RefItem]) -> RemoteSwitch {
    let local = remote_branch
        .split_once('/')
        .map_or(remote_branch, |(_, local)| local);
    if refs
        .iter()
        .any(|item| item.kind == gg_engine::refs::RefKind::Branch && item.name == local)
    {
        RemoteSwitch::Local(local.to_string())
    } else {
        RemoteSwitch::Track(remote_branch.to_string())
    }
}

/// Command readiness shared by the menu bar and the icon toolbar.
#[derive(Clone, Copy, Debug)]
struct FrameReadiness {
    has_repo: bool,
    network_idle: bool,
    branch_idle: bool,
    stash_idle: bool,
    behind: u64,
    pull_reason: Option<&'static str>,
    rewrite_reason: Option<&'static str>,
    /// Why **Stash changes** is unavailable, if it is.
    stash_reason: Option<&'static str>,
    stashes: usize,
    /// Changed paths shown on the Changes tab badge; zero when unknown.
    changes: usize,
    /// Why Branch › Interactive rebase of current branch… cannot run.
    branch_rewrite_reason: Option<&'static str>,
}

/// A window-frame command chosen from the toolbar or search, applied after
/// the frame is drawn through one dispatch and readiness.
#[derive(Clone, Debug, PartialEq, Eq)]
enum FrameCommand {
    Fetch,
    Pull,
    Push,
    ForcePush,
    NewBranch,
    NewTag,
    TrackBranch,
    Merge,
    RebaseOnto,
    Stash,
    Stashes,
    /// Interactive rebase of the current branch's own commits.
    Rewrite,
    /// Interactive rebase from the selected commit through HEAD.
    RewriteSelected,
    /// Repository › Recovery…: Backups and Reflog in one window.
    Recovery,
    Worktrees,
    /// Repository › Submodules…: every gitlink with its status.
    Submodules,
    /// Repository › LFS…: the LFS pointers in HEAD, missing first.
    Lfs,
    Open,
    Clone,
    /// New repository…, on this folder when given.
    NewRepository(Option<PathBuf>),
    Preferences,
    Shortcuts,
    LocalChanges,
    History,
    ToggleRefs,
    /// Filter history by the text.
    Filter(String),
    /// Inspect a ref's commit without moving HEAD.
    Inspect(String),
    /// Open the latest commit that changed this path, on that file.
    OpenFile(String),
    OpenRecent(PathBuf),
    ClearRecent,
    Quit,
    /// Open the toolbar branch switcher.
    SwitchTo,
    SetUpstream,
    RenameBranch,
    DeleteBranch,
    ReviewSelected,
    CompareHead,
    CopyId,
    CherryPick,
    Revert,
    ResetHere,
    /// Interactive rebase with the selected commit preset to this action.
    RebasePreset(gg_engine::rewrite::Action),
    StageAll,
    UnstageAll,
    DiscardAll,
    CommitNow,
    AmendHead,
    ResolveConflicts,
    ToggleDetails,
    SetDensity(settings::Density),
    SetDiffLayout(settings::DiffLayout),
    SetTheme(settings::Theme),
    Documentation,
    About,
    /// A menu item that is shown but cannot run yet.
    Unavailable,
}

/// The commands the macOS menu bar moves to its app menu.
fn native_menu_role(command: &FrameCommand) -> Option<native_menu::ItemRole> {
    match command {
        FrameCommand::About => Some(native_menu::ItemRole::About),
        FrameCommand::Preferences => Some(native_menu::ItemRole::Settings),
        FrameCommand::Quit => Some(native_menu::ItemRole::Quit),
        _ => None,
    }
}

/// Branch › Rename… for the current branch.
struct RenameDialog {
    old: String,
    new: String,
    error: Option<String>,
}

/// Branch › Set upstream… for the current branch.
struct UpstreamDialog {
    branch: String,
    upstream: String,
    error: Option<String>,
}

/// Branch › Delete…: choose the local branch, then the usual confirmation.
struct DeletePick {
    name: String,
}

/// Open `Ctrl+K` search state.
#[derive(Clone, Debug)]
struct SearchPalette {
    query: String,
    selected: usize,
    focused: bool,
    /// Typed into the toolbar field; results open under `anchor`.
    in_field: bool,
    anchor: egui::Rect,
    /// File results for `files_query`, recomputed only when it changes.
    files_query: String,
    file_items: Vec<search::Item<FrameCommand>>,
}

impl Default for SearchPalette {
    fn default() -> Self {
        Self {
            query: String::new(),
            selected: 0,
            focused: false,
            in_field: false,
            anchor: egui::Rect::NOTHING,
            files_query: String::new(),
            file_items: Vec::new(),
        }
    }
}

fn search_field_id() -> egui::Id {
    egui::Id::unique("toolbar-search-field")
}

/// Most tracked paths kept for the search Files group.
const SEARCH_FILE_LIMIT: usize = 50_000;

/// Fixed height of the search results popup.
const SEARCH_RESULTS_HEIGHT: f32 = 360.0;

/// Fixed height of the operation area: two lines.
const OPERATION_AREA_HEIGHT: f32 = operation_band::HEIGHT;

/// Height of the menu bar row (final design: 26 px).
const MENU_BAR_HEIGHT: f32 = 26.0;

/// Height of the toolbar row under the menu bar (final design: 34 px).
const TOOLBAR_HEIGHT: f32 = 34.0;

/// Height of the footer (status line) at the window bottom.
const FOOTER_HEIGHT: f32 = 25.0;

/// Windows at least this wide dock the refs sidebar; narrower ones open it
/// as a drawer over the view (narrow proposal).
const REFS_DOCK_WIDTH: f32 = 1120.0;

/// Width of the refs drawer.
const REFS_DRAWER_WIDTH: f32 = 248.0;

fn refs_drawer_id() -> egui::Id {
    egui::Id::unique("refs-drawer")
}

/// Paints a count badge on the top-right corner of `rect`; nothing for zero.
fn paint_badge(ui: &egui::Ui, rect: egui::Rect, count: u64) {
    if count == 0 {
        return;
    }
    let text = if count > 99 {
        "99+".into()
    } else {
        count.to_string()
    };
    let font = egui::FontId::proportional(9.0);
    let ink = egui::Color32::from_rgb(27, 27, 27);
    let galley = ui.painter().layout_no_wrap(text, font, ink);
    let size = egui::vec2((galley.size().x + 6.0).max(12.0), 12.0);
    let badge =
        egui::Rect::from_min_size(egui::pos2(rect.right() - size.x + 1.0, rect.top()), size);
    ui.painter()
        .rect_filled(badge, 6.0, egui::Color32::from_rgb(237, 166, 78));
    ui.painter()
        .galley(badge.center() - galley.size() / 2.0, galley, ink);
}

/// An inline count badge after a tab label, as on the mockup's Changes
/// tab; empty for zero.
fn count_badge<'a>(count: u64) -> egui::Atom<'a> {
    if count == 0 {
        return egui::Atom::default();
    }
    let text = if count > 99 {
        "99+".to_string()
    } else {
        count.to_string()
    };
    let width = 8.0 + 6.0 * text.chars().count() as f32;
    egui::Atom::paint(egui::vec2(width.max(16.0), 16.0), move |ui, args| {
        let font = egui::FontId::proportional(10.0);
        let ink = egui::Color32::from_rgb(27, 27, 27);
        let galley = ui.painter().layout_no_wrap(text.clone(), font, ink);
        ui.painter()
            .rect_filled(args.rect, 8.0, egui::Color32::from_rgb(237, 166, 78));
        ui.painter()
            .galley(args.rect.center() - galley.size() / 2.0, galley, ink);
    })
}

/// The OS window title, as in the mockups: `Git Good — ~/path`, and the rebase
/// step while one is stopped.
fn window_title(
    path: &str,
    home: Option<&str>,
    rebase: Option<&gg_engine::mutation::RebaseProgress>,
) -> String {
    let shown = match home.filter(|home| !home.is_empty()) {
        Some(home) if path == home => "~".to_string(),
        Some(home) => match path.strip_prefix(home) {
            Some(rest) if rest.starts_with('/') => format!("~{rest}"),
            _ => path.to_string(),
        },
        None => path.to_string(),
    };
    let mut title = format!("Git Good — {shown}");
    if let Some(progress) = rebase {
        let (step, total) = progress.position();
        if progress.stop == gg_engine::mutation::RebaseStop::Conflict {
            title.push_str(&format!(" — rebase conflict · step {step} of {total}"));
        } else {
            title.push_str(&format!(
                " — rebasing {} · step {step} of {total}",
                progress.branch.as_deref().unwrap_or("detached HEAD")
            ));
        }
    }
    title
}

/// The folder gg opens when launched without a path: the working directory,
/// or, outside a repository and with Reopen last repository on, the most
/// recent repository that still is one.
fn launch_folder(
    cwd: PathBuf,
    reopen_last: bool,
    recent: &[PathBuf],
    is_repository: impl Fn(&Path) -> bool,
) -> PathBuf {
    if !reopen_last || is_repository(&cwd) {
        return cwd;
    }
    recent
        .iter()
        .find(|path| is_repository(path))
        .cloned()
        .unwrap_or(cwd)
}

fn is_repository(path: &Path) -> bool {
    std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--git-dir"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Preferences › Commit › Signing: the signing configuration as read-only
/// rows, each naming its Git key, then a link to the repository config.
/// The rows keep their place while the configuration is read. Returns the
/// config file to open when the link is clicked.
fn signing_rows(ui: &mut egui::Ui, signing: Option<&SigningConfig>) -> Option<PathBuf> {
    let mut edit = None;
    if signing.is_none() {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(50));
    }
    let config = signing.map(|signing| &signing.config);
    egui::Grid::new("signing-config")
        .num_columns(2)
        .min_col_width(150.0)
        .min_row_height(22.0)
        .spacing([16.0, 2.0])
        .show(ui, |ui| {
            ui.label("Format");
            ui.horizontal(|ui| {
                if let Some(config) = config {
                    ui.label(&config.format);
                    ui.weak("· gpg.format");
                }
            });
            ui.end_row();
            ui.label("Signing key");
            ui.horizontal(|ui| {
                if let Some(config) = config {
                    match &config.signing_key {
                        Some(key) => {
                            ui.add(
                                egui::Label::new(egui::RichText::new(key).monospace()).truncate(),
                            );
                        }
                        None => {
                            ui.weak("Not configured");
                        }
                    }
                    ui.weak("· user.signingkey");
                }
            });
            ui.end_row();
            ui.label("Allowed signers");
            ui.horizontal(|ui| {
                if let Some(config) = config {
                    match &config.allowed_signers {
                        Some(file) => {
                            ui.add(
                                egui::Label::new(egui::RichText::new(file).monospace()).truncate(),
                            );
                        }
                        None => {
                            ui.weak("Not set");
                        }
                    }
                    ui.weak("· gpg.ssh.allowedSignersFile");
                }
            });
            ui.end_row();
            ui.label("");
            ui.horizontal(|ui| {
                ui.weak("Read from Git config.");
                let file = signing.and_then(|signing| signing.file.clone());
                if ui
                    .add_enabled(
                        file.is_some(),
                        egui::Link::new("Edit in repository config…"),
                    )
                    .clicked()
                {
                    edit = file;
                }
            });
            ui.end_row();
        });
    if let Some(note) = config.and_then(|config| config.note.as_deref()) {
        ui.weak(note);
    }
    edit
}

/// `Name <email>` from the repository's Git config, for Preferences ›
/// Commit; `None` when Git has no identity.
/// `git version 2.x`, for Copy version info.
fn git_version() -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("--version")
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn git_identity(repo: &Path) -> Option<String> {
    let read = |key: &str| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["config", "--get", key])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|value| !value.is_empty())
    };
    match (read("user.name"), read("user.email")) {
        (Some(name), Some(email)) => Some(format!("{name} <{email}>")),
        (Some(name), None) => Some(name),
        (None, Some(email)) => Some(format!("<{email}>")),
        (None, None) => None,
    }
}

/// The `Ctrl+K` chip at the right end of the empty toolbar search field.
fn search_key_chip(ui: &egui::Ui, field: egui::Rect) {
    let visuals = ui.visuals();
    let galley = ui.painter().layout_no_wrap(
        "Ctrl+K".into(),
        egui::FontId::proportional(11.0),
        visuals.weak_text_color(),
    );
    let chip = egui::Rect::from_min_size(
        egui::pos2(
            field.right() - galley.size().x - 14.0,
            field.center().y - galley.size().y / 2.0 - 2.0,
        ),
        galley.size() + egui::vec2(8.0, 4.0),
    );
    ui.painter().rect_stroke(
        chip,
        3.0,
        visuals.widgets.noninteractive.bg_stroke,
        egui::StrokeKind::Inside,
    );
    ui.painter().galley(
        chip.min + egui::vec2(4.0, 2.0),
        galley,
        visuals.weak_text_color(),
    );
}

/// One icon button in the window toolbar; returns whether it was clicked.
fn toolbar_button(
    ui: &mut egui::Ui,
    glyph: &str,
    badge: Option<u64>,
    tooltip: &str,
    disabled: Option<&str>,
) -> bool {
    let button = egui::Button::new(crate::phosphor::icon(glyph).size(17.0))
        .min_size(egui::vec2(28.0, 26.0))
        .frame_when_inactive(false);
    let response = ui.add_enabled(disabled.is_none(), button);
    paint_badge(ui, response.rect, badge.unwrap_or(0));
    let response = response.on_hover_text(tooltip);
    let response = match disabled {
        Some(reason) => response.on_disabled_hover_text(format!("{tooltip}\n{reason}")),
        None => response,
    };
    response.clicked()
}

struct MergeDialog {
    reference: String,
    no_ff: bool,
    error: Option<String>,
}

struct RebaseDialog {
    reference: String,
    autostash: bool,
    error: Option<String>,
}

struct RewriteJob {
    /// The receipt with how the rewritten branch stands against its upstream.
    #[allow(clippy::type_complexity)]
    receiver: std::sync::mpsc::Receiver<
        Result<
            (
                gg_engine::rewrite::Receipt,
                Option<gg_engine::sync::UpstreamRelation>,
            ),
            String,
        >,
    >,
    restore_row: Option<usize>,
    /// What the worker has seen so far: the backup, then Git's steps.
    live: std::sync::Arc<std::sync::Mutex<RewriteLive>>,
}

/// A running rewrite as its worker reports it.
#[derive(Default)]
struct RewriteLive {
    backup: Option<String>,
    progress: Option<gg_engine::mutation::RebaseProgress>,
}

struct BranchDeleteDialog {
    name: String,
    error: Option<String>,
}

struct TagDeleteDialog {
    name: String,
    error: Option<String>,
}

struct TagCreateDialog {
    name: String,
    target: String,
    message: String,
    sign: bool,
    signing_ready: bool,
    signing_note: Option<String>,
    error: Option<String>,
    pending: Option<std::sync::mpsc::Receiver<Result<gg_git::tag::CreateReceipt, String>>>,
}

#[derive(Clone)]
struct CreateBranchDialog {
    name: String,
    commit: String,
    selector: String,
    switch: bool,
    recovery_source: Option<return_location::RecoverySource>,
    error: Option<String>,
}

impl CreateBranchDialog {
    fn title(&self) -> &'static str {
        if self.recovery_source.is_some() {
            "Create branch here"
        } else {
            "New branch"
        }
    }
}

enum ConflictUpdate {
    /// The unmerged paths, the other staged paths, and the blocks left in
    /// each unmerged path.
    Paths {
        paths: Vec<String>,
        also: Vec<String>,
        counts: std::collections::HashMap<String, usize>,
    },
    Content {
        content: Box<gg_engine::conflict::Content>,
        snapshot: Box<gg_engine::conflict::Snapshot>,
        blocks: Result<Vec<gg_engine::conflict::Segment>, String>,
        /// The working file was edited outside the blocks.
        edited: bool,
        names: Option<gg_engine::conflict::SideNames>,
    },
}

struct ConflictView {
    paths: Vec<String>,
    selected: usize,
    previous_selected: usize,
    keep: Option<String>,
    content: Option<gg_engine::conflict::Content>,
    error: Option<String>,
    loading: bool,
    worker: Option<std::sync::mpsc::Receiver<Result<ConflictUpdate, String>>>,
    snapshot: Option<gg_engine::conflict::Snapshot>,
    preview: gg_engine::conflict::Resolution,
    /// Per-block choices for a text conflict.
    blocks: Option<conflict_blocks::Blocks>,
    /// Why this file has no text blocks.
    blocks_note: Option<String>,
    /// Resolve the whole file from one source instead of block by block.
    whole_file: bool,
    /// Files resolved and staged while the workspace was open.
    resolved: Vec<String>,
    /// The working file no longer holds what Git wrote at conflict time, so
    /// staging the blocks would overwrite edits made outside them.
    edited: bool,
    /// **Replace with blocks…** is asking before it overwrites those edits.
    confirm_replace: bool,
    /// Who Ours and Theirs are for the operation in progress.
    names: Option<gg_engine::conflict::SideNames>,
    /// Staged paths that are not conflicted: what the operation changed.
    also: Vec<String>,
    /// Conflict blocks left in each unmerged path.
    counts: std::collections::HashMap<String, usize>,
    /// An option chosen for a block edited by hand, asking first.
    confirm_block: Option<(usize, gg_engine::conflict::BlockChoice)>,
}

impl Default for ConflictView {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            selected: 0,
            previous_selected: 0,
            keep: None,
            content: None,
            error: None,
            loading: false,
            worker: None,
            snapshot: None,
            preview: gg_engine::conflict::Resolution::Working,
            blocks: None,
            blocks_note: None,
            whole_file: false,
            resolved: Vec::new(),
            edited: false,
            confirm_replace: false,
            names: None,
            also: Vec::new(),
            counts: Default::default(),
            confirm_block: None,
        }
    }
}

/// What **Stage resolution** publishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConflictStage {
    /// The composed blocks, or the Whole file preview source.
    Preview,
    /// The working file as it is, whatever the blocks say.
    WorkingFile,
}

fn next_conflict_selection(paths: &[String], keep: Option<&str>, previous: usize) -> usize {
    keep.and_then(|path| paths.iter().position(|candidate| candidate == path))
        .unwrap_or(previous)
        .min(paths.len().saturating_sub(1))
}

fn conflict_preview_available(view: &ConflictView) -> bool {
    let (Some(content), Some(snapshot)) = (&view.content, &view.snapshot) else {
        return false;
    };
    match view.preview {
        gg_engine::conflict::Resolution::Base => content.base.is_some(),
        gg_engine::conflict::Resolution::Ours => content.ours.is_some(),
        gg_engine::conflict::Resolution::Theirs => content.theirs.is_some(),
        gg_engine::conflict::Resolution::Working => {
            snapshot.working.is_some() || snapshot.working_hash.is_some()
        }
        gg_engine::conflict::Resolution::Deletion => content.deleted_side().is_some(),
    }
}

fn conflict_preview_reason(view: &ConflictView) -> &'static str {
    let deleted = view
        .content
        .as_ref()
        .and_then(gg_engine::conflict::Content::deleted_side);
    let chosen = match view.preview {
        gg_engine::conflict::Resolution::Ours => Some(gg_engine::conflict::Side::Ours),
        gg_engine::conflict::Resolution::Theirs => Some(gg_engine::conflict::Side::Theirs),
        _ => None,
    };
    if view.content.is_none() || view.snapshot.is_none() {
        "Load the conflict before staging a resolution."
    } else if deleted.is_some() && deleted == chosen {
        "This side deleted the file. Choose Keep deletion to stage the removal."
    } else if !conflict_preview_available(view) {
        "This preview source has no content. Choose another source or resolve it externally."
    } else {
        "Wait for the current operation to finish."
    }
}

fn conflict_preview_text(
    content: &gg_engine::conflict::Content,
    snapshot: &gg_engine::conflict::Snapshot,
    resolution: gg_engine::conflict::Resolution,
) -> String {
    let side = match resolution {
        gg_engine::conflict::Resolution::Base => content.base.as_deref(),
        gg_engine::conflict::Resolution::Ours => content.ours.as_deref(),
        gg_engine::conflict::Resolution::Theirs => content.theirs.as_deref(),
        gg_engine::conflict::Resolution::Deletion => {
            return format!(
                "{} will be deleted; the other side's changes to it are discarded.",
                content.path
            );
        }
        gg_engine::conflict::Resolution::Working => {
            return match &snapshot.working {
                Some(bytes) if bytes.contains(&0) => {
                    format!("[binary working file, {} bytes]", bytes.len())
                }
                Some(bytes) => String::from_utf8_lossy(bytes).into_owned(),
                None => snapshot
                    .working_note
                    .clone()
                    .unwrap_or_else(|| "(working file absent)".into()),
            };
        }
    };
    side.unwrap_or("(absent)").to_string()
}

#[derive(Clone, Copy)]
enum MutationAction {
    Continue,
    Skip,
    Abort,
}

/// What owns the operation band under the toolbar. Short jobs (network,
/// branch work, a merge that never stops) report in the footer instead.
enum OperationArea {
    Mutation {
        operation: gg_engine::mutation::Operation,
        conflicts: usize,
        /// A worker runs; the band's buttons disable in place.
        pending: bool,
    },
    Rewrite {
        refreshing: bool,
    },
}

enum OperationOutcome {
    /// A merge or fast-forward: the receipt, the branch, and how many
    /// commits it brought in.
    Merge(gg_engine::merge::Receipt, String, Option<usize>),
    /// A rebase of the named branch.
    Rebase(gg_engine::rebase::Receipt, String),
    Recovery {
        operation: gg_engine::mutation::Operation,
        action: MutationAction,
        receipt: gg_engine::mutation::MutationReceipt,
    },
    Conflict {
        file: String,
    },
    /// A staged resolution was undone; `conflicted` when Git recreated the
    /// conflict.
    Reopened {
        file: String,
        conflicted: bool,
    },
}

#[derive(Clone, Copy)]
enum CommitAction {
    Review,
    CherryPick,
    Revert,
    Reset(gg_engine::reset::Mode),
    Rewrite,
    Compare,
}

enum CommitCommand {
    /// An item of the Commit menu, mirrored in the Commit actions popup.
    Menu(FrameCommand),
    CopySubject,
    OpenSignature,
    /// Reset current branch to here ▸ Soft / Mixed / Hard.
    Reset(gg_engine::reset::Mode),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OutcomeSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecoveryAction {
    Backups,
    ResolveConflicts,
    RetryFetch,
    RetryPush,
    ManageStashes,
    RetrySwitch,
    StashAndSwitch,
    PullRebase,
    /// Push the rewritten branch with force-with-lease.
    ForcePush,
    /// Push a rewritten branch whose upstream is still an ancestor.
    Push,
    /// Fetch after a rejected push, to see the remote's new commits.
    Fetch,
}

struct Outcome {
    severity: OutcomeSeverity,
    summary: String,
    details: Option<String>,
    recovery: Option<RecoveryAction>,
    /// When it was reported; a plain success fades after [`SUCCESS_FADE`].
    at: std::time::Instant,
}

/// How long a success with nothing to act on stays in the footer.
const SUCCESS_FADE: std::time::Duration = std::time::Duration::from_secs(8);

fn recovery_label(action: RecoveryAction) -> &'static str {
    match action {
        RecoveryAction::Backups => "Recovery…",
        RecoveryAction::ResolveConflicts => "Resolve conflicts…",
        RecoveryAction::RetryFetch => "Retry fetch",
        RecoveryAction::RetryPush => "Retry push",
        RecoveryAction::ManageStashes => "Stashes…",
        RecoveryAction::RetrySwitch => "Retry switch",
        RecoveryAction::StashAndSwitch => "Stash and switch",
        RecoveryAction::PullRebase => "Pull",
        RecoveryAction::ForcePush => "Push with force-with-lease…",
        RecoveryAction::Push => "Push",
        RecoveryAction::Fetch => "Fetch",
    }
}

/// The follow-up after a rewrite: the push the rewritten branch needs, from
/// how it now stands against its upstream (local refs only).
fn rewrite_follow_up(
    conflicts: bool,
    offer_force_lease: bool,
    relation: Option<gg_engine::sync::UpstreamRelation>,
) -> RecoveryAction {
    use gg_engine::sync::UpstreamRelation;
    match relation {
        _ if conflicts => RecoveryAction::ResolveConflicts,
        // Only unpublished commits changed: a plain push publishes them.
        Some(UpstreamRelation::Ahead) => RecoveryAction::Push,
        // Published commits were rewritten: replacing them needs a lease.
        Some(UpstreamRelation::Diverged) if offer_force_lease => RecoveryAction::ForcePush,
        _ => RecoveryAction::Backups,
    }
}

/// The follow-up after a failed push: Fetch for a rejection, Retry push only
/// when a retry can help.
fn push_failure_follow_up(error: &str) -> Option<RecoveryAction> {
    match gg_git::sync::push_failure(error) {
        gg_engine::sync::PushFailure::Rejected => Some(RecoveryAction::Fetch),
        gg_engine::sync::PushFailure::Transient => Some(RecoveryAction::RetryPush),
        gg_engine::sync::PushFailure::Other => None,
    }
}

/// The Commit actions popup, matching the History inspector's menu: the
/// Commit menu's own entries (one command table) as 24 px rows with the
/// check column, **Copy subject** and **Signature details…** under Copy ID,
/// and **Reset current branch to here ▸** as Soft / Mixed / Hard. Disabled
/// mutations keep their rows, greyed, under one band giving the reason.
fn commit_action_menu(
    ui: &mut egui::Ui,
    menu: &[menus::Entry<FrameCommand>],
) -> Option<CommitCommand> {
    use menus::Entry;
    ui.set_min_width(260.0);
    let mutation = |command: &FrameCommand| {
        !matches!(
            command,
            FrameCommand::ReviewSelected | FrameCommand::CompareHead | FrameCommand::CopyId
        )
    };
    // One band for the reason every disabled mutation shares.
    let reasons = menus::flatten(&[menus::Menu {
        title: "Commit",
        entries: menu.to_vec(),
    }])
    .into_iter()
    .filter(|(_, item)| mutation(&item.command))
    .map(|(_, item)| item.disabled)
    .collect::<Vec<_>>();
    let band = reasons
        .first()
        .cloned()
        .flatten()
        .filter(|first| reasons.iter().all(|reason| reason.as_ref() == Some(first)));
    let mut command = None;
    let mut banded = false;
    for entry in menu {
        let Entry::Item(item) = entry else {
            if let Entry::Separator = entry {
                ui.separator();
            }
            continue;
        };
        if mutation(&item.command)
            && !banded
            && let Some(reason) = &band
        {
            banded = true;
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(220, 195, 125, 24))
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(reason)
                                .small()
                                .color(egui::Color32::from_rgb(220, 195, 125)),
                        )
                        .wrap(),
                    );
                });
        }
        if item.command == FrameCommand::ResetHere {
            use egui::containers::menu::SubMenuButton;
            let text = if item.disabled.is_none() {
                egui::RichText::new("Reset current branch to here")
                    .color(ui.visuals().error_fg_color)
            } else {
                egui::RichText::new("Reset current branch to here")
            };
            ui.add_enabled_ui(item.disabled.is_none(), |ui| {
                let button = menus::row(false, text).right_text(SubMenuButton::arrow_atom(None));
                SubMenuButton::from_button(button).ui(ui, |ui| {
                    ui.set_min_width(240.0);
                    for (mode, label, danger) in [
                        (
                            gg_engine::reset::Mode::Soft,
                            "Soft · keep index and files",
                            false,
                        ),
                        (gg_engine::reset::Mode::Mixed, "Mixed · keep files", false),
                        (
                            gg_engine::reset::Mode::Hard,
                            "Hard · discard changes…",
                            true,
                        ),
                    ] {
                        let text = if danger {
                            egui::RichText::new(label).color(ui.visuals().error_fg_color)
                        } else {
                            egui::RichText::new(label)
                        };
                        let default = mode == gg_engine::reset::Mode::Mixed;
                        let row = menus::row(default, text).shortcut_text(if default {
                            "default"
                        } else {
                            ""
                        });
                        if ui.add(row).clicked() {
                            command = Some(CommitCommand::Reset(mode));
                            ui.close();
                        }
                    }
                });
            });
            continue;
        }
        if menus::item_row(ui, item, band.is_none() || !mutation(&item.command)) {
            command = Some(CommitCommand::Menu(item.command.clone()));
            ui.close();
        }
        if item.command == FrameCommand::CopyId {
            let enabled = item.disabled.is_none();
            if ui
                .add_enabled(enabled, menus::row(false, "Copy subject"))
                .clicked()
            {
                command = Some(CommitCommand::CopySubject);
                ui.close();
            }
            if ui
                .add_enabled(enabled, menus::row(false, "Signature details…"))
                .clicked()
            {
                command = Some(CommitCommand::OpenSignature);
                ui.close();
            }
        }
    }
    command
}

fn short_id(reference: &str) -> String {
    reference.chars().take(12).collect()
}

fn worktree_path(bytes: &[u8]) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
    }
}

#[derive(Clone, Default)]
struct WorktreeState {
    selected: Option<String>,
    query: String,
    scroll_offset: f32,
    narrow_details: bool,
    details_visible: bool,
}

struct WorktreeWindow {
    /// The folder to open once the pending create succeeds.
    open_created: Option<PathBuf>,
    snapshot: Option<gg_engine::worktree::Snapshot>,
    selected: Option<usize>,
    error: Option<String>,
    notice: Option<String>,
    create: Option<CreateWorktreeDialog>,
    confirm_remove: Option<usize>,
    /// Lock… asks for an optional reason: the folder and the reason typed.
    lock: Option<(PathBuf, String)>,
    busy: Option<String>,
    pending: Option<worktree_worker::Action>,
}

struct CreateWorktreeDialog {
    destination: String,
    create: bool,
    branch: String,
    at: String,
    error: Option<String>,
    /// The destination follows the branch (`<repo>-<branch>` beside the
    /// repository) until it is edited.
    destination_auto: bool,
    /// Switch to the new worktree once it is created.
    open_after: bool,
}

/// The Worktrees window's ⋯ menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorktreeMore {
    CopyPath,
    Reveal,
    Lock,
    Unlock,
    Prune,
}

/// A worktree's STATUS pill colour: blue for the one shown, green clean,
/// amber modified, red missing, grey locked or unreadable.
fn worktree_tone(worktree: &gg_engine::worktree::Worktree) -> inspect_ui::Tone {
    use inspect_ui::Tone;
    match worktree.status_label() {
        "Current" => Tone::Blue,
        "Clean" => Tone::Green,
        "Modified" => Tone::Amber,
        "Missing" => Tone::Red,
        _ => Tone::Neutral,
    }
}

/// `text` in the 11.5 px monospace font, cut from the start with `…` so
/// its end (a worktree's own folder) fits `width`.
fn fit_tail(ui: &egui::Ui, text: &str, width: f32) -> String {
    let font = egui::FontId::monospace(11.5);
    let fits = |candidate: &str| {
        ui.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(candidate.to_owned(), font.clone(), egui::Color32::WHITE)
                .size()
                .x
        }) <= width
    };
    if fits(text) {
        return text.to_owned();
    }
    let starts: Vec<usize> = text.char_indices().map(|(at, _)| at).collect();
    for &start in starts.iter().skip(1) {
        let candidate = format!("…{}", &text[start..]);
        if fits(&candidate) {
            return candidate;
        }
    }
    "…".into()
}

/// Why Open is unavailable for `worktree`, or `None` when it can open.
fn worktree_open_reason(
    worktree: &gg_engine::worktree::Worktree,
    path: &Path,
    switch_pending: bool,
) -> Option<String> {
    if worktree.current {
        Some("This is the worktree shown now.".into())
    } else if worktree.prunable.is_some() || !path.exists() {
        Some("The worktree folder is missing.".into())
    } else if worktree.bare {
        Some("A bare repository has no files to open.".into())
    } else if switch_pending {
        Some("Wait for the current operation to finish.".into())
    } else {
        None
    }
}

/// The details STATE line: what the status means and what blocks removal.
fn worktree_state_line(worktree: &gg_engine::worktree::Worktree) -> String {
    let mut parts = Vec::new();
    if worktree.main {
        parts.push("Main worktree".to_string());
    }
    if worktree.current {
        parts.push("shown now".into());
    }
    match (&worktree.locked, &worktree.prunable, worktree.dirty) {
        (Some(reason), _, _) if reason.is_empty() => parts.push("locked".into()),
        (Some(reason), _, _) => parts.push(format!("locked: {reason}")),
        (_, Some(reason), _) if reason.is_empty() => parts.push("folder missing".into()),
        (_, Some(reason), _) => parts.push(format!("folder missing: {reason}")),
        (_, _, Some(true)) => parts.push("has local changes".into()),
        (_, _, Some(false)) => parts.push("no local changes".into()),
        _ => parts.push("status unavailable".into()),
    }
    let mut line = parts.join(" · ");
    if let Some(first) = line.get(..1) {
        line = first.to_uppercase() + &line[1..];
    }
    line
}

/// The default worktree folder for `branch`: a sibling of the repository
/// named `<repo>-<branch>`, slashes turned into dashes.
fn worktree_default_destination(repo: &Path, branch: &str) -> String {
    let name = repo.file_name().unwrap_or_default().to_string_lossy();
    let parent = repo.parent().unwrap_or(repo);
    let branch = branch.trim().replace(['/', '\\'], "-");
    if branch.is_empty() {
        return String::new();
    }
    parent
        .join(format!("{name}-{branch}"))
        .display()
        .to_string()
}

use submodules_view::SubmoduleWindow;

use lfs_view::LfsWindow;

use signature_view::SignatureWindow;

struct Browser {
    /// The 36 px logo on the welcome view.
    welcome_logo: Option<egui::TextureHandle>,
    path: String,
    review: Option<review_view::Review>,
    working: Option<status_view::StatusView>,
    working_drafts: std::collections::HashMap<PathBuf, status_view::Draft>,
    pending_commit: Option<String>,
    branch: Option<std::sync::mpsc::Receiver<Result<gg_git::branch::Receipt, String>>>,
    branch_label: Option<String>,
    /// The OS window title last sent, so it is only sent on change.
    window_title: String,
    branch_error: Option<String>,
    branch_warning: Option<String>,
    branch_target: Option<(String, bool)>,
    branch_after_create: Option<String>,
    pending_switch: Option<String>,

    network: Option<network::Worker>,
    network_progress: Option<gg_engine::sync::Progress>,
    network_error: Option<String>,
    network_notice: Option<String>,
    clone_dialog: Option<CloneDialog>,
    /// Parent folder chosen in the clone dialog for this session only.
    clone_folder: Option<PathBuf>,
    folders: folders::Roots,
    prefs: settings::Prefs,
    /// Text in the toolbar search field.
    search_text: String,
    /// Content width of the last frame, for menu readiness.
    ctx_width: f32,
    /// Repositories opened before, newest first.
    recent: Vec<settings::Recent>,
    /// The recent list as last read or saved, so a save keeps what other
    /// windows changed since.
    recent_base: Vec<settings::Recent>,
    /// The welcome view's selection, filter, and background reads.
    welcome: welcome_view::Welcome,
    new_repository: Option<new_repository::Dialog>,
    /// Open Local Changes once the requested repository has loaded.
    open_changes_after_load: bool,
    about_open: bool,
    /// The window's top row: whether it is the title bar, and its caption
    /// buttons' style (`titlebar`, `caption_style`).
    chrome: titlebar::Chrome,
    caption_style: Option<titlebar::CaptionStyleFn>,
    /// macOS: the native menu bar, which hides the in-window menus.
    native_menu: Option<native_menu::NativeMenu<FrameCommand>>,
    /// The larger logo drawn in About Git Good.
    about_logo: Option<egui::TextureHandle>,
    discard_confirm: bool,
    rename_dialog: Option<RenameDialog>,
    upstream_dialog: Option<UpstreamDialog>,
    delete_pick: Option<DeletePick>,
    folders_dialog: Option<FoldersDialog>,
    force_push: Option<ForcePushDialog>,
    force_push_pending: bool,
    track_dialog: Option<String>,
    stash_dialog: Option<StashDialog>,
    stash_notice: Option<String>,
    mutation: Option<gg_engine::mutation::OperationState>,
    mutation_notice: Option<String>,
    mutation_error: Option<String>,
    mutation_path: Option<PathBuf>,
    outcome: Option<Outcome>,
    outcome_details_open: bool,
    /// The receipt a notice field showed and when it appeared, for the fade.
    notice_seen: Option<(String, std::time::Instant)>,
    #[cfg(debug_assertions)]
    screenshot_requested: bool,
    #[cfg(debug_assertions)]
    open_local: bool,
    /// Debug: `GG_REBASE=<commit>` opens the interactive rebase from there.
    open_rebase: Option<String>,
    /// Debug: `GG_REVIEW=<commit>` opens that commit in Review.
    open_review: Option<String>,
    /// Debug: `GG_WORKTREES=1` opens the Worktrees window.
    open_worktrees: bool,
    /// The reopened commit Local Changes was last opened for.
    reopen_seen: Option<String>,
    operation_worker: Option<std::sync::mpsc::Receiver<Result<OperationOutcome, String>>>,
    operation_repo: Option<PathBuf>,
    operation_restore_row: Option<usize>,
    merge_dialog: Option<MergeDialog>,
    rebase_dialog: Option<RebaseDialog>,
    rebase_target: Option<String>,
    stash_switch: Option<StashSwitchDialog>,
    pull_rebase: Option<PullRebase>,
    rewrite_dialog: Option<rebase_dialog::Dialog>,
    rewrite_job: Option<RewriteJob>,
    rewrite_refresh: Option<u64>,
    commit_actions: bool,
    local_summary: Option<LocalSummary>,
    local_summary_job: Option<std::sync::mpsc::Receiver<LocalSummary>>,
    window_focused: bool,
    stash_picker: StashPicker,
    /// Where the toolbar's Stashes button sits, when it opened the picker.
    stash_anchor: Option<egui::Pos2>,
    search: Option<SearchPalette>,
    branch_filter: String,
    help_shortcuts: bool,
    /// The Keyboard shortcuts window's filter text.
    shortcuts_filter: String,
    conflict_view: Option<ConflictView>,
    reset_confirm: Option<(String, gg_engine::reset::Mode)>,
    branch_delete: Option<BranchDeleteDialog>,
    tag_delete: Option<TagDeleteDialog>,
    tag_create: Option<TagCreateDialog>,
    create_branch: Option<CreateBranchDialog>,
    recovery: Option<RecoveryView>,
    worktree_window: Option<WorktreeWindow>,
    worktree_worker: Option<worktree_worker::Worker>,
    worktree_generation: u64,
    worktree_states: std::collections::HashMap<PathBuf, WorktreeState>,
    submodule_window: Option<SubmoduleWindow>,
    /// Repository › Submodules….
    submodule_list: Option<submodules_view::ListWindow>,
    submodule_worker: Option<submodule_worker::Worker>,
    submodule_generation: u64,
    lfs_window: Option<LfsWindow>,
    /// Repository › LFS….
    lfs_list: Option<lfs_view::ListWindow>,
    lfs_worker: Option<lfs_worker::Worker>,
    lfs_generation: u64,
    signature_window: Option<SignatureWindow>,
    signature_worker: Option<signature_worker::Worker>,
    signature_generation: u64,
    refs_visible: bool,
    /// The refs sidebar in Changes and Review; hidden by default, as in
    /// their mockups.
    refs_visible_elsewhere: bool,
    /// File and line counts for the inspected commit, and their worker.
    commit_stats: Option<(String, commit_stats::Stats)>,
    /// The branch switcher's keyboard cursor.
    branch_cursor: usize,
    /// Delete on origin, awaiting confirmation: `origin/feature` and its
    /// last fetched commit.
    delete_remote: Option<(String, String)>,
    commit_stats_worker: Option<commit_stats::Worker>,
    /// The chosen row of the narrow commit details' FILES list.
    details_file: usize,
    /// What each open destructive dialog will remove, read in the background.
    consequences: std::collections::BTreeMap<&'static str, consequence::Lines>,
    /// Per open destructive dialog, the re-read of its lists on confirm.
    guards: std::collections::BTreeMap<&'static str, consequence::Guard>,
    refs_query: String,
    /// The refs drawer is open (windows narrower than [`REFS_DOCK_WIDTH`]).
    refs_drawer: bool,
    /// Focus the drawer's filter on its next frame.
    refs_drawer_focus: bool,
    /// The drawer's keyboard cursor over [`refs_view::cursor_rows`].
    refs_drawer_cursor: usize,
    /// The filter the cursor was placed for.
    refs_drawer_query: String,
    refs_width: f32,
    state: state::State,
    pending: Option<stream::Worker>,
    /// The private backup the last rewrite saved; Abort restores it.
    rewrite_backup: Option<String>,
    /// The last rewrite's branch and steps, kept for the refresh band.
    rewrite_steps: (String, Vec<operation_band::Step>),
    /// The Continue, Skip, or Abort the operation worker is running.
    mutation_busy: Option<operation_band::Command>,
    /// Conflict blocks per unmerged path, as the workspace last read them.
    conflict_counts: std::collections::HashMap<String, usize>,
    /// Watches the open repository; settled changes refresh what they touch.
    watcher: Option<watcher::Watcher>,
    /// When the history last started loading, so a ref change it already
    /// saw does not load it again.
    last_load: Option<std::time::Instant>,
    pending_filter_edit: bool,
    worker: Option<stream::Worker>,
    /// The last update that arrived during a thumb drag; its count and
    /// extent apply on release.
    staged: Option<stream::Update>,
    viewport: Range<usize>,
    scroll_offset: f32,
    page_rows: usize,
    restore_offset: Option<f32>,
    row_rects: Vec<(usize, egui::Rect)>,
    deferred_input: Vec<egui::Event>,
    scroll_to_selection: bool,
    restore_row: Option<usize>,
    avatars: avatars::Avatars,
    history_rect: egui::Rect,
    details_rect: egui::Rect,
    /// Fades and guards the history row that changes under a still pointer.
    history_guard: target_guard::Guard<Option<String>>,
    edit_path: bool,
    details_visible: bool,
    narrow_details: bool,
}

impl Browser {
    fn new(path: PathBuf) -> Self {
        let mut app = Self {
            welcome_logo: None,
            path: path.to_string_lossy().into_owned(),
            review: None,
            working: None,
            working_drafts: std::collections::HashMap::new(),
            pending_commit: None,
            branch: None,
            branch_label: None,
            window_title: String::new(),
            branch_error: None,
            branch_warning: None,
            branch_target: None,
            branch_after_create: None,
            pending_switch: None,

            network: None,
            network_progress: None,
            network_error: None,
            network_notice: None,
            clone_dialog: None,
            clone_folder: None,
            folders: folders::Roots::load(),
            // Tests keep the defaults instead of reading the user's settings.
            search_text: String::new(),
            ctx_width: 1120.0,
            recent: Vec::new(),
            recent_base: Vec::new(),
            welcome: welcome_view::Welcome::default(),
            new_repository: None,
            open_changes_after_load: false,
            // Debug builds open About Git Good with `GG_ABOUT=1` for native review.
            about_open: cfg!(debug_assertions) && std::env::var_os("GG_ABOUT").is_some(),
            chrome: titlebar::Chrome::default(),
            caption_style: None,
            native_menu: None,
            about_logo: None,
            discard_confirm: false,
            rename_dialog: None,
            upstream_dialog: None,
            delete_pick: None,
            prefs: if cfg!(test) {
                settings::Prefs::default()
            } else {
                settings::Prefs::load()
            },
            // Debug builds open Preferences with `GG_PREFS=<section>` for
            // native review (general, appearance, diff, commit, rebase, keyboard).
            folders_dialog: if cfg!(debug_assertions) {
                std::env::var("GG_PREFS").ok().and_then(|name| {
                    PrefSection::ALL
                        .into_iter()
                        .find(|section| section.label().to_lowercase().starts_with(&name))
                        .map(FoldersDialog::new)
                })
            } else {
                None
            },
            force_push: None,
            force_push_pending: false,
            track_dialog: None,
            stash_dialog: None,
            stash_notice: None,
            notice_seen: None,
            mutation: None,
            mutation_notice: None,
            mutation_error: None,
            mutation_path: None,
            outcome: None,
            outcome_details_open: false,
            #[cfg(debug_assertions)]
            screenshot_requested: false,
            #[cfg(debug_assertions)]
            open_local: std::env::var_os("GG_LOCAL_CHANGES").is_some(),
            reopen_seen: None,
            open_rebase: if cfg!(debug_assertions) {
                std::env::var("GG_REBASE").ok()
            } else {
                None
            },
            open_review: if cfg!(debug_assertions) {
                std::env::var("GG_REVIEW").ok()
            } else {
                None
            },
            open_worktrees: cfg!(debug_assertions) && std::env::var_os("GG_WORKTREES").is_some(),
            operation_worker: None,
            operation_repo: None,
            operation_restore_row: None,
            merge_dialog: None,
            rebase_dialog: None,
            rebase_target: None,
            stash_switch: None,
            pull_rebase: None,
            rewrite_dialog: None,
            rewrite_job: None,
            rewrite_refresh: None,
            commit_actions: false,
            local_summary: None,
            local_summary_job: None,
            window_focused: true,
            stash_picker: StashPicker::default(),
            stash_anchor: None,
            // Debug builds open search with `GG_SEARCH=<query>` for native review.
            branch_filter: String::new(),
            search: if cfg!(debug_assertions) {
                std::env::var("GG_SEARCH").ok().map(|query| SearchPalette {
                    query,
                    ..Default::default()
                })
            } else {
                None
            },
            help_shortcuts: false,
            shortcuts_filter: String::new(),
            conflict_view: None,
            reset_confirm: None,
            branch_delete: None,
            tag_delete: None,
            tag_create: None,
            create_branch: None,
            recovery: None,
            worktree_window: None,
            worktree_worker: None,
            worktree_generation: 0,
            worktree_states: std::collections::HashMap::new(),
            submodule_window: None,
            submodule_list: None,
            submodule_worker: None,
            submodule_generation: 0,
            lfs_window: None,
            lfs_list: None,
            lfs_worker: None,
            lfs_generation: 0,
            signature_window: None,
            signature_worker: None,
            signature_generation: 0,
            refs_visible: true,
            refs_visible_elsewhere: false,
            commit_stats: None,
            branch_cursor: 0,
            delete_remote: None,
            commit_stats_worker: None,
            details_file: 0,
            consequences: std::collections::BTreeMap::new(),
            guards: std::collections::BTreeMap::new(),
            refs_query: String::new(),
            refs_drawer: false,
            refs_drawer_focus: false,
            refs_drawer_cursor: 0,
            refs_drawer_query: String::new(),
            refs_width: 0.0,
            state: state::State::default(),
            pending: None,
            watcher: None,
            last_load: None,
            rewrite_backup: None,
            rewrite_steps: Default::default(),
            mutation_busy: None,
            conflict_counts: Default::default(),
            pending_filter_edit: false,
            worker: None,
            staged: None,
            viewport: 0..64,
            scroll_offset: 0.0,
            page_rows: 1,
            restore_offset: None,
            row_rects: Vec::new(),
            deferred_input: Vec::new(),
            scroll_to_selection: false,
            restore_row: None,
            avatars: avatars::Avatars::default(),
            history_rect: egui::Rect::NOTHING,
            details_rect: egui::Rect::NOTHING,
            history_guard: Default::default(),
            edit_path: false,
            details_visible: true,
            narrow_details: cfg!(debug_assertions) && std::env::var_os("GG_DETAILS").is_some(),
        };
        // Tests keep the defaults instead of reading the user's settings.
        if !cfg!(test) {
            app.recent = settings::recent();
            app.recent_base = app.recent.clone();
        }
        app
    }

    fn start_branch(&mut self, ctx: &egui::Context, name: String, create: bool) {
        if self.branch.is_some() || self.network.is_some() {
            return;
        }
        let Some(path) = self
            .state
            .history
            .as_ref()
            .map(|history| history.path.clone())
        else {
            self.branch_error = Some("Open a repository before changing branches".into());
            return;
        };
        let target_name = name.clone();
        let label_name = name.clone();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let repaint = ctx.clone();
        match std::thread::Builder::new()
            .name("gg-branch".into())
            .spawn(move || {
                let result = if create {
                    gg_git::branch::create(&path, &name)
                } else {
                    gg_git::branch::switch(&path, &name)
                };
                let _ = tx.send(result);
                repaint.request_repaint();
            }) {
            Ok(_) => {
                self.branch = Some(rx);
                self.clear_failure();
                self.branch_target = Some((target_name, create));
                self.branch_label = Some(if create {
                    format!("Creating {label_name}…")
                } else {
                    format!("Switching to {label_name}…")
                });
                self.branch_error = None;
                self.branch_warning = None;
            }
            Err(e) => self.branch_error = Some(format!("Start branch operation: {e}")),
        }
    }

    fn start_branch_stashing(
        &mut self,
        ctx: &egui::Context,
        name: String,
        include_untracked: bool,
    ) {
        if self.branch.is_some() || self.network.is_some() {
            return;
        }
        let Some(path) = self
            .state
            .history
            .as_ref()
            .map(|history| history.path.clone())
        else {
            self.branch_error = Some("Open a repository before changing branches".into());
            return;
        };
        let target_name = name.clone();
        let target = name.clone();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let repaint = ctx.clone();
        match std::thread::Builder::new()
            .name("gg-branch-stash".into())
            .spawn(move || {
                let result = gg_git::branch::switch_with_stash(&path, &name, include_untracked);
                let _ = tx.send(result);
                repaint.request_repaint();
            }) {
            Ok(_) => {
                self.branch = Some(rx);
                self.clear_failure();
                self.branch_target = Some((target_name, false));
                self.branch_label = Some(match self.changed_files() {
                    Some(count) => format!(
                        "Stashing {} and switching to {target}…",
                        operation_band::plural(count, "file")
                    ),
                    None => format!("Stashing changes and switching to {target}…"),
                });
                self.branch_error = None;
                self.branch_warning = None;
            }
            Err(e) => self.branch_error = Some(format!("Start branch operation: {e}")),
        }
    }

    fn pull_rebase_target(&self, full_name: &str) -> Option<PullRebase> {
        let path = self.stash_path()?;
        let tracking = self
            .state
            .refs
            .tracking
            .iter()
            .find(|tracking| tracking.full_name == full_name)?;
        Some(PullRebase {
            path,
            branch: tracking.full_name.clone(),
            remote: tracking.remote.clone(),
            upstream: tracking.upstream_full_name.clone(),
            upstream_label: tracking.upstream.clone(),
        })
    }

    fn start_pull_rebase(&mut self, ctx: &egui::Context) {
        if self.network.is_some() || self.branch.is_some() || self.operation_busy() {
            return;
        }
        let full_name = match self
            .state
            .refs
            .head
            .as_ref()
            .and_then(|head| head.name.as_deref())
        {
            Some(name) => name.to_string(),
            None => {
                self.branch_error =
                    Some("Switch to a local branch before pulling with rebase.".into());
                return;
            }
        };
        let Some(target) = self.pull_rebase_target(&full_name) else {
            self.branch_error = Some("The current branch has no upstream to pull from.".into());
            return;
        };
        self.force_push_pending = false;
        match network::Worker::pull(
            ctx,
            target.path.clone(),
            target.remote.clone(),
            target.branch.clone(),
            self.prefs.fetch_prune,
        )
        .map(|worker| worker.labelled(format!("Fetching {}", target.remote)))
        {
            Ok(worker) => {
                self.pull_rebase = Some(target);
                self.network = Some(worker);
                self.clear_failure();
                self.network_progress = None;
                self.network_error = None;
                self.network_notice = None;
            }
            Err(error) => self.network_error = Some(error),
        }
    }

    /// Switch to a remote branch (`origin/feature`) as the branch switcher
    /// does: to the local branch of that name when there is one, otherwise
    /// to a new local branch tracking it.
    fn switch_remote(&mut self, ctx: &egui::Context, remote_branch: String) {
        match remote_switch(&remote_branch, &self.state.refs.items) {
            RemoteSwitch::Local(name) => self.start_branch(ctx, name, false),
            RemoteSwitch::Track(remote) => self.start_track(ctx, remote),
        }
    }

    fn start_track(&mut self, ctx: &egui::Context, remote_branch: String) {
        if self.branch.is_some() || self.network.is_some() {
            return;
        }
        let Some(path) = self
            .state
            .history
            .as_ref()
            .map(|history| history.path.clone())
        else {
            self.branch_error = Some("Open a repository before tracking a remote branch".into());
            return;
        };
        let label = match remote_branch.split_once('/') {
            Some((_, local)) => format!("Creating {local} from {remote_branch}…"),
            None => format!("Creating {remote_branch}…"),
        };
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let repaint = ctx.clone();
        match std::thread::Builder::new()
            .name("gg-branch-track".into())
            .spawn(move || {
                let result = gg_git::branch::track_remote(&path, &remote_branch);
                let _ = tx.send(result);
                repaint.request_repaint();
            }) {
            Ok(_) => {
                self.branch = Some(rx);
                self.clear_failure();
                self.branch_label = Some(label);
                self.branch_error = None;
                self.branch_warning = None;
            }
            Err(e) => {
                self.branch_error = Some(format!("Start remote branch checkout: {e}"));
            }
        }
    }

    fn start_fetch(&mut self, ctx: &egui::Context, remote: Option<String>, prune: bool) {
        if self.branch.is_some() || self.network.is_some() {
            return;
        }
        self.force_push_pending = false;
        let Some(path) = self
            .state
            .history
            .as_ref()
            .map(|history| history.path.clone())
        else {
            self.network_error = Some("Open a repository before fetching".into());
            return;
        };
        let label = format!(
            "Fetching {}",
            remote.clone().unwrap_or_else(|| self.tracking_remote())
        );
        match network::Worker::fetch(ctx, path, remote, prune).map(|worker| worker.labelled(label))
        {
            Ok(worker) => {
                self.network = Some(worker);
                self.clear_failure();
                self.network_progress = None;
                self.network_error = None;
                self.network_notice = None;
            }
            Err(error) => self.network_error = Some(error),
        }
    }

    fn start_push(&mut self, ctx: &egui::Context) {
        if self.branch.is_some() || self.network.is_some() {
            return;
        }
        self.force_push_pending = false;
        let Some(path) = self
            .state
            .history
            .as_ref()
            .map(|history| history.path.clone())
        else {
            self.network_error = Some("Open a repository before pushing".into());
            return;
        };
        let set_upstream = self.state.refs.current_tracking().is_none();
        let label = format!(
            "Pushing {} to {}",
            self.current_branch_name(),
            self.tracking_remote()
        );
        match network::Worker::push(ctx, path, None, None, set_upstream, None)
            .map(|worker| worker.labelled(label))
        {
            Ok(worker) => {
                self.network = Some(worker);
                self.clear_failure();
                self.network_progress = None;
                self.network_error = None;
                self.network_notice = None;
            }
            Err(error) => self.network_error = Some(error),
        }
    }

    fn force_push_target(&self) -> Option<ForcePushDialog> {
        let tracking = self.state.refs.current_tracking()?;
        let lease = self
            .state
            .refs
            .items
            .iter()
            .find(|item| item.full_name == tracking.upstream_full_name)
            .map(|item| item.commit.clone())
            .filter(|commit| !commit.is_empty())?;
        Some(ForcePushDialog {
            branch: tracking.branch.clone(),
            remote: tracking.remote.clone(),
            remote_branch: tracking.remote_branch.clone(),
            upstream: tracking.upstream.clone(),
            lease,
        })
    }

    fn start_force_push(&mut self, ctx: &egui::Context, dialog: ForcePushDialog) {
        if self.branch.is_some() || self.network.is_some() {
            return;
        }
        let Some(path) = self
            .state
            .history
            .as_ref()
            .map(|history| history.path.clone())
        else {
            self.network_error = Some("Open a repository before pushing".into());
            return;
        };
        self.force_push_pending = true;
        let label = format!("Force-pushing {} to {}", dialog.branch, dialog.remote);
        match network::Worker::push(
            ctx,
            path,
            Some(dialog.remote),
            Some(dialog.branch),
            false,
            Some(dialog.lease),
        )
        .map(|worker| worker.labelled(label))
        {
            Ok(worker) => {
                self.network = Some(worker);
                self.clear_failure();
                self.network_progress = None;
                self.network_error = None;
                self.network_notice = None;
            }
            Err(error) => {
                self.force_push_pending = false;
                self.network_error = Some(error);
            }
        }
    }

    /// Delete on origin: one confirmation naming what goes and what stays.
    fn draw_delete_remote(&mut self, ctx: &egui::Context) {
        let Some((name, commit)) = self.delete_remote.clone() else {
            return;
        };
        let (remote, branch) = name.split_once('/').unwrap_or(("origin", name.as_str()));
        let (remote, branch) = (remote.to_string(), branch.to_string());
        let mut open = true;
        let mut submit = false;
        let mut cancel = false;
        let ready = self.network.is_none() && self.branch.is_none();
        egui::Window::new("Delete remote branch")
            .id(egui::Id::unique("delete-remote"))
            .modal(ctx, egui::Id::unique("delete-remote"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(440.0)
            .default_pos(ctx.content_rect().center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                ui.label(format!("Delete {branch} on {remote}?"));
                ui.weak(format!(
                    "Last fetched at {}. Git refuses if someone pushed to it since. Local branches and other clones keep their copies.",
                    short_id(&commit)
                ));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        submit |= ui
                            .add_enabled(
                                ready,
                                consequence::danger_button(format!("Delete on {remote}"), ready),
                            )
                            .clicked();
                        cancel |= ui
                            .button(("Cancel", egui::RichText::new("Esc").small().weak()))
                            .clicked();
                    });
                });
            });
        if submit && let Some(path) = self.state.history.as_ref().map(|h| h.path.clone()) {
            self.delete_remote = None;
            let label = format!("Deleting {branch} on {remote}");
            match network::Worker::delete_remote(ctx, path, remote, branch, commit)
                .map(|worker| worker.labelled(label))
            {
                Ok(worker) => {
                    self.network = Some(worker);
                    self.network_progress = None;
                    self.network_error = None;
                    self.network_notice = None;
                }
                Err(error) => self.network_error = Some(error),
            }
        } else if cancel || !open {
            self.delete_remote = None;
        }
    }

    fn draw_force_push(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &self.force_push else {
            self.consequences.remove("force-replaced");
            self.consequences.remove("force-new");
            return;
        };
        // What the push replaces on the remote and what replaces it.
        let (lease, branch) = (dialog.lease.clone(), dialog.branch.clone());
        let replaced_range = format!("{branch}..{lease}");
        let new_range = format!("{lease}..{branch}");
        let mut replaced = self.take_consequence(
            "force-replaced",
            &["log", "--oneline", "--no-decorate", &replaced_range],
            ctx,
        );
        let mut new = self.take_consequence(
            "force-new",
            &["log", "--oneline", "--no-decorate", &new_range],
            ctx,
        );
        let count = replaced.as_mut().and_then(consequence::Lines::count);
        let Some(dialog) = &mut self.force_push else {
            return;
        };
        let mut open = true;
        let mut submit = false;
        let mut cancel = false;
        egui::Window::new("Force push with lease")
            .id(egui::Id::unique("force-push-lease")).modal(ctx, egui::Id::unique("force-push-lease"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(440.0)
            .default_pos(ctx.content_rect().center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Rewrite {}/{} with local {}?",
                    dialog.remote, dialog.remote_branch, dialog.branch
                ));
                ui.weak(format!(
                    "Lease: {} at {}",
                    dialog.upstream,
                    short_id(&dialog.lease)
                ));
                ui.set_width(480.0);
                if let Some(replaced) = &mut replaced {
                    consequence::list(
                        ui,
                        "REPLACED ON THE REMOTE",
                        replaced,
                        egui::Color32::from_rgb(245, 160, 160),
                    );
                }
                if let Some(new) = &mut new {
                    consequence::list(
                        ui,
                        "PUSHED IN THEIR PLACE",
                        new,
                        egui::Color32::from_rgb(150, 220, 170),
                    );
                }
                ui.weak("Git rejects this push if the remote branch no longer points at that lease, leaving the remote unchanged. This never uses bare --force.");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ready = self.network.is_none() && self.branch.is_none();
                        let label = match count {
                            Some(0) | None => "Force push with lease".to_string(),
                            Some(count) => format!(
                                "Replace {}",
                                consequence::counted(count, "remote commit", "remote commits")
                            ),
                        };
                        if ui
                            .add_enabled(ready, consequence::danger_button(label, ready))
                            .clicked()
                        {
                            submit = true;
                        }
                        if ui
                            .button(("Cancel", egui::RichText::new("Esc").small().weak()))
                            .clicked()
                        {
                            cancel = true;
                        }
                    });
                });
            });
        if let Some(replaced) = replaced {
            self.consequences.insert("force-replaced", replaced);
        }
        if let Some(new) = new {
            self.consequences.insert("force-new", new);
        }
        if submit && let Some(dialog) = self.force_push.take() {
            self.start_force_push(ctx, dialog);
        }
        if cancel || !open {
            self.force_push = None;
        }
    }

    /// Apply the saved theme, density, and avatar source.
    fn apply_prefs(&self, ctx: &egui::Context) {
        ctx.set_theme(match self.prefs.effective_theme() {
            settings::Theme::System => egui::ThemePreference::System,
            settings::Theme::Light => egui::ThemePreference::Light,
            settings::Theme::Dark => egui::ThemePreference::Dark,
        });
        density::set(self.prefs.density);
        avatars::set_network(self.prefs.effective_network_avatars());
        avatars::set_sources(&self.prefs.avatar_sources);
        settings::set_reader_mode(ctx, self.prefs.reader_mode);
        settings::set_image_layout(ctx, self.prefs.image_layout);
        settings::set_diff_layout(ctx, self.prefs.diff_layout);
        settings::set_diff_options(ctx, self.prefs.diff_options());
        settings::set_diff_invisibles(ctx, self.prefs.show_invisibles);
        settings::set_diff_wrap(ctx, self.prefs.wrap_lines);
        settings::set_subject_guide(ctx, self.prefs.subject_guide);
        settings::set_run_hooks(ctx, self.prefs.run_hooks);
        ctx.set_zoom_factor(f32::from(self.prefs.zoom) / 100.0);
        density::set_code_font(self.prefs.code_font);
        density::set_history(
            self.prefs.history_avatars,
            self.prefs.history_tags,
            self.prefs.relative_dates,
        );
    }

    /// Save a remembered starting value when it changes. A save failure only
    /// loses the memory for the next launch, so it is not reported.
    fn remember(&mut self, change: impl FnOnce(&mut settings::Prefs)) {
        let before = self.prefs.clone();
        change(&mut self.prefs);
        if self.prefs != before {
            let _ = self.prefs.save();
        }
    }

    /// Keep layout choices made outside Preferences: the details pane and
    /// the image comparison layout chosen in the reader.
    fn remember_layout(&mut self, ctx: &egui::Context) {
        let details = self.details_visible;
        let layout = settings::image_layout(ctx);
        let diff_layout = settings::diff_layout(ctx);
        self.remember(|prefs| {
            prefs.details_visible = details;
            prefs.image_layout = layout;
            prefs.diff_layout = diff_layout;
        });
    }

    fn open_preferences(&mut self, section: PrefSection) {
        match &mut self.folders_dialog {
            Some(dialog) => dialog.section = section,
            None => {
                let mut dialog = FoldersDialog::new(section);
                dialog.identity = git_identity(Path::new(&self.path));
                let (send, receive) = std::sync::mpsc::channel();
                let repo = PathBuf::from(&self.path);
                std::thread::spawn(move || {
                    let _ = send.send(read_signing(&repo));
                });
                dialog.signing_read = Some(receive);
                self.folders_dialog = Some(dialog);
            }
        }
    }

    /// Repository › Preferences…: a modal window whose changes apply and
    /// save immediately.
    fn draw_folders_dialog(&mut self, ctx: &egui::Context) {
        if self.folders_dialog.is_none() {
            return;
        }
        let shortcuts = {
            let ready = self.frame_readiness();
            let mut list = menus::flatten(&self.menu_table(&ready))
                .into_iter()
                .filter(|(_, item)| !item.key.is_empty())
                .map(|(path, item)| (format!("{path} › {}", item.label), item.key.to_string()))
                .collect::<Vec<_>>();
            list.extend(
                [
                    ("Search", "Ctrl+K"),
                    ("Search, starting from the History filter", "Ctrl+F"),
                    ("Commit actions", "Alt+A"),
                    ("History: move · page", "j k / Up Down · PgUp PgDn"),
                    ("Local Changes: stage · unstage the selected file", "S · U"),
                    (
                        "Reader: change · file · history · blame",
                        "[ ] · Alt+Up/Down · H · B",
                    ),
                    ("Conflict blocks: choose · move", "1–4 · [ ]"),
                    ("Close a view or popup", "Esc"),
                ]
                .map(|(action, key)| (action.to_string(), key.to_string())),
            );
            list
        };
        let Some(dialog) = &mut self.folders_dialog else {
            return;
        };
        // Count repositories once per folder list change, not every frame.
        if dialog.counts.len() != self.folders.entries().len()
            || dialog
                .counts
                .iter()
                .zip(self.folders.entries())
                .any(|((counted, _), folder)| counted != folder)
        {
            dialog.counts = self
                .folders
                .entries()
                .iter()
                .map(|folder| (folder.clone(), folders::repository_count(folder)))
                .collect();
        }
        let mut open = true;
        let mut add = None;
        let mut remove = None;
        let mut prefer = None;
        let mut moved = None;
        let mut add_current = false;
        let mut prefs = self.prefs.clone();
        let mut show_shortcuts = false;
        let mut edit_config: Option<PathBuf> = None;
        let default_folder = self.folders.default_folder();
        let offline_override = std::env::var_os("GG_OFFLINE").is_some();
        // Modal, as in the mockup: a fixed size keeps it in place while
        // folders come and go; Esc, a click outside, × or Close dismiss it.
        // Sized like the mockup (about 900 × 720), within the window.
        let screen = ctx.content_rect().size();
        let width = (screen.x - 80.0).clamp(560.0, 900.0);
        let body = (screen.y - 140.0).clamp(300.0, 600.0);
        let modal = egui::Modal::new(egui::Id::unique("preferences"))
            .backdrop_color(egui::Color32::from_black_alpha(140))
            .show(ctx, |ui| {
                ui.set_width(width);
                // A fixed header row: title, search, and ×.
                ui.allocate_ui_with_layout(
                    egui::vec2(width, 36.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                    ui.set_min_size(egui::vec2(width, 36.0));
                    ui.label(egui::RichText::new("Preferences").size(18.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("×").on_hover_text("Close (Esc)").clicked() {
                            open = false;
                        }
                        ui.add(
                            egui::TextEdit::singleline(&mut dialog.search)
                                .hint_text("Search preferences")
                                .desired_width(220.0),
                        );
                    });
                    },
                );
                let visible = PrefSection::ALL
                    .into_iter()
                    .filter(|section| section.matches(&dialog.search))
                    .collect::<Vec<_>>();
                if let Some(first) = visible.first()
                    && !visible.contains(&dialog.section)
                {
                    dialog.section = *first;
                }
                ui.separator();
                // The body has one fixed size, so adding folders or switching
                // sections never resizes or moves the window.
                ui.allocate_ui_with_layout(
                    egui::vec2(width, body),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                    ui.set_min_size(egui::vec2(width, body));
                    ui.set_max_height(body);
                    ui.vertical(|ui| {
                        ui.set_width(190.0);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for section in PrefSection::ALL {
                            // Sections without a match keep their slot, empty.
                            let shown = visible.contains(&section);
                            let response = ui.add_visible(
                                shown,
                                egui::Button::selectable(dialog.section == section, section.label())
                                    .min_size(egui::vec2(190.0, 30.0)),
                            );
                            if shown && response.clicked() {
                                dialog.section = section;
                            }
                        }
                        if visible.is_empty() {
                            ui.weak("No matching settings");
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt("preferences-section")
                        .max_height(body)
                        .auto_shrink([false, false])
                        .show(ui, |ui| ui.vertical(|ui| match dialog.section {
                        PrefSection::General => {
                            ui.strong("Clone folders");
                            ui.weak(
                                "Folders where you keep repositories. Clone starts in the preferred one and offers the others in this order, then the launch folder.",
                            );
                            ui.add_space(4.0);
                            egui::ScrollArea::vertical()
                                .id_salt("clone-folders-list")
                                .max_height(170.0)
                                .min_scrolled_height(170.0)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    let count = self.folders.entries().len();
                                    if count == 0 {
                                        ui.weak("No saved folders. Add the current folder or type one below.");
                                    }
                                    for (index, root) in self.folders.entries().iter().enumerate() {
                                        ui.horizontal(|ui| {
                                            if ui
                                                .radio(root == &default_folder, "")
                                                .on_hover_text("Preferred: Clone starts here")
                                                .clicked()
                                            {
                                                prefer = Some(index);
                                            }
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.small_button("Remove").clicked() {
                                                        remove = Some(index);
                                                    }
                                                    if ui
                                                        .add_enabled(
                                                            index + 1 < count,
                                                            egui::Button::new(
                                                                phosphor::icon(phosphor::ARROW_DOWN).size(11.0),
                                                            )
                                                            .frame_when_inactive(false),
                                                        )
                                                        .on_hover_text("Move down")
                                                        .clicked()
                                                    {
                                                        moved = Some((index, false));
                                                    }
                                                    if ui
                                                        .add_enabled(
                                                            index > 0,
                                                            egui::Button::new(
                                                                phosphor::icon(phosphor::ARROW_UP).size(11.0),
                                                            )
                                                            .frame_when_inactive(false),
                                                        )
                                                        .on_hover_text("Move up")
                                                        .clicked()
                                                    {
                                                        moved = Some((index, true));
                                                    }
                                                    match dialog.counts.get(index).and_then(|(_, n)| *n) {
                                                        Some(n) => {
                                                            ui.weak(format!(
                                                                "{n} repositor{}",
                                                                if n == 1 { "y" } else { "ies" }
                                                            ));
                                                        }
                                                        None => {
                                                            ui.colored_label(
                                                                ui.visuals().warn_fg_color,
                                                                "missing",
                                                            )
                                                            .on_hover_text("This folder does not exist.");
                                                        }
                                                    }
                                                    let width = ui.available_width().max(0.0);
                                                    let (rect, response) = ui.allocate_exact_size(
                                                        egui::vec2(width, 18.0),
                                                        egui::Sense::hover(),
                                                    );
                                                    let label = root.to_string_lossy().into_owned();
                                                    bounded_text::paint(
                                                        ui,
                                                        rect,
                                                        &label,
                                                        egui::FontId::proportional(12.0),
                                                        ui.visuals().text_color(),
                                                    );
                                                    response.on_hover_text(label);
                                                },
                                            );
                                        });
                                    }
                                });
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut dialog.new_root)
                                        .hint_text("Absolute folder")
                                        .desired_width((ui.available_width() - 84.0).max(120.0)),
                                );
                                if ui
                                    .button("Browse…")
                                    .on_hover_text("Choose a folder with the desktop dialog")
                                    .clicked()
                                    && let Some(path) = folder_picker::pick_folder(
                                        "Clone folder",
                                        Some(Path::new(dialog.new_root.trim())),
                                    )
                                {
                                    dialog.new_root = path.to_string_lossy().into_owned();
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.button("Add folder").clicked() {
                                    add = Some(dialog.new_root.clone());
                                }
                                if ui.button("Add current").clicked() {
                                    add_current = true;
                                }
                            });
                            if let Some(error) = &dialog.error {
                                ui.colored_label(ui.visuals().error_fg_color, error);
                            }
                            ui.checkbox(
                                &mut prefs.clone_named_after_repo,
                                "Name the clone after the repository (e.g. ~/dev/gg)",
                            );
                            ui.separator();
                            ui.strong("Launch");
                            ui.checkbox(
                                &mut prefs.reopen_last,
                                "Reopen the last repository when launched outside one",
                            );
                            ui.weak("Without a path Git Good opens the current folder; when that is not a repository, the most recent one opens instead.");
                            ui.separator();
                            ui.strong("Fetch");
                            ui.checkbox(
                                &mut prefs.fetch_prune,
                                "Remove remote-tracking branches deleted on the remote",
                            );
                            ui.separator();
                            ui.strong("Editor");
                            ui.add(
                                egui::TextEdit::singleline(&mut prefs.editor)
                                    .hint_text("System default app")
                                    .desired_width(320.0),
                            );
                            ui.weak(
                                "Command for Open in editor; {file} is the file's path (added at the end when absent). Empty opens the file with the system's default app.",
                            );
                            ui.separator();
                            ui.strong("Dialog defaults");
                            ui.weak(
                                "Forms start with these choices and remember the last one you submitted; each still asks.",
                            );
                            ui.checkbox(
                                &mut prefs.stash_include_untracked,
                                "Stash: include untracked files",
                            );
                            ui.checkbox(
                                &mut prefs.stash_keep_index,
                                "Stash: keep staged changes in the index",
                            );
                            ui.checkbox(&mut prefs.merge_no_ff, "Merge: always create a merge commit");
                            ui.checkbox(
                                &mut prefs.branch_switch,
                                "New branch: switch after creation",
                            );
                        }
                        PrefSection::Appearance => {
                            egui::Grid::new("prefs-appearance")
                                .num_columns(2)
                                .spacing([16.0, 10.0])
                                .show(ui, |ui| {
                                    ui.label("Theme");
                                    ui.horizontal(|ui| {
                                        for theme in settings::Theme::ALL {
                                            ui.selectable_value(&mut prefs.theme, theme, theme.label());
                                        }
                                    });
                                    ui.end_row();
                                    ui.label("Density");
                                    ui.horizontal(|ui| {
                                        ui.selectable_value(&mut prefs.density, settings::Density::Compact, "Compact")
                                            .on_hover_text("22 px rows");
                                        ui.selectable_value(&mut prefs.density, settings::Density::Comfortable, "Comfortable")
                                            .on_hover_text("28 px rows");
                                    });
                                    ui.end_row();
                                    ui.label("Interface zoom");
                                    egui::ComboBox::from_id_salt("prefs-zoom")
                                        .selected_text(format!("{}%", prefs.zoom))
                                        .show_ui(ui, |ui| {
                                            for zoom in settings::ZOOMS {
                                                crate::menus::choose(ui, &mut prefs.zoom, zoom, format!("{zoom}%"));
                                            }
                                        });
                                    ui.end_row();
                                    ui.label("History");
                                    ui.vertical(|ui| {
                                        ui.checkbox(&mut prefs.history_avatars, "Show avatars");
                                        ui.add_enabled(
                                            !offline_override && prefs.history_avatars,
                                            egui::Checkbox::new(
                                                &mut prefs.network_avatars,
                                                "Load avatars from the network",
                                            ),
                                        )
                                        .on_hover_text("Off: cached avatars and initials only.")
                                        .on_disabled_hover_text(if offline_override {
                                            "GG_OFFLINE is set for this launch."
                                        } else {
                                            "Avatars are hidden."
                                        });
                                        // Sources, tried in this order; each
                                        // names what it learns.
                                        let network = !offline_override
                                            && prefs.history_avatars
                                            && prefs.network_avatars;
                                        for source in settings::AvatarSource::ALL {
                                            let mut on = prefs.avatar_sources.contains(&source);
                                            let changed = ui
                                                .add_enabled(
                                                    network,
                                                    egui::Checkbox::new(&mut on, source.label()),
                                                )
                                                .on_hover_text(source.note())
                                                .changed();
                                            if changed {
                                                prefs.avatar_sources.retain(|kept| *kept != source);
                                                if on {
                                                    prefs.avatar_sources.push(source);
                                                }
                                                prefs.avatar_sources.sort_by_key(|kept| {
                                                    settings::AvatarSource::ALL
                                                        .iter()
                                                        .position(|candidate| candidate == kept)
                                                });
                                            }
                                        }
                                        ui.checkbox(&mut prefs.history_tags, "Show tags in rows");
                                        ui.checkbox(&mut prefs.relative_dates, "Relative dates");
                                    });
                                    ui.end_row();
                                    ui.label("Code font size");
                                    let px = |tenths: u16| format!("{} px", f32::from(tenths) / 10.0);
                                    egui::ComboBox::from_id_salt("prefs-code-font")
                                        .selected_text(px(prefs.code_font))
                                        .show_ui(ui, |ui| {
                                            for size in settings::CODE_FONTS {
                                                crate::menus::choose(ui, &mut prefs.code_font, size, px(size));
                                            }
                                        });
                                    ui.end_row();
                                    // macOS always has its own title bar.
                                    if !cfg!(target_os = "macos") {
                                        ui.label("Title bar");
                                        ui.vertical(|ui| {
                                            ui.checkbox(
                                                &mut prefs.system_title_bar,
                                                "Use the system title bar",
                                            )
                                            .on_hover_text(
                                                "Off: Git Good's menu bar is the title bar, with its own window buttons.",
                                            );
                                            ui.weak("Takes effect when Git Good restarts.");
                                        });
                                        ui.end_row();
                                    }
                                });
                        }
                        PrefSection::Diff => {
                            egui::Grid::new("prefs-diff")
                                .num_columns(2)
                                .spacing([16.0, 10.0])
                                .show(ui, |ui| {
                                    ui.label("Default layout");
                                    ui.horizontal(|ui| {
                                        ui.selectable_value(&mut prefs.diff_layout, settings::DiffLayout::Inline, "Inline");
                                        ui.selectable_value(
                                            &mut prefs.diff_layout,
                                            settings::DiffLayout::SideBySide,
                                            "Side by side",
                                        );
                                    })
                                    .response
                                    .on_hover_text("Also View › Inline diff / Side-by-side diff and the reader's toggle.");
                                    ui.end_row();
                                    ui.label("Context lines");
                                    ui.horizontal(|ui| {
                                        for lines in settings::DIFF_CONTEXTS {
                                            ui.selectable_value(&mut prefs.diff_context, lines, lines.to_string());
                                        }
                                    })
                                    .response
                                    .on_hover_text("Unchanged lines around each change in Review and Local Changes.");
                                    ui.end_row();
                                    ui.label("Whitespace");
                                    egui::ComboBox::from_id_salt("prefs-diff-whitespace")
                                        .selected_text(
                                            settings::WHITESPACE
                                                .iter()
                                                .find(|(choice, _)| *choice == prefs.diff_whitespace)
                                                .map_or("", |(_, label)| *label),
                                        )
                                        .show_ui(ui, |ui| {
                                            for (choice, label) in settings::WHITESPACE {
                                                crate::menus::choose(ui, &mut prefs.diff_whitespace, choice, label);
                                            }
                                        })
                                        .response
                                        .on_hover_text("Hunk and line staging needs Show all changes; whole files still stage.");
                                    ui.end_row();
                                    ui.label("Display");
                                    ui.vertical(|ui| {
                                        ui.checkbox(&mut prefs.wrap_lines, "Wrap long lines")
                                            .on_hover_text("Long lines continue on the next rows instead of scrolling sideways.");
                                        ui.checkbox(&mut prefs.show_invisibles, "Show invisible characters on changed lines")
                                            .on_hover_text("Tabs as → and trailing spaces as ·, dimmed, on added and removed lines.");
                                    });
                                    ui.end_row();
                                });
                            ui.separator();
                            ui.strong("Open files in");
                            ui.horizontal(|ui| {
                                ui.radio_value(
                                    &mut prefs.reader_mode,
                                    settings::ReaderMode::Diff,
                                    "Diff",
                                );
                                ui.radio_value(
                                    &mut prefs.reader_mode,
                                    settings::ReaderMode::Full,
                                    "Full file",
                                );
                            });
                            ui.weak("Each newly opened reader starts here; D and F still switch.");
                            ui.separator();
                            ui.strong("Compare images");
                            ui.horizontal(|ui| {
                                for layout in settings::ImageLayout::ALL {
                                    ui.radio_value(&mut prefs.image_layout, layout, layout.label());
                                }
                            });
                            ui.weak("Also remembered from the last choice in the reader.");
                        }
                        PrefSection::Commit => {
                            ui.strong("Identity");
                            ui.horizontal(|ui| {
                                ui.label(
                                    dialog
                                        .identity
                                        .as_deref()
                                        .map_or("No user.name or user.email in Git config".to_string(), |identity| {
                                            format!("From Git config: {identity}")
                                        }),
                                );
                            });
                            ui.weak("Set with git config user.name and user.email.");
                            ui.separator();
                            ui.strong("Subject length guide");
                            ui.horizontal(|ui| {
                                for guide in settings::SUBJECT_GUIDES {
                                    let label = if guide == 0 {
                                        "Off".to_string()
                                    } else {
                                        format!("{guide} characters")
                                    };
                                    ui.selectable_value(&mut prefs.subject_guide, guide, label);
                                }
                            });
                            ui.weak("The commit box counts the subject and warns past the guide.");
                            ui.separator();
                            ui.strong("Hooks");
                            ui.checkbox(
                                &mut prefs.run_hooks,
                                "Run commit hooks; keep the draft if a hook fails",
                            );
                            ui.weak("Off skips the pre-commit and commit-msg hooks, like git commit --no-verify.");
                            ui.separator();
                            ui.strong("Sign tags");
                            ui.checkbox(&mut prefs.sign_tags, "Create tag starts with Sign checked");
                            ui.separator();
                            ui.strong("Sign commits");
                            for sign in settings::SignCommits::ALL {
                                ui.radio_value(&mut prefs.sign_commits, sign, sign.label());
                            }
                            ui.weak(
                                "Sets the Sign checkbox when Local Changes opens; it can still be changed per commit.",
                            );
                            if let Some(read) = dialog.signing_read.as_ref().and_then(|read| read.try_recv().ok()) {
                                dialog.signing = Some(read);
                                dialog.signing_read = None;
                            }
                            if let Some(file) = signing_rows(ui, dialog.signing.as_ref()) {
                                edit_config = Some(file);
                            }
                        }
                        PrefSection::Rebase => {
                            ui.strong("Rebase");
                            ui.checkbox(
                                &mut prefs.rebase_autostash,
                                "Stash working changes first",
                            );
                            ui.weak(
                                "Rebase and interactive rebase dialogs start with this choice; each rebase still asks.",
                            );
                            ui.checkbox(&mut prefs.confirm_destructive, "Confirm before Drop and Reset");
                            ui.weak("Off: Commit › Drop… runs without the plan, and Reset current branch to here… runs a Soft or Mixed reset at once. A Hard reset always asks first. Both still save a backup.");
                            ui.separator();
                            ui.strong("Pull strategy");
                            ui.horizontal(|ui| {
                                for strategy in settings::PullStrategy::ALL {
                                    ui.selectable_value(&mut prefs.pull_strategy, strategy, strategy.label());
                                }
                            });
                            ui.weak("Pull fetches the upstream, then rebases onto it, merges it, or only fast-forwards.");
                            ui.separator();
                            ui.strong("Push");
                            ui.checkbox(
                                &mut prefs.offer_force_lease,
                                "Offer force-with-lease after a rewrite",
                            );
                            ui.weak("After rewriting a published branch, the status line offers Push with force-with-lease….");
                            ui.separator();
                            ui.strong("Keep private backups");
                            ui.horizontal(|ui| {
                                for days in settings::BACKUP_DAYS {
                                    let label = if days == 0 {
                                        "Forever".to_string()
                                    } else {
                                        format!("{days} days")
                                    };
                                    ui.selectable_value(&mut prefs.backup_days, days, label);
                                }
                            });
                            ui.weak("Older backups are deleted when a repository opens.");
                            ui.separator();
                            ui.strong("Recovery");
                            ui.weak(
                                "Every rewrite creates a private backup ref first. Repository › Recovery… lists them with the reflog.",
                            );
                        }
                        PrefSection::Keyboard => {
                            ui.weak(
                                "Single letters act on the focused list and never fire while typing; Ctrl chords are global.",
                            );
                            egui::Grid::new("prefs-keyboard")
                                .num_columns(2)
                                .striped(true)
                                .spacing([16.0, 4.0])
                                .show(ui, |ui| {
                                    for (action, key) in &shortcuts {
                                        ui.label(action);
                                        ui.monospace(key);
                                        ui.end_row();
                                    }
                                });
                            if ui.button("Show keyboard shortcuts…").clicked() {
                                show_shortcuts = true;
                            }
                        }
                    }));
                    },
                );
                ui.add_space(4.0);
                ui.separator();
                ui.horizontal(|ui| {
                    let saved = settings::path()
                        .map(|path| {
                            let folder = path.parent().unwrap_or(&path).display().to_string();
                            match std::env::var("HOME") {
                                Ok(home) if !home.is_empty() && folder.starts_with(&home) => {
                                    format!("~{}/", &folder[home.len()..])
                                }
                                _ => format!("{folder}/"),
                            }
                        })
                        .unwrap_or_else(|| "no configuration folder".into());
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!("Changes apply immediately. Saved to {saved}"))
                                .small()
                                .weak(),
                        )
                        .truncate(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            open = false;
                        }
                    });
                });
            });
        if modal.should_close() {
            open = false;
        }
        if let Some(file) = edit_config
            && let Err(error) = editor::open(&self.prefs.editor, &file)
            && let Some(dialog) = &mut self.folders_dialog
        {
            dialog.error = Some(error);
        }
        if show_shortcuts {
            // The shortcuts window would sit under the modal: close it first.
            open = false;
            self.help_shortcuts = true;
        }
        if prefs != self.prefs {
            self.prefs = prefs;
            self.apply_prefs(ctx);
            let error = self.prefs.save().err();
            if let Some(dialog) = &mut self.folders_dialog {
                dialog.error = error;
            }
        }
        if let Some(index) = prefer {
            let error = self.prefer_clone_folder(index);
            if let Some(dialog) = &mut self.folders_dialog {
                dialog.error = error;
            }
        }
        if let Some((index, up)) = moved
            && self.folders.move_entry(index, up)
        {
            let error = self.folders.save().err();
            if let Some(dialog) = &mut self.folders_dialog {
                dialog.error = error;
            }
        }
        if remove.is_some() || add.is_some() || add_current {
            let adding = add.is_some() || add_current;
            let error = if let Some(index) = remove {
                self.folders.remove(index);
                self.folders.save().err()
            } else {
                let value = add
                    .unwrap_or_else(|| folders::starting_folder().to_string_lossy().into_owned());
                match self.folders.add(&value) {
                    Ok(true) => self.folders.save().err(),
                    Ok(false) => Some("Already in the list.".into()),
                    Err(error) => Some(error),
                }
            };
            if let Some(dialog) = &mut self.folders_dialog {
                if adding && error.is_none() {
                    dialog.new_root.clear();
                }
                dialog.error = error;
            }
        }
        if !open {
            self.folders_dialog = None;
        }
    }

    fn draw_stash_switch(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.stash_switch else {
            return;
        };
        let mut open = true;
        let mut submit = false;
        let mut cancel = false;
        egui::Window::new("Stash changes and switch")
            .id(egui::Id::unique("stash-switch")).modal(ctx, egui::Id::unique("stash-switch"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(440.0)
            .default_pos(ctx.content_rect().center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Local changes block switching to {}.",
                    dialog.target
                ));
                ui.label(
                    "Stash the changes, switch branches, and keep the stash entry so it can be restored from Stashes.",
                );
                ui.checkbox(
                    &mut dialog.include_untracked,
                    "Include untracked files in the stash",
                );
                if let Some(error) = &dialog.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.horizontal(|ui| {
                    if ui.button("Stash and switch").clicked() {
                        submit = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if submit {
            let target = dialog.target.clone();
            let include_untracked = dialog.include_untracked;
            self.remember(|prefs| prefs.stash_include_untracked = include_untracked);
            self.stash_switch = None;
            self.pending_switch = Some(target.clone());
            self.start_branch_stashing(ctx, target, include_untracked);
        } else if cancel || !open {
            self.stash_switch = None;
        }
    }

    /// Keep the chosen parent folder for later clones in this session.
    fn choose_clone_folder(&mut self, folder: PathBuf) {
        if let Some(dialog) = &mut self.clone_dialog {
            dialog.folder = folder.to_string_lossy().into_owned();
        }
        self.clone_folder = Some(folder);
    }

    /// The parent folder a new clone starts in: this session's choice, else
    /// the saved default.
    fn clone_start_folder(&self) -> PathBuf {
        self.clone_folder
            .clone()
            .unwrap_or_else(|| self.folders.default_folder())
    }

    /// Mark a saved folder as the default for this and later launches.
    fn prefer_clone_folder(&mut self, index: usize) -> Option<String> {
        if !self.folders.set_preferred(index) {
            return None;
        }
        let error = self.folders.save().err();
        self.clone_folder = None;
        let folder = self.folders.default_folder();
        if let Some(dialog) = &mut self.clone_dialog {
            dialog.folder = folder.to_string_lossy().into_owned();
        }
        error
    }

    fn start_clone(&mut self, ctx: &egui::Context, url: String, destination: Option<PathBuf>) {
        if self.branch.is_some() || self.network.is_some() {
            return;
        }
        self.force_push_pending = false;
        let name = gg_engine::sync::repository_name(&url).unwrap_or_else(|| url.clone());
        match network::Worker::clone(ctx, url, destination)
            .map(|worker| worker.labelled(format!("Cloning {name}")))
        {
            Ok(worker) => {
                self.network = Some(worker);
                self.clear_failure();
                self.network_progress = None;
                self.network_error = None;
                self.network_notice = None;
            }
            Err(error) => self.network_error = Some(error),
        }
    }

    fn default_track_branch(&self) -> String {
        self.state
            .refs
            .items
            .iter()
            .find(|item| item.kind == gg_engine::refs::RefKind::Remote)
            .map(|item| item.name.clone())
            .unwrap_or_default()
    }

    fn finish_network(
        &mut self,
        ctx: &egui::Context,
        result: Result<network::Outcome, String>,
        operation: Option<gg_engine::sync::Operation>,
    ) {
        let force_push = self.force_push_pending;
        self.force_push_pending = false;
        self.network = None;
        self.network_progress = None;
        match result {
            Ok(network::Outcome::Fetch(receipt)) => {
                self.network_error = None;
                self.network_notice = Some(format!("Fetched {}", receipt.remote));
                self.load(ctx);
            }
            Ok(network::Outcome::Pull { fetch, relation }) => {
                self.network_error = None;
                self.network_notice = Some(format!("Fetched {}", fetch.remote));
                let pull = self
                    .pull_rebase
                    .take()
                    .filter(|pull| self.stash_path().as_ref() == Some(&pull.path));
                self.load(ctx);
                if let Some(pull) = pull {
                    self.network_notice = None;
                    let Some(relation) = relation else {
                        self.set_outcome(
                            OutcomeSeverity::Error,
                            format!(
                                "{} was not found on {}. Check the branch's upstream.",
                                pull.upstream_label, pull.remote
                            ),
                            None,
                            None,
                        );
                        return;
                    };
                    if !relation.has_incoming() {
                        self.set_outcome(
                            OutcomeSeverity::Info,
                            format!("Already up to date with {}.", pull.upstream_label),
                            None,
                            None,
                        );
                        return;
                    }
                    let started = match self.prefs.pull_strategy {
                        settings::PullStrategy::Rebase => {
                            self.start_rebase(ctx, &pull.upstream, false)
                        }
                        settings::PullStrategy::Merge => {
                            self.start_merge(ctx, &pull.upstream, false)
                        }
                        settings::PullStrategy::FastForward => {
                            self.start_fast_forward(ctx, &pull.upstream)
                        }
                    };
                    if let Err(error) = started {
                        self.set_outcome(OutcomeSeverity::Error, error, None, None);
                    }
                }
            }
            Ok(network::Outcome::Push(receipt)) => {
                self.network_error = None;
                self.network_notice = Some(if receipt.force_with_lease {
                    format!(
                        "Force-pushed {} to {} · lease {}",
                        receipt.branch,
                        receipt.upstream.as_deref().unwrap_or(&receipt.remote),
                        receipt
                            .lease
                            .as_deref()
                            .map(short_id)
                            .unwrap_or_else(|| "revalidated".into())
                    )
                } else {
                    match &receipt.upstream {
                        Some(upstream) => format!(
                            "Pushed {} to {} · upstream {upstream}",
                            receipt.branch, receipt.remote
                        ),
                        None => format!("Pushed {} to {}", receipt.branch, receipt.remote),
                    }
                });
                self.load(ctx);
            }
            Ok(network::Outcome::DeleteRemote(message)) => {
                self.network_error = None;
                self.network_notice = Some(message);
                self.load(ctx);
            }
            Ok(network::Outcome::Clone(receipt)) => {
                self.network_error = None;
                self.clear_operation_state();
                self.network_notice = Some(format!("Cloned {}", receipt.path.display()));
                if let Some(working) = &self.working {
                    self.working_drafts
                        .insert(working.repository_key(), working.capture_draft());
                }
                self.working = None;
                self.review = None;
                self.path = receipt.path.to_string_lossy().into_owned();
                self.state.history = None;
                self.state.rows.clear();
                self.state.graphs.clear();
                self.state.details = None;
                self.state.selected = None;
                self.state.selected_row = None;
                self.state.refs = Default::default();
                self.refs_query.clear();
                self.recovery = None;
                self.load(ctx);
            }
            Err(error) => {
                if matches!(operation, Some(gg_engine::sync::Operation::Fetch)) {
                    self.pull_rebase = None;
                }
                let repository = !matches!(operation, Some(gg_engine::sync::Operation::Clone))
                    && self.state.history.is_some();
                let cancelled = error.strip_suffix(gg_git::sync::CANCELLED);
                let error = match cancelled {
                    // Git was stopped mid-way and part of it may have run:
                    // re-read refs, history and status before Retry.
                    Some(label) if repository => format!(
                        "{label} cancelled. Git Good re-read the repository; check what changed before retrying."
                    ),
                    _ if force_push
                        && matches!(operation, Some(gg_engine::sync::Operation::Push)) =>
                    {
                        format!("Force push with lease failed: {error}")
                    }
                    _ => error,
                };
                if repository {
                    self.load(ctx);
                    self.refresh_working(ctx);
                }
                let recovery = match operation {
                    Some(gg_engine::sync::Operation::Push) if !force_push => {
                        push_failure_follow_up(&error)
                    }
                    Some(gg_engine::sync::Operation::Fetch) => Some(RecoveryAction::RetryFetch),
                    _ => None,
                };
                // The first line is the status line; Git's output sits behind Details….
                let (summary, details) = match error.split_once('\n') {
                    Some((summary, details)) if !details.trim().is_empty() => {
                        (summary.to_string(), Some(details.trim().to_string()))
                    }
                    _ => (error.trim().to_string(), None),
                };
                self.set_outcome(OutcomeSeverity::Error, summary, details, recovery);
                self.network_error = None;
            }
        }
    }

    fn stash_path(&self) -> Option<PathBuf> {
        self.state
            .history
            .as_ref()
            .map(|history| history.path.clone())
    }

    fn push_stash(
        &mut self,
        ctx: &egui::Context,
        message: &str,
        include_untracked: bool,
        keep_index: bool,
    ) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before stashing changes.")?;
        let message = (!message.trim().is_empty()).then_some(message.trim());
        let receipt = gg_git::stash::push(&path, message, include_untracked, keep_index)?;
        self.stash_notice = Some(format!(
            "Stashed{}",
            receipt
                .reference
                .as_deref()
                .map(|reference| format!(" {reference}"))
                .unwrap_or_default()
        ));
        self.load(ctx);
        self.refresh_working(ctx);
        Ok(())
    }

    fn restore_stash_inner(
        &mut self,
        ctx: &egui::Context,
        reference: &str,
        remove: bool,
        restore_index: bool,
    ) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before restoring a stash.")?;
        let receipt = if remove {
            gg_git::stash::pop(&path, Some(reference), restore_index)?
        } else {
            gg_git::stash::apply(&path, Some(reference), restore_index)?
        };
        let summary = if receipt.conflicts.is_empty() {
            format!("Restored {reference}")
        } else {
            format!(
                "Restored {reference} · {}",
                operation_band::plural(receipt.conflicts.len(), "conflict")
            )
        };
        self.stash_notice = Some(summary.clone());
        let conflicts = !receipt.conflicts.is_empty();
        self.set_outcome(
            if conflicts {
                OutcomeSeverity::Warning
            } else {
                OutcomeSeverity::Info
            },
            summary,
            conflicts.then(|| receipt.conflicts.join("\n")),
            conflicts.then_some(RecoveryAction::ManageStashes),
        );
        self.load(ctx);
        self.refresh_working(ctx);
        Ok(())
    }

    fn drop_stash(&mut self, ctx: &egui::Context, reference: &str) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before dropping a stash.")?;
        gg_git::stash::drop(&path, Some(reference))?;
        self.stash_notice = Some(format!("Dropped {reference}"));
        self.load(ctx);
        self.refresh_working(ctx);
        Ok(())
    }

    fn reflog_references(&self) -> Vec<String> {
        let mut references = vec!["HEAD".to_string()];
        references.extend(
            self.state
                .refs
                .items
                .iter()
                .filter(|item| item.kind == gg_engine::refs::RefKind::Branch)
                .map(|item| item.name.clone()),
        );
        references.sort();
        references.dedup();
        references
    }

    fn refresh_reflog(&mut self) {
        if let Some(view) = &mut self.recovery {
            view.reread();
        }
    }

    fn refresh_backups(&mut self) {
        if let Some(view) = &mut self.recovery {
            view.reread();
        }
    }

    fn refresh_working(&mut self, ctx: &egui::Context) {
        if let Some(working) = &mut self.working {
            working.refresh(ctx);
        }
        self.refresh_local_summary(ctx);
    }

    /// Runs a toolbar or search command. Readiness is checked by the caller,
    /// which shows the same disabled reason as the menus.
    fn run_frame_command(
        &mut self,
        ctx: &egui::Context,
        command: FrameCommand,
        go_local: &mut bool,
        go_history: &mut bool,
    ) {
        self.refresh_before_command(ctx);
        match command {
            FrameCommand::Fetch => self.start_fetch(ctx, None, self.prefs.fetch_prune),
            FrameCommand::Pull => self.start_pull_rebase(ctx),
            FrameCommand::Push => self.start_push(ctx),
            FrameCommand::ForcePush => {
                if let Some(target) = self.force_push_target() {
                    self.force_push = Some(target);
                }
            }
            FrameCommand::NewBranch => self.open_new_branch(),
            FrameCommand::NewTag => self.open_new_tag(),
            FrameCommand::TrackBranch => self.track_dialog = Some(self.default_track_branch()),
            FrameCommand::Merge => self.open_merge_dialog(),
            FrameCommand::RebaseOnto => {
                self.rebase_dialog = Some(RebaseDialog {
                    reference: String::new(),
                    autostash: self.prefs.rebase_autostash,
                    error: None,
                });
            }
            FrameCommand::Stash => self.open_stash_dialog(),
            // The picker is the one stash surface (proposal).
            FrameCommand::Stashes => {
                self.stash_picker.confirm_drop = false;
                self.stash_picker.error = None;
                egui::Popup::open_id(ctx, stash_picker_popup_id());
            }
            FrameCommand::Rewrite => {
                let start = self
                    .local_summary
                    .as_ref()
                    .and_then(|summary| summary.branch_start.clone());
                if let Some((start, _)) = start
                    && let Err(error) = self.open_rewrite_dialog(&start)
                {
                    self.mutation_error = Some(error);
                }
            }
            FrameCommand::RewriteSelected => {
                if let Some(id) = self.state.selected.clone() {
                    self.run_commit_action(ctx, id, CommitAction::Rewrite);
                }
            }
            // Reopens on the source last shown; Backups the first time.
            FrameCommand::Recovery => self.open_recovery_window(ctx),
            FrameCommand::Worktrees => self.open_worktrees(ctx),
            FrameCommand::Submodules => self.open_submodule_list(ctx),
            FrameCommand::Lfs => self.open_lfs_list(ctx),
            FrameCommand::Open => self.edit_path = true,
            FrameCommand::NewRepository(folder) => {
                self.new_repository = Some(new_repository::Dialog::new(folder));
            }
            FrameCommand::Clone => {
                let mut dialog =
                    CloneDialog::from_url(clipboard::repository_url(), self.clone_start_folder());
                if !self.prefs.clone_named_after_repo {
                    dialog.name_from_url = false;
                    dialog.name.clear();
                }
                self.clone_dialog = Some(dialog);
            }
            FrameCommand::Preferences => {
                let section = self
                    .folders_dialog
                    .as_ref()
                    .map_or(PrefSection::General, |dialog| dialog.section);
                self.open_preferences(section);
            }
            FrameCommand::Shortcuts => self.help_shortcuts = true,
            FrameCommand::LocalChanges => *go_local = true,
            FrameCommand::History => *go_history = true,
            FrameCommand::ToggleRefs => self.toggle_refs_or_drawer(ctx),
            FrameCommand::Filter(query) => {
                self.state.query = query;
                self.filter_changed();
                *go_history = true;
            }
            FrameCommand::Inspect(commit) => {
                self.jump_to_ref(commit);
                *go_history = true;
            }
            FrameCommand::OpenFile(file) => self.open_file(ctx, file),
            FrameCommand::ClearRecent => {
                self.recent.clear();
                if let Err(error) = settings::save_recent(&mut self.recent, &mut self.recent_base) {
                    self.set_outcome(OutcomeSeverity::Warning, error, None, None);
                }
            }
            FrameCommand::OpenRecent(path) => {
                self.path = path.to_string_lossy().into_owned();
                self.load(ctx);
            }
            FrameCommand::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            FrameCommand::SwitchTo => egui::Popup::open_id(ctx, branch_switcher_popup_id()),
            FrameCommand::SetUpstream => {
                if let Some(branch) = self.current_branch() {
                    let upstream = self
                        .state
                        .refs
                        .current_tracking()
                        .map(|tracking| tracking.upstream.clone())
                        .unwrap_or_default();
                    self.upstream_dialog = Some(UpstreamDialog {
                        branch,
                        upstream,
                        error: None,
                    });
                }
            }
            FrameCommand::RenameBranch => {
                if let Some(branch) = self.current_branch() {
                    self.rename_dialog = Some(RenameDialog {
                        new: branch.clone(),
                        old: branch,
                        error: None,
                    });
                }
            }
            FrameCommand::DeleteBranch => {
                let current = self.current_branch();
                if let Some(name) = self.state.refs.items.iter().find_map(|item| {
                    (item.kind == gg_engine::refs::RefKind::Branch
                        && Some(&item.name) != current.as_ref())
                    .then(|| item.name.clone())
                }) {
                    self.delete_pick = Some(DeletePick { name });
                }
            }
            FrameCommand::ReviewSelected => self.on_selected(ctx, CommitAction::Review),
            FrameCommand::CompareHead => self.on_selected(ctx, CommitAction::Compare),
            FrameCommand::CherryPick => self.on_selected(ctx, CommitAction::CherryPick),
            FrameCommand::Revert => self.on_selected(ctx, CommitAction::Revert),
            FrameCommand::ResetHere => {
                self.on_selected(ctx, CommitAction::Reset(gg_engine::reset::Mode::Mixed));
            }
            FrameCommand::CopyId => {
                if let Some(id) = &self.state.selected {
                    ctx.copy_text(id.clone());
                }
            }
            FrameCommand::RebasePreset(action) => self.rebase_preset(ctx, action),
            FrameCommand::StageAll => self.change_all(ctx, gg_git::worktree_changes::stage_all),
            FrameCommand::UnstageAll => {
                self.change_all(ctx, gg_git::worktree_changes::unstage_all);
            }
            FrameCommand::DiscardAll => self.discard_confirm = true,
            FrameCommand::CommitNow => {
                *go_local = false;
                self.open_working(ctx);
                if let Some(working) = &mut self.working {
                    working.commit_now(ctx);
                }
            }
            FrameCommand::AmendHead => {
                self.open_working(ctx);
                if let Some(working) = &mut self.working
                    && let Err(error) = working.start_amend(ctx)
                {
                    self.mutation_error = Some(error);
                }
            }
            FrameCommand::ResolveConflicts => self.open_conflict_view(ctx),
            FrameCommand::ToggleDetails => {
                if self.narrow_details_mode() {
                    self.narrow_details = !self.narrow_details;
                } else {
                    self.details_visible = !self.details_visible;
                }
            }
            FrameCommand::SetDensity(density) => {
                self.prefs.density = density;
                self.save_prefs(ctx);
            }
            FrameCommand::SetDiffLayout(layout) => {
                settings::set_diff_layout(ctx, layout);
                settings::choose_reader_layout(ctx, layout);
                self.remember(|prefs| prefs.diff_layout = layout);
            }
            FrameCommand::SetTheme(theme) => {
                self.prefs.theme = theme;
                self.save_prefs(ctx);
            }
            FrameCommand::Documentation => {
                ctx.open_url(egui::OpenUrl::new_tab("https://github.com/baol/gg#readme"));
            }
            FrameCommand::About => self.about_open = true,
            FrameCommand::Unavailable => {}
        }
    }

    /// Menu shortcuts, read before any view so Ctrl+Shift chords win over
    /// their Ctrl twins (Ctrl+Shift+F over the filter's Ctrl+F). Each runs
    /// only when its menu item is enabled.
    fn menu_shortcuts(&mut self, ctx: &egui::Context) {
        bounded_text::disclosure_keys(ctx);
        // The non-modal help and details windows take Esc before any view.
        if (self.help_shortcuts || self.about_open || self.outcome_details_open)
            && ctx.memory(|m| m.top_modal_layer()).is_none()
            && !egui::Popup::is_any_open(ctx)
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            if self.outcome_details_open {
                self.outcome_details_open = false;
            } else if self.help_shortcuts {
                self.help_shortcuts = false;
            } else {
                self.about_open = false;
            }
        }
        if !crate::navigation::available(ctx) {
            return;
        }
        let shift = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
        let typing = ctx.text_edit_focused();
        let chords = [
            (shift, egui::Key::F, FrameCommand::Fetch),
            (shift, egui::Key::L, FrameCommand::Pull),
            (shift, egui::Key::P, FrameCommand::Push),
            (shift, egui::Key::B, FrameCommand::SwitchTo),
            (shift, egui::Key::Z, FrameCommand::Stash),
            (shift, egui::Key::S, FrameCommand::StageAll),
            (shift, egui::Key::U, FrameCommand::UnstageAll),
            (egui::Modifiers::COMMAND, egui::Key::O, FrameCommand::Open),
            (
                egui::Modifiers::COMMAND,
                egui::Key::B,
                FrameCommand::NewBranch,
            ),
            (egui::Modifiers::COMMAND, egui::Key::Q, FrameCommand::Quit),
            (
                egui::Modifiers::COMMAND,
                egui::Key::R,
                FrameCommand::RebasePreset(gg_engine::rewrite::Action::Reword),
            ),
        ];
        let mut chosen = None;
        for (modifiers, key, command) in chords {
            if chosen.is_none() && ctx.input_mut(|i| i.consume_key(modifiers, key)) {
                chosen = Some(command);
            }
        }
        if chosen.is_none() && !typing {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
                chosen = Some(FrameCommand::RenameBranch);
            } else if !egui::Popup::is_any_open(ctx)
                && self.working.is_none()
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete))
            {
                // Del deletes a branch from History; popups and Local Changes
                // keep the key for their own lists.
                chosen = Some(FrameCommand::DeleteBranch);
            } else if ctx
                .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Questionmark))
            {
                chosen = Some(FrameCommand::Shortcuts);
            }
        }
        let Some(command) = chosen else {
            return;
        };
        let ready = self.frame_readiness();
        let enabled = menus::flatten(&self.menu_table(&ready))
            .into_iter()
            .any(|(_, item)| item.command == command && item.disabled.is_none());
        if enabled {
            let (mut local, mut history) = (false, false);
            self.run_frame_command(ctx, command, &mut local, &mut history);
            if local {
                self.open_working(ctx);
            }
        }
    }

    /// Dialogs opened from the Branch, Changes, and Help menus.
    fn draw_menu_dialogs(&mut self, ctx: &egui::Context) {
        let path = self.stash_path();
        let mut reload = false;
        if let Some(dialog) = &mut self.rename_dialog {
            let mut open = true;
            let mut cancel = false;
            let mut submit = false;
            egui::Window::new("Rename branch")
                .id(egui::Id::unique("rename-branch"))
                .modal(ctx, egui::Id::unique("rename-branch"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(format!("Rename {} to:", dialog.old));
                    let field =
                        ui.add(egui::TextEdit::singleline(&mut dialog.new).desired_width(320.0));
                    submit |= field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.weak("The upstream setting moves with the branch.");
                    if let Some(error) = &dialog.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.horizontal(|ui| {
                        submit |= ui
                            .add_enabled(
                                !dialog.new.trim().is_empty() && dialog.new.trim() != dialog.old,
                                egui::Button::new("Rename"),
                            )
                            .clicked();
                        cancel |= ui.button("Cancel").clicked();
                    });
                });
            if cancel {
                open = false;
            }
            if submit && let Some(path) = &path {
                match gg_git::branch::rename(path, &dialog.old, dialog.new.trim()) {
                    Ok(_) => {
                        open = false;
                        reload = true;
                    }
                    Err(error) => dialog.error = Some(error),
                }
            }
            if !open {
                self.rename_dialog = None;
            }
        }
        if let Some(dialog) = &mut self.upstream_dialog {
            let mut open = true;
            let mut cancel = false;
            let mut submit = false;
            let remotes = self
                .state
                .refs
                .items
                .iter()
                .filter(|item| item.kind == gg_engine::refs::RefKind::Remote)
                .map(|item| item.name.clone())
                .collect::<Vec<_>>();
            egui::Window::new("Set upstream")
                .id(egui::Id::unique("set-upstream"))
                .modal(ctx, egui::Id::unique("set-upstream"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(format!("Upstream for {}:", dialog.branch));
                    egui::ComboBox::from_id_salt("upstream-choice")
                        .width(320.0)
                        .selected_text(if dialog.upstream.is_empty() {
                            "None".to_string()
                        } else {
                            dialog.upstream.clone()
                        })
                        .show_ui(ui, |ui| {
                            crate::menus::choose(ui, &mut dialog.upstream, String::new(), "None");
                            for name in &remotes {
                                crate::menus::choose(ui, &mut dialog.upstream, name.clone(), name);
                            }
                        });
                    ui.weak("Pull, push, and the ahead/behind counts follow the upstream.");
                    if let Some(error) = &dialog.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.horizontal(|ui| {
                        submit |= ui.button("Set upstream").clicked();
                        cancel |= ui.button("Cancel").clicked();
                    });
                });
            if cancel {
                open = false;
            }
            if submit && let Some(path) = &path {
                let result = if dialog.upstream.is_empty() {
                    gg_git::branch::unset_upstream(path, &dialog.branch)
                } else {
                    gg_git::branch::set_upstream(path, &dialog.branch, &dialog.upstream)
                };
                match result {
                    Ok(_) => {
                        open = false;
                        reload = true;
                    }
                    Err(error) => dialog.error = Some(error),
                }
            }
            if !open {
                self.upstream_dialog = None;
            }
        }
        if let Some(pick) = &mut self.delete_pick {
            let mut open = true;
            let mut cancel = false;
            let mut next = false;
            let current = self
                .state
                .refs
                .head
                .as_ref()
                .and_then(|head| head.name.as_deref())
                .map(|name| name.strip_prefix("refs/heads/").unwrap_or(name).to_string());
            let branches = self
                .state
                .refs
                .items
                .iter()
                .filter(|item| {
                    item.kind == gg_engine::refs::RefKind::Branch
                        && Some(&item.name) != current.as_ref()
                })
                .map(|item| item.name.clone())
                .collect::<Vec<_>>();
            egui::Window::new("Delete branch")
                .id(egui::Id::unique("delete-branch-pick"))
                .modal(ctx, egui::Id::unique("delete-branch-pick"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label("Local branch to delete:");
                    egui::ComboBox::from_id_salt("delete-branch-choice")
                        .width(320.0)
                        .selected_text(pick.name.clone())
                        .show_ui(ui, |ui| {
                            for name in &branches {
                                crate::menus::choose(ui, &mut pick.name, name.clone(), name);
                            }
                        });
                    ui.weak("The current branch cannot be deleted. The next step confirms.");
                    ui.horizontal(|ui| {
                        next |= ui.button("Delete…").clicked();
                        cancel |= ui.button("Cancel").clicked();
                    });
                });
            if cancel {
                open = false;
            }
            if next {
                self.branch_delete = Some(BranchDeleteDialog {
                    name: pick.name.clone(),
                    error: None,
                });
                open = false;
            }
            if !open {
                self.delete_pick = None;
            }
        }
        const DISCARD_SLOTS: [&str; 2] = ["discard", "discard-head"];
        if !self.discard_confirm {
            for slot in DISCARD_SLOTS {
                self.consequences.remove(slot);
            }
            self.guards.remove("discard");
        }
        if self.discard_confirm {
            let mut open = true;
            let mut cancel = false;
            let mut confirm = false;
            // Discard only when a re-read on confirm matches what was shown.
            let mut guard = self.guards.remove("discard").unwrap_or_default();
            let run = guard.poll(&mut self.consequences) == Some(true);
            let mut files =
                self.take_consequence("discard", &["status", "--porcelain=v1", "-uno"], ctx);
            let mut head = self.take_consequence("discard-head", &["rev-parse", "HEAD"], ctx);
            let count = files.as_mut().and_then(consequence::Lines::count);
            let head_ready = head.as_mut().is_none_or(|head| head.get().is_some());
            let notice = guard.changed.then_some(consequence::CHANGED);
            let busy = guard.busy();
            egui::Window::new("Discard all tracked changes")
                .id(egui::Id::unique("discard-all"))
                .modal(ctx, egui::Id::unique("discard-all"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.set_width(460.0);
                    ui.label(
                        "Restore every tracked file to HEAD? Staged and unstaged changes are both lost.",
                    );
                    if let Some(files) = &mut files {
                        consequence::list(
                            ui,
                            "WILL BE RESTORED TO HEAD",
                            files,
                            egui::Color32::from_rgb(220, 195, 125),
                        );
                    }
                    ui.weak(
                        "This cannot be undone. Untracked files are kept; files only added to the index become untracked.",
                    );
                    consequence::notice(ui, notice);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let label = match count {
                                Some(count) => format!(
                                    "Discard {}, staged and unstaged",
                                    consequence::counted(count, "file", "files")
                                ),
                                None => "Discard tracked changes".into(),
                            };
                            let ready = count.is_some_and(|count| count > 0) && head_ready;
                            let enabled = ready && !busy;
                            confirm |= ui
                                .add_enabled(enabled, consequence::danger_button(label, ready))
                                .clicked();
                            cancel |= ui
                                .button(("Cancel", egui::RichText::new("Esc").small().weak()))
                                .clicked();
                        });
                    });
                    if count.is_some_and(|count| count > 0)
                        && head_ready
                        && !busy
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        confirm = true;
                    }
                });
            for (slot, lines) in DISCARD_SLOTS.into_iter().zip([files, head]) {
                if let Some(lines) = lines {
                    self.consequences.insert(slot, lines);
                }
            }
            if confirm {
                guard.confirm(&self.consequences, &DISCARD_SLOTS, ctx);
            }
            self.guards.insert("discard", guard);
            if cancel {
                open = false;
            }
            if run {
                self.discard_confirm = false;
                self.change_all(ctx, gg_git::worktree_changes::discard_all);
            } else if !open {
                self.discard_confirm = false;
            }
        }
        if self.about_open && branding::show_about(ctx, &mut self.about_logo) {
            self.about_open = false;
        }
        if reload {
            self.load(ctx);
        }
    }

    /// Refresh what changed on disk: the history and refs when HEAD, a ref,
    /// or an operation changed (unless a load already started after the
    /// change), otherwise the worktree status and the frame summary.
    fn apply_changes(
        &mut self,
        ctx: &egui::Context,
        changes: watcher::Changes,
        first: std::time::Instant,
    ) {
        if changes.refs && self.last_load.is_none_or(|load| load < first) {
            self.load(ctx);
            self.refresh_working(ctx);
        } else if changes.any() {
            self.refresh_mutation();
            self.refresh_working(ctx);
        }
    }

    /// Before a command runs, apply changes the watcher has seen but not yet
    /// settled, and re-read the operation state, so the command acts on the
    /// repository as it is now.
    fn refresh_before_command(&mut self, ctx: &egui::Context) {
        if let Some((changes, first)) = self.watcher.as_mut().and_then(|watcher| watcher.flush()) {
            self.apply_changes(ctx, changes, first);
        }
        self.refresh_mutation();
    }

    /// Delete old private backups in the background, by Preferences ›
    /// Rebase & recovery › Keep private backups; off by default.
    fn prune_backups(&self, path: &Path) {
        if self.prefs.backup_days == 0 || cfg!(test) {
            return;
        }
        let path = path.to_path_buf();
        let age = std::time::Duration::from_secs(u64::from(self.prefs.backup_days) * 86_400);
        let _ = std::thread::Builder::new()
            .name("gg-prune-backups".into())
            .spawn(move || {
                let _ = gg_git::backup::prune(&path, age);
            });
    }

    /// Open a repository-relative path's working file in the editor from
    /// Preferences; a failure goes to the status line.
    fn open_in_editor(&mut self, request: &editor::Request) {
        let Some(workdir) = self.stash_path() else {
            return;
        };
        let file = workdir.join(&request.path);
        let result = if !file.exists() {
            Err(format!("{} is not in the working tree.", request.path))
        } else if request.folder {
            editor::reveal(&file)
        } else {
            editor::open(&self.prefs.editor, &file)
        };
        if let Err(error) = result {
            self.set_outcome(OutcomeSeverity::Error, error, None, None);
        }
    }

    /// The Commit menu's entries, which the Commit actions popup mirrors.
    fn commit_menu_entries(&self) -> Vec<menus::Entry<FrameCommand>> {
        let ready = self.frame_readiness();
        self.menu_table(&ready)
            .into_iter()
            .find(|menu| menu.title == "Commit")
            .map(|menu| menu.entries)
            .unwrap_or_default()
    }

    /// A commit named with more than its hash, wherever gg shows one:
    /// `0f99a42b · subject · author · date` from the loaded history, or
    /// just the 8-character ID when the commit is not loaded.
    fn commit_summary(&self, id: &str) -> String {
        let short = &id[..id.len().min(8)];
        let commit = self
            .state
            .details
            .as_ref()
            .filter(|commit| commit.id == id)
            .or_else(|| self.state.rows.values().find(|commit| commit.id == id));
        match commit {
            Some(commit) => format!(
                "{short} · {} · {} · {}",
                commit.subject(),
                commit.author,
                commit.time.split_whitespace().next().unwrap_or_default()
            ),
            None => short.to_string(),
        }
    }

    /// The checked-out branch, unless HEAD is detached.
    fn current_branch(&self) -> Option<String> {
        self.state
            .refs
            .head
            .as_ref()
            .filter(|head| !head.detached)
            .and_then(|head| head.name.as_deref())
            .map(|name| name.strip_prefix("refs/heads/").unwrap_or(name).to_string())
    }

    fn on_selected(&mut self, ctx: &egui::Context, action: CommitAction) {
        if let Some(id) = self.state.selected.clone() {
            self.run_commit_action(ctx, id, action);
        }
    }

    /// Apply and save a preference changed from a menu.
    fn save_prefs(&mut self, ctx: &egui::Context) {
        self.apply_prefs(ctx);
        if let Err(error) = self.prefs.save() {
            self.mutation_error = Some(error);
        }
    }

    /// Commit › Reword / Reopen / Fixup / Drop: the interactive rebase dialog
    /// with the selected commit preset, so the plan stays reviewable.
    fn rebase_preset(&mut self, ctx: &egui::Context, action: gg_engine::rewrite::Action) {
        let Some(id) = self.state.selected.clone() else {
            return;
        };
        // Fixup needs the commit below in the range.
        let from = if action == gg_engine::rewrite::Action::Fixup {
            match self
                .state
                .displayed_details()
                .and_then(|commit| commit.parents.first().cloned())
            {
                Some(parent) => parent,
                None => return,
            }
        } else {
            id.clone()
        };
        if let Err(error) = self.open_rewrite_dialog(&from) {
            self.mutation_error = Some(error);
            return;
        }
        if let Some(dialog) = &mut self.rewrite_dialog {
            dialog.preset(&id, action);
        }
        // Confirm before Drop off: run the drop without showing the plan.
        if action == gg_engine::rewrite::Action::Drop
            && !self.prefs.confirm_destructive
            && let Some(rebase_dialog::Outcome::Start {
                from,
                steps,
                autostash,
            }) = self
                .rewrite_dialog
                .as_ref()
                .and_then(|dialog| dialog.start_now())
        {
            match self.begin_rewrite(ctx, from, steps, autostash) {
                Ok(()) => self.rewrite_dialog = None,
                Err(error) => {
                    if let Some(dialog) = &mut self.rewrite_dialog {
                        dialog.error = Some(error);
                    }
                }
            }
        }
    }

    /// Stage or unstage every change, then refresh what shows them.
    fn change_all(&mut self, ctx: &egui::Context, change: fn(&Path) -> Result<(), String>) {
        let Some(path) = self.stash_path() else {
            return;
        };
        match change(&path) {
            Ok(()) => {
                self.refresh_working(ctx);
                self.refresh_local_summary(ctx);
            }
            Err(error) => self.mutation_error = Some(error),
        }
    }

    /// Open the latest commit that changed `file` in Review, on that file.
    fn open_file(&mut self, ctx: &egui::Context, file: String) {
        let Some(path) = self.stash_path() else {
            return;
        };
        match gg_git::files::last_change(&path, &file) {
            Ok(Some((commit, subject))) => {
                if self.working.is_some() {
                    self.close_to_history(ctx);
                }
                let location = self.capture_return_location();
                self.open_review_at(ctx, commit, subject, Some(location));
                if let Some(review) = self.review.take() {
                    self.review = Some(review.select_path(file));
                }
            }
            Ok(None) => {
                self.mutation_error = Some(format!("No commit in this history changes {file}."))
            }
            Err(error) => self.mutation_error = Some(error),
        }
    }

    /// Applies an edited history filter, as typing in the filter field does.
    fn filter_changed(&mut self) {
        if self.worker.is_some() || self.pending.is_some() {
            self.state.start_filter();
            self.pending_filter_edit = self.pending.is_some();
            self.staged = None;
            self.viewport = 0..64;
        } else {
            self.state.reconcile_selection();
        }
        self.scroll_to_selection = true;
    }

    /// Search items: every frame command with its menu home and readiness,
    /// then branches, remotes, and tags.
    /// The menu bar as one table: titles, items, shortcuts, and disabled
    /// reasons, shared with search (final design, action map).
    fn narrow_details_mode(&self) -> bool {
        self.ctx_width < 1040.0
    }

    /// Run what a refs sidebar or refs drawer row asked for.
    fn apply_refs_action(&mut self, ctx: &egui::Context, action: refs_view::Action) {
        // The action is applied after the frame that drew the click; draw
        // again now so a dialog or the job it starts shows at once instead
        // of waiting for the next mouse move.
        ctx.request_repaint();
        match action {
            refs_view::Action::OpenWorktree(path) => {
                let (mut local, mut history) = (false, false);
                self.run_frame_command(
                    ctx,
                    FrameCommand::OpenRecent(PathBuf::from(path)),
                    &mut local,
                    &mut history,
                );
            }
            refs_view::Action::Switch(name) => self.start_branch(ctx, name, false),
            refs_view::Action::SwitchRemote(name) => self.switch_remote(ctx, name),
            refs_view::Action::Jump(id) => self.jump_to_ref(id),
            refs_view::Action::OpenStash(index) => {
                self.stash_picker.selected = index;
                self.stash_picker.confirm_drop = false;
                self.stash_picker.error = None;
                egui::Popup::open_id(ctx, stash_picker_popup_id());
            }
            refs_view::Action::Merge(name) => {
                self.merge_dialog = Some(MergeDialog {
                    reference: name,
                    no_ff: self.prefs.merge_no_ff,
                    error: None,
                });
            }
            refs_view::Action::Rebase(name) => {
                self.rebase_dialog = Some(RebaseDialog {
                    reference: name,
                    autostash: self.prefs.rebase_autostash,
                    error: None,
                });
            }
            refs_view::Action::DeleteBranch(name) => {
                self.branch_delete = Some(BranchDeleteDialog { name, error: None });
            }
            refs_view::Action::DeleteTag(name) => {
                self.tag_delete = Some(TagDeleteDialog { name, error: None });
            }
            refs_view::Action::VerifyTag(name) => self.open_signature(ctx, name),
            refs_view::Action::Rename(name) => {
                self.rename_dialog = Some(RenameDialog {
                    new: name.clone(),
                    old: name,
                    error: None,
                });
            }
            refs_view::Action::SetUpstream(name) => {
                let upstream = self
                    .state
                    .refs
                    .tracking
                    .iter()
                    .find(|tracking| tracking.branch == name)
                    .map(|tracking| tracking.upstream.clone())
                    .unwrap_or_default();
                self.upstream_dialog = Some(UpstreamDialog {
                    branch: name,
                    upstream,
                    error: None,
                });
            }
            refs_view::Action::NewBranchAt(commit, name) => {
                self.create_branch = Some(CreateBranchDialog {
                    name: String::new(),
                    commit,
                    selector: name,
                    switch: self.prefs.branch_switch,
                    recovery_source: None,
                    error: None,
                });
            }
            refs_view::Action::Compare(commit) => {
                let location = self.capture_return_location();
                self.open_compare(ctx, commit, location);
            }
            refs_view::Action::Push => self.start_push(ctx),
            refs_view::Action::DeleteRemote(name, commit) => {
                self.delete_remote = Some((name, commit));
            }
        }
    }

    /// The refs sidebar as a 248 px drawer over the dimmed view (narrow
    /// proposal), for windows too narrow for the docked sidebar: opened by
    /// the toolbar's sidebar button or Ctrl+\, focus in its filter, Up/Down
    /// choose, Enter shows the ref in History and closes, Esc or a click
    /// outside closes.
    fn draw_refs_drawer(&mut self, ctx: &egui::Context) {
        if !self.refs_drawer {
            return;
        }
        if self.state.history.is_none() || ctx.content_rect().width() >= REFS_DOCK_WIDTH {
            self.refs_drawer = false;
            return;
        }
        let screen = ctx.content_rect();
        let body = egui::Rect::from_min_max(
            egui::pos2(
                screen.left(),
                screen.top() + self.menu_bar_height() + TOOLBAR_HEIGHT,
            ),
            egui::pos2(screen.right(), screen.bottom() - FOOTER_HEIGHT),
        );
        let drawer =
            egui::Rect::from_min_size(body.min, egui::vec2(REFS_DRAWER_WIDTH, body.height()));
        // A click outside the drawer (the dimmed view or the toolbar's
        // sidebar button) closes it; the click that opened it does not.
        let mut close = !self.refs_drawer_focus
            && !egui::Popup::is_any_open(ctx)
            && ctx.input(|i| {
                i.pointer.primary_clicked()
                    && i.pointer
                        .interact_pos()
                        .is_some_and(|p| !drawer.contains(p))
            });
        close |= ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Backslash));
        let rows = refs_view::cursor_rows(&self.state.refs, &self.refs_query);
        // The drawer's first frame only sizes it, so focus is requested
        // until the filter has it.
        if self.refs_drawer_focus {
            if ctx.memory(|m| m.has_focus(refs_view::filter_id())) {
                self.refs_drawer_focus = false;
            } else {
                ctx.memory_mut(|m| m.request_focus(refs_view::filter_id()));
            }
        }
        let (up, down, enter, escape) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Enter),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        });
        if self.refs_query != self.refs_drawer_query {
            self.refs_drawer_query = self.refs_query.clone();
            self.refs_drawer_cursor = 0;
        }
        if down {
            self.refs_drawer_cursor += 1;
        }
        if up {
            self.refs_drawer_cursor = self.refs_drawer_cursor.saturating_sub(1);
        }
        self.refs_drawer_cursor = self.refs_drawer_cursor.min(rows.len().saturating_sub(1));
        let cursor = rows.get(self.refs_drawer_cursor);
        refs_view::set_cursor(cursor.map(|item| item.full_name.clone()));
        let mut action = None;
        if enter && let Some(item) = cursor {
            action = Some(refs_view::Action::Jump(item.commit.clone()));
        }
        let refs = self.state.refs.clone();
        let busy = self.branch.is_some()
            || self.network.is_some()
            || self.mutation.is_some()
            || self.operation_busy();
        let stashes = self
            .local_summary
            .as_ref()
            .map(|summary| summary.stashes.clone())
            .unwrap_or_default();
        let worktrees = self
            .local_summary
            .as_ref()
            .map(|summary| summary.worktrees.clone())
            .unwrap_or_default();
        egui::Area::new(refs_drawer_id())
            .order(egui::Order::Foreground)
            .fixed_pos(drawer.min)
            .show(ctx, |ui| {
                // The view behind the drawer is dimmed: painted first on the
                // drawer's own layer, whose painter spans the window.
                ui.ctx().layer_painter(ui.layer_id()).rect_filled(
                    body,
                    0.0,
                    egui::Color32::from_black_alpha(110),
                );
                egui::Frame::side_top_panel(ui.style().as_ref())
                    .inner_margin(8.0)
                    .shadow(ui.visuals().popup_shadow)
                    .show(ui, |ui| {
                        let inner = drawer.size() - egui::vec2(16.0, 16.0);
                        ui.set_min_size(inner);
                        ui.set_max_size(inner);
                        ui.vertical(|ui| {
                            let footer = 20.0;
                            ui.allocate_ui(egui::vec2(inner.x, inner.y - footer), |ui| {
                                if let Some(chosen) = refs_view::show(
                                    ui,
                                    &refs,
                                    &stashes,
                                    &worktrees,
                                    &mut self.refs_query,
                                    busy,
                                    &self.state.ref_lanes,
                                ) {
                                    action = Some(chosen);
                                }
                            });
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new("Enter: show in history · Esc: close")
                                        .small()
                                        .weak(),
                                )
                                .truncate(),
                            );
                        });
                    });
            });
        refs_view::set_cursor(None);
        if let Some(action) = action {
            // Showing a ref in History closes the drawer; menu actions open
            // their dialogs over it, closed too.
            close = true;
            self.apply_refs_action(ctx, action);
        }
        if close || escape {
            self.refs_drawer = false;
            crate::navigation::focus_content(ctx);
        } else {
            // A modal layer keeps History's keys and clicks away while the
            // drawer stays open.
            let layer = egui::LayerId::new(egui::Order::Foreground, refs_drawer_id());
            ctx.memory_mut(|m| m.set_modal_layer(layer));
        }
    }

    /// Move the History selection `distance` rows (j/k, arrows, paging).
    fn step_selection(&mut self, ctx: &egui::Context, distance: i32) {
        if self.state.count == 0 {
            return;
        }
        if self.worker.is_some() {
            let row = self.state.selected_row.unwrap_or(0);
            let next = row
                .saturating_add_signed(distance as isize)
                .min(self.state.count - 1);
            self.state.select_row(next);
        } else {
            self.state.move_selection(distance);
        }
        self.details_file = 0;
        crate::navigation::focus_content(ctx);
        self.scroll_to_selection = true;
    }

    /// Review the selected commit from the narrow details, opened on `path`
    /// or the FILES row chosen there; Esc in Review returns to the details.
    fn review_from_details(&mut self, ctx: &egui::Context, path: Option<String>) {
        let shown = self
            .state
            .displayed_details()
            .map(|commit| commit.id.clone());
        let path = path.or_else(|| {
            self.commit_stats
                .as_ref()
                .filter(|(id, _)| Some(id) == shown.as_ref())
                .and_then(|(_, stats)| stats.rows.get(self.details_file))
                .map(|file| file.path.clone())
        });
        let location = self.capture_return_location();
        self.open_review(ctx, location);
        if let Some(path) = path
            && let Some(review) = self.review.take()
        {
            self.review = Some(review.select_path(path));
        }
    }

    /// File and line counts of the displayed commit, counted in the
    /// background; `None` until they arrive.
    fn selected_commit_stats(&mut self, ctx: &egui::Context) -> Option<commit_stats::Stats> {
        let id = self.state.displayed_details()?.id.clone();
        if let Some(worker) = &self.commit_stats_worker
            && let Some(stats) = worker.poll()
        {
            self.commit_stats = Some((worker.commit.clone(), stats));
            self.commit_stats_worker = None;
        }
        let known = self
            .commit_stats
            .as_ref()
            .filter(|(commit, _)| *commit == id)
            .map(|(_, stats)| stats.clone());
        let running = self
            .commit_stats_worker
            .as_ref()
            .is_some_and(|worker| worker.commit == id);
        if known.is_none()
            && !running
            && let Some(history) = &self.state.history
        {
            self.commit_stats_worker = Some(commit_stats::Worker::start(
                history.path.clone(),
                id,
                ctx.clone(),
            ));
        }
        known
    }

    fn menu_table(&self, ready: &FrameReadiness) -> Vec<menus::Menu<FrameCommand>> {
        use menus::{Entry, Menu, item};
        let busy = "Wait for the current operation to finish";
        let no_repo = "Open a repository first";
        let idle = |ok: bool| -> Option<&'static str> {
            if ok {
                None
            } else if ready.has_repo {
                Some(busy)
            } else {
                Some(no_repo)
            }
        };
        let selected = if !ready.has_repo {
            Some(no_repo)
        } else if self.state.selected.is_some() {
            None
        } else {
            Some("Select a commit in History first")
        };
        let mutate = |ok: bool| if ok { selected } else { Some(busy) };
        let head = self.state.refs.head.as_ref();
        let current = head
            .filter(|head| !head.detached)
            .and_then(|head| head.name.as_deref())
            .map(|name| name.strip_prefix("refs/heads/").unwrap_or(name).to_string());
        let on_branch = match current {
            Some(_) => idle(ready.branch_idle),
            None if ready.has_repo => Some("Switch to a local branch first"),
            None => Some(no_repo),
        };
        let other_branches = self.state.refs.items.iter().any(|item| {
            item.kind == gg_engine::refs::RefKind::Branch && Some(&item.name) != current.as_ref()
        });
        let delete = if !ready.has_repo {
            Some(no_repo)
        } else if other_branches {
            idle(ready.branch_idle)
        } else {
            Some("No other local branch to delete")
        };
        let force = if !ready.has_repo {
            Some(no_repo)
        } else if self.force_push_target().is_some() {
            idle(ready.network_idle)
        } else {
            Some("Force with lease requires an upstream remote-tracking ref; track and fetch first")
        };
        let changes = if ready.changes > 0 {
            idle(ready.has_repo && !self.working_mutation_pending())
        } else if ready.has_repo {
            Some("No local changes")
        } else {
            Some(no_repo)
        };
        let conflicts = self
            .mutation
            .as_ref()
            .is_some_and(|state| !state.conflicts.is_empty());
        let selected_parent = self
            .state
            .displayed_details()
            .and_then(|commit| commit.parents.first().cloned());
        let fixup = match (selected, &selected_parent) {
            (Some(reason), _) => Some(reason),
            (None, None) => Some("The first commit has nothing below it to combine into"),
            (None, Some(_)) => ready.rewrite_reason,
        };
        let rewrite = selected.or(ready.rewrite_reason);
        let recent = if self.recent.is_empty() {
            vec![
                item("No recent repositories", "", FrameCommand::Unavailable)
                    .unless(Some("Repositories you open are listed here"))
                    .entry(),
            ]
        } else {
            // Folder name first, the path after it; the open repository is
            // checked and a missing folder stays listed but disabled.
            let home = std::env::var("HOME").ok();
            let open = self
                .state
                .history
                .as_ref()
                .map(|history| history.path.clone());
            let mut entries = self
                .recent
                .iter()
                .map(|entry| &entry.path)
                .filter(|path| path.is_absolute())
                .take(settings::RECENT_LIMIT)
                .map(|path| {
                    let name = path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string());
                    let shown = window_title(&path.display().to_string(), home.as_deref(), None);
                    let missing = !path.exists();
                    item(
                        format!("{name}   {}", shown.trim_start_matches("Git Good — ")),
                        "",
                        FrameCommand::OpenRecent(path.clone()),
                    )
                    .checked(open.as_deref().is_some_and(|open| {
                        open == path.as_path()
                            || open.canonicalize().ok().as_deref() == Some(path.as_path())
                    }))
                    .unless(if missing {
                        Some("Not found: the folder is missing")
                    } else {
                        self.repository_switch_pending().then_some(busy)
                    })
                    .entry()
                })
                .collect::<Vec<_>>();
            entries.push(menus::Entry::Separator);
            entries.push(item("Clear recent", "", FrameCommand::ClearRecent).entry());
            entries
        };
        let quit = if cfg!(target_os = "macos") {
            "Cmd+Q"
        } else {
            "Ctrl+Q"
        };
        let mut repository = vec![
            item("Open…", "Ctrl+O", FrameCommand::Open)
                .unless(self.repository_switch_pending().then_some(busy))
                .entry(),
            item("Clone…", "", FrameCommand::Clone)
                .unless(
                    (self.network.is_some() || self.branch.is_some() || self.operation_busy())
                        .then_some(busy),
                )
                .hint("Clone a repository and open it; a URL on the clipboard pre-fills the form")
                .entry(),
            item("New repository…", "", FrameCommand::NewRepository(None))
                .unless(self.repository_switch_pending().then_some(busy))
                .hint("git init in a folder, optionally committing the files already there")
                .entry(),
            Entry::Submenu("Open recent".into(), recent),
            Entry::Separator,
            item("Fetch", "Ctrl+Shift+F", FrameCommand::Fetch)
                .unless(idle(ready.network_idle))
                .hint("Fetch the default remote without changing local branches")
                .entry(),
            item(
                if ready.behind > 0 {
                    format!(
                        "Pull {} ({})",
                        ready.behind,
                        self.prefs.pull_strategy.menu_label()
                    )
                } else {
                    format!("Pull ({})", self.prefs.pull_strategy.menu_label())
                },
                "Ctrl+Shift+L",
                FrameCommand::Pull,
            )
            .unless(ready.pull_reason)
            .hint("Fetch the upstream and rebase the current branch onto it")
            .entry(),
            item("Push", "Ctrl+Shift+P", FrameCommand::Push)
                .unless(idle(ready.network_idle))
                .hint("Push the current branch without force; set upstream when absent")
                .entry(),
            item("Push with force-with-lease…", "", FrameCommand::ForcePush)
                .unless(force)
                .entry(),
            Entry::Separator,
            item("Worktrees…", "", FrameCommand::Worktrees)
                .unless(idle(ready.has_repo))
                .entry(),
            item("Submodules…", "", FrameCommand::Submodules)
                .unless(idle(ready.has_repo))
                .hint("Every submodule with its recorded and checked-out commit")
                .entry(),
            item("LFS…", "", FrameCommand::Lfs)
                .unless(idle(ready.has_repo))
                .hint("LFS pointers in HEAD; fetch the missing objects together")
                .entry(),
            item("Recovery…", "", FrameCommand::Recovery)
                .unless(idle(ready.has_repo))
                .hint("Private backups taken before every rewrite, and the reflog")
                .entry(),
        ];
        if !cfg!(target_os = "macos") {
            repository.extend([
                Entry::Separator,
                item("Preferences…", "Ctrl+,", FrameCommand::Preferences).entry(),
                item("Quit", quit, FrameCommand::Quit).entry(),
            ]);
        }
        let mut table = Vec::new();
        if cfg!(target_os = "macos") {
            table.push(Menu {
                title: native_menu::APP_NAME,
                entries: vec![
                    item("Settings…", "Cmd+,", FrameCommand::Preferences).entry(),
                    Entry::Separator,
                    item("Quit", quit, FrameCommand::Quit).entry(),
                ],
            });
        }
        table.push(Menu {
            title: "Repository",
            entries: repository,
        });
        table.push(Menu {
            title: "Branch",
            entries: vec![
                item("New branch…", "Ctrl+B", FrameCommand::NewBranch)
                    .unless(idle(ready.branch_idle))
                    .entry(),
                item("New tag…", "", FrameCommand::NewTag)
                    .unless(idle(ready.branch_idle))
                    .entry(),
                // Same readiness as the toolbar's branch button it opens.
                item("Switch to…", "Ctrl+Shift+B", FrameCommand::SwitchTo)
                    .unless(idle(ready.has_repo))
                    .entry(),
                Entry::Separator,
                item("Merge into current…", "", FrameCommand::Merge)
                    .unless(on_branch)
                    .entry(),
                item("Rebase current onto…", "", FrameCommand::RebaseOnto)
                    .unless(on_branch)
                    .entry(),
                // The branch's own commits, independent of the selection.
                item(
                    "Interactive rebase of current branch…",
                    "",
                    FrameCommand::Rewrite,
                )
                .unless(ready.branch_rewrite_reason)
                .hint("Rewrite the commits the upstream, or else the default branch, lacks")
                .entry(),
                item("Set upstream…", "", FrameCommand::SetUpstream)
                    .unless(on_branch)
                    .entry(),
                item("Track remote branch…", "", FrameCommand::TrackBranch)
                    .unless(idle(ready.branch_idle))
                    .entry(),
                Entry::Separator,
                item("Rename…", "F2", FrameCommand::RenameBranch)
                    .unless(on_branch)
                    .entry(),
                item("Delete…", "Del", FrameCommand::DeleteBranch)
                    .unless(delete)
                    .danger()
                    .entry(),
            ],
        });
        table.push(Menu {
            title: "Commit",
            entries: vec![
                item("Review changes", "Enter", FrameCommand::ReviewSelected)
                    .unless(selected)
                    .entry(),
                item("Compare with HEAD", "", FrameCommand::CompareHead)
                    .unless(selected)
                    .entry(),
                item("Copy ID", "Ctrl+C", FrameCommand::CopyId)
                    .unless(selected)
                    .entry(),
                Entry::Separator,
                item("Cherry-pick onto HEAD", "", FrameCommand::CherryPick)
                    .unless(mutate(self.mutation.is_none()))
                    .entry(),
                item("Revert", "", FrameCommand::Revert)
                    .unless(mutate(self.mutation.is_none()))
                    .entry(),
                Entry::Separator,
                item(
                    "Interactive rebase from here…",
                    "R",
                    FrameCommand::RewriteSelected,
                )
                .unless(rewrite)
                .entry(),
                item(
                    "Reword…",
                    "Ctrl+R",
                    FrameCommand::RebasePreset(gg_engine::rewrite::Action::Reword),
                )
                .unless(rewrite)
                .entry(),
                item(
                    "Reopen for editing…",
                    "E",
                    FrameCommand::RebasePreset(gg_engine::rewrite::Action::Edit),
                )
                .unless(rewrite)
                .entry(),
                item(
                    "Fixup into…",
                    "F",
                    FrameCommand::RebasePreset(gg_engine::rewrite::Action::Fixup),
                )
                .unless(fixup)
                .entry(),
                item(
                    "Drop…",
                    "D",
                    FrameCommand::RebasePreset(gg_engine::rewrite::Action::Drop),
                )
                .unless(rewrite)
                .danger()
                .entry(),
                Entry::Separator,
                item("Reset current branch to here…", "", FrameCommand::ResetHere)
                    .unless(mutate(self.mutation.is_none()))
                    .danger()
                    .entry(),
            ],
        });
        table.push(Menu {
            title: "Changes",
            entries: vec![
                item("Stage all", "Ctrl+Shift+S", FrameCommand::StageAll)
                    .unless(changes)
                    .entry(),
                item("Unstage all", "Ctrl+Shift+U", FrameCommand::UnstageAll)
                    .unless(changes.or_else(|| {
                        self.working
                            .as_ref()
                            .filter(|working| working.amending())
                            .map(|_| status_view::UNSTAGE_ALL_AMENDING)
                    }))
                    .entry(),
                item("Discard all tracked changes…", "", FrameCommand::DiscardAll)
                    .unless(changes)
                    .hint(
                        "Restore tracked files, staged and unstaged, to HEAD; untracked files are kept",
                    )
                    .danger()
                    .entry(),
                Entry::Separator,
                item("Commit", "Ctrl+Enter", FrameCommand::CommitNow)
                    .unless(idle(ready.has_repo))
                    .hint("Commit the staged changes with the message in Local Changes")
                    .entry(),
                item("Amend HEAD", "Ctrl+Shift+Enter", FrameCommand::AmendHead)
                    .unless(idle(ready.has_repo))
                    .entry(),
                Entry::Separator,
                item("Stash changes…", "Ctrl+Shift+Z", FrameCommand::Stash)
                    .unless(ready.stash_reason)
                    .entry(),
                item("Stashes…", "", FrameCommand::Stashes)
                    .unless(
                        idle(ready.stash_idle)
                            .or((ready.stashes == 0).then_some("No stashes")),
                    )
                    .hint("Apply, pop, show, or drop a stash entry")
                    .entry(),
                Entry::Separator,
                item("Resolve conflicts…", "", FrameCommand::ResolveConflicts)
                    .unless(if ready.has_repo {
                        (!conflicts).then_some("No conflicts to resolve")
                    } else {
                        Some(no_repo)
                    })
                    .entry(),
            ],
        });
        let details = if self.narrow_details_mode() {
            self.narrow_details
        } else {
            self.details_visible
        };
        let sidebar = self.ctx_width >= 1120.0;
        table.push(Menu {
            title: "View",
            entries: vec![
                item("Local Changes", "Ctrl+1", FrameCommand::LocalChanges)
                    .unless(idle(ready.has_repo))
                    .checked(self.working.is_some())
                    .entry(),
                item("History", "Ctrl+2", FrameCommand::History)
                    .unless(idle(ready.has_repo))
                    .checked(self.working.is_none() && self.review.is_none())
                    .entry(),
                Entry::Separator,
                // Narrower than the docked sidebar, it opens the drawer.
                item("Refs sidebar", "Ctrl+\\", FrameCommand::ToggleRefs)
                    .unless((!ready.has_repo).then_some(no_repo))
                    .checked((self.refs_shown() && sidebar) || self.refs_drawer)
                    .entry(),
                item("Commit inspector", "Space", FrameCommand::ToggleDetails)
                    .unless(idle(ready.has_repo))
                    .checked(details)
                    .entry(),
                Entry::Separator,
                item(
                    "Inline diff",
                    "",
                    FrameCommand::SetDiffLayout(settings::DiffLayout::Inline),
                )
                .checked(self.prefs.diff_layout == settings::DiffLayout::Inline)
                .entry(),
                item(
                    "Side-by-side diff",
                    "",
                    FrameCommand::SetDiffLayout(settings::DiffLayout::SideBySide),
                )
                .checked(self.prefs.diff_layout == settings::DiffLayout::SideBySide)
                .entry(),
                Entry::Separator,
                item(
                    "Compact density",
                    "",
                    FrameCommand::SetDensity(match self.prefs.density {
                        settings::Density::Compact => settings::Density::Comfortable,
                        settings::Density::Comfortable => settings::Density::Compact,
                    }),
                )
                .checked(self.prefs.density == settings::Density::Compact)
                .entry(),
                Entry::Submenu(
                    format!("Theme: {}", self.prefs.theme.label()),
                    settings::Theme::ALL
                        .into_iter()
                        .map(|theme| {
                            item(theme.label(), "", FrameCommand::SetTheme(theme))
                                .checked(self.prefs.theme == theme)
                                .entry()
                        })
                        .collect(),
                ),
            ],
        });
        table.push(Menu {
            title: "Help",
            entries: vec![
                item("Keyboard shortcuts", "?", FrameCommand::Shortcuts).entry(),
                item("Documentation", "", FrameCommand::Documentation)
                    .hint("Open the Git Good README in the browser")
                    .entry(),
                item("About Git Good", "", FrameCommand::About).entry(),
            ],
        });
        table
    }

    fn search_items(&self, ready: &FrameReadiness) -> Vec<search::Item<FrameCommand>> {
        use search::Group;
        // Every menu item, named with its menu; recent repositories form
        // their own group, named with their folder.
        let home = std::env::var("HOME").ok();
        let mut items = menus::flatten(&self.menu_table(ready))
            .into_iter()
            .filter(|(_, item)| item.command != FrameCommand::Unavailable)
            .map(|(path, item)| match &item.command {
                FrameCommand::OpenRecent(folder) => search::Item {
                    group: Group::Repositories,
                    label: folder
                        .file_name()
                        .map_or(item.label, |name| name.to_string_lossy().into_owned()),
                    context: welcome_view::tilde(folder, home.as_deref()),
                    key: "",
                    disabled: item.disabled,
                    command: item.command,
                },
                _ => search::Item {
                    group: Group::Actions,
                    label: item.label,
                    context: format!("{path} menu"),
                    key: item.key,
                    disabled: item.disabled,
                    command: item.command,
                },
            })
            .collect::<Vec<_>>();
        let head = self
            .state
            .refs
            .head
            .as_ref()
            .and_then(|head| head.name.as_deref());
        for item in &self.state.refs.items {
            let (group, kind) = match item.kind {
                gg_engine::refs::RefKind::Branch => (Group::Branches, "branch"),
                gg_engine::refs::RefKind::Remote => (Group::Remotes, "remote"),
                gg_engine::refs::RefKind::Tag => (Group::Tags, "tag"),
            };
            let current = Some(item.full_name.as_str()) == head;
            items.push(search::Item {
                group,
                label: item.name.clone(),
                context: format!(
                    "{kind}{} · {}",
                    if current { " · current" } else { "" },
                    item.commit.chars().take(8).collect::<String>()
                ),
                key: "",
                disabled: None,
                command: FrameCommand::Inspect(item.commit.clone()),
            });
        }
        items
    }

    /// Draws the `Ctrl+K` search popup and returns the command to run.
    fn close_search(&mut self, ctx: &egui::Context) {
        self.search = None;
        self.search_text.clear();
        ctx.memory_mut(|m| m.surrender_focus(search_field_id()));
    }

    fn draw_search(&mut self, ctx: &egui::Context, ready: &FrameReadiness) -> Option<FrameCommand> {
        let palette = self.search.as_mut()?;
        let query = palette.query.clone();
        if palette.files_query != query {
            palette.files_query = query.clone();
            palette.file_items = search::files(
                &query,
                self.local_summary
                    .as_ref()
                    .map(|summary| summary.files.as_slice())
                    .unwrap_or_default(),
                |path| FrameCommand::OpenFile(path.to_string()),
            );
        }
        let file_items = palette.file_items.clone();
        let mut items = {
            let base = self.search_items(ready);
            search::results(&query, &base, FrameCommand::Filter(query.trim().to_owned()))
        };
        items.extend(file_items);
        let menu_bar_height = self.menu_bar_height();
        let palette = self.search.as_mut()?;
        if palette.selected >= items.len() {
            palette.selected = items.len().saturating_sub(1);
        }
        let (down, up, enter, escape) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if down && palette.selected + 1 < items.len() {
            palette.selected += 1;
        }
        if up {
            palette.selected = palette.selected.saturating_sub(1);
        }
        let mut chosen = enter.then_some(palette.selected);
        let width = 520.0_f32.min(ctx.content_rect().width() - 32.0);
        // Results keep one fixed height however many match.
        let results = |ui: &mut egui::Ui,
                       palette: &mut SearchPalette,
                       chosen: &mut Option<usize>| {
            let mut group = None;
            egui::ScrollArea::vertical()
                .id_salt("search-results")
                // A fixed-height list keeps its bar track, so the popup
                // never changes width as the bar would fade in and out.
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .min_scrolled_height(SEARCH_RESULTS_HEIGHT)
                .max_height(SEARCH_RESULTS_HEIGHT)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if items.is_empty() {
                        ui.weak("No matching commands, refs, or files");
                    }
                    for (index, item) in items.iter().enumerate() {
                        if group != Some(item.group) {
                            group = Some(item.group);
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(item.group.title()).small().weak());
                        }
                        let text = if item.context.is_empty() {
                            item.label.clone()
                        } else {
                            format!("{}   ·   {}", item.label, item.context)
                        };
                        let response = ui.add_enabled(
                            item.disabled.is_none(),
                            egui::Button::selectable(palette.selected == index, text)
                                .truncate()
                                .right_text(item.key)
                                .min_size(egui::vec2(ui.available_width(), 24.0)),
                        );
                        let response = match &item.disabled {
                            Some(reason) => response.on_disabled_hover_text(reason),
                            None => response,
                        };
                        if response.clicked() {
                            *chosen = Some(index);
                        }
                        if palette.selected == index && (down || up) {
                            response.scroll_to_me(None);
                        }
                    }
                });
            ui.add_space(4.0);
            ui.add(
                    egui::Label::new(
                        egui::RichText::new(
                            "Enter: run · Up/Down: choose · Esc: close · every action also lives in a menu",
                        )
                        .weak(),
                    )
                    .truncate(),
                );
        };
        let mut close = escape;
        if palette.in_field {
            let anchor = palette.anchor;
            // Right-aligned with the field, never past the window edge.
            let right = anchor.right().min(ctx.content_rect().right());
            let left = (right - width).max(ctx.content_rect().left() + 8.0);
            let area = egui::Area::new(egui::Id::unique("frame-search"))
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(left, anchor.bottom() + 4.0))
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_width(width);
                        results(ui, palette, &mut chosen);
                    });
                });
            let outside = ctx.input(|i| {
                i.pointer.any_pressed()
                    && i.pointer.interact_pos().is_some_and(|pos| {
                        !area.response.rect.contains(pos) && !anchor.contains(pos)
                    })
            });
            close |= outside;
        } else {
            let id = egui::Id::unique("frame-search");
            let area = egui::Modal::default_area(id).anchor(
                egui::Align2::RIGHT_TOP,
                egui::vec2(-8.0, menu_bar_height + 34.0),
            );
            let modal = egui::Modal::new(id)
                .area(area)
                .backdrop_color(egui::Color32::from_black_alpha(40))
                .show(ctx, |ui| {
                    ui.set_width(width);
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut palette.query)
                            .hint_text("Search commands, branches, remotes, tags, and files")
                            .desired_width(f32::INFINITY),
                    );
                    if !palette.focused {
                        edit.request_focus();
                        palette.focused = true;
                    }
                    if edit.changed() {
                        palette.selected = 0;
                    }
                    ui.add_space(4.0);
                    results(ui, palette, &mut chosen);
                });
            close |= modal.should_close();
        }
        let command = chosen
            .and_then(|index| items.get(index))
            .filter(|item| item.disabled.is_none())
            .map(|item| item.command.clone());
        if close || command.is_some() {
            self.close_search(ctx);
        }
        command
    }

    /// The interactive rebase in progress, when the summary has read one.
    fn rebase_progress(&self) -> Option<&gg_engine::mutation::RebaseProgress> {
        self.mutation
            .as_ref()
            .filter(|state| state.operation == gg_engine::mutation::Operation::Rebase)?;
        self.local_summary.as_ref()?.rebase.as_ref()
    }

    /// Open Local Changes once for each newly reopened commit, with its
    /// message in the commit box; a root commit opens in amend mode.
    fn open_reopened_commit(&mut self, ctx: &egui::Context) {
        let Some(gg_engine::mutation::RebaseStop::Reopened {
            base,
            commit,
            message,
            ..
        }) = self.rebase_progress().map(|progress| progress.stop.clone())
        else {
            return;
        };
        if self.reopen_seen.as_deref() == Some(commit.as_str()) || self.rewrite_dialog.is_some() {
            return;
        }
        self.reopen_seen = Some(commit);
        self.open_working(ctx);
        if let Some(working) = &mut self.working {
            if base.is_some() {
                working.prefill_message(&message);
            } else if let Err(error) = working.start_amend(ctx) {
                self.mutation_error = Some(error);
            }
        }
    }

    /// Whether the refs sidebar shows in the current view: History keeps it
    /// open by default; Changes and Review hide it until toggled.
    fn refs_shown(&self) -> bool {
        if self.working.is_some() || self.review.is_some() || self.conflict_view.is_some() {
            self.refs_visible_elsewhere
        } else {
            self.refs_visible
        }
    }

    /// The sidebar button and Ctrl+\: the docked sidebar when the window is
    /// wide enough, otherwise the refs drawer.
    fn toggle_refs_or_drawer(&mut self, ctx: &egui::Context) {
        if ctx.content_rect().width() >= REFS_DOCK_WIDTH {
            self.toggle_refs();
        } else if self.state.history.is_some() {
            self.refs_drawer = !self.refs_drawer;
            self.refs_drawer_focus = self.refs_drawer;
            // The cursor starts on the current branch.
            let head = self.state.refs.head.as_ref().and_then(|h| h.name.clone());
            self.refs_drawer_query = self.refs_query.clone();
            self.refs_drawer_cursor = refs_view::cursor_rows(&self.state.refs, &self.refs_query)
                .iter()
                .position(|item| Some(&item.full_name) == head.as_ref())
                .unwrap_or(0);
        }
    }

    fn toggle_refs(&mut self) {
        if self.working.is_some() || self.review.is_some() || self.conflict_view.is_some() {
            self.refs_visible_elsewhere = !self.refs_visible_elsewhere;
        } else {
            self.refs_visible = !self.refs_visible;
        }
    }

    /// Take the consequence list for `slot` out of its slot, (re)reading it
    /// when its Git arguments change; put it back with `self.consequences`
    /// while the dialog stays open.
    fn take_consequence(
        &mut self,
        slot: &'static str,
        args: &[&str],
        ctx: &egui::Context,
    ) -> Option<consequence::Lines> {
        let repo = self.state.history.as_ref()?.path.clone();
        match self.consequences.remove(slot) {
            Some(lines)
                if lines
                    .key
                    .iter()
                    .map(String::as_str)
                    .eq(args.iter().copied()) =>
            {
                Some(lines)
            }
            _ => Some(consequence::Lines::git(&repo, args, ctx)),
        }
    }

    /// The Reset dialog's untracked and ignored files that a hard reset to
    /// `commit` would overwrite, read in the background like
    /// [`Self::take_consequence`].
    fn take_overwritten(
        &mut self,
        commit: &str,
        ctx: &egui::Context,
    ) -> Option<consequence::Lines> {
        let repo = self.state.history.as_ref()?.path.clone();
        let key = vec!["overwritten-untracked".to_owned(), commit.to_owned()];
        match self.consequences.remove("reset-overwritten") {
            Some(lines) if lines.key == key => Some(lines),
            _ => {
                let commit = commit.to_owned();
                Some(consequence::Lines::with(key, ctx, move || {
                    gg_git::reset::overwritten_untracked(&repo, &commit)
                }))
            }
        }
    }

    /// The toolbar current-branch button and its filterable switcher popup.
    fn branch_switcher_button(
        &mut self,
        ui: &mut egui::Ui,
        ready: &FrameReadiness,
        compact: bool,
    ) -> Option<BranchPick> {
        let head = self.state.refs.head.as_ref();
        // During a rebase the button keeps the branch's name, as in the
        // mockup; the step is in the operation area and the window title.
        let rebasing = self.rebase_progress().map(|progress| {
            progress
                .branch
                .clone()
                .unwrap_or_else(|| "detached HEAD".into())
        });
        let name = match (rebasing, head.and_then(|head| head.name.as_deref())) {
            (Some(branch), _) => branch,
            (None, Some(full)) => full.strip_prefix("refs/heads/").unwrap_or(full).to_string(),
            (None, None) if head.is_some_and(|head| head.detached) => "detached HEAD".into(),
            (None, None) => "no branch".into(),
        };
        // While rebasing, the counts give way to the step, as in the stop
        // mockups (`main  rebasing 2/5`).
        let step = self.rebase_progress().map(|progress| {
            let (step, total) = progress.position();
            let verb = if progress.stop == gg_engine::mutation::RebaseStop::Conflict {
                "conflict"
            } else {
                "rebasing"
            };
            (
                format!("{verb} {step}/{total}"),
                rebase_stop::palette(progress).ink,
            )
        });
        let tracking = self
            .state
            .refs
            .current_tracking()
            .filter(|_| step.is_none())
            .map(|tracking| {
                format!(
                    "{}{} {}{}",
                    crate::phosphor::ARROW_UP,
                    tracking.ahead.unwrap_or(0),
                    crate::phosphor::ARROW_DOWN,
                    tracking.behind.unwrap_or(0)
                )
            })
            .unwrap_or_default();
        let detail = match &step {
            Some((step, ink)) => egui::RichText::new(step).color(*ink),
            None => egui::RichText::new(&tracking).small().weak(),
        };
        let icon = crate::phosphor::icon(crate::phosphor::GIT_BRANCH).size(15.0);
        let caret = crate::phosphor::icon(crate::phosphor::CARET_DOWN).size(10.0);
        let button = if compact {
            egui::Button::new((icon, caret))
        } else {
            egui::Button::new((icon, egui::RichText::new(&name), detail, caret)).truncate()
        }
        .min_size(egui::vec2(28.0, 24.0));
        let response = ui
            .add_enabled(ready.has_repo, button)
            .on_hover_text(format!(
                "{name} {}\nSwitch branch",
                step.as_ref()
                    .map_or(tracking.as_str(), |(step, _)| step.as_str())
            ));
        if response.clicked() {
            self.branch_filter.clear();
            self.branch_cursor = 0;
        }
        let branches = switcher_branches(&self.state.refs, &self.branch_filter);
        let remotes = switcher_remotes(&self.state.refs, &self.branch_filter);
        let filter = &mut self.branch_filter;
        let mut pick = None;
        // Keyboard: Up/Down move a cursor over the rows that can run, Enter
        // runs it (branch menus proposal). Rows: local branches, then remotes.
        let runnable = branches
            .iter()
            .filter(|(_, current)| !*current)
            .map(|(name, _)| BranchPick::Switch(name.clone()))
            .chain(remotes.iter().map(|name| BranchPick::Track(name.clone())))
            .collect::<Vec<_>>();
        let popup_open = egui::Popup::is_id_open(ui.ctx(), branch_switcher_popup_id());
        if popup_open {
            let (down, up, enter) = ui.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                    i.key_pressed(egui::Key::Enter),
                )
            });
            if down {
                self.branch_cursor = (self.branch_cursor + 1).min(runnable.len().saturating_sub(1));
            }
            if up {
                self.branch_cursor = self.branch_cursor.saturating_sub(1);
            }
            self.branch_cursor = self.branch_cursor.min(runnable.len().saturating_sub(1));
            if enter && ready.branch_idle {
                pick = runnable.get(self.branch_cursor).cloned();
            }
        }
        let cursor = runnable.get(self.branch_cursor).cloned();
        egui::Popup::from_toggle_button_response(&response)
            .id(branch_switcher_popup_id())
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_width(320.0);
                let edit = ui.add(
                    egui::TextEdit::singleline(filter)
                        .hint_text("Switch to branch…")
                        .desired_width(f32::INFINITY),
                );
                if !edit.has_focus() && filter.is_empty() {
                    edit.request_focus();
                }
                let switch_reason = if ready.branch_idle {
                    None
                } else {
                    Some("Wait for the current operation to finish")
                };
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    if branches.is_empty() {
                        ui.weak("No matching branches");
                    }
                    for (branch, current) in &branches {
                        let reason = if *current {
                            Some("Already on this branch")
                        } else {
                            switch_reason
                        };
                        let label = if *current && !tracking.is_empty() {
                            format!("{branch}  {tracking}  ·  current")
                        } else if *current {
                            format!("{branch}  ·  current")
                        } else {
                            branch.clone()
                        };
                        let at_cursor = cursor.as_ref() == Some(&BranchPick::Switch(branch.clone()));
                        let mut row = menus::row(*current, label)
                            .truncate()
                            .min_size(egui::vec2(ui.available_width(), menus::ROW_HEIGHT));
                        if at_cursor {
                            row = row.fill(ui.visuals().widgets.hovered.weak_bg_fill);
                        }
                        let response = ui.add_enabled(reason.is_none(), row);
                        let response = match reason {
                            Some(reason) => response.on_disabled_hover_text(reason),
                            None => response.on_hover_text(
                                "Switch after the clean-worktree check; local changes are never discarded",
                            ),
                        };
                        if response.clicked() {
                            pick = Some(BranchPick::Switch(branch.clone()));
                        }
                    }
                    if !remotes.is_empty() {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("REMOTE BRANCHES").small().weak());
                    }
                    for remote in &remotes {
                        let at_cursor = cursor.as_ref() == Some(&BranchPick::Track(remote.clone()));
                        let mut row = menus::row(
                            false,
                            (
                                egui::RichText::new(remote),
                                egui::Atom::grow(),
                                egui::RichText::new("track and switch").small().weak(),
                            ),
                        )
                        .truncate()
                        .min_size(egui::vec2(ui.available_width(), menus::ROW_HEIGHT));
                        if at_cursor {
                            row = row.fill(ui.visuals().widgets.hovered.weak_bg_fill);
                        }
                        let response = ui
                            .add_enabled(ready.branch_idle, row)
                            .on_disabled_hover_text("Wait for the current operation to finish");
                        if response.clicked() {
                            pick = Some(BranchPick::Track(remote.clone()));
                        }
                    }
                });
                ui.label(
                    egui::RichText::new("Up/Down choose · Enter switch · Esc close")
                        .small()
                        .weak(),
                );
                ui.separator();
                if ui
                    .add_enabled(
                        ready.branch_idle,
                        menus::row(false, "New branch from HEAD…")
                            .min_size(egui::vec2(ui.available_width(), menus::ROW_HEIGHT)),
                    )
                    .clicked()
                {
                    pick = Some(BranchPick::New);
                }
            });
        if pick.is_some() {
            egui::Popup::close_all(ui.ctx());
        }
        pick
    }

    /// The **Stashes** picker popup, opened by **Changes › Stashes…**, the
    /// sidebar's stash rows, and the footer recovery; centered under the
    /// toolbar.
    fn stash_picker(&mut self, ctx: &egui::Context, layer: egui::LayerId) -> Option<StashPick> {
        if !egui::Popup::is_id_open(ctx, stash_picker_popup_id()) {
            self.stash_anchor = None;
            return None;
        }
        let entries = self
            .local_summary
            .as_ref()
            .map(|summary| summary.stashes.clone())
            .unwrap_or_default();
        let menu_bar_height = self.menu_bar_height();
        let picker = &mut self.stash_picker;
        let mut pick = None;
        // Under the toolbar's Stashes button when it opened the picker,
        // otherwise centred under the toolbar (menu and sidebar).
        let (anchor, align) = match self.stash_anchor {
            Some(corner) => (corner, egui::RectAlign::BOTTOM_START),
            None => (
                egui::pos2(
                    ctx.content_rect().center().x,
                    menu_bar_height + TOOLBAR_HEIGHT + 4.0,
                ),
                egui::RectAlign::BOTTOM,
            ),
        };
        egui::Popup::new(stash_picker_popup_id(), ctx.clone(), anchor, layer)
            .open_memory(None)
            .align(align)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_width(STASH_PICKER_WIDTH);
                if let Some(entry) = stash_picker_keys(ui.ctx(), picker, entries.len())
                    .and_then(|apply| apply.then(|| entries.get(picker.selected)).flatten())
                {
                    pick = Some(if picker.confirm_drop {
                        picker.confirm_drop = false;
                        StashPick::Drop(entry.reference.clone())
                    } else {
                        StashPick::Apply(entry.reference.clone())
                    });
                }
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("STASHES").small().weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Up/Down choose · Enter apply · Del drop")
                                .small()
                                .weak(),
                        );
                    });
                });
                // A fixed-height list with fixed column slots, so the popup
                // never changes size as entries or counts arrive.
                egui::ScrollArea::vertical()
                    .id_salt("stash-picker-list")
                    .max_height(STASH_PICKER_LIST)
                    .min_scrolled_height(STASH_PICKER_LIST)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for (index, entry) in entries.iter().enumerate() {
                            if stash_picker_row(ui, entry, picker.selected == index).clicked() {
                                picker.selected = index;
                                picker.confirm_drop = false;
                            }
                        }
                    });
                ui.separator();
                ui.allocate_ui_with_layout(
                    egui::vec2(STASH_PICKER_WIDTH, STASH_PICKER_ACTIONS),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_size(egui::vec2(STASH_PICKER_WIDTH, STASH_PICKER_ACTIONS));
                        ui.set_max_height(STASH_PICKER_ACTIONS);
                        let Some(entry) = entries.get(picker.selected) else {
                            return;
                        };
                        stash_picker_actions(ui, entry, picker, &mut pick);
                    },
                );
            });
        if pick.is_some() {
            egui::Popup::close_all(ctx);
        }
        pick
    }
}

/// Width of the Stashes picker popup.
const STASH_PICKER_WIDTH: f32 = 440.0;
/// Height of one two-line picker row.
const STASH_PICKER_ROW: f32 = 40.0;
/// Height of the picker's entry list: six rows.
const STASH_PICKER_LIST: f32 = 5.0 * STASH_PICKER_ROW;
/// Height reserved for the picker's actions, its drop confirmation, and an error.
const STASH_PICKER_ACTIONS: f32 = 88.0;

/// Display message without the `WIP on <branch>: ` or `On <branch>: ` prefix
/// that the branch column already shows.
fn stash_display_message(entry: &gg_engine::stash::StashEntry) -> &str {
    match &entry.branch {
        Some(_) => entry
            .message
            .split_once(": ")
            .map_or(entry.message.as_str(), |(_, rest)| rest),
        None => entry.message.as_str(),
    }
}

/// One picker row: ref, message, branch, age, and file count in fixed slots.
fn stash_picker_row(
    ui: &mut egui::Ui,
    entry: &gg_engine::stash::StashEntry,
    selected: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), STASH_PICKER_ROW),
        egui::Sense::click(),
    );
    if selected {
        ui.painter()
            .rect_filled(rect, 3.0, colors::selection(ui.visuals()));
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, 3.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    // Two lines, as in the mockup: the ref on the left; the message over
    // `on <branch> · <age> · <files>`.
    let weak = ui.visuals().weak_text_color();
    let strong = ui.visuals().strong_text_color();
    let inner = rect.shrink2(egui::vec2(8.0, 3.0));
    let reference =
        egui::Rect::from_min_max(inner.min, egui::pos2(inner.left() + 64.0, inner.bottom()));
    let text =
        egui::Rect::from_min_max(egui::pos2(reference.right() + 8.0, inner.top()), inner.max);
    let (top, bottom) = text.split_top_bottom_at_fraction(0.55);
    bounded_text::paint(
        ui,
        reference,
        entry.reference.as_str(),
        egui::FontId::monospace(11.0),
        weak,
    );
    bounded_text::paint(
        ui,
        top,
        stash_display_message(entry),
        egui::FontId::proportional(12.5),
        strong,
    );
    let count = match entry.files {
        Some(1) => "1 file".to_string(),
        Some(count) => format!("{count} files"),
        None => String::new(),
    };
    let mut details = format!(
        "on {} · {}",
        entry.branch.as_deref().unwrap_or("detached"),
        entry.age
    );
    if !count.is_empty() {
        details.push_str(&format!(" · {count}"));
    }
    bounded_text::paint(ui, bottom, &details, egui::FontId::proportional(11.0), weak);
    response.on_hover_text(format!(
        "{} · {}\n{}{}",
        entry.reference,
        entry.message,
        entry.age,
        if count.is_empty() {
            String::new()
        } else {
            format!(" · {count}")
        }
    ))
}

/// Apply, Pop, Show changes, and Drop… for the selected stash, or the drop
/// confirmation, in the picker's fixed action area.
/// Keys in the open Stashes picker: Up/Down select, Del asks to drop, Esc
/// cancels the drop question. Returns `Some(true)` when Enter runs the
/// default action (Apply, or Drop while the drop question shows).
fn stash_picker_keys(ctx: &egui::Context, picker: &mut StashPicker, count: usize) -> Option<bool> {
    if count == 0 || ctx.text_edit_focused() {
        return None;
    }
    let presses = |key| ctx.input_mut(|i| i.count_and_consume_key(egui::Modifiers::NONE, key));
    let pressed = |key| presses(key) > 0;
    let down = presses(egui::Key::ArrowDown);
    let up = presses(egui::Key::ArrowUp);
    if down + up > 0 {
        picker.selected = (picker.selected + down).saturating_sub(up).min(count - 1);
        picker.confirm_drop = false;
    }
    if pressed(egui::Key::Delete) {
        picker.confirm_drop = true;
    }
    if picker.confirm_drop && pressed(egui::Key::Escape) {
        picker.confirm_drop = false;
    }
    pressed(egui::Key::Enter).then_some(true)
}

fn stash_picker_actions(
    ui: &mut egui::Ui,
    entry: &gg_engine::stash::StashEntry,
    picker: &mut StashPicker,
    pick: &mut Option<StashPick>,
) {
    if picker.confirm_drop {
        ui.label(format!(
            "Drop {}? This removes the stash entry, not working files.",
            entry.reference
        ));
        ui.horizontal(|ui| {
            if ui
                .add(consequence::danger_button(
                    format!("Drop {}", entry.reference),
                    true,
                ))
                .clicked()
            {
                *pick = Some(StashPick::Drop(entry.reference.clone()));
                picker.confirm_drop = false;
            }
            if ui
                .button(("Cancel", egui::RichText::new("Esc").small().weak()))
                .clicked()
            {
                picker.confirm_drop = false;
            }
        });
    } else {
        ui.checkbox(&mut picker.restore_index, "Restore staged state")
            .on_hover_text("Apply and Pop also restore what was staged when the stash was made");
        ui.horizontal(|ui| {
            if ui
                .button(format!("Apply {}", entry.reference))
                .on_hover_text("Restore the changes and keep the stash entry")
                .clicked()
            {
                *pick = Some(StashPick::Apply(entry.reference.clone()));
            }
            if ui
                .button("Pop")
                .on_hover_text("Restore the changes, then remove the stash entry")
                .clicked()
            {
                *pick = Some(StashPick::Pop(entry.reference.clone()));
            }
            if ui.button("Show changes").clicked() {
                *pick = Some(StashPick::Show {
                    commit: entry.commit.clone(),
                    message: entry.message.clone(),
                });
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(egui::RichText::new("Drop…").color(ui.visuals().error_fg_color))
                    .clicked()
                {
                    picker.confirm_drop = true;
                }
            });
        });
    }
    if let Some(error) = &picker.error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
}

impl Browser {
    fn apply_stash_pick(&mut self, ctx: &egui::Context, pick: StashPick) {
        let result = match pick {
            StashPick::Apply(reference) => {
                let index = self.stash_picker.restore_index;
                self.restore_stash_inner(ctx, &reference, false, index)
            }
            StashPick::Pop(reference) => {
                let index = self.stash_picker.restore_index;
                self.restore_stash_inner(ctx, &reference, true, index)
            }
            StashPick::Drop(reference) => self.drop_stash(ctx, &reference),
            StashPick::Show { commit, message } => {
                let return_location = self.capture_return_location();
                self.open_review_at(ctx, commit, message, Some(return_location));
                Ok(())
            }
        };
        if let Err(error) = result {
            self.stash_picker.error = Some(error.clone());
            self.set_outcome(OutcomeSeverity::Error, error, None, None);
        }
        self.refresh_local_summary(ctx);
    }

    /// Re-read the worktree, stash, and branch facts shown in the window frame.
    fn refresh_local_summary(&mut self, ctx: &egui::Context) {
        let path = self
            .stash_path()
            .unwrap_or_else(|| PathBuf::from(&self.path));
        if path.as_os_str().is_empty() {
            self.local_summary = None;
            self.local_summary_job = None;
            return;
        }
        // Replacing the receiver drops any older, now stale result.
        let (sender, receiver) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = sender.send(LocalSummary::read(path));
            ctx.request_repaint();
        });
        self.local_summary_job = Some(receiver);
    }

    fn poll_local_summary(&mut self) {
        let Some(receiver) = &self.local_summary_job else {
            return;
        };
        match receiver.try_recv() {
            Ok(summary) => {
                self.local_summary_job = None;
                let count = summary.stashes.len();
                self.local_summary = Some(summary);
                if self.stash_picker.selected >= count {
                    self.stash_picker.selected = count.saturating_sub(1);
                }
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.local_summary_job = None,
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    fn open_working(&mut self, ctx: &egui::Context) {
        if self.working.is_some() {
            return;
        }
        let Some(history) = &self.state.history else {
            return;
        };
        self.review = None;
        let mut working = status_view::StatusView::new(history.path.clone(), ctx);
        working.apply_sign_default(self.prefs.sign_commits);
        if let Some(draft) = self.working_drafts.get(&working.repository_key()).cloned() {
            working.restore_draft(draft, ctx);
        }
        self.working = Some(working);
        ctx.request_repaint();
    }

    fn close_to_history(&mut self, ctx: &egui::Context) {
        if self.working_mutation_pending() {
            return;
        }
        self.conflict_view = None;
        if let Some(working) = &mut self.working
            && let Some(id) = working.take_commit()
        {
            self.pending_commit = Some(id);
        }
        if let Some(working) = &self.working {
            self.working_drafts
                .insert(working.repository_key(), working.capture_draft());
        }
        let had_working = self.working.take().is_some();
        let review_location = self
            .review
            .as_mut()
            .and_then(|review| review.take_return_location());
        let had_review = self.review.take().is_some();
        if !had_working && !had_review {
            return;
        }
        if let Some(id) = self.pending_commit.take() {
            self.state.selected = Some(id);
            self.scroll_offset = 0.0;
            self.restore_offset = None;
            self.scroll_to_selection = true;
            self.load(ctx);
            self.refresh_reflog();
            self.refresh_backups();
        } else if let Some(location) = review_location {
            self.restore_return_location(ctx, location);
        } else {
            self.restore_offset = Some(self.scroll_offset);
        }
        // The History destination always shows the list, even when the
        // recorded caller was the narrow Details overlay.
        self.narrow_details = false;
        ctx.request_repaint();
    }

    /// Drop work that belonged to the previous repository.
    fn clear_operation_state(&mut self) {
        self.mutation = None;
        self.mutation_error = None;
        self.mutation_notice = None;
        self.outcome = None;
        self.outcome_details_open = false;
        self.network_notice = None;
        self.stash_notice = None;
        self.branch_warning = None;
        self.operation_worker = None;
        self.mutation_busy = None;
        self.operation_repo = None;
        self.operation_restore_row = None;
        self.rewrite_job = None;
        self.rewrite_refresh = None;
        self.conflict_view = None;
        self.pull_rebase = None;
        self.rebase_target = None;
        self.stash_switch = None;
    }

    /// Detect an operation already in progress when a repository is opened.
    fn sync_mutation(&mut self) {
        // A replacement for another path may still be loading. Keep the old
        // operation state clear until the requested history is adopted.
        if self.state.requested.as_ref().is_some_and(|requested| {
            self.state
                .history
                .as_ref()
                .is_none_or(|history| &history.path != requested)
        }) {
            return;
        }
        let path = self
            .state
            .history
            .as_ref()
            .map(|history| history.path.clone());
        if path == self.mutation_path {
            return;
        }
        if self.mutation_path.is_some() {
            self.clear_operation_state();
        }
        if let Some(path) = &path {
            self.welcome.opening = None;
            self.welcome.failed = None;
            settings::push_recent(&mut self.recent, path.clone(), settings::now());
            if let Err(error) = settings::save_recent(&mut self.recent, &mut self.recent_base) {
                self.mutation_error = Some(error);
            }
        }
        self.mutation_path = path;
        if self.mutation.is_none() {
            self.refresh_mutation();
        }
    }

    fn refresh_mutation(&mut self) {
        let Some(path) = self.stash_path() else {
            return;
        };
        if let Ok(state) = gg_git::mutation::state(&path) {
            self.mutation = state;
        }
    }

    fn start_mutation(
        &mut self,
        ctx: &egui::Context,
        operation: gg_engine::mutation::Operation,
    ) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before changing history.")?;
        let commit = self
            .state
            .selected
            .clone()
            .ok_or("Select a commit before cherry-picking or reverting.")?;
        let restore_row = self.state.selected_row;
        let receipt = match operation {
            gg_engine::mutation::Operation::CherryPick => {
                gg_git::mutation::cherry_pick(&path, &commit)?
            }
            gg_engine::mutation::Operation::Revert => gg_git::mutation::revert(&path, &commit)?,
            gg_engine::mutation::Operation::Merge => {
                return Err("Use the Merge action for merge operations.".into());
            }
            gg_engine::mutation::Operation::Rebase => {
                return Err("Use the Rebase action for rebase operations.".into());
            }
        };
        self.mutation_error = None;
        self.mutation_notice = Some(match operation {
            gg_engine::mutation::Operation::CherryPick => format!(
                "Cherry-picked {}{}",
                commit,
                if receipt.conflicts.is_empty() {
                    String::new()
                } else {
                    format!(" · {} conflict(s)", receipt.conflicts.len())
                }
            ),
            gg_engine::mutation::Operation::Revert => format!(
                "Reverted {}{}",
                commit,
                if receipt.conflicts.is_empty() {
                    String::new()
                } else {
                    format!(" · {} conflict(s)", receipt.conflicts.len())
                }
            ),
            gg_engine::mutation::Operation::Merge => String::new(),
            gg_engine::mutation::Operation::Rebase => String::new(),
        });
        self.conflict_view = None;
        self.restore_row = restore_row;
        self.refresh_mutation();
        self.load(ctx);
        self.refresh_reflog();
        self.refresh_backups();
        self.refresh_working(ctx);
        Ok(())
    }

    fn delete_branch(
        &mut self,
        ctx: &egui::Context,
        name: &str,
        force: bool,
    ) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before deleting a branch.")?;
        gg_git::branch::delete(&path, name, force)?;
        self.mutation_notice = Some(format!("Deleted branch {name}"));
        self.branch_delete = None;
        self.load(ctx);
        Ok(())
    }

    fn delete_tag(&mut self, ctx: &egui::Context, name: &str) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before deleting a tag.")?;
        gg_git::tag::delete(&path, name)?;
        self.mutation_notice = Some(format!("Deleted tag {name}"));
        self.tag_delete = None;
        self.load(ctx);
        Ok(())
    }

    fn start_reset(
        &mut self,
        ctx: &egui::Context,
        commit: &str,
        mode: gg_engine::reset::Mode,
    ) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before resetting.")?;
        let receipt = gg_git::reset::reset(&path, commit, mode)?;
        self.mutation_error = None;
        self.mutation_notice = Some(format!(
            "Reset to {}{}",
            self.commit_summary(&receipt.commit),
            if mode == gg_engine::reset::Mode::Hard {
                " · hard"
            } else {
                ""
            }
        ));
        self.refresh_mutation();
        self.load(ctx);
        self.refresh_reflog();
        self.refresh_backups();
        self.refresh_working(ctx);
        Ok(())
    }

    /// No repository is open (or one is still being read from the welcome
    /// view), so the welcome view replaces History.
    fn welcome_shown(&self) -> bool {
        self.state.history.is_none()
            && self.working.is_none()
            && self.review.is_none()
            && (self.pending.is_none() || self.welcome.opening.is_some())
    }

    /// Classify a finished read that left no repository open: a recent row
    /// that failed becomes its amber line; any other folder becomes the
    /// not-a-repository line.
    fn settle_welcome(&mut self) {
        if self.pending.is_some() {
            return;
        }
        let opening = self.welcome.opening.take();
        let (Some(error), Some(requested)) = (&self.state.error, &self.state.requested) else {
            return;
        };
        if self.welcome.settled.as_ref() == Some(requested) {
            return;
        }
        self.welcome.settled = Some(requested.clone());
        if opening.as_ref() == Some(requested) {
            let failure = if requested.is_dir() {
                welcome_view::Failure::NotRepository
            } else {
                welcome_view::Failure::Missing
            };
            self.welcome.failed = Some((requested.clone(), failure));
            self.welcome.reread();
        } else {
            self.welcome.launch = Some((requested.clone(), error.clone()));
        }
    }

    fn save_recent(&mut self) {
        if let Err(error) = settings::save_recent(&mut self.recent, &mut self.recent_base) {
            self.set_outcome(OutcomeSeverity::Warning, error, None, None);
        }
    }

    /// Open `path` as the repository, leaving whatever was shown.
    fn open_repository(&mut self, ctx: &egui::Context, path: PathBuf) {
        if let Some(working) = &self.working {
            self.working_drafts
                .insert(working.repository_key(), working.capture_draft());
        }
        self.working = None;
        self.review = None;
        self.recovery = None;
        self.path = path.to_string_lossy().into_owned();
        self.load(ctx);
    }

    fn run_welcome_command(&mut self, ctx: &egui::Context, command: welcome_view::Command) {
        use welcome_view::Command;
        let frame = match command {
            Command::Open => FrameCommand::Open,
            Command::Clone => FrameCommand::Clone,
            Command::NewRepository(folder) => FrameCommand::NewRepository(folder),
            Command::Preferences => {
                self.open_preferences(PrefSection::General);
                return;
            }
            Command::OpenRecent(path) => {
                if self.repository_switch_pending() {
                    return;
                }
                self.welcome.failed = None;
                self.welcome.opening = Some(path.clone());
                self.open_repository(ctx, path);
                return;
            }
            Command::Remove(path) => {
                self.recent.retain(|entry| entry.path != path);
                if self
                    .welcome
                    .failed
                    .as_ref()
                    .is_some_and(|(failed, _)| *failed == path)
                {
                    self.welcome.failed = None;
                }
                self.save_recent();
                return;
            }
            Command::Locate(path) => {
                let start = path.ancestors().find(|folder| folder.is_dir());
                if let Some(found) = folder_picker::pick_folder("Locate repository", start) {
                    self.relocate_recent(&path, found.clone());
                    self.run_welcome_command(ctx, Command::OpenRecent(found));
                }
                return;
            }
        };
        let (mut local, mut history) = (false, false);
        self.run_frame_command(ctx, frame, &mut local, &mut history);
    }

    /// New repository…: on success the repository opens, in History when
    /// it committed and in Local Changes otherwise.
    fn draw_new_repository(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.new_repository else {
            return;
        };
        let home = std::env::var("HOME").ok();
        match dialog.show(ctx, home.as_deref()) {
            Some(new_repository::Action::Created(receipt)) => {
                self.new_repository = None;
                self.welcome.launch = None;
                self.open_changes_after_load = receipt.head.is_none();
                let shown = welcome_view::tilde(&receipt.path, home.as_deref());
                self.set_outcome(
                    OutcomeSeverity::Info,
                    match receipt.head {
                        Some(_) => format!(
                            "Created {shown} on {} with {} files in Initial commit",
                            receipt.branch, receipt.files
                        ),
                        None => format!("Created {shown} on {}", receipt.branch),
                    },
                    None,
                    None,
                );
                self.welcome.opening = Some(receipt.path.clone());
                self.open_repository(ctx, receipt.path);
            }
            Some(new_repository::Action::OpenRepository(path)) => {
                self.new_repository = None;
                self.welcome.opening = Some(path.clone());
                self.open_repository(ctx, path);
            }
            Some(new_repository::Action::Closed) => self.new_repository = None,
            None => {}
        }
    }

    /// Drag and drop on the welcome view: an overlay names the folder; a
    /// repository (or a file inside one) opens, any other folder opens New
    /// repository… on it.
    fn drop_folders(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.welcome.drop_rx
            && let Ok((folder, repository)) = rx.try_recv()
        {
            self.welcome.drop_rx = None;
            if repository {
                self.welcome.opening = Some(folder.clone());
                self.open_repository(ctx, folder);
            } else {
                self.new_repository = Some(new_repository::Dialog::new(Some(folder)));
            }
        }
        if !self.welcome_shown() || self.new_repository.is_some() || self.edit_path {
            return;
        }
        let (hovered, dropped) = ctx.input(|input| {
            (
                input.raw.hovered_files.clone(),
                input.raw.dropped_files.clone(),
            )
        });
        if let Some(file) = hovered.first() {
            let home = std::env::var("HOME").ok();
            let folder = file.path.as_deref().map(|path| {
                welcome_view::tilde(&welcome_view::dropped_folder(path), home.as_deref())
            });
            welcome_view::paint_drop_overlay(ctx, folder.as_deref());
        }
        if let Some(path) = dropped.first().map(|file| file.path().to_path_buf()) {
            let folder = welcome_view::dropped_folder(&path);
            let (tx, rx) = std::sync::mpsc::channel();
            let repaint = ctx.clone();
            std::thread::spawn(move || {
                let repository = is_repository(&folder);
                let _ = tx.send((folder, repository));
                repaint.request_repaint();
            });
            self.welcome.drop_rx = Some(rx);
        }
    }

    /// Replace the recent entry for `old` with `new`, in place.
    fn relocate_recent(&mut self, old: &Path, new: PathBuf) {
        self.recent.retain(|entry| entry.path != new);
        if let Some(entry) = self.recent.iter_mut().find(|entry| entry.path == old) {
            entry.path = new;
        }
        self.welcome.failed = None;
        self.save_recent();
    }

    fn working_mutation_pending(&self) -> bool {
        self.working.as_ref().is_some_and(|working| working.busy())
    }

    fn repository_switch_pending(&self) -> bool {
        self.network.is_some() || self.branch.is_some() || self.operation_busy()
    }

    fn operation_busy(&self) -> bool {
        self.operation_worker.is_some()
            || self.rewrite_job.is_some()
            || self.rewrite_refresh.is_some()
            || self.working_mutation_pending()
    }

    /// Operations that make Local Changes mutations incompatible.
    fn external_operation_reason(&self) -> Option<String> {
        if self.rewrite_job.is_some() {
            Some("Wait for the history rewrite to finish".into())
        } else if self.rewrite_refresh.is_some() {
            Some("Wait for history to refresh".into())
        } else if self.operation_worker.is_some() {
            Some("Wait for the current operation to finish".into())
        } else if self.network.is_some() {
            Some("Wait for the network operation to finish".into())
        } else if self.branch.is_some() {
            Some("Wait for the branch operation to finish".into())
        } else {
            None
        }
    }

    /// The operation owning the band beneath the toolbar: one Git holds open,
    /// or a rewrite while it runs and its history is read.
    fn operation_area(&self) -> Option<OperationArea> {
        if let Some(state) = &self.mutation {
            return Some(OperationArea::Mutation {
                operation: state.operation,
                conflicts: state.conflicts.len(),
                pending: self.operation_busy() || self.network.is_some() || self.branch.is_some(),
            });
        }
        if self.rewrite_job.is_some() {
            return Some(OperationArea::Rewrite { refreshing: false });
        }
        if self.rewrite_refresh.is_some() {
            return Some(OperationArea::Rewrite { refreshing: true });
        }
        None
    }

    /// Changed files in the working tree, when known.
    fn changed_files(&self) -> Option<usize> {
        self.local_summary
            .as_ref()
            .and_then(|summary| summary.changes)
    }

    /// The current branch's remote, else `origin`.
    fn tracking_remote(&self) -> String {
        self.state
            .refs
            .current_tracking()
            .map(|tracking| tracking.remote.clone())
            .unwrap_or_else(|| "origin".into())
    }

    /// The current branch's short name, or `HEAD` when detached.
    fn current_branch_name(&self) -> String {
        self.state
            .refs
            .head
            .as_ref()
            .and_then(|head| head.name.as_deref())
            .map(|name| name.strip_prefix("refs/heads/").unwrap_or(name).to_string())
            .unwrap_or_else(|| "HEAD".into())
    }

    /// The ref a commit is known by other than the current branch: a
    /// branch, then a remote branch, then a tag.
    fn ref_for_commit(&self, commit: &str) -> Option<String> {
        let current = self
            .state
            .refs
            .head
            .as_ref()
            .and_then(|head| head.name.clone());
        let candidates = self
            .state
            .refs
            .for_commit(commit)
            .filter(|item| Some(&item.full_name) != current.as_ref())
            .collect::<Vec<_>>();
        [
            gg_engine::refs::RefKind::Branch,
            gg_engine::refs::RefKind::Remote,
            gg_engine::refs::RefKind::Tag,
        ]
        .into_iter()
        .find_map(|kind| candidates.iter().find(|item| item.kind == kind))
        .map(|item| item.name.clone())
    }

    /// The band for a merge, cherry-pick, or revert Git holds open.
    fn mutation_band(&self) -> Option<operation_band::Band> {
        let state = self.mutation.as_ref()?;
        let pending = self.operation_busy() || self.network.is_some() || self.branch.is_some();
        let current = self.current_branch_name();
        let incoming = (state.operation == gg_engine::mutation::Operation::Merge)
            .then(|| {
                state
                    .commit
                    .as_deref()
                    .and_then(|id| self.ref_for_commit(id))
            })
            .flatten();
        let blocks = state
            .conflicts
            .iter()
            .map(|path| {
                self.conflict_view
                    .as_ref()
                    .filter(|view| view.paths.get(view.selected) == Some(path))
                    .and_then(|view| view.blocks.as_ref().map(conflict_blocks::Blocks::left))
                    .or_else(|| {
                        self.conflict_view
                            .as_ref()
                            .and_then(|view| view.counts.get(path).copied())
                    })
                    .or_else(|| self.conflict_counts.get(path).copied())
                    .unwrap_or(1)
            })
            .sum::<usize>();
        // Every staged file still counts until Git sees it resolved.
        let files = state.conflicts.len();
        Some(operation_band::mutation_band(
            &operation_band::Subject {
                operation: state.operation,
                current: &current,
                incoming: incoming.as_deref(),
                commit: state.commit.as_deref(),
            },
            operation_band::Conflicts {
                files,
                blocks: blocks.max(files),
            },
            self.conflict_view.is_some(),
            pending,
            self.mutation_busy,
            Vec::new(),
        ))
    }

    /// The band while a rewrite runs, then while its history is read.
    fn rewrite_band(&self, refreshing: bool) -> operation_band::Band {
        let (branch, planned) = &self.rewrite_steps;
        let live = self
            .rewrite_job
            .as_ref()
            .and_then(|job| job.live.lock().ok().and_then(|live| live.progress.clone()));
        let steps = match live {
            Some(progress) => operation_band::steps_from_progress(&progress),
            None => planned.clone(),
        };
        let branch = if branch.is_empty() {
            self.current_branch_name()
        } else {
            branch.clone()
        };
        operation_band::rewrite_band(&branch, steps, refreshing)
    }

    /// The one message the footer shows: a failure, then something
    /// blocked, then a running job, then a receipt (for 8 s), then the key
    /// hints.
    fn footer_message(&self, narrow: bool) -> footer::Message {
        use footer::{Act, Message, Tone, button};
        let rereading = self.pending.is_some();
        let with_actions = |mut message: Message, outcome: &Outcome| {
            if let Some(action) = outcome.recovery {
                let mut recovery = button(
                    recovery_label(action),
                    Act::Recovery(action),
                    outcome.severity != OutcomeSeverity::Info,
                );
                // A network retry waits until the repository was re-read.
                if rereading
                    && matches!(
                        action,
                        RecoveryAction::RetryFetch | RecoveryAction::RetryPush
                    )
                {
                    recovery.enabled = false;
                    recovery.tip = "Wait for Git Good to re-read the repository".into();
                }
                message.buttons.push(recovery);
            }
            if outcome.details.is_some() {
                message
                    .buttons
                    .push(button("Details…", Act::Details, false));
            }
            message.dismiss = true;
            message
        };
        let failure = |text: &str| {
            let mut message = Message::split(Tone::Failed, text);
            message.dismiss = true;
            message
        };
        // Failures.
        if let Some(outcome) = self
            .outcome
            .as_ref()
            .filter(|outcome| outcome.severity == OutcomeSeverity::Error)
        {
            return with_actions(Message::split(Tone::Failed, &outcome.summary), outcome);
        }
        if let Some(error) = &self.network_error {
            return failure(error);
        }
        if let Some(error) = self
            .state
            .error
            .as_ref()
            .filter(|_| self.state.history.is_some() || self.pending.is_some())
        {
            let mut message = failure(error);
            if self.state.history.is_some() {
                message.tail = " · showing previous results".into();
            }
            message.dismiss = false;
            return message;
        }
        if let Some(error) = self.branch_error.as_ref().or(self.mutation_error.as_ref()) {
            return failure(error);
        }
        // Blocked.
        if let Some(outcome) = self
            .outcome
            .as_ref()
            .filter(|outcome| outcome.severity == OutcomeSeverity::Warning)
        {
            return with_actions(Message::split(Tone::Blocked, &outcome.summary), outcome);
        }
        if let Some(warning) = &self.branch_warning {
            let mut message = Message::split(Tone::Blocked, warning);
            message.dismiss = true;
            return message;
        }
        // Running jobs. A band names its own Continue, Skip, or Abort.
        if self.operation_worker.is_some() && self.mutation_busy.is_none() {
            return Message::new(
                Tone::Progress,
                self.mutation_notice
                    .clone()
                    .unwrap_or_else(|| "Working…".into()),
            );
        }
        if let Some(worker) = &self.network {
            if worker.is_cancelled() {
                let mut message = Message::new(
                    Tone::Progress,
                    format!("Cancelling {}…", worker.operation.as_str()),
                );
                message.right = "waits for Git to stop".into();
                return message;
            }
            let mut message = match &self.network_progress {
                Some(progress) => {
                    let mut message = Message::new(Tone::Progress, worker.label.clone());
                    (message.tail, message.bar) = footer::progress_tail(progress);
                    message.bar = message.bar.or(Some(0.0));
                    message
                }
                None => Message::new(
                    Tone::Progress,
                    format!("Starting {}…", worker.operation.as_str()),
                ),
            };
            message.buttons.push(button("Cancel", Act::Cancel, false));
            return message;
        }
        if let Some(label) = &self.branch_label {
            return Message::new(Tone::Progress, label.clone());
        }
        if let Some(backup) = self
            .rewrite_job
            .as_ref()
            .and_then(|job| job.live.lock().ok().and_then(|live| live.backup.clone()))
        {
            return Message::new(Tone::Idle, format!("Backup: {backup}"));
        }
        if self.pending.is_some() {
            let mut message = Message::new(
                Tone::Progress,
                format!(
                    "Reading {}",
                    welcome_view::tilde(
                        self.state
                            .requested
                            .as_deref()
                            .unwrap_or(std::path::Path::new(".")),
                        std::env::var("HOME").ok().as_deref()
                    )
                ),
            );
            if self.state.history.is_some() {
                message.tail = " · showing previous results".into();
            }
            return message;
        }
        if !self.state.finished && self.worker.is_some() {
            return Message::new(
                Tone::Progress,
                format!(
                    "{} · {} commits examined",
                    if self.state.query.is_empty() {
                        "Counting history"
                    } else {
                        "Searching history"
                    },
                    self.state.scanned
                ),
            );
        }
        // Receipts, until they fade.
        if let Some(outcome) = &self.outcome {
            return with_actions(Message::split(Tone::Success, &outcome.summary), outcome);
        }
        if let Some((notice, _)) = self
            .notice_seen
            .as_ref()
            .filter(|(_, at)| at.elapsed() < SUCCESS_FADE)
        {
            let mut message = Message::split(Tone::Success, notice);
            message.dismiss = true;
            return message;
        }
        let hints = if self.state.history.is_none() && !self.recent.is_empty() && narrow {
            "Up/Down, Enter: open   ·   Del: remove   ·   Ctrl+O: open"
        } else if self.state.history.is_none() && !self.recent.is_empty() {
            "Up/Down: choose   ·   Enter: open   ·   Del: remove from recent   ·   Ctrl+O: open   ·   Ctrl+K: search   ·   ?: shortcuts"
        } else if self.state.history.is_none() {
            "Ctrl+O: open   ·   Ctrl+K: search   ·   ?: shortcuts"
        } else if narrow {
            "j/k: select · Enter: review · Space: details · Esc: history · Ctrl/Cmd+F: filter"
        } else {
            "j k: select   ·   Enter: review   ·   Alt+A: actions   ·   R: rebase from here   ·   Ctrl+K: search   ·   Ctrl/Cmd+F: filter   ·   Ctrl/Cmd+C: copy ID"
        };
        let mut message = Message::new(Tone::Idle, hints);
        message.changes = self.changed_files();
        message
    }

    /// The receipt a notice field holds now; a new one restarts the fade.
    fn track_notice(&mut self) {
        let current = self
            .network_notice
            .clone()
            .or_else(|| self.stash_notice.clone())
            .or_else(|| {
                self.operation_worker
                    .is_none()
                    .then(|| self.mutation_notice.clone())
                    .flatten()
            });
        match current {
            Some(text) if self.notice_seen.as_ref().map(|(seen, _)| seen) != Some(&text) => {
                self.notice_seen = Some((text, std::time::Instant::now()));
            }
            Some(_) => {}
            None => self.notice_seen = None,
        }
    }

    /// A failure stays in the footer until it is dismissed or the next job
    /// starts.
    fn clear_failure(&mut self) {
        if self
            .outcome
            .as_ref()
            .is_some_and(|outcome| outcome.severity == OutcomeSeverity::Error)
        {
            self.dismiss_outcome();
        }
    }

    /// Dismiss what the footer shows.
    fn dismiss_footer(&mut self) {
        if self.outcome.is_some() {
            self.dismiss_outcome();
        } else if self.network_error.is_some() {
            self.network_error = None;
        } else if self.branch_error.is_some() {
            self.branch_error = None;
        } else if self.mutation_error.is_some() {
            self.mutation_error = None;
        } else if self.branch_warning.is_some() {
            self.branch_warning = None;
        } else {
            self.network_notice = None;
            self.stash_notice = None;
            self.mutation_notice = None;
            self.notice_seen = None;
        }
    }

    fn dismiss_outcome(&mut self) {
        if self.outcome.as_ref().is_some_and(|outcome| {
            matches!(
                outcome.recovery,
                Some(RecoveryAction::RetrySwitch | RecoveryAction::StashAndSwitch)
            )
        }) {
            self.pending_switch = None;
        }
        self.outcome = None;
        self.outcome_details_open = false;
    }

    fn set_outcome(
        &mut self,
        severity: OutcomeSeverity,
        summary: String,
        details: Option<String>,
        recovery: Option<RecoveryAction>,
    ) {
        self.outcome = Some(Outcome {
            severity,
            summary,
            details,
            recovery,
            at: std::time::Instant::now(),
        });
    }

    fn run_recovery_action(&mut self, ctx: &egui::Context, action: RecoveryAction) {
        match action {
            RecoveryAction::Backups => {
                self.open_recovery(ctx, return_location::RecoverySource::Backups)
            }
            RecoveryAction::ResolveConflicts => self.open_conflict_view(ctx),
            RecoveryAction::RetryFetch => self.start_fetch(ctx, None, self.prefs.fetch_prune),
            RecoveryAction::RetryPush => self.start_push(ctx),
            RecoveryAction::RetrySwitch => {
                if let Some(target) = self.pending_switch.clone() {
                    self.start_branch(ctx, target, false);
                }
            }
            RecoveryAction::ManageStashes => egui::Popup::open_id(ctx, stash_picker_popup_id()),
            RecoveryAction::StashAndSwitch => {
                if let Some(target) = self.pending_switch.clone() {
                    self.stash_switch = Some(StashSwitchDialog {
                        target,
                        include_untracked: self.prefs.stash_include_untracked,
                        error: None,
                    });
                }
            }
            RecoveryAction::PullRebase => self.start_pull_rebase(ctx),
            RecoveryAction::ForcePush => {
                if let Some(target) = self.force_push_target() {
                    self.force_push = Some(target);
                }
            }
            RecoveryAction::Push => self.start_push(ctx),
            RecoveryAction::Fetch => {
                let remote = self
                    .state
                    .refs
                    .current_tracking()
                    .map(|tracking| tracking.remote.clone());
                self.start_fetch(ctx, remote, self.prefs.fetch_prune);
            }
        }
    }

    fn mutation_reason(&self) -> Option<&'static str> {
        if self.mutation.is_some() {
            Some("Recover the operation in progress first.")
        } else if self.operation_busy() {
            Some("Wait for the current operation to finish.")
        } else if self.branch.is_some() || self.network.is_some() {
            Some("Wait for the branch or network operation to finish.")
        } else {
            None
        }
    }

    fn run_commit_action(&mut self, ctx: &egui::Context, id: String, action: CommitAction) {
        let return_location = self.capture_return_location();
        self.state.selected = Some(id.clone());
        self.state.reconcile_selection();
        match action {
            CommitAction::Review => self.open_review(ctx, return_location),
            CommitAction::CherryPick => {
                if let Err(error) =
                    self.start_mutation(ctx, gg_engine::mutation::Operation::CherryPick)
                {
                    self.mutation_error = Some(error);
                }
            }
            CommitAction::Revert => {
                if let Err(error) = self.start_mutation(ctx, gg_engine::mutation::Operation::Revert)
                {
                    self.mutation_error = Some(error);
                }
            }
            CommitAction::Reset(mode) => {
                // The chosen mode runs as chosen; Hard always asks first
                // because it discards working changes.
                if self.prefs.confirm_destructive || mode == gg_engine::reset::Mode::Hard {
                    self.reset_confirm = Some((id, mode));
                } else if let Err(error) = self.start_reset(ctx, &id, mode) {
                    self.mutation_error = Some(error);
                }
            }
            CommitAction::Rewrite => {
                if let Err(error) = self.open_rewrite_dialog(&id) {
                    self.mutation_error = Some(error);
                }
            }
            CommitAction::Compare => self.open_compare(ctx, id, return_location),
        }
    }

    fn apply_commit_command(
        &mut self,
        ctx: &egui::Context,
        id: &str,
        subject: &str,
        command: CommitCommand,
    ) {
        match command {
            CommitCommand::Menu(command) => {
                let (mut local, mut history) = (false, false);
                self.run_frame_command(ctx, command, &mut local, &mut history);
                if local {
                    self.open_working(ctx);
                }
            }
            CommitCommand::CopySubject => ctx.copy_text(subject.to_string()),
            CommitCommand::OpenSignature => self.open_signature(ctx, id.to_string()),
            CommitCommand::Reset(mode) => {
                self.run_commit_action(ctx, id.to_string(), CommitAction::Reset(mode));
            }
        }
    }

    fn capture_return_location(&self) -> return_location::ReturnLocation {
        let location = return_location::HistoryLocation {
            selected: self.state.selected.clone(),
            query: self.state.query.clone(),
            offset: self.scroll_offset,
        };
        if self.narrow_details {
            return_location::ReturnLocation::Details(location)
        } else {
            return_location::ReturnLocation::History(location)
        }
    }

    fn restore_return_location(
        &mut self,
        ctx: &egui::Context,
        location: return_location::ReturnLocation,
    ) {
        if let return_location::ReturnLocation::Recovery {
            source,
            reference,
            entry,
        } = &location
        {
            self.open_recovery_at(ctx, *source, reference.as_deref(), entry);
            return;
        }
        let shows_details = location.shows_details();
        let Some(location) = location.location().cloned() else {
            return;
        };
        let query_changed = self.state.query != location.query;
        if query_changed {
            self.state.query = location.query;
            self.state.start_filter();
        }
        if query_changed || self.state.selected != location.selected {
            self.state.selected = location.selected;
            self.state.reconcile_selection();
        }
        self.scroll_offset = location.offset;
        self.restore_offset = Some(location.offset);
        self.narrow_details = shows_details;
        ctx.request_repaint();
    }

    fn start_operation(
        &mut self,
        ctx: &egui::Context,
        label: String,
        apply: impl FnOnce() -> Result<OperationOutcome, String> + Send + 'static,
    ) -> Result<(), String> {
        if self.operation_busy() {
            return Err("Wait for the current operation to finish.".into());
        }
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let repaint = ctx.clone();
        std::thread::Builder::new()
            .name("gg-operation".into())
            .spawn(move || {
                let result = apply();
                let _ = tx.send(result);
                repaint.request_repaint();
            })
            .map_err(|error| format!("Start operation: {error}"))?;
        self.operation_worker = Some(rx);
        self.clear_failure();
        self.operation_repo = self.stash_path();
        self.operation_restore_row = self.state.selected_row;
        self.mutation_error = None;
        self.mutation_notice = Some(label);
        Ok(())
    }

    fn finish_operation(&mut self, ctx: &egui::Context, outcome: OperationOutcome) {
        let restore_row = self.operation_restore_row.take();
        self.refresh_local_summary(ctx);
        match outcome {
            OperationOutcome::Merge(receipt, branch, incoming) => {
                let commits = incoming
                    .filter(|count| *count > 0)
                    .map(|count| format!(" · {}", operation_band::plural(count, "commit")))
                    .unwrap_or_default();
                let summary = match receipt.kind {
                    Some(gg_engine::merge::Kind::UpToDate) => {
                        format!("{branch} is already up to date with {}", receipt.reference)
                    }
                    Some(gg_engine::merge::Kind::FastForward) => {
                        format!("Fast-forwarded {branch} to {}{commits}", receipt.reference)
                    }
                    _ => format!("Merged {} into {branch}{commits}", receipt.reference),
                };
                // A merge Git stopped is the band's to report.
                if receipt.conflicts.is_empty() && !receipt.in_progress {
                    self.mutation_notice = Some(summary.clone());
                    self.set_outcome(OutcomeSeverity::Info, summary, None, None);
                } else {
                    self.mutation_notice = None;
                }
                self.conflict_view = None;
                self.restore_row = restore_row;
                self.refresh_mutation();
                self.load(ctx);
                self.refresh_working(ctx);
            }
            OperationOutcome::Rebase(receipt, branch) => {
                self.rebase_target = None;
                // A rebase Git stopped is the band's to report.
                if receipt.in_progress || !receipt.conflicts.is_empty() {
                    self.mutation_notice = None;
                } else {
                    let summary = format!("Rebased {branch} onto {}", receipt.reference);
                    self.mutation_notice = Some(summary.clone());
                    self.set_outcome(OutcomeSeverity::Info, summary, None, None);
                }
                self.conflict_view = None;
                self.restore_row = restore_row;
                self.refresh_mutation();
                self.load(ctx);
                self.refresh_reflog();
                self.refresh_backups();
                self.refresh_working(ctx);
            }
            OperationOutcome::Conflict { file } => {
                let summary = format!("Resolved {file}");
                self.mutation_notice = Some(summary.clone());
                self.set_outcome(OutcomeSeverity::Info, summary, None, None);
                self.refresh_mutation();
                if let Some(view) = &mut self.conflict_view
                    && !view.resolved.contains(&file)
                {
                    view.resolved.push(file.clone());
                }
                self.refresh_conflict_view(ctx, Some(&file));
                self.load(ctx);
                self.refresh_working(ctx);
            }
            OperationOutcome::Reopened { file, conflicted } => {
                let summary = if conflicted {
                    format!("Reopened the conflict in {file}")
                } else {
                    format!("Unstaged {file}")
                };
                self.set_outcome(OutcomeSeverity::Info, summary, None, None);
                self.refresh_mutation();
                if let Some(view) = &mut self.conflict_view {
                    view.resolved.retain(|path| path != &file);
                }
                self.refresh_conflict_view(ctx, conflicted.then_some(file.as_str()));
                self.load(ctx);
                self.refresh_working(ctx);
            }
            OperationOutcome::Recovery {
                operation,
                action,
                receipt,
            } => {
                // Git stopping again is the band's to report.
                if receipt.conflicts.is_empty() {
                    let summary = match action {
                        MutationAction::Continue => format!("Continued {}", operation.as_str()),
                        MutationAction::Skip => format!("Skipped {}", operation.as_str()),
                        MutationAction::Abort => format!("Aborted {}", operation.as_str()),
                    };
                    self.mutation_notice = Some(summary.clone());
                    self.set_outcome(OutcomeSeverity::Info, summary, None, None);
                } else {
                    self.mutation_notice = None;
                }
                self.refresh_mutation();
                if self.mutation.is_none() {
                    self.conflict_view = None;
                }
                self.restore_row = restore_row;
                self.load(ctx);
                self.refresh_working(ctx);
            }
        }
    }

    fn recover_mutation(
        &mut self,
        ctx: &egui::Context,
        operation: gg_engine::mutation::Operation,
        action: MutationAction,
    ) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before recovering history.")?;
        let command = match action {
            MutationAction::Continue => operation_band::Command::Continue,
            MutationAction::Skip => operation_band::Command::Skip,
            MutationAction::Abort => operation_band::Command::Abort,
        };
        let label = format!("{}…", operation_band::busy_label(operation, command));
        self.start_operation(ctx, label, move || {
            let receipt = match action {
                MutationAction::Continue => gg_git::mutation::continue_operation(&path, operation)?,
                MutationAction::Skip => gg_git::mutation::skip_operation(&path, operation)?,
                MutationAction::Abort => gg_git::mutation::abort_operation(&path, operation)?,
            };
            Ok(OperationOutcome::Recovery {
                operation,
                action,
                receipt,
            })
        })?;
        // The band keeps its place and names the job until the result arrives.
        self.mutation_busy = Some(command);
        Ok(())
    }

    fn start_merge(
        &mut self,
        ctx: &egui::Context,
        reference: &str,
        no_ff: bool,
    ) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before merging.")?;
        let reference = reference.to_string();
        let branch = self.current_branch_name();
        let label = format!("Merging {reference} into {branch}…");
        self.start_operation(ctx, label, move || {
            let incoming = gg_git::merge::incoming(&path, &reference).ok();
            gg_git::merge::merge(&path, &reference, no_ff)
                .map(|receipt| OperationOutcome::Merge(receipt, branch, incoming))
        })
    }

    /// Pull with the fast-forward-only strategy: move the branch to the
    /// upstream, or refuse when they have diverged.
    fn start_fast_forward(&mut self, ctx: &egui::Context, reference: &str) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before pulling.")?;
        let reference = reference.to_string();
        let branch = self.current_branch_name();
        let label = format!("Fast-forwarding {branch} to {reference}…");
        self.start_operation(ctx, label, move || {
            let incoming = gg_git::merge::incoming(&path, &reference).ok();
            gg_git::merge::fast_forward(&path, &reference)
                .map(|receipt| OperationOutcome::Merge(receipt, branch, incoming))
        })
    }

    fn start_rebase(
        &mut self,
        ctx: &egui::Context,
        reference: &str,
        autostash: bool,
    ) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before rebasing.")?;
        let reference = reference.to_string();
        let branch = self.current_branch_name();
        let label = format!("Rebasing {branch} onto {reference}…");
        self.rebase_target = Some(reference.clone());
        let started = self.start_operation(ctx, label, move || {
            gg_git::rebase::rebase(&path, &reference, autostash)
                .map(|receipt| OperationOutcome::Rebase(receipt, branch))
        });
        if started.is_err() {
            self.rebase_target = None;
        }
        started
    }

    fn open_rewrite_dialog(&mut self, from: &str) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before rewriting history.")?;
        if self.mutation.is_some() {
            return Err("Recover the operation in progress before rewriting history.".into());
        }
        let (commits, base) = gg_git::rewrite::plan_range(&path, from)?;
        let mut dialog = rebase_dialog::Dialog::new(path, commits, base, &self.state.refs)?;
        dialog.autostash = self.prefs.rebase_autostash;
        self.rewrite_dialog = Some(dialog);
        Ok(())
    }

    fn begin_rewrite(
        &mut self,
        ctx: &egui::Context,
        from: String,
        steps: Vec<gg_engine::rewrite::Step>,
        autostash: bool,
    ) -> Result<(), String> {
        if self.rewrite_job.is_some() || self.rewrite_refresh.is_some() {
            return Err("A rewrite is already running.".into());
        }
        if self.mutation.is_some() {
            return Err("Recover the operation in progress before rewriting history.".into());
        }
        if self.operation_worker.is_some() || self.network.is_some() || self.branch.is_some() {
            return Err("Wait for the current operation to finish before rewriting.".into());
        }
        let path = self
            .stash_path()
            .ok_or("Open a repository before rewriting history.")?;
        let restore_row = self.state.selected_row;
        let operation_path = path.clone();
        // The band shows the plan until Git reports its own steps.
        let planned = steps
            .iter()
            .filter(|step| step.action != gg_engine::rewrite::Action::Drop)
            .enumerate()
            .map(|(index, step)| operation_band::Step {
                commit: step.commit.clone(),
                action: step.action.as_str().into(),
                subject: String::new(),
                state: if index == 0 {
                    operation_band::StepState::Current
                } else {
                    operation_band::StepState::Next
                },
            })
            .collect();
        self.rewrite_steps = (self.current_branch_name(), planned);
        let live = std::sync::Arc::new(std::sync::Mutex::new(RewriteLive::default()));
        let worker_live = std::sync::Arc::clone(&live);
        let (sender, receiver) = std::sync::mpsc::channel();
        let repaint = ctx.clone();
        std::thread::Builder::new()
            .name("gg-rewrite".into())
            .spawn(move || {
                // Git's steps, read beside the rewrite while it runs.
                let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let watcher = {
                    let (done, live, path, repaint) = (
                        std::sync::Arc::clone(&done),
                        std::sync::Arc::clone(&worker_live),
                        path.clone(),
                        repaint.clone(),
                    );
                    std::thread::Builder::new()
                        .name("gg-rewrite-steps".into())
                        .spawn(move || {
                            while !done.load(std::sync::atomic::Ordering::Relaxed) {
                                std::thread::sleep(std::time::Duration::from_millis(150));
                                if let Ok(Some(progress)) = gg_git::mutation::rebase_progress(&path)
                                    && let Ok(mut live) = live.lock()
                                    && live.progress.as_ref() != Some(&progress)
                                {
                                    live.progress = Some(progress);
                                    repaint.request_repaint();
                                }
                            }
                        })
                        .ok()
                };
                // The follow-up push depends on how the result stands against
                // the upstream; read it here, off the UI thread.
                let result = gg_git::rewrite::rewrite_observed(
                    &path,
                    &from,
                    &steps,
                    autostash,
                    &mut |backup| {
                        if let Ok(mut live) = worker_live.lock() {
                            live.backup = Some(backup.to_string());
                        }
                        repaint.request_repaint();
                    },
                )
                .map(|receipt| {
                    let relation = gg_git::sync::upstream_relation(&path, None).ok().flatten();
                    (receipt, relation)
                });
                done.store(true, std::sync::atomic::Ordering::Relaxed);
                if let Some(watcher) = watcher {
                    let _ = watcher.join();
                }
                let _ = sender.send(result);
                repaint.request_repaint();
            })
            .map_err(|error| format!("Start rewrite: {error}"))?;
        self.rewrite_job = Some(RewriteJob {
            receiver,
            restore_row,
            live,
        });
        self.operation_repo = Some(operation_path);
        self.rewrite_dialog = None;
        Ok(())
    }

    fn finish_rewrite(
        &mut self,
        ctx: &egui::Context,
        receipt: gg_engine::rewrite::Receipt,
        relation: Option<gg_engine::sync::UpstreamRelation>,
        restore_row: Option<usize>,
    ) -> u64 {
        self.refresh_local_summary(ctx);
        self.mutation_error = None;
        let conflicts = receipt.in_progress || !receipt.conflicts.is_empty();
        let rewrote = operation_band::plural(receipt.steps, "commit");
        let summary = match (&receipt.backup, receipt.in_progress) {
            (_, true) => format!("Stopped rewriting {rewrote}"),
            (Some(backup), false) => format!("Rewrote {rewrote} · backup {backup}"),
            (None, false) => format!("Rewrote {rewrote}"),
        };
        self.mutation_notice = None;
        let mut details = Vec::new();
        if let Some(backup) = &receipt.backup {
            details.push(format!("Backup: {backup}"));
        }
        self.rewrite_backup = receipt.backup.clone();
        if !receipt.conflicts.is_empty() {
            details.push(format!("Conflicts: {}", receipt.conflicts.join(", ")));
        }
        // A rewrite Git stopped is the band's to report.
        if !conflicts {
            self.set_outcome(
                OutcomeSeverity::Info,
                summary,
                (!details.is_empty()).then(|| details.join("\n")),
                Some(rewrite_follow_up(
                    conflicts,
                    self.prefs.offer_force_lease,
                    relation,
                )),
            );
        }
        self.conflict_view = None;
        self.restore_row = restore_row;
        self.refresh_mutation();
        self.load(ctx);
        self.refresh_reflog();
        self.refresh_backups();
        self.refresh_working(ctx);
        self.state.generation
    }

    fn open_stash_dialog(&mut self) {
        self.stash_dialog = Some(StashDialog {
            message: String::new(),
            include_untracked: self.prefs.stash_include_untracked,
            keep_index: self.prefs.stash_keep_index,
            error: None,
        });
    }

    fn open_merge_dialog(&mut self) {
        self.merge_dialog = Some(MergeDialog {
            reference: String::new(),
            no_ff: self.prefs.merge_no_ff,
            error: None,
        });
    }

    /// Readiness shared by the window menus and toolbar.
    fn frame_readiness(&self) -> FrameReadiness {
        let has_repo = self.state.history.is_some();
        let network_idle = self.network.is_none()
            && self.branch.is_none()
            && self.mutation.is_none()
            && !self.operation_busy();
        let branch_idle = self.branch.is_none()
            && self.network.is_none()
            && self.mutation.is_none()
            && !self.operation_busy();
        let tracking = self.state.refs.current_tracking();
        let behind = tracking.and_then(|tracking| tracking.behind).unwrap_or(0);
        let pull_reason = if !has_repo {
            Some("Open a repository first")
        } else if !network_idle {
            Some("Wait for the current operation to finish")
        } else if tracking.is_none() {
            Some("The current branch has no upstream to pull from")
        } else {
            // Cached counts may be stale: Pull fetches first, then decides.
            None
        };
        let stash_idle =
            has_repo && self.branch.is_none() && self.network.is_none() && !self.operation_busy();
        let summary = self.local_summary.as_ref();
        let stash_reason = if !has_repo {
            Some("Open a repository first")
        } else if !stash_idle {
            Some("Wait for the current operation to finish")
        } else if summary.and_then(|summary| summary.changes) == Some(0) {
            Some("No local changes to stash")
        } else {
            None
        };
        let branch_rewrite = summary.and_then(|summary| summary.branch_start.as_ref());
        let branch_rewrite_reason = if !has_repo {
            Some("Open a repository first")
        } else if let Some(reason) = self.mutation_reason() {
            Some(reason)
        } else if summary.is_none() {
            Some("Reading the current branch…")
        } else if branch_rewrite.is_none() {
            Some(
                "The current branch has no commits beyond its upstream or the default branch; use Commit › Interactive rebase from here… on a selected commit",
            )
        } else {
            None
        };
        FrameReadiness {
            has_repo,
            network_idle: has_repo && network_idle,
            branch_idle: has_repo && branch_idle && self.state.refs.head.is_some(),
            stash_idle,
            stash_reason,
            stashes: summary.map_or(0, |summary| summary.stashes.len()),
            changes: summary.and_then(|summary| summary.changes).unwrap_or(0),
            branch_rewrite_reason,
            behind,
            pull_reason,
            rewrite_reason: if self.state.selected.is_none() {
                Some("Select a commit in History first")
            } else {
                self.mutation_reason()
            },
        }
    }

    fn open_new_tag(&mut self) {
        let Some(path) = self.stash_path() else {
            return;
        };
        let target = self
            .state
            .selected
            .clone()
            .or_else(|| {
                self.state
                    .refs
                    .head
                    .as_ref()
                    .and_then(|head| head.commit.clone())
            })
            .unwrap_or_default();
        if target.is_empty() {
            self.mutation_error = Some("Create the first commit before creating a tag.".into());
            return;
        }
        let config = gg_git::signature::config(&path);
        let ready = config.ready();
        self.tag_create = Some(TagCreateDialog {
            name: String::new(),
            target,
            message: String::new(),
            sign: config.tag_signing || self.prefs.sign_tags,
            signing_ready: ready,
            signing_note: if ready {
                None
            } else {
                Some(
                    config
                        .note
                        .unwrap_or_else(|| "Signing key or tool is unavailable.".into()),
                )
            },
            error: None,
            pending: None,
        });
    }

    fn start_tag_create(&mut self, ctx: &egui::Context) {
        let Some(path) = self.stash_path() else {
            return;
        };
        let Some(dialog) = &mut self.tag_create else {
            return;
        };
        if dialog.pending.is_some() {
            return;
        }
        let name = dialog.name.trim().to_string();
        let target = dialog.target.trim().to_string();
        let message = dialog.message.clone();
        if name.is_empty() {
            dialog.error = Some("Enter a tag name.".into());
            return;
        }
        if target.is_empty() {
            dialog.error = Some("Enter a target reference.".into());
            return;
        }
        if message.trim().is_empty() {
            dialog.error = Some("Enter an annotated tag message.".into());
            return;
        }
        let signed = dialog.sign;
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let repaint = ctx.clone();
        match std::thread::Builder::new()
            .name("gg-tag-create".into())
            .spawn(move || {
                let result = gg_git::tag::create(&path, &name, &target, &message, signed);
                let _ = tx.send(result);
                repaint.request_repaint();
            }) {
            Ok(_) => {
                dialog.pending = Some(rx);
                dialog.error = None;
            }
            Err(error) => dialog.error = Some(format!("Start tag creation: {error}")),
        }
    }

    fn poll_tag_create(&mut self) -> Option<Result<gg_git::tag::CreateReceipt, String>> {
        let dialog = self.tag_create.as_mut()?;
        let receiver = dialog.pending.as_ref()?;
        match receiver.try_recv() {
            Ok(result) => {
                dialog.pending = None;
                Some(result)
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                dialog.pending = None;
                Some(Err("Tag worker stopped. Review refs and try again.".into()))
            }
        }
    }

    fn draw_tag_create(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.poll_tag_create() {
            match result {
                Ok(receipt) => {
                    self.mutation_notice = Some(format!(
                        "Created {} tag {}{}",
                        if receipt.signed {
                            "signed"
                        } else {
                            "annotated"
                        },
                        receipt.tag,
                        if receipt.target == "HEAD" {
                            String::new()
                        } else {
                            format!(" at {}", short_id(&receipt.target))
                        }
                    ));
                    self.tag_create = None;
                    self.load(ctx);
                }
                Err(error) => {
                    if let Some(dialog) = &mut self.tag_create {
                        dialog.error = Some(error);
                    }
                }
            }
        }
        if self.tag_create.is_none() {
            return;
        }
        let mut open = true;
        let mut submit = false;
        let mut cancel = false;
        {
            let Some(dialog) = &mut self.tag_create else {
                return;
            };
            egui::Window::new("Create tag")
                .id(egui::Id::unique("create-tag")).modal(ctx, egui::Id::unique("create-tag"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(420.0)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    egui::Grid::new("create-tag-fields")
                        .num_columns(2)
                        .spacing([10.0, 6.0])
                        .show(ui, |ui| {
                            ui.weak("Name");
                            ui.add(
                                egui::TextEdit::singleline(&mut dialog.name)
                                    .hint_text("v1.0.0")
                                    .desired_width(f32::INFINITY),
                            );
                            ui.end_row();
                            ui.weak("Target");
                            ui.add(
                                egui::TextEdit::singleline(&mut dialog.target)
                                    .hint_text("HEAD or a commit/branch")
                                    .desired_width(f32::INFINITY),
                            );
                            ui.end_row();
                            ui.weak("Message");
                            ui.add(
                                egui::TextEdit::multiline(&mut dialog.message)
                                    .hint_text("Annotated tag message")
                                    .desired_rows(3)
                                    .desired_width(f32::INFINITY),
                            );
                            ui.end_row();
                        });
                    let mut sign = dialog.sign;
                    let ready = dialog.signing_ready;
                    let busy = dialog.pending.is_some();
                    let response = ui.add_enabled(
                        ready && !busy,
                        egui::Checkbox::new(&mut sign, "Sign tag"),
                    );
                    let response = if ready {
                        response
                            .on_hover_text("Create a signed tag with the configured key and format")
                    } else {
                        response.on_disabled_hover_text(
                            dialog
                                .signing_note
                                .as_deref()
                                .unwrap_or("Signing is unavailable."),
                        )
                    };
                    if response.changed() {
                        dialog.sign = sign;
                    }
                    if let Some(note) = &dialog.signing_note {
                        ui.weak(note);
                    }
                    ui.weak("Tags are created at the target shown above; HEAD and the index are not changed.");
                    if let Some(error) = &dialog.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.horizontal(|ui| {
                        let can_create =
                            !busy && (!dialog.sign || dialog.signing_ready);
                        let response = ui.add_enabled(
                            can_create,
                            egui::Button::new(if dialog.sign {
                                "Create signed tag"
                            } else {
                                "Create tag"
                            }),
                        );
                        let response = if can_create {
                            response
                        } else if busy {
                            response.on_disabled_hover_text("Tag creation is running")
                        } else {
                            response.on_disabled_hover_text(
                                dialog
                                    .signing_note
                                    .as_deref()
                                    .unwrap_or("Signing is unavailable."),
                            )
                        };
                        if response.clicked() {
                            submit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                    if busy {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.weak("Creating tag…");
                        });
                    }
                });
        }
        if submit {
            self.start_tag_create(ctx);
        }
        if cancel || !open {
            self.tag_create = None;
        }
    }

    fn open_new_branch(&mut self) {
        let Some(history) = &self.state.history else {
            return;
        };
        let commit = self
            .state
            .refs
            .head
            .as_ref()
            .and_then(|head| head.commit.clone())
            .or_else(|| self.state.selected.clone())
            .unwrap_or_default();
        if commit.is_empty() {
            self.branch_error = Some("Create the first commit before creating a branch.".into());
            return;
        }
        let _ = history;
        self.create_branch = Some(CreateBranchDialog {
            name: String::new(),
            commit,
            selector: "current HEAD".into(),
            switch: self.prefs.branch_switch,
            recovery_source: None,
            error: None,
        });
    }

    /// The conflict workspace in the main area, under the operation band: the
    /// Changes view while conflicts exist ([`conflict_workspace`]).
    fn draw_conflict_workspace(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        let operation = self.mutation.as_ref().map(|state| state.operation);
        let conflict_pending =
            self.operation_busy() || self.network.is_some() || self.branch.is_some();
        let free = crate::navigation::available(ctx);
        // Ctrl+S stages, also while typing in the Result.
        let save_key =
            free && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S));
        let typing = ctx.text_edit_focused();
        let mut confirm = None;
        if let Some(view) = &mut self.conflict_view
            && !view.whole_file
            && view.confirm_block.is_none()
            && let Some(blocks) = &mut view.blocks
            && free
        {
            // Undo and redo restore the Result with its origins.
            let redo = ctx.input_mut(|i| {
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::Z,
                ) || i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)
            });
            if redo {
                blocks.redo();
            } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
                blocks.undo();
            }
            if !typing {
                confirm = blocks.keys(ctx);
            }
        }
        if let Some(view) = &mut self.conflict_view
            && confirm.is_some()
        {
            view.confirm_block = confirm;
        }
        let editor_tip = if self.prefs.editor.trim().is_empty() {
            "Open the working file with the system's default app (set an editor in Preferences)"
        } else {
            "Open the working file with the editor from Preferences"
        };
        let mut actions = match &mut self.conflict_view {
            Some(view) => conflict_workspace::show(
                ui,
                view,
                &conflict_workspace::Info {
                    operation,
                    pending: conflict_pending,
                    editor_tip,
                },
            ),
            None => Vec::new(),
        };
        if save_key
            && !conflict_pending
            && let Some(view) = &self.conflict_view
        {
            let blocks = view.blocks.as_ref().filter(|_| !view.whole_file);
            match blocks {
                Some(blocks) if blocks.complete() && view.edited => {
                    actions.push(conflict_workspace::Action::ConfirmReplace);
                }
                Some(blocks) if blocks.complete() => {
                    actions.push(conflict_workspace::Action::Stage(ConflictStage::Preview));
                }
                None if conflict_preview_available(view) => {
                    actions.push(conflict_workspace::Action::Stage(ConflictStage::Preview));
                }
                _ => {}
            }
        }
        // A question opens in the frame that asked it.
        if let Some(view) = &mut self.conflict_view {
            actions.retain(|action| {
                let ask = *action == conflict_workspace::Action::ConfirmReplace;
                view.confirm_replace |= ask;
                !ask
            });
        }
        self.draw_conflict_questions(ctx, conflict_pending, &mut actions);
        for action in actions {
            match action {
                conflict_workspace::Action::Select(index) => {
                    if let Some(view) = &mut self.conflict_view {
                        view.selected = index;
                    }
                    self.load_conflict_content(ctx);
                }
                conflict_workspace::Action::Refresh => self.load_conflict_content(ctx),
                conflict_workspace::Action::OpenInEditor => {
                    let file = self
                        .stash_path()
                        .zip(
                            self.conflict_view
                                .as_ref()
                                .and_then(|view| view.paths.get(view.selected).cloned()),
                        )
                        .map(|(workdir, path)| workdir.join(path));
                    if let Some(file) = file
                        && let Some(view) = &mut self.conflict_view
                    {
                        view.error = editor::open(&self.prefs.editor, &file).err();
                    }
                }
                conflict_workspace::Action::ConfirmReplace => {}
                conflict_workspace::Action::Stage(stage) => {
                    if let Err(error) = self.stage_conflict(ctx, stage)
                        && let Some(view) = &mut self.conflict_view
                    {
                        view.error = Some(error);
                    }
                }
                conflict_workspace::Action::Unstage(file) => {
                    if let Err(error) = self.reopen_conflict(ctx, file)
                        && let Some(view) = &mut self.conflict_view
                    {
                        view.error = Some(error);
                    }
                }
            }
        }
    }

    /// The workspace's two questions: **Replace with blocks…** over a file
    /// edited elsewhere, and an option over a block edited by hand.
    fn draw_conflict_questions(
        &mut self,
        ctx: &egui::Context,
        pending: bool,
        actions: &mut Vec<conflict_workspace::Action>,
    ) {
        let Some(view) = &mut self.conflict_view else {
            return;
        };
        let file = view.paths.get(view.selected).cloned().unwrap_or_default();
        let question = if view.confirm_replace {
            Some((
                "Replace edited file",
                format!(
                    "Staging the blocks overwrites {file}. The edits made outside the conflict blocks are lost."
                ),
                "Replace and stage",
            ))
        } else {
            view.confirm_block.map(|(index, choice)| {
                (
                    "Replace edited block",
                    format!(
                        "Conflict {} was edited by hand in the Result. {} replaces those edits; Ctrl+Z brings them back.",
                        index + 1,
                        choice.label()
                    ),
                    "Replace block",
                )
            })
        };
        let Some((title, body, confirm)) = question else {
            return;
        };
        let mut open = true;
        let mut cancelled =
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        let mut accepted =
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        egui::Window::new(title)
            .id(egui::Id::unique("conflict-question"))
            .modal(ctx, egui::Id::unique("conflict-question"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_pos(ctx.content_rect().center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                ui.set_width(420.0);
                ui.add(egui::Label::new(body).wrap());
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(consequence::danger_button(confirm, true)).clicked() {
                            accepted = true;
                        }
                        if ui
                            .button(("Cancel", egui::RichText::new("Esc").small().weak()))
                            .clicked()
                        {
                            cancelled = true;
                        }
                    });
                });
            });
        if accepted || cancelled || !open {
            if view.confirm_replace {
                view.confirm_replace = false;
                if accepted && !pending {
                    actions.push(conflict_workspace::Action::Stage(ConflictStage::Preview));
                }
            } else if let Some((index, choice)) = view.confirm_block.take()
                && accepted
                && let Some(blocks) = &mut view.blocks
            {
                blocks.choose(index, choice);
            }
        }
    }

    /// − on a resolved file: Git recreates its conflict (or unstages it).
    fn reopen_conflict(&mut self, ctx: &egui::Context, file: String) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before resolving conflicts.")?;
        let label = format!("Unstaging {file}…");
        self.start_operation(ctx, label, move || {
            let conflicted = gg_git::conflict::reopen(&path, &file)?;
            Ok(OperationOutcome::Reopened { file, conflicted })
        })
    }

    /// The conflict workspace is the Changes view while conflicts exist.
    fn open_conflict_view(&mut self, ctx: &egui::Context) {
        self.open_working(ctx);
        self.refresh_conflict_view(ctx, None);
    }

    fn refresh_conflict_view(&mut self, ctx: &egui::Context, keep: Option<&str>) {
        let Some(path) = self.stash_path() else {
            return;
        };
        let view = self.conflict_view.get_or_insert_with(ConflictView::default);
        view.keep = keep.map(str::to_string);
        view.previous_selected = view.selected;
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let repaint = ctx.clone();
        if std::thread::Builder::new()
            .name("gg-conflicts".into())
            .spawn(move || {
                let result = gg_git::conflict::unmerged(&path).map(|paths| {
                    // Blocks left per file and what else the operation
                    // changed, read here, off the UI thread.
                    let counts = paths
                        .iter()
                        .filter_map(|file| {
                            gg_git::conflict::blocks(&path, file).ok().map(|segments| {
                                (file.clone(), gg_engine::conflict::block_count(&segments))
                            })
                        })
                        .collect();
                    let also = gg_git::conflict::also_changed(&path).unwrap_or_default();
                    ConflictUpdate::Paths {
                        paths,
                        also,
                        counts,
                    }
                });
                let _ = tx.send(result);
                repaint.request_repaint();
            })
            .is_err()
        {
            view.error = Some("Could not start the conflict reader.".into());
            view.loading = false;
            return;
        }
        view.loading = true;
        view.error = None;
        view.worker = Some(rx);
    }

    fn poll_conflict_view(&mut self, ctx: &egui::Context) {
        let message = match self
            .conflict_view
            .as_ref()
            .and_then(|view| view.worker.as_ref())
            .map(|rx| rx.try_recv())
        {
            None | Some(Err(std::sync::mpsc::TryRecvError::Empty)) => return,
            Some(Ok(result)) => result,
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                Err("Conflict reader stopped. Reopen conflicts and retry.".into())
            }
        };
        let mut load_content = false;
        let operation = self.mutation.as_ref().map(|state| state.operation);
        if let Some(view) = &mut self.conflict_view {
            view.worker = None;
            view.loading = false;
            match message {
                Ok(ConflictUpdate::Paths {
                    paths,
                    also,
                    counts,
                }) => {
                    let keep = view.keep.take();
                    view.also = also;
                    self.conflict_counts = counts.clone();
                    view.counts = counts;
                    view.selected =
                        next_conflict_selection(&paths, keep.as_deref(), view.previous_selected);
                    view.paths = paths;
                    view.content = None;
                    view.snapshot = None;
                    view.blocks = None;
                    view.blocks_note = None;
                    view.edited = false;
                    view.confirm_replace = false;
                    view.error = None;
                    load_content = !view.paths.is_empty();
                }
                Ok(ConflictUpdate::Content {
                    content,
                    snapshot,
                    blocks,
                    edited,
                    names,
                }) => {
                    view.content = Some(*content);
                    view.snapshot = Some(*snapshot);
                    view.edited = edited;
                    view.confirm_replace = false;
                    view.names = names;
                    match blocks {
                        Ok(segments) => {
                            let markers =
                                conflict_workspace::markers(operation, view.names.as_ref());
                            view.blocks =
                                Some(conflict_blocks::Blocks::with_markers(segments, markers));
                            view.blocks_note = None;
                        }
                        Err(note) => {
                            view.blocks = None;
                            view.blocks_note = Some(note);
                        }
                    }
                    view.preview = gg_engine::conflict::Resolution::Working;
                    view.confirm_block = None;
                    view.error = None;
                }
                Err(error) => {
                    view.content = None;
                    view.error = Some(error);
                }
            }
        }
        if load_content {
            self.load_conflict_content(ctx);
        }
    }

    fn load_conflict_content(&mut self, ctx: &egui::Context) {
        let Some(path) = self.stash_path() else {
            return;
        };
        let Some(view) = &mut self.conflict_view else {
            return;
        };
        let Some(file) = view.paths.get(view.selected).cloned() else {
            view.content = None;
            view.snapshot = None;
            view.loading = false;
            return;
        };
        view.content = None;
        view.snapshot = None;
        view.blocks = None;
        view.blocks_note = None;
        view.edited = false;
        view.confirm_replace = false;
        view.preview = gg_engine::conflict::Resolution::Working;
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let repaint = ctx.clone();
        if std::thread::Builder::new()
            .name("gg-conflict-content".into())
            .spawn(move || {
                let result = (|| {
                    let content = gg_git::conflict::read(&path, &file)?;
                    let snapshot = gg_git::conflict::snapshot(&path, &file)?;
                    let blocks = if content.binary {
                        Err("This file is binary. Choose a whole side.".into())
                    } else {
                        gg_git::conflict::blocks(&path, &file)
                    };
                    let edited = match (&blocks, &snapshot.working) {
                        (Ok(segments), Some(working)) => {
                            gg_engine::conflict::edited_outside(segments, working)
                        }
                        _ => false,
                    };
                    let names = gg_git::conflict::side_names(&path).ok().flatten();
                    Ok(ConflictUpdate::Content {
                        content: Box::new(content),
                        snapshot: Box::new(snapshot),
                        blocks,
                        edited,
                        names,
                    })
                })();
                let _ = tx.send(result);
                repaint.request_repaint();
            })
            .is_err()
        {
            view.error = Some("Could not start the conflict content reader.".into());
            view.loading = false;
            return;
        }
        view.loading = true;
        view.error = None;
        view.worker = Some(rx);
    }

    fn stage_conflict(&mut self, ctx: &egui::Context, stage: ConflictStage) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before resolving conflicts.")?;
        let file = self
            .conflict_view
            .as_ref()
            .and_then(|view| view.paths.get(view.selected))
            .cloned()
            .ok_or("Select a conflicted path first.")?;
        let snapshot = self
            .conflict_view
            .as_ref()
            .and_then(|view| view.snapshot.clone())
            .ok_or("Load the conflict before staging its resolution.")?;
        let resolution = match stage {
            ConflictStage::WorkingFile => None,
            ConflictStage::Preview => self.conflict_view.as_ref().map(|view| view.preview),
        }
        .unwrap_or(gg_engine::conflict::Resolution::Working);
        let composed = match self.conflict_view.as_ref() {
            _ if stage == ConflictStage::WorkingFile => None,
            Some(view) if !view.whole_file => match &view.blocks {
                Some(blocks) if blocks.complete() => Some(blocks.text()),
                Some(_) => return Err("Choose a resolution for every block first.".into()),
                None => None,
            },
            _ => None,
        };
        let label = format!("Staging {file}…");
        self.start_operation(ctx, label, move || {
            match composed {
                Some(text) => gg_git::conflict::publish_text(&path, &file, &snapshot, &text)?,
                None => gg_git::conflict::publish(&path, &file, &snapshot, resolution)?,
            }
            Ok(OperationOutcome::Conflict { file })
        })
    }

    /// Reload history, refs and the frame summary.
    fn load(&mut self, ctx: &egui::Context) {
        self.welcome.settled = None;
        self.branch_error = None;
        self.network_error = None;
        let path = PathBuf::from(&self.path);
        let switching = self
            .state
            .history
            .as_ref()
            .is_some_and(|history| history.path != path);
        if switching {
            self.clear_operation_state();
            self.mutation_path = None;
        } else {
            self.refresh_mutation();
        }
        self.state.begin(path.clone());
        if switching {
            self.local_summary = None;
        }
        self.refresh_local_summary(ctx);
        self.last_load = Some(std::time::Instant::now());
        if switching || self.watcher.is_none() {
            self.prune_backups(&path);
        }
        if self
            .watcher
            .as_ref()
            .is_none_or(|watcher| watcher.workdir != path)
        {
            let repaint = ctx.clone();
            self.watcher = watcher::Watcher::start(&path, move || {
                repaint.request_repaint_after(std::time::Duration::from_millis(350));
            })
            .ok();
        }
        let request = stream::Request {
            revision: self.state.filter_revision,
            query: self.state.query.clone(),
            selected_id: self.state.selected.clone(),
            selected_row: self.restore_row.take(),
            // A jump cleared the rows, so it starts at the top on its own
            // selection; a reload keeps its old top row.
            anchor_id: self
                .state
                .rows
                .get(&((self.scroll_offset / crate::density::row()) as usize))
                .map(|c| c.id.clone()),
            ..Default::default()
        };
        self.staged = None;
        self.pending_filter_edit = false;
        match stream::Worker::start(
            path,
            request,
            self.state.history.as_ref().map(|h| h.path.clone()),
            ctx.clone(),
        ) {
            Ok(worker) => self.pending = Some(worker),
            Err(error) => self.state.error = Some(error),
        }
    }

    fn jump_to_ref(&mut self, id: String) {
        if !self.state.query.is_empty() {
            self.state.query.clear();
            self.state.start_filter();
        }
        self.state.selected = Some(id);
        self.state.selected_row = None;
        self.state.details = None;
        self.scroll_to_selection = false;
    }

    fn open_review(
        &mut self,
        ctx: &egui::Context,
        return_location: return_location::ReturnLocation,
    ) {
        if let (Some(history), Some(id)) = (&self.state.history, &self.state.selected) {
            let subject = self
                .state
                .displayed_details()
                .map(|c| c.subject().to_owned())
                .unwrap_or_default();
            let message = self
                .state
                .displayed_details()
                .filter(|commit| commit.id == *id)
                .map(|commit| commit.message.clone());
            let mut review = review_view::Review::new(
                gg_engine::review::ReviewRequest {
                    path: history.path.clone(),
                    revision: id.clone(),
                    parent: None,
                    against: None,
                },
                subject,
                ctx,
            );
            if let Some(message) = message {
                review = review.with_message(message);
            }
            review.set_return_location(return_location);
            self.review = Some(review);
        }
    }

    fn open_review_at(
        &mut self,
        ctx: &egui::Context,
        revision: String,
        subject: String,
        return_location: Option<return_location::ReturnLocation>,
    ) {
        let Some(history) = &self.state.history else {
            return;
        };
        let mut review = review_view::Review::new(
            gg_engine::review::ReviewRequest {
                path: history.path.clone(),
                revision,
                parent: None,
                against: None,
            },
            subject,
            ctx,
        );
        if let Some(location) = return_location {
            review.set_return_location(location);
        }
        self.review = Some(review);
    }

    /// Open Recovery on the source last shown (Backups the first time),
    /// re-reading it when it is already open.
    fn open_recovery_window(&mut self, ctx: &egui::Context) {
        let Some(path) = self.stash_path() else {
            return;
        };
        match &mut self.recovery {
            Some(view) => {
                view.open = true;
                view.reread();
            }
            None => self.recovery = Some(RecoveryView::new(path, ctx)),
        }
        ctx.request_repaint();
    }

    fn open_recovery(&mut self, ctx: &egui::Context, source: return_location::RecoverySource) {
        self.open_recovery_window(ctx);
        if let Some(view) = &mut self.recovery {
            view.source = source;
        }
    }

    fn open_recovery_at(
        &mut self,
        ctx: &egui::Context,
        source: return_location::RecoverySource,
        reference: Option<&str>,
        commit: &str,
    ) {
        self.open_recovery(ctx, source);
        if let Some(view) = &mut self.recovery {
            if let Some(reference) = reference {
                view.set_reference(reference);
            }
            view.select(source, commit);
        }
    }

    fn open_worktrees(&mut self, ctx: &egui::Context) {
        if self.worktree_window.is_none() {
            self.worktree_window = Some(WorktreeWindow {
                open_created: None,
                snapshot: None,
                selected: None,
                error: None,
                notice: None,
                create: None,
                confirm_remove: None,
                lock: None,
                busy: None,
                pending: None,
            });
        }
        if self
            .worktree_window
            .as_ref()
            .is_some_and(|window| window.snapshot.is_none() && window.busy.is_none())
        {
            self.request_worktrees(ctx, worktree_worker::Action::List);
        }
    }

    fn request_worktrees(&mut self, ctx: &egui::Context, action: worktree_worker::Action) {
        let repo = self
            .stash_path()
            .unwrap_or_else(|| PathBuf::from(&self.path));
        if self.worktree_worker.is_none() {
            match worktree_worker::Worker::start(ctx.clone()) {
                Ok(worker) => self.worktree_worker = Some(worker),
                Err(error) => {
                    if let Some(window) = &mut self.worktree_window {
                        window.error = Some(error);
                        window.busy = None;
                    }
                    return;
                }
            }
        }
        self.worktree_generation += 1;
        let generation = self.worktree_generation;
        let label = match &action {
            // Reads keep the window busy without announcing it.
            worktree_worker::Action::List => "",
            worktree_worker::Action::Add(_) => "Adding worktree…",
            worktree_worker::Action::Remove(_) => "Removing worktree…",
            worktree_worker::Action::Prune => "Pruning missing worktrees…",
            worktree_worker::Action::Unlock(_) => "Unlocking worktree…",
            worktree_worker::Action::Lock(..) => "Locking worktree…",
        };
        if let Some(window) = &mut self.worktree_window {
            window.busy = Some(label.into());
            window.error = None;
            window.notice = None;
            window.pending = Some(action.clone());
        }
        if let Some(worker) = &self.worktree_worker {
            worker.request(generation, repo, action);
        }
    }

    /// Apply a worktree update; returns a created worktree to open.
    fn poll_worktrees(&mut self) -> Option<PathBuf> {
        let update = self
            .worktree_worker
            .as_ref()
            .and_then(worktree_worker::Worker::poll)?;
        if update.generation != self.worktree_generation {
            return None;
        }
        let window = self.worktree_window.as_mut()?;
        let mut open_created = None;
        window.busy = None;
        match update.result {
            Ok(snapshot) => {
                let len = snapshot.worktrees.len();
                window.snapshot = Some(snapshot);
                window.selected = window
                    .selected
                    .map(|index| index.min(len.saturating_sub(1)));
                if window.selected.is_none() && len > 0 {
                    window.selected = Some(0);
                }
                window.error = None;
                if let Some(notice) = update.notice {
                    if notice.starts_with("Added") {
                        window.create = None;
                        open_created = window.open_created.take();
                    } else if notice.starts_with("Removed") {
                        window.confirm_remove = None;
                    }
                    window.notice = Some(notice);
                }
                window.pending = None;
            }
            Err(error) => {
                if let Some(dialog) = &mut window.create {
                    dialog.error = Some(error.clone());
                }
                window.error = Some(error);
            }
        }
        open_created
    }

    fn worktree_key(&self) -> PathBuf {
        let path = self
            .state
            .history
            .as_ref()
            .map(|history| history.path.clone())
            .unwrap_or_else(|| PathBuf::from(&self.path));
        std::fs::canonicalize(&path).unwrap_or(path)
    }

    fn save_worktree_state(&mut self) {
        let key = self.worktree_key();
        self.worktree_states.insert(
            key,
            WorktreeState {
                selected: self.state.selected.clone(),
                query: self.state.query.clone(),
                scroll_offset: self.scroll_offset,
                narrow_details: self.narrow_details,
                details_visible: self.details_visible,
            },
        );
    }

    fn restore_worktree_state(&mut self, path: &PathBuf) {
        let key = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
        let state = self.worktree_states.get(&key).cloned().unwrap_or_default();
        self.state.query = state.query;
        self.state.selected = state.selected;
        self.scroll_offset = state.scroll_offset;
        self.restore_offset = Some(state.scroll_offset);
        self.narrow_details = state.narrow_details;
        self.details_visible = state.details_visible;
        self.restore_row = None;
        self.scroll_to_selection = false;
    }

    fn switch_worktree(&mut self, ctx: &egui::Context, path: PathBuf) {
        if self.repository_switch_pending() {
            return;
        }
        self.save_worktree_state();
        if let Some(working) = &self.working {
            self.working_drafts
                .insert(working.repository_key(), working.capture_draft());
        }
        self.working = None;
        self.review = None;
        self.recovery = None;
        self.conflict_view = None;
        self.worktree_window = None;
        self.edit_path = false;
        self.path = path.to_string_lossy().into_owned();
        self.restore_worktree_state(&path);
        self.load(ctx);
    }

    fn draw_worktrees(&mut self, ctx: &egui::Context) {
        if let Some(path) = self.poll_worktrees() {
            self.worktree_window = None;
            self.switch_worktree(ctx, path);
            return;
        }
        let branches = self
            .state
            .refs
            .items
            .iter()
            .filter(|item| item.kind == gg_engine::refs::RefKind::Branch)
            .map(|item| item.name.clone())
            .collect::<Vec<_>>();
        let repository_switch_pending = self.repository_switch_pending();
        // Each worktree's HEAD as `id · subject · author · date` when the
        // history has it.
        let summaries: std::collections::HashMap<String, String> = self
            .worktree_window
            .as_ref()
            .and_then(|window| window.snapshot.as_ref())
            .map(|snapshot| {
                snapshot
                    .worktrees
                    .iter()
                    .filter_map(|worktree| worktree.head.clone())
                    .map(|head| {
                        let summary = self.commit_summary(&head);
                        (head, summary)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let Some(window) = &mut self.worktree_window else {
            return;
        };
        let mut open = true;
        let mut create_open = false;
        let mut open_path = None;
        let mut remove_open = None;
        let mut submit_create = None;
        let mut cancel_create = false;
        let mut submit_remove = None;
        let mut cancel_remove = false;
        let mut prune = false;
        let mut unlock = None;
        let mut lock = None;
        let mut cancel_lock = false;
        let repo_path = self
            .state
            .history
            .as_ref()
            .map(|history| history.path.clone());
        let max_width = (ctx.content_rect().width() - 24.0).max(360.0);
        let id = egui::Id::unique("worktrees");
        let keys = inspect_ui::owns_keys(ctx, id)
            && window.create.is_none()
            && window.confirm_remove.is_none()
            && window.lock.is_none();
        let home = std::env::var("HOME").ok();
        // Missing worktrees last; Git's order otherwise (main first).
        let order: Vec<usize> = window
            .snapshot
            .as_ref()
            .map(|snapshot| {
                let mut order: Vec<usize> = (0..snapshot.worktrees.len()).collect();
                order.sort_by_key(|&index| snapshot.worktrees[index].prunable.is_some());
                order
            })
            .unwrap_or_default();
        let missing = window.snapshot.as_ref().map_or(0, |snapshot| {
            snapshot
                .worktrees
                .iter()
                .filter(|worktree| worktree.prunable.is_some())
                .count()
        });
        if window.selected.is_none_or(|index| !order.contains(&index)) {
            window.selected = order.first().copied();
        }
        let mut selected = window.selected;
        let mut lock_open = None;
        let mut reveal = None;
        let list_height = (ctx.content_rect().height() - 300.0)
            .clamp(3.0 * inspect_ui::ROW, 12.0 * inspect_ui::ROW);
        egui::Window::new("Worktrees")
            .id(id)
            .title_bar(false)
            .collapsible(false)
            .resizable(true)
            .default_width(max_width.min(820.0))
            .max_width(max_width)
            .default_pos(ctx.content_rect().center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                let count = order.len().to_string();
                if inspect_ui::header(ui, "Worktrees", None, |ui| {
                    if window.snapshot.is_some() {
                        inspect_ui::pill(ui, &count, inspect_ui::Tone::Neutral);
                    }
                }) {
                    open = false;
                }
                // BRANCH and PATH share what STATUS and HEAD leave.
                let rest = (ui.available_width() - 16.0 - 104.0 - 74.0 - 30.0).max(160.0);
                let columns = [rest * 0.38, rest * 0.62, 104.0, 0.0];
                inspect_ui::table_header(ui, &columns, &["BRANCH", "PATH", "STATUS", "HEAD"]);
                let width = ui.available_width();
                ui.allocate_ui_with_layout(
                    egui::vec2(width, list_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_size(egui::vec2(width, list_height));
                        if window
                            .snapshot
                            .as_ref()
                            .is_some_and(|snapshot| snapshot.worktrees.is_empty())
                        {
                            ui.add_space(list_height / 2.0 - 22.0);
                            ui.vertical_centered(|ui| {
                                ui.label("No worktrees registered.");
                                ui.weak("New worktree… checks a branch out in another folder.");
                            });
                        }
                        let Some(snapshot) = &window.snapshot else {
                            // Until the first list arrives the table stays empty.
                            return;
                        };
                        egui::ScrollArea::vertical()
                            .id_salt("worktree-rows")
                            .max_height(list_height)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for &index in &order {
                                    let worktree = &snapshot.worktrees[index];
                                    let response = inspect_ui::table_row(
                                        ui,
                                        selected == Some(index),
                                        &columns,
                                        |cells| {
                                            let mut branch =
                                                egui::RichText::new(worktree.branch_label());
                                            if worktree.branch.is_none() {
                                                branch = branch.italics().weak();
                                            }
                                            cells[0].add(egui::Label::new(branch).truncate());
                                            if worktree.main {
                                                cells[0].weak("· main worktree");
                                            }
                                            // Worktrees share their parent folders,
                                            // so the path keeps its end.
                                            let path = welcome_view::tilde(
                                                &worktree_path(&worktree.path_bytes),
                                                home.as_deref(),
                                            );
                                            let fitted = fit_tail(
                                                &cells[1],
                                                &path,
                                                cells[1].available_width(),
                                            );
                                            cells[1]
                                                .label(inspect_ui::mono(fitted))
                                                .on_hover_text(&worktree.path);
                                            inspect_ui::pill(
                                                &mut cells[2],
                                                worktree.status_label(),
                                                worktree_tone(worktree),
                                            );
                                            match worktree.head.as_deref() {
                                                Some(head) => cells[3].label(inspect_ui::mono(
                                                    inspect_ui::short(head),
                                                )),
                                                None => cells[3].weak("—"),
                                            };
                                        },
                                    );
                                    if response.clicked() {
                                        selected = Some(index);
                                    }
                                    if response.double_clicked() {
                                        selected = Some(index);
                                        let path = worktree_path(&worktree.path_bytes);
                                        if worktree_open_reason(
                                            worktree,
                                            &path,
                                            repository_switch_pending,
                                        )
                                        .is_none()
                                        {
                                            open_path = Some(path);
                                        }
                                    }
                                }
                            });
                    },
                );
                let current = window
                    .snapshot
                    .as_ref()
                    .zip(selected)
                    .and_then(|(snapshot, index)| snapshot.worktrees.get(index));
                // The selected worktree: its full path, HEAD and state.
                ui.separator();
                inspect_ui::grid(ui, "worktree-details", |ui| {
                    let Some(worktree) = current else {
                        inspect_ui::row(ui, "PATH", |ui| {
                            ui.weak("—");
                        });
                        return;
                    };
                    inspect_ui::row(ui, "PATH", |ui| {
                        ui.add(egui::Label::new(inspect_ui::mono(&worktree.path)).truncate());
                        inspect_ui::copy_button(ui, &worktree.path, "path");
                    });
                    inspect_ui::row(ui, "HEAD", |ui| {
                        match worktree.head.as_deref() {
                            Some(head) => {
                                let summary = summaries
                                    .get(head)
                                    .cloned()
                                    .unwrap_or_else(|| inspect_ui::short(head).to_string());
                                ui.add(egui::Label::new(inspect_ui::mono(summary)).truncate());
                            }
                            None => {
                                ui.weak("—");
                            }
                        };
                    });
                    inspect_ui::row(ui, "STATE", |ui| {
                        ui.add(egui::Label::new(worktree_state_line(worktree)).truncate());
                    });
                });
                let path = current.map(|worktree| worktree_path(&worktree.path_bytes));
                let open_reason = current.zip(path.as_ref()).and_then(|(worktree, path)| {
                    worktree_open_reason(worktree, path, repository_switch_pending)
                });
                let remove_reason = current.and_then(|worktree| worktree.can_remove().err());
                let idle = window.busy.is_none();
                // The footer's left: progress, the last result, or why the
                // primary is disabled.
                let busy_text = window.busy.clone().filter(|busy| !busy.is_empty());
                let left = if let Some(busy) = &busy_text {
                    Some((busy.as_str(), ui.visuals().weak_text_color()))
                } else if let Some(error) = &window.error {
                    Some((error.as_str(), inspect_ui::Tone::Red.text()))
                } else if let Some(notice) = &window.notice {
                    Some((notice.as_str(), inspect_ui::Tone::Green.text()))
                } else {
                    open_reason
                        .as_deref()
                        .filter(|_| current.is_some_and(|worktree| worktree.prunable.is_none()))
                        .map(|reason| (reason, ui.visuals().weak_text_color()))
                };
                let mut more = None;
                ui.separator();
                ui.horizontal(|ui| {
                    ui.set_min_height(26.0);
                    ui.add_enabled_ui(current.is_some() && idle, |ui| {
                        ui.menu_button(phosphor::icon(phosphor::DOTS_THREE), |ui| {
                            use menus::item;
                            let mut list = vec![
                                item("Copy path", "", WorktreeMore::CopyPath).entry(),
                                item("Show in file manager", "", WorktreeMore::Reveal).entry(),
                                menus::Entry::Separator,
                            ];
                            if let Some(worktree) = current {
                                if worktree.locked.is_some() {
                                    list.push(item("Unlock", "", WorktreeMore::Unlock).entry());
                                } else if !worktree.main && worktree.prunable.is_none() {
                                    list.push(item("Lock…", "", WorktreeMore::Lock).entry());
                                }
                            }
                            if missing > 0 {
                                list.push(
                                    item(
                                        format!("Prune {missing} missing"),
                                        "",
                                        WorktreeMore::Prune,
                                    )
                                    .entry(),
                                );
                            }
                            menus::entries(ui, &list, &mut more);
                        })
                        .response
                        .on_hover_text("More actions");
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if current.is_some_and(|worktree| worktree.prunable.is_some()) {
                            // A missing folder cannot open; forgetting it is
                            // what it needs.
                            if ui
                                .add_enabled(
                                    idle,
                                    inspect_ui::primary(format!("Prune {missing} missing"), None),
                                )
                                .on_hover_text(
                                    "Forget worktrees whose folders are gone; their branches stay",
                                )
                                .clicked()
                            {
                                prune = true;
                            }
                        } else {
                            let enabled = current.is_some() && open_reason.is_none();
                            let button = if enabled {
                                inspect_ui::primary("Open", Some("Enter"))
                            } else {
                                egui::Button::new("Open")
                            };
                            if ui
                                .add_enabled(enabled, button)
                                .on_disabled_hover_text(open_reason.clone().unwrap_or_default())
                                .clicked()
                            {
                                open_path = path.clone();
                            }
                        }
                        if ui
                            .add_enabled(
                                idle && current.is_some() && remove_reason.is_none(),
                                inspect_ui::secondary("Remove…", "Del"),
                            )
                            .on_disabled_hover_text(remove_reason.clone().unwrap_or_default())
                            .clicked()
                        {
                            remove_open = selected;
                        }
                        if ui
                            .add_enabled(idle, egui::Button::new("New worktree…"))
                            .clicked()
                        {
                            create_open = true;
                        }
                        if let Some((text, color)) = left {
                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    if busy_text.is_some() {
                                        ui.spinner();
                                    }
                                    ui.add(
                                        egui::Label::new(egui::RichText::new(text).color(color))
                                            .truncate(),
                                    )
                                    .on_hover_text(text);
                                },
                            );
                        }
                    });
                });
                match more {
                    Some(WorktreeMore::CopyPath) => {
                        if let Some(worktree) = current {
                            ui.ctx().copy_text(worktree.path.clone());
                        }
                    }
                    Some(WorktreeMore::Reveal) => reveal = path.clone(),
                    Some(WorktreeMore::Lock) => lock_open = path.clone(),
                    Some(WorktreeMore::Unlock) => unlock = path.clone(),
                    Some(WorktreeMore::Prune) => prune = true,
                    None => {}
                }
            });
        // Keys: Up/Down select, Enter opens, Del removes, Esc closes.
        if keys && let Some(snapshot) = &window.snapshot {
            let step = crate::navigation::step(ctx, 8);
            if step != 0 && !order.is_empty() {
                let at = selected
                    .and_then(|index| order.iter().position(|&i| i == index))
                    .unwrap_or(0) as isize;
                let at = (at + step).clamp(0, order.len() as isize - 1) as usize;
                selected = Some(order[at]);
            }
            let (enter, delete, escape) = ctx.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Delete),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                )
            });
            let current = selected.and_then(|index| snapshot.worktrees.get(index));
            if let Some(worktree) = current {
                let path = worktree_path(&worktree.path_bytes);
                if enter
                    && worktree_open_reason(worktree, &path, repository_switch_pending).is_none()
                {
                    open_path = Some(path);
                } else if delete && window.busy.is_none() && worktree.can_remove().is_ok() {
                    remove_open = selected;
                }
            }
            if escape {
                open = false;
            }
        }
        window.selected = selected;
        if let Some(path) = lock_open {
            window.lock = Some((path, String::new()));
        }
        if let Some((path, reason)) = &mut window.lock {
            let shown = path.display().to_string();
            match inspect_ui::confirm(
                ctx,
                "lock-worktree",
                "Lock worktree",
                "Lock",
                None,
                false,
                |ui| {
                    ui.label(format!("Lock {shown}?"));
                    ui.weak("Git then never prunes or removes it, for example while it is on a removable disk.");
                    ui.add_space(4.0);
                    ui.label(inspect_ui::caption("REASON (OPTIONAL)"));
                    ui.add(
                        egui::TextEdit::singleline(reason)
                            .hint_text("on a USB disk")
                            .desired_width(f32::INFINITY),
                    );
                },
            ) {
                Some(inspect_ui::Decision::Confirm) => {
                    lock = Some((path.clone(), reason.clone()));
                }
                Some(inspect_ui::Decision::Cancel) => cancel_lock = true,
                None => {}
            }
        }
        if let Some(dialog) = &mut window.create {
            // Branch first, then the folder, which follows the branch until
            // edited (proposal); the shared header, label grid and footer.
            let busy = window.busy.is_some();
            let ready = !dialog.destination.trim().is_empty() && !dialog.branch.trim().is_empty();
            let reason = if busy {
                Some("Adding the worktree…")
            } else if dialog.branch.trim().is_empty() {
                Some(if dialog.create {
                    "Name the new branch."
                } else {
                    "Choose a branch."
                })
            } else if dialog.destination.trim().is_empty() {
                Some("Choose a folder.")
            } else {
                None
            };
            let mut decision = None;
            let modal = egui::Modal::new(egui::Id::unique("create-worktree"))
                .backdrop_color(egui::Color32::from_black_alpha(140))
                .show(ctx, |ui| {
                    ui.set_width(480.0);
                    if inspect_ui::header(ui, "New worktree", None, |_| {}) {
                        decision = Some(inspect_ui::Decision::Cancel);
                    }
                    ui.add_space(4.0);
                    let before = dialog.branch.clone();
                    inspect_ui::grid(ui, "create-worktree-grid", |ui| {
                        inspect_ui::row(ui, "BRANCH", |ui| {
                            if let Some(create) = inspect_ui::segmented(
                                ui,
                                dialog.create,
                                &[(true, "New".into()), (false, "Existing".into())],
                            ) {
                                dialog.create = create;
                            }
                        });
                        inspect_ui::row(ui, "", |ui| {
                            if dialog.create {
                                ui.add(
                                    egui::TextEdit::singleline(&mut dialog.branch)
                                        .hint_text("feature/name")
                                        .desired_width(300.0),
                                );
                            } else if branches.is_empty() {
                                ui.weak("No local branches are available.");
                            } else {
                                egui::ComboBox::from_id_salt("worktree-branch")
                                    .selected_text(if dialog.branch.is_empty() {
                                        "Choose a branch"
                                    } else {
                                        &dialog.branch
                                    })
                                    .width(300.0)
                                    .show_ui(ui, |ui| {
                                        for branch in &branches {
                                            crate::menus::choose(
                                                ui,
                                                &mut dialog.branch,
                                                branch.clone(),
                                                branch,
                                            );
                                        }
                                    });
                            }
                        });
                        if dialog.create {
                            inspect_ui::row(ui, "START AT", |ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut dialog.at)
                                        .hint_text("HEAD")
                                        .desired_width(300.0),
                                );
                            });
                        }
                        inspect_ui::row(ui, "FOLDER", |ui| {
                            let field = ui.add(
                                egui::TextEdit::singleline(&mut dialog.destination)
                                    .desired_width(222.0),
                            );
                            if field.changed() {
                                dialog.destination_auto = false;
                            }
                            if ui
                                .button("Browse…")
                                .on_hover_text("Choose a folder with the desktop dialog")
                                .clicked()
                                && let Some(path) = folder_picker::pick_folder(
                                    "Worktree destination",
                                    Some(Path::new(dialog.destination.trim())),
                                )
                            {
                                dialog.destination = path.to_string_lossy().into_owned();
                                dialog.destination_auto = false;
                            }
                        });
                        inspect_ui::row(ui, "", |ui| {
                            ui.checkbox(&mut dialog.open_after, "Open after create");
                        });
                    });
                    if dialog.destination_auto
                        && dialog.branch != before
                        && let Some(repo) = &repo_path
                    {
                        dialog.destination = worktree_default_destination(repo, &dialog.branch);
                    }
                    let left = match &dialog.error {
                        Some(error) => Some((error.as_str(), inspect_ui::Tone::Red.text())),
                        None => reason.map(|reason| (reason, inspect_ui::Tone::Amber.text())),
                    };
                    inspect_ui::footer(ui, left, |ui| {
                        let enabled = ready && !busy;
                        let button = if enabled {
                            inspect_ui::primary("Create worktree", Some("Enter"))
                        } else {
                            egui::Button::new("Create worktree")
                        };
                        if ui.add_enabled(enabled, button).clicked() {
                            decision = Some(inspect_ui::Decision::Confirm);
                        }
                        if ui.add(inspect_ui::secondary("Cancel", "Esc")).clicked() {
                            decision = Some(inspect_ui::Decision::Cancel);
                        }
                    });
                    if ready && !busy && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        decision = Some(inspect_ui::Decision::Confirm);
                    }
                });
            if modal.should_close() && decision.is_none() {
                decision = Some(inspect_ui::Decision::Cancel);
            }
            match decision {
                Some(inspect_ui::Decision::Confirm) => {
                    submit_create = Some((
                        PathBuf::from(dialog.destination.trim()),
                        dialog.create,
                        dialog.branch.trim().to_string(),
                        dialog.at.trim().to_string(),
                    ));
                }
                Some(inspect_ui::Decision::Cancel) => cancel_create = true,
                None => {}
            }
        }
        if let Some(index) = window.confirm_remove
            && let Some(snapshot) = &window.snapshot
            && let Some(worktree) = snapshot.worktrees.get(index)
        {
            let reason = window.busy.is_some().then_some("Removing the worktree…");
            let error = window.error.as_deref();
            match inspect_ui::confirm(
                ctx,
                "remove-worktree",
                "Remove worktree",
                "Remove worktree",
                reason,
                true,
                |ui| {
                    inspect_ui::grid(ui, "remove-worktree-grid", |ui| {
                        inspect_ui::row(ui, "BRANCH", |ui| {
                            ui.label(worktree.branch_label());
                        });
                        inspect_ui::row(ui, "FOLDER", |ui| {
                            let fitted = fit_tail(ui, &worktree.path, ui.available_width());
                            ui.label(inspect_ui::mono(fitted))
                                .on_hover_text(&worktree.path);
                        });
                    });
                    ui.add_space(4.0);
                    ui.label("The folder is deleted. The branch stays, with all its commits.");
                    if let Some(error) = error {
                        ui.colored_label(inspect_ui::Tone::Red.text(), error);
                    }
                },
            ) {
                Some(inspect_ui::Decision::Confirm) => {
                    submit_remove = Some(worktree_path(&worktree.path_bytes));
                }
                Some(inspect_ui::Decision::Cancel) => cancel_remove = true,
                None => {}
            }
        }
        if open_path.is_some() {
            self.worktree_window = None;
        }
        if create_open && let Some(window) = &mut self.worktree_window {
            window.create = Some(CreateWorktreeDialog {
                destination: String::new(),
                create: true,
                branch: String::new(),
                at: "HEAD".into(),
                error: None,
                destination_auto: true,
                open_after: true,
            });
        }
        if let Some(index) = remove_open
            && let Some(window) = &mut self.worktree_window
        {
            window.confirm_remove = Some(index);
        }
        if let Some((destination, create, branch, at)) = submit_create
            && let Some(window) = &mut self.worktree_window
        {
            window.open_created = window
                .create
                .as_ref()
                .filter(|dialog| dialog.open_after)
                .map(|_| destination.clone());
            let request = gg_engine::worktree::AddRequest {
                destination,
                branch: (!create).then_some(branch.clone()),
                create: create.then_some(branch),
                at: (create && !at.is_empty()).then_some(at),
            };
            if let Some(dialog) = &mut window.create {
                dialog.error = None;
            }
            self.request_worktrees(ctx, worktree_worker::Action::Add(request));
        }
        if let Some(path) = submit_remove {
            self.request_worktrees(ctx, worktree_worker::Action::Remove(path));
        }
        if prune {
            self.request_worktrees(ctx, worktree_worker::Action::Prune);
        }
        if let Some(path) = unlock {
            self.request_worktrees(ctx, worktree_worker::Action::Unlock(path));
        }
        if let Some((path, reason)) = lock {
            if let Some(window) = &mut self.worktree_window {
                window.lock = None;
            }
            self.request_worktrees(ctx, worktree_worker::Action::Lock(path, reason));
        }
        if cancel_lock && let Some(window) = &mut self.worktree_window {
            window.lock = None;
        }
        if let Some(path) = reveal
            && let Err(error) = editor::open_folder(&path)
            && let Some(window) = &mut self.worktree_window
        {
            window.error = Some(error);
        }
        if cancel_create && let Some(window) = &mut self.worktree_window {
            window.create = None;
        }
        if cancel_remove && let Some(window) = &mut self.worktree_window {
            window.confirm_remove = None;
        }
        if !open {
            self.worktree_window = None;
        }
        if let Some(path) = open_path {
            self.switch_worktree(ctx, path);
        }
    }

    fn open_submodule(&mut self, ctx: &egui::Context, request: crate::diff_view::SubmoduleRequest) {
        let action = submodule_worker::Action::Read {
            before: request.before.clone(),
            after: request.after.clone(),
        };
        self.submodule_window = Some(SubmoduleWindow::new(request));
        self.request_submodule(ctx, action);
    }

    fn request_submodule(&mut self, ctx: &egui::Context, action: submodule_worker::Action) {
        let Some(repo) = self.stash_path() else {
            return;
        };
        let Some(window) = &self.submodule_window else {
            return;
        };
        let path_bytes = window.request.path_bytes.clone();
        if self.submodule_worker.is_none() {
            match submodule_worker::Worker::start(ctx.clone()) {
                Ok(worker) => self.submodule_worker = Some(worker),
                Err(error) => {
                    if let Some(window) = &mut self.submodule_window {
                        window.error = Some(error);
                        window.busy = None;
                    }
                    return;
                }
            }
        }
        self.submodule_generation += 1;
        let generation = self.submodule_generation;
        let label = match action {
            // Reads keep the window busy without announcing it.
            submodule_worker::Action::Read { .. } => "",
            submodule_worker::Action::Update => "Updating submodule…",
        };
        if let Some(window) = &mut self.submodule_window {
            window.busy = Some(label.into());
            window.error = None;
            window.notice = None;
        }
        if let Some(worker) = &self.submodule_worker {
            worker.request(generation, repo, path_bytes, action);
        }
    }

    fn poll_submodule(&mut self) {
        let Some(update) = self
            .submodule_worker
            .as_ref()
            .and_then(submodule_worker::Worker::poll)
        else {
            return;
        };
        if update.generation != self.submodule_generation {
            return;
        }
        let Some(window) = &mut self.submodule_window else {
            return;
        };
        window.busy = None;
        match update.result {
            Ok((details, receipt)) => {
                window.details = Some(details);
                window.error = None;
                if let Some(receipt) = receipt {
                    window.notice = Some(format!(
                        "Updated {} to {}.",
                        receipt.path,
                        receipt
                            .target
                            .as_deref()
                            .map(|target| target.chars().take(8).collect::<String>())
                            .unwrap_or_else(|| "the index commit".into())
                    ));
                    window.refresh_parent = true;
                }
            }
            Err(error) => window.error = Some(error),
        }
    }

    fn draw_submodule(&mut self, ctx: &egui::Context) {
        self.poll_submodule();
        self.draw_submodule_list(ctx);
        let Some(request) = self
            .submodule_window
            .as_ref()
            .map(|window| window.request.clone())
        else {
            return;
        };
        let nested = self
            .stash_path()
            .map(|repo| repo.join(worktree_path(&request.path_bytes)));
        let Some(window) = &mut self.submodule_window else {
            return;
        };
        let can_open = window
            .details
            .as_ref()
            .is_some_and(|details| details.initialized)
            && nested.as_ref().is_some_and(|path| path.exists());
        match submodules_view::draw_details(ctx, window, can_open) {
            Some(submodules_view::DetailsAction::Close) => self.submodule_window = None,
            Some(submodules_view::DetailsAction::AskUpdate) => window.confirm_update = true,
            Some(submodules_view::DetailsAction::CancelUpdate) => window.confirm_update = false,
            Some(submodules_view::DetailsAction::Update) => {
                window.confirm_update = false;
                self.request_submodule(ctx, submodule_worker::Action::Update);
            }
            Some(submodules_view::DetailsAction::OpenNested) => {
                if let Some(path) = nested {
                    self.submodule_window = None;
                    self.submodule_list = None;
                    self.switch_worktree(ctx, path);
                    return;
                }
            }
            None => {}
        }
        let refresh = self
            .submodule_window
            .as_mut()
            .is_some_and(|window| std::mem::take(&mut window.refresh_parent));
        if refresh {
            self.refresh_working(ctx);
            if let Some(list) = &mut self.submodule_list {
                list.reread();
            }
        }
    }

    /// Repository › Submodules…: opens the list, re-reading it when open.
    fn open_submodule_list(&mut self, ctx: &egui::Context) {
        let Some(path) = self.stash_path() else {
            return;
        };
        match &mut self.submodule_list {
            Some(list) => list.reread(),
            None => self.submodule_list = Some(submodules_view::ListWindow::new(path, ctx)),
        }
    }

    fn draw_submodule_list(&mut self, ctx: &egui::Context) {
        let Some(list) = &mut self.submodule_list else {
            return;
        };
        match list.draw(ctx) {
            Some(submodules_view::ListAction::Close) => self.submodule_list = None,
            Some(submodules_view::ListAction::Details(request)) => {
                self.open_submodule(ctx, request);
            }
            Some(submodules_view::ListAction::OpenNested(path_bytes)) => {
                if let Some(repo) = self.stash_path() {
                    self.submodule_list = None;
                    self.switch_worktree(ctx, repo.join(worktree_path(&path_bytes)));
                }
            }
            None => {}
        }
    }

    fn open_signature(&mut self, ctx: &egui::Context, reference: String) {
        self.signature_window = Some(SignatureWindow::new(reference.clone()));
        self.request_signature(ctx, signature_worker::Action::Load { reference });
    }

    fn request_signature(&mut self, ctx: &egui::Context, action: signature_worker::Action) {
        let Some(repo) = self.stash_path() else {
            return;
        };
        if self.signature_worker.is_none() {
            match signature_worker::Worker::start(ctx.clone()) {
                Ok(worker) => self.signature_worker = Some(worker),
                Err(error) => {
                    if let Some(window) = &mut self.signature_window {
                        window.error = Some(error);
                        window.busy = None;
                    }
                    return;
                }
            }
        }
        self.signature_generation += 1;
        let generation = self.signature_generation;
        if let Some(window) = &mut self.signature_window {
            window.busy = Some("Verifying signature…".into());
            window.error = None;
        }
        if let Some(worker) = &self.signature_worker {
            worker.request(generation, repo, action);
        }
    }

    fn poll_signature(&mut self) {
        let Some(update) = self
            .signature_worker
            .as_ref()
            .and_then(signature_worker::Worker::poll)
        else {
            return;
        };
        if update.generation != self.signature_generation {
            return;
        }
        let Some(window) = &mut self.signature_window else {
            return;
        };
        window.busy = None;
        match update.result {
            Ok((_, verification)) => {
                window.verification = Some(verification);
                window.error = None;
            }
            Err(error) => window.error = Some(error),
        }
    }

    fn draw_signature(&mut self, ctx: &egui::Context) {
        self.poll_signature();
        let Some(window) = &self.signature_window else {
            return;
        };
        match signature_view::draw(ctx, window) {
            Some(signature_view::Action::Close) => self.signature_window = None,
            Some(signature_view::Action::Settings) => self.open_preferences(PrefSection::Commit),
            None => {}
        }
    }

    fn open_lfs(&mut self, ctx: &egui::Context, request: crate::diff_view::LfsRequest) {
        self.lfs_window = Some(LfsWindow::new(request.clone()));
        self.request_lfs(ctx, lfs_worker::Action::Read(request));
    }

    fn request_lfs(&mut self, ctx: &egui::Context, action: lfs_worker::Action) {
        let Some(repo) = self.stash_path() else {
            return;
        };
        if self.lfs_worker.is_none() {
            match lfs_worker::Worker::start(ctx.clone()) {
                Ok(worker) => self.lfs_worker = Some(worker),
                Err(error) => {
                    if let Some(window) = &mut self.lfs_window {
                        window.error = Some(error);
                        window.busy = None;
                    }
                    return;
                }
            }
        }
        self.lfs_generation += 1;
        let generation = self.lfs_generation;
        let label = match action {
            // Reads keep the window busy without announcing it.
            lfs_worker::Action::Read(_) => "",
            lfs_worker::Action::Fetch(_) => "Fetching LFS object…",
        };
        if let Some(window) = &mut self.lfs_window {
            window.busy = Some(label.into());
            window.error = None;
            window.notice = None;
        }
        if let Some(worker) = &self.lfs_worker {
            worker.request(generation, repo, action);
        }
    }

    fn poll_lfs(&mut self) {
        let Some(update) = self.lfs_worker.as_ref().and_then(lfs_worker::Worker::poll) else {
            return;
        };
        if update.generation != self.lfs_generation {
            return;
        }
        let Some(window) = &mut self.lfs_window else {
            return;
        };
        window.busy = None;
        match update.result {
            Ok(loaded) => {
                window.details = Some(loaded.details);
                window.content = loaded.content;
                window.preview_note = loaded.preview_note;
                window.texture = None;
                window.error = None;
                if let Some(receipt) = loaded.receipt {
                    window.notice = Some(if receipt.available {
                        format!(
                            "Fetched {} ({}).",
                            inspect_ui::short(&receipt.oid),
                            inspect_ui::bytes(receipt.size)
                        )
                    } else {
                        format!(
                            "Fetch completed but {} is still unavailable.",
                            inspect_ui::short(&receipt.oid)
                        )
                    });
                    if let Some(list) = &mut self.lfs_list {
                        list.reread();
                    }
                }
            }
            Err(error) => window.error = Some(error),
        }
    }

    fn draw_lfs(&mut self, ctx: &egui::Context) {
        self.poll_lfs();
        self.draw_lfs_list(ctx);
        let repo = self.stash_path();
        let Some(window) = &mut self.lfs_window else {
            return;
        };
        let request = window.request.clone();
        match lfs_view::draw_details(ctx, window, repo.as_deref()) {
            Some(lfs_view::DetailsAction::Close) => self.lfs_window = None,
            Some(lfs_view::DetailsAction::AskFetch) => window.confirm_fetch = true,
            Some(lfs_view::DetailsAction::CancelFetch) => window.confirm_fetch = false,
            Some(lfs_view::DetailsAction::Fetch) => {
                window.confirm_fetch = false;
                self.request_lfs(ctx, lfs_worker::Action::Fetch(request));
            }
            None => {}
        }
    }

    /// Repository › LFS…: opens the list, re-reading it when open.
    fn open_lfs_list(&mut self, ctx: &egui::Context) {
        let Some(path) = self.stash_path() else {
            return;
        };
        match &mut self.lfs_list {
            Some(list) => list.reread(),
            None => self.lfs_list = Some(lfs_view::ListWindow::new(path, ctx)),
        }
    }

    fn draw_lfs_list(&mut self, ctx: &egui::Context) {
        let Some(list) = &mut self.lfs_list else {
            return;
        };
        match list.draw(ctx) {
            Some(lfs_view::ListAction::Close) => self.lfs_list = None,
            Some(lfs_view::ListAction::Details(pointer)) => {
                self.open_lfs(ctx, lfs_view::request(&pointer));
            }
            None => {}
        }
    }

    fn open_recovery_review(
        &mut self,
        ctx: &egui::Context,
        source: return_location::RecoverySource,
        reference: Option<String>,
        commit: String,
        subject: String,
        selected: String,
    ) {
        self.recovery = None;
        let Some(history) = &self.state.history else {
            return;
        };
        let mut review = review_view::Review::new(
            gg_engine::review::ReviewRequest {
                path: history.path.clone(),
                revision: commit,
                parent: None,
                against: None,
            },
            subject,
            ctx,
        );
        review.set_return_location(return_location::ReturnLocation::Recovery {
            source,
            reference,
            entry: selected,
        });
        self.review = Some(review);
    }

    fn open_compare(
        &mut self,
        ctx: &egui::Context,
        id: String,
        return_location: return_location::ReturnLocation,
    ) {
        if let Some(path) = self
            .state
            .history
            .as_ref()
            .map(|history| history.path.clone())
        {
            self.state.selected = Some(id.clone());
            self.state.reconcile_selection();
            let subject = self
                .state
                .displayed_details()
                .map(|c| c.subject().to_owned())
                .unwrap_or_default();
            let mut review = review_view::Review::new(
                gg_engine::review::ReviewRequest {
                    path,
                    revision: id,
                    parent: None,
                    against: Some("HEAD".into()),
                },
                subject,
                ctx,
            );
            review.set_return_location(return_location);
            self.review = Some(review);
        }
    }

    /// Input the window frame adds before egui sees it: the release an OS
    /// move or resize kept (`titlebar`), and the native menu's key
    /// equivalents as the key presses they were (`native_menu`).
    fn window_input(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        if self.chrome.custom {
            titlebar::release_after_os_resize(ctx, input);
        }
        if let Some(menu) = self.native_menu.as_mut() {
            menu.raw_input(input);
        }
    }

    /// The menu bar's height: none while the macOS menu bar holds the menus.
    fn menu_bar_height(&self) -> f32 {
        if self.native_menu.is_some() {
            0.0
        } else {
            MENU_BAR_HEIGHT
        }
    }

    fn prepare_input(&mut self, input: &mut egui::RawInput) {
        // egui uses the frame's final pointer position for scrollbar release.
        // Keep later motion in the following frame, preserving event order.
        let mut events = std::mem::take(&mut self.deferred_input);
        events.append(&mut input.events);
        if let Some(index) = events.iter().enumerate().find_map(|(i, event)| {
            (matches!(event, egui::Event::PointerButton { pressed: false, .. })
                && events[i + 1..].iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::PointerMoved(_) | egui::Event::PointerGone
                    )
                }))
            .then_some(i)
        }) {
            self.deferred_input = events.split_off(index + 1);
        }
        input.events = events;
    }

    fn draw_ui(&mut self, ui: &mut egui::Ui) {
        // A shortcut pressed with the keyboard on the menu bar leaves the bar
        // and runs, so this comes before any shortcut is read.
        crate::menu_keys::leave_for_shortcut(ui.ctx());
        // The open repository's absolute path, else the requested one.
        let shown_path = self
            .state
            .history
            .as_ref()
            .map(|history| {
                let path = std::path::absolute(&history.path).unwrap_or(history.path.clone());
                path.canonicalize().unwrap_or(path).display().to_string()
            })
            .unwrap_or_else(|| self.path.clone());
        let title = window_title(
            &shown_path,
            std::env::var("HOME").ok().as_deref(),
            self.rebase_progress(),
        );
        if title != self.window_title {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
        self.sync_mutation();
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        // Esc closes search before any view can take the key.
        if self.search.is_some()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.close_search(ctx);
        } else if self.conflict_view.is_some()
            && crate::navigation::available(ctx)
            && !ctx.text_edit_focused()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            // Esc leaves the conflict workspace for the plain changes list.
            self.conflict_view = None;
            self.open_working(ctx);
        }
        self.ctx_width = ctx.content_rect().width();
        if let Some((changes, first)) = self.watcher.as_mut().and_then(|watcher| watcher.poll()) {
            self.apply_changes(ctx, changes, first);
        }
        if self.welcome_shown() {
            self.settle_welcome();
            let available = crate::navigation::activation_available(ctx)
                && self.new_repository.is_none()
                && !self.edit_path
                && self.search.is_none();
            if let Some(command) = self.welcome.keys(ctx, &self.recent, available) {
                self.run_welcome_command(ctx, command);
            }
        }
        self.menu_shortcuts(ctx);
        self.poll_local_summary();
        let focused = ctx.input(|i| i.focused);
        if focused && !self.window_focused {
            self.refresh_local_summary(ctx);
            // The Worktrees window has no Refresh button: it re-reads when
            // gg gets focus back (accepted proposal).
            if self
                .worktree_window
                .as_ref()
                .is_some_and(|window| window.busy.is_none())
            {
                self.request_worktrees(ctx, worktree_worker::Action::List);
            }
            // Recovery, Submodules and LFS re-read the same way.
            if let Some(view) = &mut self.recovery {
                view.reread();
            }
            if let Some(list) = &mut self.submodule_list {
                list.reread();
            }
            if let Some(list) = &mut self.lfs_list {
                list.reread();
            }
        }
        self.window_focused = focused;
        colors::apply(ctx);
        #[cfg(debug_assertions)]
        if self.open_local
            && self.working.is_none()
            && self.review.is_none()
            && self.state.history.is_some()
        {
            self.open_local = false;
            self.open_working(ctx);
        }
        // A new repository without a commit opens in Local Changes.
        if self.open_changes_after_load
            && self.pending.is_none()
            && self.working.is_none()
            && self.state.history.is_some()
        {
            self.open_changes_after_load = false;
            self.open_working(ctx);
        }
        self.drop_folders(ctx);
        self.open_reopened_commit(ctx);
        self.remember_layout(ctx);
        #[cfg(debug_assertions)]
        if self.state.history.is_some()
            && let Some(revision) = self.open_review.take()
        {
            self.open_review_at(ctx, revision, String::new(), None);
        }
        #[cfg(debug_assertions)]
        if self.state.history.is_some() && std::mem::take(&mut self.open_worktrees) {
            self.open_worktrees(ctx);
        }
        #[cfg(debug_assertions)]
        if self.state.history.is_some()
            && let Some(from) = self.open_rebase.take()
            && let Err(error) = self.open_rewrite_dialog(&from)
        {
            self.mutation_error = Some(error);
        }
        if let Some(result) = self.branch.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Some(Err("Branch worker stopped. Refresh refs.".into()))
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
        }) {
            self.branch = None;
            self.branch_label = None;
            let target = self.branch_target.take();
            match result {
                Ok(receipt) => {
                    self.pending_switch = None;
                    let created = self.branch_after_create.take().is_some() || receipt.created;
                    if self.outcome.as_ref().is_some_and(|outcome| {
                        matches!(
                            outcome.recovery,
                            Some(RecoveryAction::RetrySwitch | RecoveryAction::StashAndSwitch)
                        )
                    }) {
                        self.outcome = None;
                    }
                    let warnings =
                        (!receipt.warnings.is_empty()).then(|| receipt.warnings.join(" · "));
                    self.branch_warning = warnings.clone();
                    let pull = (!receipt.created)
                        .then(|| self.pull_rebase_target(&format!("refs/heads/{}", receipt.branch)))
                        .flatten();
                    let summary = if receipt.created {
                        format!("Created {}", receipt.branch)
                    } else {
                        format!("Switched to {}", receipt.branch)
                    };
                    if warnings.is_some() || pull.is_some() {
                        self.set_outcome(
                            OutcomeSeverity::Info,
                            summary,
                            warnings,
                            pull.is_some().then_some(RecoveryAction::PullRebase),
                        );
                    }
                    // History holds HEAD and every ref, so switching from
                    // one branch to another existing one changes no row:
                    // move HEAD and select its tip in the list already
                    // shown. A new branch, or leaving a detached HEAD (whose
                    // commits may drop out), reads the history again.
                    let from_branch = self
                        .state
                        .refs
                        .head
                        .as_ref()
                        .is_some_and(|head| !head.detached && head.commit.is_some());
                    if from_branch && !created {
                        self.state.refs.head = Some(gg_engine::refs::Head {
                            name: Some(format!("refs/heads/{}", receipt.branch)),
                            detached: false,
                            commit: Some(receipt.head.clone()),
                        });
                        self.jump_to_ref(receipt.head);
                        self.scroll_to_selection = true;
                        // The switch's own HEAD change must not make the
                        // watcher read the unchanged history again.
                        self.last_load = Some(std::time::Instant::now());
                        self.refresh_mutation();
                        self.refresh_local_summary(ctx);
                        self.refresh_working(ctx);
                    } else {
                        self.state.query.clear();
                        self.state.start_filter();
                        self.state.selected = Some(receipt.head);
                        self.state.selected_row = None;
                        self.state.details = None;
                        self.scroll_offset = 0.0;
                        self.scroll_to_selection = true;
                        self.load(ctx);
                    }
                }
                Err(error) => {
                    self.branch_warning = None;
                    if let Some((name, false)) = target {
                        self.pending_switch = Some(name.clone());
                        let created = self.branch_after_create.take();
                        let dirty = gg_git::dirty_tree_error(&error);
                        let reason = match self.changed_files() {
                            Some(count) if dirty => format!(
                                "{} would be overwritten",
                                operation_band::plural(count, "changed file")
                            ),
                            _ if dirty => "changed files would be overwritten".into(),
                            _ => git_reason(&error)
                                .unwrap_or_else(|| error.lines().next().unwrap_or_default())
                                .to_string(),
                        };
                        // Git's full output stays readable behind Details….
                        let details = error.contains('\n').then(|| error.clone());
                        let summary = match &created {
                            Some(branch) => {
                                format!("Created {branch} but can’t switch to it: {reason}")
                            }
                            None => format!("Can’t switch to {name}: {reason}"),
                        };
                        let recovery = if dirty {
                            RecoveryAction::StashAndSwitch
                        } else {
                            RecoveryAction::RetrySwitch
                        };
                        if created.is_some() {
                            // The new branch still has to appear.
                            self.load(ctx);
                        }
                        self.set_outcome(
                            OutcomeSeverity::Warning,
                            summary,
                            details,
                            Some(recovery),
                        );
                    } else {
                        self.branch_error = Some(error);
                    }
                }
            }
        }
        let mut network_done = None;
        let network_operation = self.network.as_ref().map(|worker| worker.operation);
        if let Some(worker) = self.network.as_ref() {
            loop {
                match worker.try_recv() {
                    Ok(network::Message::Progress(update)) => {
                        self.network_progress = Some(update);
                    }
                    Ok(network::Message::Done(result)) => {
                        network_done = Some(result);
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        network_done =
                            Some(Err("Network worker stopped. Git Good re-read the repository; check what changed before retrying.".into()));
                        break;
                    }
                }
            }
        }
        if let Some(result) = network_done {
            self.finish_network(ctx, result, network_operation);
        }
        let rewrite_done = match self.rewrite_job.as_ref().map(|job| job.receiver.try_recv()) {
            None | Some(Err(std::sync::mpsc::TryRecvError::Empty)) => None,
            Some(Ok(result)) => Some(result),
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                Some(Err("Rewrite worker stopped. Refresh and retry.".into()))
            }
        };
        if let Some(result) = rewrite_done {
            let restore_row = self.rewrite_job.take().and_then(|job| job.restore_row);
            let same_repository = self.operation_repo.as_ref() == self.stash_path().as_ref();
            self.operation_repo = None;
            if same_repository {
                match result {
                    Ok((receipt, relation)) => {
                        let generation = self.finish_rewrite(ctx, receipt, relation, restore_row);
                        self.rewrite_refresh = Some(generation);
                    }
                    Err(error) => {
                        self.refresh_mutation();
                        self.set_outcome(
                            OutcomeSeverity::Error,
                            error,
                            None,
                            Some(RecoveryAction::Backups),
                        );
                    }
                }
            } else {
                self.mutation_notice = None;
            }
        }
        let operation_done = match self.operation_worker.as_ref().map(|rx| rx.try_recv()) {
            None | Some(Err(std::sync::mpsc::TryRecvError::Empty)) => None,
            Some(Ok(result)) => Some(result),
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                Some(Err("Operation worker stopped. Refresh and retry.".into()))
            }
        };
        if let Some(result) = operation_done {
            self.operation_worker = None;
            self.mutation_busy = None;
            let same_repository = self.operation_repo.as_ref() == self.stash_path().as_ref();
            self.operation_repo = None;
            if !same_repository {
                self.operation_restore_row = None;
                self.mutation_notice = None;
            } else {
                match result {
                    Ok(outcome) => self.finish_operation(ctx, outcome),
                    Err(error) => {
                        self.operation_restore_row = None;
                        self.mutation_notice = None;
                        if let Some(target) = self.rebase_target.take()
                            && gg_git::dirty_tree_error(&error)
                        {
                            self.rebase_dialog = Some(RebaseDialog {
                                reference: target,
                                autostash: true,
                                error: Some(
                                    "Working tree is not clean. Autostash is enabled: submit Rebase to stash the changes and continue."
                                        .into(),
                                ),
                            });
                        }
                        self.set_outcome(OutcomeSeverity::Error, error.clone(), None, None);
                        self.mutation_error = Some(error.clone());
                        if let Some(view) = &mut self.conflict_view {
                            view.error = Some(error);
                        }
                        self.refresh_mutation();
                    }
                }
            }
        }
        self.poll_conflict_view(ctx);
        let narrow = ctx.content_rect().width() < 1040.0;
        let wide_refs = !narrow
            && self.refs_shown()
            && self.state.history.is_some()
            && ctx.content_rect().width() >= 1120.0;
        history_view::set_graph_width(graph::column_width(self.state.graph_lanes));
        if self.working.is_none() && self.review.is_none() {
            self.state.viewport = self.viewport.clone();
            // The thumb keeps its extent and grab offset while dragged, and
            // on the frame it is released, where egui still places it.
            let dragging = ctx.input(|i| {
                i.pointer.any_released()
                    || i.pointer.primary_down()
                        && i.pointer.press_origin().is_some_and(|p| {
                            self.history_rect.contains(p) && p.x >= self.history_rect.right() - 18.0
                        })
            });
            // A replacement shows on arrival; a reload keeps the old rows
            // until its old top row is found, so it opens at that row.
            if !dragging && let Some(result) = self.pending.as_ref().and_then(stream::Worker::poll)
            {
                match result {
                    Ok(update) if self.pending_filter_edit && !self.state.is_current(&update) => {}
                    Ok(update)
                        if self.rewrite_refresh.is_none()
                            && update.request.anchor_id.is_some()
                            && update.anchor_row.is_none()
                            && !update.complete
                            && update.error.is_none() => {}
                    Ok(update) => {
                        self.restore_offset = Some(
                            update
                                .anchor_row
                                .map(|row| {
                                    row as f32 * crate::density::row()
                                        + self.scroll_offset % crate::density::row()
                                })
                                .unwrap_or(0.0),
                        );
                        self.state.accept(update, true, true);
                        self.worker = self.pending.take();
                        self.pending_filter_edit = false;
                        self.viewport = 0..64;
                        self.scroll_to_selection = false;
                    }
                    Err(error) => {
                        self.state.error = Some(error);
                        self.pending = None;
                    }
                }
            }
            if self.pending.is_none() {
                // The extent held during a drag applies on release.
                if !dragging && let Some(update) = self.staged.take() {
                    self.state.accept(update, false, true);
                }
                if let Some(result) = self.worker.as_ref().and_then(stream::Worker::poll) {
                    match result {
                        Ok(update) if !self.state.is_current(&update) => {}
                        Ok(update) => {
                            if self.state.selected_row.is_none() && update.selection.is_some() {
                                self.scroll_to_selection = true;
                            }
                            if dragging {
                                self.staged = Some(update.clone());
                            }
                            self.state.accept(update, false, !dragging);
                        }
                        Err(error) => self.state.error = Some(error),
                    }
                }
            }
            self.avatars.poll(ctx, true);
            if crate::navigation::available(ctx) {
                if ctx.input_mut(|i| {
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                        || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
                }) {
                    self.narrow_details = false;
                }
                if self.state.selected.is_some()
                    && crate::navigation::activation_available(ctx)
                    && ctx.input_mut(|i| {
                        i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                            || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)
                    })
                {
                    if narrow && self.narrow_details {
                        // Enter in the details reviews the chosen file.
                        self.review_from_details(ctx, None);
                    } else {
                        let location = self.capture_return_location();
                        self.open_review(ctx, location);
                    }
                }
                if crate::navigation::activation_available(ctx)
                    && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Space))
                {
                    if narrow {
                        self.narrow_details = !self.narrow_details;
                    } else {
                        self.details_visible = !self.details_visible;
                    }
                }
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::ALT, egui::Key::A)) {
                    self.commit_actions = true;
                }
                if crate::navigation::activation_available(ctx)
                    && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::R))
                    && self.frame_readiness().rewrite_reason.is_none()
                    && let Some(id) = self.state.selected.clone()
                {
                    self.run_commit_action(ctx, id, CommitAction::Rewrite);
                }
                for (key, action) in [
                    (egui::Key::E, gg_engine::rewrite::Action::Edit),
                    (egui::Key::F, gg_engine::rewrite::Action::Fixup),
                    (egui::Key::D, gg_engine::rewrite::Action::Drop),
                ] {
                    if crate::navigation::activation_available(ctx)
                        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key))
                        && self.frame_readiness().rewrite_reason.is_none()
                    {
                        self.rebase_preset(ctx, action);
                    }
                }
                let page = self.page_rows.saturating_sub(1).max(1) as i32;
                for (modifiers, key, distance) in [
                    (egui::Modifiers::NONE, egui::Key::ArrowUp, -1),
                    (egui::Modifiers::NONE, egui::Key::ArrowDown, 1),
                    (egui::Modifiers::NONE, egui::Key::K, -1),
                    (egui::Modifiers::NONE, egui::Key::J, 1),
                    (egui::Modifiers::CTRL, egui::Key::P, -1),
                    (egui::Modifiers::CTRL, egui::Key::N, 1),
                    (egui::Modifiers::NONE, egui::Key::PageUp, -page),
                    (egui::Modifiers::NONE, egui::Key::PageDown, page),
                    (egui::Modifiers::ALT, egui::Key::V, -page),
                ] {
                    // In the narrow details j/k still step commits, while
                    // Up/Down choose a file in the FILES list.
                    let details = narrow && self.narrow_details;
                    let file_key = matches!(key, egui::Key::ArrowUp | egui::Key::ArrowDown)
                        && modifiers == egui::Modifiers::NONE;
                    if details
                        && (file_key
                            || matches!(
                                key,
                                egui::Key::PageUp | egui::Key::PageDown | egui::Key::V
                            ))
                    {
                        if file_key && ctx.input_mut(|i| i.consume_key(modifiers, key)) {
                            self.details_file =
                                self.details_file.saturating_add_signed(distance as isize);
                        }
                        continue;
                    }
                    if ctx.input_mut(|i| i.consume_key(modifiers, key)) {
                        self.step_selection(ctx, distance);
                    }
                }
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::C))
                    && let Some(id) = &self.state.selected
                {
                    ctx.copy_text(id.clone());
                }
            }
        }
        if self.commit_actions {
            self.commit_actions = false;
            if self.state.selected.is_some() {
                if narrow {
                    self.narrow_details = true;
                } else {
                    self.details_visible = true;
                }
                egui::Popup::open_id(ctx, details_view::commit_actions_popup_id());
            }
        }
        let mut go_local = false;
        let mut go_history = false;
        let working_mutation_pending = self.working_mutation_pending();
        let ready = self.frame_readiness();
        let shortcuts = crate::navigation::available(ctx);
        let open_preferences = shortcuts
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma));
        if shortcuts && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Num1))
        {
            go_local = self.state.history.is_some();
        }
        if shortcuts && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Num2))
        {
            go_history = true;
        }
        if shortcuts
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Backslash))
        {
            self.toggle_refs_or_drawer(ctx);
        }
        let table = self.menu_table(&ready);
        let menu_command;
        if let Some(native) = self.native_menu.as_mut() {
            // macOS: the menus are in the menu bar at the top of the screen.
            native.sync(&table, native_menu_role);
            menu_command = native.chosen(ctx).into_iter().next();
        } else {
            // The window identity: repository and path, plus the operation
            // while one runs. The branch lives on the toolbar's branch button only.
            let identity = match &self.state.history {
                Some(history) if history.shallow => {
                    format!("{}  ·  shallow", self.window_title)
                }
                Some(_) => self.window_title.clone(),
                None => "Git Good — open a repository".into(),
            };
            if let Some(read) = &self.caption_style {
                self.chrome.style = read(ctx);
            }
            menu_command = titlebar::show(
                ui,
                &self.chrome,
                MENU_BAR_HEIGHT,
                &table,
                &identity,
                FrameCommand::Quit,
            );
        }
        let mut tool = menu_command;
        let mut open_search = false;
        let mut search_field: Option<(egui::Rect, bool)> = None;
        let mut branch_pick = None;
        egui::Panel::top("navigation")
            .exact_size(TOOLBAR_HEIGHT)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let has_repo = self.state.history.is_some();
                    let local_selected = self.working.is_some();
                    let sidebar_available = ctx.content_rect().width() >= 1120.0;
                    // Without a repository the toolbar holds only search;
                    // the repository controls appear once one opens.
                    if has_repo {
                    // Narrower than the docked sidebar, the button opens the
                    // refs drawer over the view.
                    let sidebar = ui
                        .add(
                            egui::Button::selectable(
                                (self.refs_shown() && sidebar_available) || self.refs_drawer,
                                crate::phosphor::icon(crate::phosphor::SIDEBAR_SIMPLE).size(17.0),
                            )
                            .min_size(egui::vec2(28.0, 26.0)),
                        )
                        .on_hover_text(if sidebar_available {
                            "Refs sidebar  Ctrl+\\"
                        } else {
                            "Refs  Ctrl+\\\nThe window is narrower than 1120 px, so the refs open as a drawer over the view"
                        });
                    if sidebar.clicked() {
                        self.toggle_refs_or_drawer(ctx);
                    }
                    ui.add_space(6.0);
                    let changes = ui
                        .add_enabled(
                            has_repo,
                            egui::Button::selectable(
                                local_selected,
                                ("Changes", count_badge(ready.changes as u64)),
                            )
                            .min_size(egui::vec2(96.0, 24.0)),
                        )
                        .on_hover_text(match ready.changes {
                            0 => "Local Changes: stage, unstage, commit, and amend".to_string(),
                            1 => "Local Changes: 1 changed file".to_string(),
                            count => format!("Local Changes: {count} changed files"),
                        });
                    if changes.clicked() {
                        go_local = true;
                    }
                    if ui
                        .add_enabled(
                            has_repo && !working_mutation_pending,
                            egui::Button::selectable(!local_selected, "History")
                                .min_size(egui::vec2(84.0, 24.0)),
                        )
                        .on_hover_text(if working_mutation_pending {
                            "Wait for the current Local Changes operation to finish"
                        } else {
                            "Browse commit history"
                        })
                        .clicked()
                    {
                        go_history = true;
                    }
                    ui.add_space(8.0);
                    let busy = Some("Wait for the current operation to finish");
                    let no_repo = Some("Open a repository first");
                    let idle = |ok: bool| {
                        if ok {
                            None
                        } else if has_repo {
                            busy
                        } else {
                            no_repo
                        }
                    };
                    // Everyday navigation and sync only; every other command
                    // lives in its menu and in search.
                    use crate::phosphor as icon;
                    if toolbar_button(ui, icon::DOWNLOAD_SIMPLE, None, "Fetch  Ctrl+Shift+F", idle(ready.network_idle)) {
                        tool = Some(FrameCommand::Fetch);
                    }
                    let strategy = self.prefs.pull_strategy.menu_label();
                    let upstream = self
                        .state
                        .refs
                        .current_tracking()
                        .map(|tracking| format!(" from {}", tracking.upstream))
                        .unwrap_or_default();
                    let pull_tip = if ready.behind > 0 {
                        format!("Pull {}{upstream} ({strategy})  Ctrl+Shift+L", ready.behind)
                    } else {
                        format!("Pull{upstream} ({strategy})  Ctrl+Shift+L")
                    };
                    if toolbar_button(
                        ui,
                        icon::ARROW_LINE_DOWN,
                        Some(ready.behind),
                        &pull_tip,
                        ready.pull_reason,
                    ) {
                        tool = Some(FrameCommand::Pull);
                    }
                    if toolbar_button(ui, icon::ARROW_LINE_UP, None, "Push  Ctrl+Shift+P", idle(ready.network_idle)) {
                        tool = Some(FrameCommand::Push);
                    }
                    // The owner's toolbar: sync, new branch, stash with its
                    // picker, rebase from the selected commit, and recovery.
                    // New tag and Merge live in the Branch menu and search.
                    let separator = |ui: &mut egui::Ui| {
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                    };
                    separator(ui);
                    if toolbar_button(ui, icon::GIT_BRANCH, None, "New branch…  Ctrl+B", idle(ready.branch_idle)) {
                        tool = Some(FrameCommand::NewBranch);
                    }
                    separator(ui);
                    if toolbar_button(ui, icon::ARCHIVE, None, "Stash changes…  Ctrl+Shift+Z", ready.stash_reason) {
                        tool = Some(FrameCommand::Stash);
                    }
                    let stashes_reason = if !ready.stash_idle {
                        idle(false)
                    } else if ready.stashes == 0 {
                        Some("No stashes")
                    } else {
                        None
                    };
                    let stashes = ui
                        .add_enabled(
                            stashes_reason.is_none(),
                            egui::Button::new((
                                icon::icon(icon::BOX_ARROW_UP).size(17.0),
                                egui::RichText::new(ready.stashes.to_string()).size(11.0),
                                icon::icon(icon::CARET_DOWN).size(10.0),
                            ))
                            .min_size(egui::vec2(28.0, 26.0))
                            .frame_when_inactive(false),
                        );
                    let stashes_tip =
                        format!("Stashes ({}) · apply, pop, show, or drop", ready.stashes);
                    let stashes = match stashes_reason {
                        Some(reason) => {
                            stashes.on_disabled_hover_text(format!("{stashes_tip}\n{reason}"))
                        }
                        None => stashes.on_hover_text(stashes_tip),
                    };
                    if stashes.clicked() {
                        self.stash_anchor = Some(stashes.rect.left_bottom());
                        tool = Some(FrameCommand::Stashes);
                    }
                    separator(ui);
                    if toolbar_button(
                        ui,
                        icon::LIST_NUMBERS,
                        None,
                        "Interactive rebase from here…  R",
                        ready.rewrite_reason,
                    ) {
                        tool = Some(FrameCommand::RewriteSelected);
                    }
                    if toolbar_button(
                        ui,
                        icon::CLOCK_COUNTER_CLOCKWISE,
                        None,
                        "Recovery…",
                        idle(ready.has_repo),
                    ) {
                        tool = Some(FrameCommand::Recovery);
                    }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let icon = crate::phosphor::icon(crate::phosphor::MAGNIFYING_GLASS);
                        // A search field when there is room; narrow windows keep
                        // an icon that opens the same search with its own field.
                        let wide = if has_repo {
                            ui.available_width() >= 150.0
                        } else {
                            ctx.content_rect().width() >= welcome_view::COMPACT_WINDOW
                        };
                        if wide {
                            let roomy = ui.available_width() >= 560.0;
                            let width = if roomy {
                                330.0
                            } else if ui.available_width() >= 420.0 {
                                220.0
                            } else {
                                140.0
                            };
                            let hint = if !has_repo {
                                "Search repositories and actions"
                            } else if roomy {
                                "Search commits, branches, files, actions"
                            } else {
                                "Search"
                            };
                            let field = ui
                                .add(
                                    egui::TextEdit::singleline(&mut self.search_text)
                                        .id(search_field_id())
                                        .hint_text(format!(
                                            "{}  {hint}",
                                            crate::phosphor::MAGNIFYING_GLASS
                                        ))
                                        .desired_width(width),
                                )
                                .on_hover_text(
                                    "Search commands, branches, remotes, tags, and files (Ctrl+K)",
                                );
                            if self.search_text.is_empty() && !field.has_focus() {
                                search_key_chip(ui, field.rect);
                            }
                            search_field = Some((
                                field.rect,
                                field.has_focus() || field.changed(),
                            ));
                        } else {
                            let search = ui
                                .add(
                                    egui::Button::new(icon.size(17.0))
                                        .min_size(egui::vec2(28.0, 26.0))
                                        .frame_when_inactive(false),
                                )
                                .on_hover_text(
                                    "Search commands, branches, remotes, tags, and files (Ctrl+K)",
                                );
                            if search.clicked() {
                                open_search = true;
                            }
                        }
                        // Below 1040 px the branch button is an icon; its
                        // tooltip and menu name the branch and ↑↓.
                        let compact = !wide
                            || ui.available_width() < 230.0
                            || ctx.content_rect().width() < 1040.0;
                        if has_repo
                            && let Some(pick) = self.branch_switcher_button(ui, &ready, compact)
                        {
                            branch_pick = Some(pick);
                        }
                    });
                });
            });
        // Changes › Stashes… and the sidebar's stash rows open the picker
        // centered under the toolbar.
        if let Some(pick) = self.stash_picker(ctx, ui.layer_id()) {
            self.apply_stash_pick(ctx, pick);
        }
        match branch_pick {
            Some(BranchPick::Switch(name)) => self.start_branch(ctx, name, false),
            // Track and switch in one step (accepted proposal); the worktree
            // check and errors work as for any switch.
            Some(BranchPick::Track(name)) => self.start_track(ctx, name),
            Some(BranchPick::New) => self.open_new_branch(),
            None => {}
        }
        if open_preferences {
            let section = self
                .folders_dialog
                .as_ref()
                .map_or(PrefSection::General, |dialog| dialog.section);
            self.open_preferences(section);
        }
        // Ctrl+F opens search with the History filter, whose first result
        // filters History (there is no separate filter row).
        let filter = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F))
            && self.search.is_none();
        if filter {
            self.narrow_details = false;
            self.search_text = self.state.query.clone();
        }
        let toggle =
            filter || ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::K));
        match search_field {
            // Wide: Ctrl+K focuses the toolbar field; typing there opens results.
            Some((rect, active)) => {
                if toggle && self.search.is_some() {
                    self.close_search(ctx);
                } else if toggle || active {
                    if toggle {
                        ctx.memory_mut(|m| m.request_focus(search_field_id()));
                    }
                    let palette = self.search.get_or_insert_with(SearchPalette::default);
                    palette.in_field = true;
                    palette.anchor = rect;
                    if palette.query != self.search_text {
                        palette.query = self.search_text.clone();
                        palette.selected = 0;
                    }
                } else if let Some(palette) = &mut self.search {
                    palette.anchor = rect;
                }
            }
            None => {
                if open_search || toggle {
                    if self.search.is_some() && !open_search {
                        self.close_search(ctx);
                    } else {
                        self.search = Some(SearchPalette {
                            query: if filter {
                                self.state.query.clone()
                            } else {
                                String::new()
                            },
                            ..Default::default()
                        });
                    }
                }
            }
        }
        if self.search.is_some()
            && let Some(command) = self.draw_search(ctx, &ready)
        {
            tool = Some(command);
        }
        if let Some(command) = tool {
            self.run_frame_command(ctx, command, &mut go_local, &mut go_history);
        }
        if let Some(generation) = self.rewrite_refresh {
            if self.state.loaded_generation >= generation {
                self.rewrite_refresh = None;
                self.rewrite_dialog = None;
            } else if let Some(error) = self.state.error.clone() {
                self.rewrite_refresh = None;
                let summary = format!("Refresh failed: {error}");
                self.set_outcome(
                    OutcomeSeverity::Error,
                    summary,
                    None,
                    Some(RecoveryAction::Backups),
                );
                self.state.error = None;
                self.pending = None;
            }
        }
        let mut mutation_action = None;
        let mut resolve_conflicts = false;
        let rebase_progress = self.rebase_progress().cloned();
        // The operation band appears only while Git holds a merge, rebase,
        // cherry-pick, or revert open, or a rewrite runs. Short jobs
        // (network, branch work, a merge that never stops) report in the
        // footer instead, so they never move the panes.
        if let Some(area) = self.operation_area() {
            let busy = self.mutation.as_ref().and_then(|state| {
                self.mutation_busy
                    .map(|command| operation_band::busy_label(state.operation, command))
            });
            let band = match (&area, &rebase_progress) {
                (
                    OperationArea::Mutation {
                        operation: gg_engine::mutation::Operation::Rebase,
                        conflicts,
                        pending,
                    },
                    Some(progress),
                ) => rebase_stop::band(
                    progress,
                    *conflicts,
                    self.conflict_view.is_some(),
                    *pending,
                    busy.as_deref(),
                    self.rewrite_backup.as_deref(),
                ),
                (OperationArea::Mutation { .. }, _) => self
                    .mutation_band()
                    .unwrap_or_else(|| self.rewrite_band(false)),
                (OperationArea::Rewrite { refreshing }, _) => self.rewrite_band(*refreshing),
            };
            let keyed = band
                .button(operation_band::Command::Continue)
                .is_some_and(|button| button.enabled)
                && crate::navigation::available(ctx)
                && !ctx.text_edit_focused()
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter));
            egui::Panel::top("operation-area")
                .exact_size(OPERATION_AREA_HEIGHT)
                .show(ui, |ui| {
                    ui.set_clip_rect(ui.max_rect().expand2(egui::vec2(8.0, 4.0)));
                    match operation_band::show(ui, &band).0 {
                        Some(operation_band::Command::Continue) => {
                            mutation_action = Some(MutationAction::Continue);
                        }
                        Some(operation_band::Command::Skip) => {
                            mutation_action = Some(MutationAction::Skip);
                        }
                        Some(operation_band::Command::Abort) => {
                            mutation_action = Some(MutationAction::Abort);
                        }
                        Some(operation_band::Command::ResolveConflicts) => {
                            resolve_conflicts = true;
                        }
                        None if keyed => mutation_action = Some(MutationAction::Continue),
                        None => {}
                    }
                });
        }
        if resolve_conflicts {
            self.open_conflict_view(ctx);
        }
        if let Some(action) = mutation_action
            && let Some(operation) = self.mutation.as_ref().map(|state| state.operation)
            && let Err(error) = self.recover_mutation(ctx, operation, action)
        {
            self.mutation_error = Some(error);
        }
        if self.edit_path {
            // Repository › Open… as a modal (proposal): folder field with
            // Browse…, the recent repositories filtered by what is typed,
            // Clone… to switch dialogs, Cancel (Esc) and Open (Enter).
            let open_ready = self.pending.is_none() && !self.repository_switch_pending();
            let mut open = None;
            let mut cancel = false;
            let mut clone = false;
            let home = std::env::var("HOME").ok();
            let recent = self
                .recent
                .iter()
                .map(|entry| entry.path.clone())
                .filter(|path| path.is_absolute())
                .collect::<Vec<_>>();
            let modal = egui::Modal::new(egui::Id::unique("open-repository"))
                .backdrop_color(egui::Color32::from_black_alpha(140))
                .show(ctx, |ui| {
                    ui.set_width(520.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Open repository").size(16.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            cancel |= ui.button("×").on_hover_text("Close (Esc)").clicked();
                        });
                    });
                    ui.separator();
                    ui.label(egui::RichText::new("FOLDER").small().weak());
                    let response = ui
                        .horizontal(|ui| {
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.path)
                                    .desired_width(420.0)
                                    .hint_text("Repository path"),
                            );
                            if ui
                                .button("Browse…")
                                .on_hover_text("Choose a repository folder with the desktop dialog")
                                .clicked()
                                && let Some(path) = folder_picker::pick_folder(
                                    "Open repository",
                                    Some(Path::new(&self.path)),
                                )
                            {
                                self.path = path.to_string_lossy().into_owned();
                            }
                            response
                        })
                        .inner;
                    if !response.has_focus() && self.path.is_empty() {
                        response.request_focus();
                    }
                    let enter =
                        response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let typed = self.path.trim().to_lowercase();
                    let matches = recent
                        .iter()
                        .filter(|path| {
                            typed.is_empty()
                                || path.to_string_lossy().to_lowercase().contains(&typed)
                        })
                        .take(settings::RECENT_LIMIT)
                        .collect::<Vec<_>>();
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("RECENT").small().weak());
                    // A fixed slot so filtering never resizes the dialog.
                    let slot = settings::RECENT_LIMIT as f32 * crate::density::row();
                    ui.allocate_ui_with_layout(
                        egui::vec2(500.0, slot),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_min_size(egui::vec2(500.0, slot));
                            if matches.is_empty() {
                                ui.weak(if recent.is_empty() {
                                    "Repositories you open are listed here."
                                } else {
                                    "No recent repository matches."
                                });
                            }
                            for path in matches {
                                let name = path
                                    .file_name()
                                    .map(|name| name.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                let shown = window_title(
                                    &path.display().to_string(),
                                    home.as_deref(),
                                    None,
                                );
                                let row = ui.add(
                                    egui::Button::new((
                                        egui::RichText::new(name),
                                        egui::RichText::new(
                                            shown.trim_start_matches("Git Good — "),
                                        )
                                        .small()
                                        .weak(),
                                    ))
                                    .frame_when_inactive(false)
                                    .truncate()
                                    .min_size(egui::vec2(500.0, crate::density::row())),
                                );
                                if row.clicked() && open_ready {
                                    open = Some(path.display().to_string());
                                }
                            }
                        },
                    );
                    ui.separator();
                    ui.horizontal(|ui| {
                        clone |= ui.button("Clone…").clicked();
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let primary = egui::Button::new((
                                egui::RichText::new("Open").color(egui::Color32::WHITE),
                                egui::RichText::new("Enter")
                                    .small()
                                    .color(egui::Color32::from_rgb(184, 212, 230)),
                            ))
                            .fill(egui::Color32::from_rgb(11, 85, 119));
                            let ready = open_ready && !self.path.trim().is_empty();
                            if ui.add_enabled(ready, primary).clicked() || (ready && enter) {
                                open = Some(self.path.trim().to_string());
                            }
                            cancel |= ui
                                .button(("Cancel", egui::RichText::new("Esc").small().weak()))
                                .clicked();
                        });
                    });
                });
            if modal.should_close() {
                cancel = true;
            }
            if let Some(path) = open {
                self.path = path;
                self.recovery = None;
                if let Some(working) = &self.working {
                    self.working_drafts
                        .insert(working.repository_key(), working.capture_draft());
                }
                self.working = None;
                self.review = None;
                self.load(ctx);
                self.edit_path = false;
            } else if clone {
                self.edit_path = false;
                let (mut local, mut history) = (false, false);
                self.run_frame_command(ctx, FrameCommand::Clone, &mut local, &mut history);
            } else if cancel {
                self.edit_path = false;
            }
        }
        if wide_refs {
            let refs = self.state.refs.clone();
            let busy = self.branch.is_some()
                || self.network.is_some()
                || self.mutation.is_some()
                || self.operation_busy();
            let has_repo = self.state.history.is_some();
            let mut action = None;
            let refs_panel = egui::Panel::left("refs-sidebar")
                .default_size(210.0)
                .min_size(180.0)
                .max_size(420.0)
                .resizable(true)
                .show_separator_line(false)
                .frame(
                    egui::Frame::side_top_panel(ui.style().as_ref())
                        .inner_margin(8.0)
                        .stroke(egui::Stroke::NONE),
                )
                .show(ui, |ui| {
                    let rect = ui.available_rect_before_wrap();
                    ui.allocate_rect(rect, egui::Sense::hover());
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                    child.set_clip_rect(rect.intersect(ui.clip_rect()));
                    let ui = &mut child;
                    ui.add_enabled_ui(has_repo, |ui| {
                        let stashes = self
                            .local_summary
                            .as_ref()
                            .map(|summary| summary.stashes.clone())
                            .unwrap_or_default();
                        let worktrees = self
                            .local_summary
                            .as_ref()
                            .map(|summary| summary.worktrees.clone())
                            .unwrap_or_default();
                        action = refs_view::show(
                            ui,
                            &refs,
                            &stashes,
                            &worktrees,
                            &mut self.refs_query,
                            busy,
                            &self.state.ref_lanes,
                        );
                    });
                });
            self.refs_width = refs_panel.response.rect.width();
            if let Some(action) = action {
                self.apply_refs_action(ctx, action);
            }
        }
        self.draw_refs_drawer(ctx);
        if go_local {
            let conflicted = self
                .mutation
                .as_ref()
                .is_some_and(|state| !state.conflicts.is_empty());
            if conflicted && self.conflict_view.is_none() {
                self.open_conflict_view(ctx);
            } else {
                self.open_working(ctx);
            }
        }
        if go_history {
            self.close_to_history(ctx);
            self.narrow_details = false;
        }
        self.draw_folders_dialog(ctx);
        self.draw_menu_dialogs(ctx);
        self.draw_new_repository(ctx);
        self.draw_stash_switch(ctx);
        let mut chosen_clone_folder = None;
        if let Some(dialog) = &mut self.clone_dialog {
            let mut submit = None;
            let mut cancelled = false;
            let mut open_folders = false;
            let mut field_lost_focus = false;
            let mut open =
                !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            let default_folder = self.folders.default_folder();
            let candidates = self.folders.candidates();
            egui::Window::new("Clone repository")
                .id(egui::Id::unique("clone-repository")).modal(ctx, egui::Id::unique("clone-repository"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(520.0)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(
                        "Clone with Git credentials and progress. The worktree opens after success.",
                    );
                    ui.label("Repository URL");
                    let url = ui.add(
                        egui::TextEdit::singleline(&mut dialog.url)
                            .hint_text("https://host/owner/repo.git or git@host:owner/repo.git")
                            .desired_width(f32::INFINITY),
                    );
                    if url.changed() {
                        dialog.sync_name();
                    }
                    ui.add_space(4.0);
                    // One folder control (proposal): the saved folders, the
                    // launch folder, another folder, and the folder list.
                    let mut choose_other = false;
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("INTO FOLDER").small().weak());
                            let home = std::env::var("HOME").ok();
                            let shown = window_title(dialog.folder.trim(), home.as_deref(), None);
                            egui::ComboBox::from_id_salt("clone-folder")
                                .selected_text(shown.trim_start_matches("Git Good — ").to_string())
                                .width(260.0)
                                .show_ui(ui, |ui| {
                                    for candidate in &candidates {
                                        let label = candidate.to_string_lossy().into_owned();
                                        let text = clone_folder_label(candidate, &default_folder);
                                        if crate::menus::choice(
                                            ui,
                                            dialog.folder.trim() == label,
                                            text,
                                        )
                                        .clicked()
                                        {
                                            dialog.folder = label;
                                            chosen_clone_folder = Some(candidate.clone());
                                        }
                                    }
                                    ui.separator();
                                    choose_other |= ui
                                        .add(crate::menus::row(false, "Choose another folder…"))
                                        .clicked();
                                    open_folders |= ui
                                        .add(crate::menus::row(false, "Manage clone folders…"))
                                        .clicked();
                                });
                        });
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("NAME").small().weak());
                            let name = ui.add(
                                egui::TextEdit::singleline(&mut dialog.name)
                                    .hint_text("Project folder")
                                    .desired_width(200.0),
                            );
                            if name.changed() {
                                dialog.name_from_url = false;
                            }
                            field_lost_focus |= name.lost_focus();
                        });
                    });
                    if choose_other
                        && let Some(path) = folder_picker::pick_folder(
                            "Clone destination",
                            Some(Path::new(dialog.folder.trim())),
                        )
                    {
                        dialog.folder = path.to_string_lossy().into_owned();
                        chosen_clone_folder = Some(path);
                    }
                    let destination = dialog.destination();
                    match &destination {
                        Some(path) => {
                            ui.weak(format!("Clones into {}", path.display()));
                        }
                        None => {
                            ui.colored_label(
                                ui.visuals().warn_fg_color,
                                "Enter an absolute parent folder and one project name.",
                            );
                        }
                    }
                    let ready = !dialog.url.trim().is_empty()
                        && destination.is_some()
                        && self.network.is_none()
                        && self.branch.is_none();
                    let enter = (url.lost_focus() || field_lost_focus)
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(ready, egui::Button::new("Clone"))
                            .on_disabled_hover_text(
                                "Enter a repository URL, an absolute parent folder, and a single project name",
                            )
                            .clicked()
                            || (enter && ready)
                        {
                            submit = Some((dialog.url.trim().to_string(), destination.clone()));
                        }
                        if ui.button("Cancel").clicked() {
                            cancelled = true;
                        }
                    });
                });
            if cancelled {
                open = false;
            }
            if open_folders {
                let mut preferences = FoldersDialog::new(PrefSection::General);
                preferences.new_root = dialog.folder.trim().to_owned();
                self.folders_dialog = Some(preferences);
            }
            if let Some((url, destination)) = submit {
                self.clone_dialog = None;
                self.start_clone(ctx, url, destination);
            } else if !open {
                self.clone_dialog = None;
            }
        }
        if let Some(folder) = chosen_clone_folder {
            self.choose_clone_folder(folder);
        }
        let track_busy = self.branch.is_some() || self.network.is_some() || self.operation_busy();
        if let Some(remote_branch) = &mut self.track_dialog {
            let mut submit = false;
            let mut cancelled = false;
            let mut open =
                !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            let busy = track_busy;
            egui::Window::new("Track remote branch")
                .id(egui::Id::unique("track-remote-branch")).modal(ctx, egui::Id::unique("track-remote-branch"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(400.0)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(
                        "Create and switch to a local branch tracking a remote branch. The worktree must be clean.",
                    );
                    let response = ui.add(
                        egui::TextEdit::singleline(remote_branch)
                            .hint_text("origin/main")
                            .desired_width(f32::INFINITY),
                    );
                    let enter =
                        response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !busy && !remote_branch.trim().is_empty(),
                                egui::Button::new("Track branch"),
                            )
                            .clicked()
                            || (enter && !busy)
                        {
                            submit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancelled = true;
                        }
                    });
                });
            if cancelled {
                open = false;
            }
            if submit && !remote_branch.trim().is_empty() {
                let remote_branch = remote_branch.trim().to_string();
                self.track_dialog = None;
                self.start_track(ctx, remote_branch);
            } else if !open {
                self.track_dialog = None;
            }
        }
        let references = self.reflog_references();
        let mut recovery_action = None;
        let mut recovery_open = true;
        if let Some(view) = &mut self.recovery {
            recovery_action = view.draw(ctx, &references);
            recovery_open = view.open;
        }
        if !recovery_open {
            self.recovery = None;
        }
        self.draw_worktrees(ctx);
        if let Some(action) = recovery_action {
            match action {
                recovery_view::Action::Open {
                    source,
                    reference,
                    commit,
                    subject,
                    entry,
                } => {
                    self.open_recovery_review(ctx, source, reference, commit, subject, entry);
                }
                recovery_view::Action::Reset { commit, mode } => {
                    self.reset_confirm = Some((commit, mode));
                }
                recovery_view::Action::CreateBranch {
                    source,
                    commit,
                    selector,
                } => {
                    self.create_branch = Some(CreateBranchDialog {
                        name: String::new(),
                        commit,
                        selector,
                        switch: false,
                        recovery_source: Some(source),
                        error: None,
                    });
                }
                recovery_view::Action::Changed => {
                    self.refresh_mutation();
                    self.load(ctx);
                    self.refresh_working(ctx);
                }
            }
        }

        let mut create_branch_submit = None;
        let mut create_branch_cancel = false;
        let create_summary = self
            .create_branch
            .as_ref()
            .map(|dialog| self.commit_summary(&dialog.commit));
        if let Some(dialog) = &mut self.create_branch {
            egui::Window::new(dialog.title())
                .id(egui::Id::unique("create-branch-dialog"))
                .modal(ctx, egui::Id::unique("create-branch-dialog"))
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(format!("Create a branch at {}", dialog.selector));
                    if let Some(summary) = &create_summary {
                        ui.add(
                            egui::Label::new(egui::RichText::new(summary).monospace()).truncate(),
                        )
                        .on_hover_text(summary);
                    }
                    ui.add(
                        egui::TextEdit::singleline(&mut dialog.name)
                            .hint_text("Branch name")
                            .desired_width(300.0),
                    );
                    ui.checkbox(&mut dialog.switch, "Switch after creation");
                    if let Some(error) = &dialog.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !dialog.name.trim().is_empty(),
                                egui::Button::new("Create"),
                            )
                            .clicked()
                        {
                            create_branch_submit =
                                Some((dialog.name.trim().to_string(), dialog.switch));
                        }
                        if ui.button("Cancel").clicked() {
                            create_branch_cancel = true;
                        }
                    });
                });
        }
        if create_branch_cancel {
            self.create_branch = None;
        }
        if let Some((name, switch)) = create_branch_submit
            && let Some(dialog) = self.create_branch.clone()
        {
            // Recovery branches always start with switching off; remember
            // only the ordinary new-branch choice.
            if dialog.recovery_source.is_none() {
                self.remember(|prefs| prefs.branch_switch = switch);
            }
            match self.stash_path() {
                Some(path) => match gg_git::branch::create_at(&path, &name, &dialog.commit) {
                    Ok(receipt) => {
                        let source = dialog.recovery_source;
                        let notice = format!(
                            "Created {} at {}",
                            receipt.branch,
                            self.commit_summary(&receipt.commit)
                        );
                        self.create_branch = None;
                        if let Some(view) = &mut self.recovery
                            && source.is_some()
                        {
                            view.notice(notice);
                        }
                        if switch {
                            // Reload once the switch ends: a read started
                            // now would land after it with the old HEAD.
                            self.branch_after_create = Some(receipt.branch.clone());
                            self.start_branch(ctx, receipt.branch.clone(), false);
                            if self.branch.is_none() {
                                self.branch_after_create = None;
                                let error = self.branch_error.take();
                                self.load(ctx);
                                if let Some(error) = error {
                                    let summary = format!(
                                        "Created {} but the switch could not start: {error}",
                                        receipt.branch
                                    );
                                    self.set_outcome(OutcomeSeverity::Warning, summary, None, None);
                                }
                            }
                        } else {
                            self.load(ctx);
                        }
                        if !switch && source.is_none() {
                            self.branch_warning = Some(format!("Created {}", receipt.branch));
                        }
                    }
                    Err(error) => {
                        if let Some(dialog) = &mut self.create_branch {
                            dialog.error = Some(error);
                        }
                    }
                },
                None => {
                    if let Some(dialog) = &mut self.create_branch {
                        dialog.error = Some("Open a repository before creating a branch.".into());
                    }
                }
            }
        }

        let mut branch_delete_action = None;
        let mut branch_delete_confirmed = false;
        let mut branch_delete_open = true;
        const BRANCH_DELETE_SLOTS: [&str; 2] = ["delete-branch", "delete-branch-tips"];
        // Delete only when a re-read on confirm matches what was shown.
        let mut branch_guard = self.guards.remove("delete-branch").unwrap_or_default();
        let branch_verdict = branch_guard.poll(&mut self.consequences);
        // The commits only this branch has (dialogs proposal): an unmerged
        // branch names them and its one button forces the delete.
        let (mut unmerged, mut tips) = match self
            .branch_delete
            .as_ref()
            .map(|dialog| dialog.name.clone())
        {
            Some(name) => {
                let range = format!("HEAD..refs/heads/{name}");
                let tip = format!("refs/heads/{name}");
                (
                    self.take_consequence(
                        "delete-branch",
                        &["log", "--oneline", "--no-decorate", &range],
                        ctx,
                    ),
                    self.take_consequence("delete-branch-tips", &["rev-parse", "HEAD", &tip], ctx),
                )
            }
            None => {
                for slot in BRANCH_DELETE_SLOTS {
                    self.consequences.remove(slot);
                }
                (None, None)
            }
        };
        let unmerged_count = unmerged.as_mut().and_then(consequence::Lines::count);
        let tips_ready = tips.as_mut().is_none_or(|tips| tips.get().is_some());
        let branch_notice = branch_guard.changed.then_some(consequence::CHANGED);
        let branch_busy = branch_guard.busy();
        if let Some(dialog) = &mut self.branch_delete {
            let mut cancelled = false;
            egui::Window::new("Delete branch")
                .id(egui::Id::unique("delete-branch"))
                .modal(ctx, egui::Id::unique("delete-branch"))
                .open(&mut branch_delete_open)
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.set_width(460.0);
                    ui.label(format!("Delete local branch {}?", dialog.name));
                    if let Some(unmerged) = &mut unmerged {
                        consequence::list(
                            ui,
                            "NOT IN THE CURRENT BRANCH",
                            unmerged,
                            egui::Color32::from_rgb(245, 160, 160),
                        );
                    }
                    if let Some(error) = &dialog.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    consequence::notice(ui, branch_notice);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (label, _) = match unmerged_count {
                                Some(0) => (format!("Delete {}", dialog.name), false),
                                Some(count) => (
                                    format!(
                                        "Delete {} and {}",
                                        dialog.name,
                                        consequence::counted(
                                            count,
                                            "unmerged commit",
                                            "unmerged commits"
                                        )
                                    ),
                                    true,
                                ),
                                None => (format!("Delete {}", dialog.name), false),
                            };
                            let ready = unmerged_count.is_some() && tips_ready;
                            if ui
                                .add_enabled(
                                    ready && !branch_busy,
                                    consequence::danger_button(label, ready),
                                )
                                .clicked()
                            {
                                branch_delete_confirmed = true;
                            }
                            if ui
                                .button(("Cancel", egui::RichText::new("Esc").small().weak()))
                                .clicked()
                            {
                                cancelled = true;
                            }
                        });
                    });
                });
            // Unchanged lists: force exactly when unmerged commits were shown.
            if branch_verdict == Some(true) {
                branch_delete_action = Some((
                    dialog.name.clone(),
                    unmerged_count.is_some_and(|count| count > 0),
                ));
            }
            for (slot, lines) in BRANCH_DELETE_SLOTS
                .into_iter()
                .zip([unmerged.take(), tips.take()])
            {
                if let Some(lines) = lines {
                    self.consequences.insert(slot, lines);
                }
            }
            if branch_delete_confirmed {
                branch_guard.confirm(&self.consequences, &BRANCH_DELETE_SLOTS, ctx);
            }
            self.guards.insert("delete-branch", branch_guard);
            if cancelled {
                branch_delete_open = false;
            }
        }
        if let Some((name, force)) = branch_delete_action {
            if let Err(error) = self.delete_branch(ctx, &name, force)
                && let Some(dialog) = &mut self.branch_delete
            {
                dialog.error = Some(error);
            }
        } else if self.branch_delete.is_some() && !branch_delete_open {
            self.branch_delete = None;
        }

        let mut tag_delete_action = None;
        let mut tag_delete_open = true;
        if let Some(dialog) = &mut self.tag_delete {
            let mut cancelled = false;
            egui::Window::new("Delete tag")
                .id(egui::Id::unique("delete-tag"))
                .modal(ctx, egui::Id::unique("delete-tag"))
                .open(&mut tag_delete_open)
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(format!("Delete local tag {}?", dialog.name));
                    if let Some(error) = &dialog.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            tag_delete_action = Some(dialog.name.clone());
                        }
                        if ui.button("Cancel").clicked() {
                            cancelled = true;
                        }
                    });
                });
            if cancelled {
                tag_delete_open = false;
            }
        }
        if let Some(name) = tag_delete_action {
            if let Err(error) = self.delete_tag(ctx, &name)
                && let Some(dialog) = &mut self.tag_delete
            {
                dialog.error = Some(error);
            }
        } else if self.tag_delete.is_some() && !tag_delete_open {
            self.tag_delete = None;
        }

        let mut reset_submit = None;
        let mut reset_cancel = false;
        let mut reset_mode = None;
        let mut reset_confirmed = false;
        const RESET_SLOTS: [&str; 4] = [
            "reset-commits",
            "reset-head",
            "reset-files",
            "reset-overwritten",
        ];
        if self.reset_confirm.is_none() {
            for slot in RESET_SLOTS {
                self.consequences.remove(slot);
            }
            self.guards.remove("reset");
        }
        if let Some((commit, mode)) = self.reset_confirm.clone() {
            // A confirmation runs only when the re-read lists match what the
            // dialog showed; otherwise they update in place.
            let mut guard = self.guards.remove("reset").unwrap_or_default();
            if guard.poll(&mut self.consequences) == Some(true) {
                reset_submit = Some((commit.clone(), mode));
            }
            let summary = self.commit_summary(&commit);
            // The commits that leave the branch, and for a hard reset the
            // files whose changes are lost and the untracked files it would
            // overwrite (dialogs proposal).
            let range = format!("{commit}..HEAD");
            let mut leaving = self.take_consequence(
                "reset-commits",
                &["log", "--oneline", "--no-decorate", &range],
                ctx,
            );
            let target = format!("{commit}^{{commit}}");
            let mut head =
                self.take_consequence("reset-head", &["rev-parse", "HEAD", &target], ctx);
            let hard = mode == gg_engine::reset::Mode::Hard;
            let (mut lost, mut overwritten) = if hard {
                (
                    self.take_consequence(
                        "reset-files",
                        &["status", "--porcelain=v1", "-uno"],
                        ctx,
                    ),
                    self.take_overwritten(&commit, ctx),
                )
            } else {
                self.consequences.remove("reset-files");
                self.consequences.remove("reset-overwritten");
                (None, None)
            };
            let ready = [&mut leaving, &mut head, &mut lost, &mut overwritten]
                .into_iter()
                .all(|lines| lines.as_mut().is_none_or(|lines| lines.get().is_some()));
            let blocked = overwritten
                .as_mut()
                .and_then(consequence::Lines::count)
                .is_some_and(|count| count > 0);
            let notice = if guard.changed {
                Some(consequence::CHANGED)
            } else if blocked {
                Some("Move or delete the untracked files listed above, then reset.")
            } else {
                None
            };
            let enabled = ready && !blocked && !guard.busy();
            egui::Window::new("Reset here")
                .id(egui::Id::unique("reset-here"))
                .modal(ctx, egui::Id::unique("reset-here"))
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Reset the current branch using {} reset to",
                        mode.as_str()
                    ));
                    ui.add(egui::Label::new(egui::RichText::new(&summary).monospace()).truncate())
                        .on_hover_text(&summary);
                    ui.horizontal(|ui| {
                        for (choice, label, hint) in [
                            (
                                gg_engine::reset::Mode::Soft,
                                "Soft",
                                "Move the branch; keep the index and working files",
                            ),
                            (
                                gg_engine::reset::Mode::Mixed,
                                "Mixed",
                                "Move the branch and reset the index; keep working files",
                            ),
                            (
                                gg_engine::reset::Mode::Hard,
                                "Hard",
                                "Move the branch and discard staged and working changes",
                            ),
                        ] {
                            if ui
                                .radio(mode == choice, label)
                                .on_hover_text(hint)
                                .clicked()
                            {
                                reset_mode = Some(choice);
                            }
                        }
                    });
                    if let Some(leaving) = &mut leaving {
                        consequence::list(
                            ui,
                            "LEAVE THE BRANCH (RECOVERABLE FROM THE REFLOG)",
                            leaving,
                            ui.visuals().text_color(),
                        );
                    }
                    if let Some(lost) = &mut lost {
                        consequence::list(
                            ui,
                            "CHANGES LOST",
                            lost,
                            egui::Color32::from_rgb(245, 160, 160),
                        );
                    }
                    if let Some(overwritten) = &mut overwritten {
                        consequence::list(
                            ui,
                            "UNTRACKED FILES OVERWRITTEN",
                            overwritten,
                            egui::Color32::from_rgb(245, 160, 160),
                        );
                    }
                    consequence::notice(ui, notice);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let submit = if hard {
                                ui.add_enabled(
                                    enabled,
                                    // Stays red while a confirmation is checked.
                                    consequence::danger_button("Hard reset", ready && !blocked),
                                )
                            } else {
                                ui.add_enabled(
                                    enabled,
                                    egui::Button::new((
                                        egui::RichText::new("Reset").color(egui::Color32::WHITE),
                                        egui::RichText::new("Enter")
                                            .small()
                                            .color(egui::Color32::from_rgb(184, 212, 230)),
                                    ))
                                    .fill(egui::Color32::from_rgb(11, 85, 119)),
                                )
                            };
                            let submit = match notice {
                                Some(reason) if blocked => submit.on_disabled_hover_text(reason),
                                _ => submit,
                            };
                            if submit.clicked() {
                                reset_confirmed = true;
                            }
                            if ui
                                .button(("Cancel", egui::RichText::new("Esc").small().weak()))
                                .clicked()
                            {
                                reset_cancel = true;
                            }
                        });
                    });
                });
            for (slot, lines) in RESET_SLOTS
                .into_iter()
                .zip([leaving, head, lost, overwritten])
            {
                if let Some(lines) = lines {
                    self.consequences.insert(slot, lines);
                }
            }
            if reset_confirmed {
                guard.confirm(&self.consequences, &RESET_SLOTS, ctx);
            }
            self.guards.insert("reset", guard);
        }
        if let (Some(mode), Some((_, current))) = (reset_mode, &mut self.reset_confirm) {
            *current = mode;
            // A different mode shows different lists; confirm them anew.
            self.guards.remove("reset");
        }
        if reset_cancel {
            self.reset_confirm = None;
        }
        if let Some((commit, mode)) = reset_submit {
            self.reset_confirm = None;
            if let Err(error) = self.start_reset(ctx, &commit, mode) {
                self.mutation_error = Some(error);
            }
        }

        if self.help_shortcuts {
            let ready = self.frame_readiness();
            let groups = shortcuts::groups(&self.menu_table(&ready));
            match shortcuts::show(ctx, &mut self.shortcuts_filter, &groups) {
                Some(shortcuts::Act::Close) => self.help_shortcuts = false,
                Some(shortcuts::Act::EditKeys) => {
                    self.help_shortcuts = false;
                    self.open_preferences(PrefSection::Keyboard);
                }
                None => {}
            }
        }

        let mut merge_submit = None;
        let mut merge_open = true;
        if let Some(dialog) = &mut self.merge_dialog {
            let mut cancelled = false;
            egui::Window::new("Merge branch")
                .id(egui::Id::unique("merge-branch")).modal(ctx, egui::Id::unique("merge-branch"))
                .open(&mut merge_open)
                .collapsible(false)
                .resizable(false)
                .default_width(420.0)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label("Merge a local branch, remote-tracking ref, or tag into the current branch.");
                    ui.add(
                        egui::TextEdit::singleline(&mut dialog.reference)
                            .hint_text("origin/main or feature")
                            .desired_width(f32::INFINITY),
                    );
                    ui.checkbox(&mut dialog.no_ff, "Always create a merge commit");
                    if let Some(error) = &dialog.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !dialog.reference.trim().is_empty(),
                                egui::Button::new("Merge"),
                            )
                            .clicked()
                        {
                            merge_submit = Some((dialog.reference.trim().to_string(), dialog.no_ff));
                        }
                        if ui.button("Cancel").clicked() {
                            cancelled = true;
                        }
                    });
                });
            if cancelled {
                merge_open = false;
            }
        }
        if let Some((reference, no_ff)) = merge_submit {
            self.remember(|prefs| prefs.merge_no_ff = no_ff);
            match self.start_merge(ctx, &reference, no_ff) {
                Ok(()) => self.merge_dialog = None,
                Err(error) => {
                    if let Some(dialog) = &mut self.merge_dialog {
                        dialog.error = Some(error);
                    }
                }
            }
        } else if self.merge_dialog.is_some() && !merge_open {
            self.merge_dialog = None;
        }

        let mut rebase_submit = None;
        let mut rebase_open = true;
        if let Some(dialog) = &mut self.rebase_dialog {
            let mut cancelled = false;
            egui::Window::new("Rebase branch")
                .id(egui::Id::unique("rebase-branch")).modal(ctx, egui::Id::unique("rebase-branch"))
                .open(&mut rebase_open)
                .collapsible(false)
                .resizable(false)
                .default_width(430.0)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(
                        "Rebase the current branch onto a local branch, remote-tracking ref, or tag.",
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut dialog.reference)
                            .hint_text("origin/main, main, or v1.0")
                            .desired_width(f32::INFINITY),
                    );
                    ui.checkbox(
                        &mut dialog.autostash,
                        "Automatically stash working changes",
                    );
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        "Rebase rewrites commits. A private backup ref is created before Git starts.",
                    );
                    if let Some(error) = &dialog.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !dialog.reference.trim().is_empty(),
                                egui::Button::new("Rebase"),
                            )
                            .clicked()
                        {
                            rebase_submit =
                                Some((dialog.reference.trim().to_string(), dialog.autostash));
                        }
                        if ui.button("Cancel").clicked() {
                            cancelled = true;
                        }
                    });
                });
            if cancelled {
                rebase_open = false;
            }
        }
        if let Some((reference, autostash)) = rebase_submit {
            match self.start_rebase(ctx, &reference, autostash) {
                Ok(()) => self.rebase_dialog = None,
                Err(error) => {
                    if let Some(dialog) = &mut self.rebase_dialog {
                        dialog.error = Some(error);
                    }
                }
            }
        } else if self.rebase_dialog.is_some() && !rebase_open {
            self.rebase_dialog = None;
        }

        let rewrite_busy = self.rewrite_job.is_some();
        let rewrite_refreshing = self.rewrite_refresh.is_some();
        let mut rewrite_submit = None;
        let mut rewrite_cancel = false;
        let mut rewrite_review = None;
        if self.review.is_none()
            && let Some(dialog) = &mut self.rewrite_dialog
        {
            // Review closed by any route returns to the held plan.
            dialog.held = false;
        }
        if let Some(dialog) = &mut self.rewrite_dialog {
            let blocked = if rewrite_busy {
                Some("A rewrite is already running.")
            } else if rewrite_refreshing {
                Some("Wait for history to refresh after the last rewrite.")
            } else {
                None
            };
            match dialog.show(ctx, blocked) {
                Some(rebase_dialog::Outcome::Start {
                    from,
                    steps,
                    autostash,
                }) => rewrite_submit = Some((from, steps, autostash)),
                Some(rebase_dialog::Outcome::Cancel) => rewrite_cancel = true,
                Some(rebase_dialog::Outcome::OpenReview { commit, subject }) => {
                    dialog.held = true;
                    rewrite_review = Some((commit, subject));
                }
                None => {}
            }
        }
        if let Some((commit, subject)) = rewrite_review {
            self.open_review_at(
                ctx,
                commit,
                subject,
                Some(return_location::ReturnLocation::Rebase),
            );
        }
        if let Some((from, steps, autostash)) = rewrite_submit {
            match self.begin_rewrite(ctx, from, steps, autostash) {
                Ok(()) => self.rewrite_dialog = None,
                Err(error) => {
                    if let Some(dialog) = &mut self.rewrite_dialog {
                        dialog.error = Some(error);
                    }
                }
            }
        }
        if rewrite_cancel {
            self.rewrite_dialog = None;
        }

        let mut stash_submit = None;
        let mut stash_open = true;
        // What will be stashed, counted in the background (proposal).
        let mut stash_files = match self
            .stash_dialog
            .as_ref()
            .map(|dialog| dialog.include_untracked)
        {
            Some(untracked) => self.take_consequence(
                "stash",
                &[
                    "status",
                    "--porcelain=v1",
                    if untracked { "-unormal" } else { "-uno" },
                ],
                ctx,
            ),
            None => {
                self.consequences.remove("stash");
                None
            }
        };
        let stash_summary = stash_files.as_mut().and_then(|files| match files.get() {
            Some(Ok(lines)) => {
                let (mut staged, mut unstaged, mut untracked) = (0, 0, 0);
                for line in lines {
                    let mut chars = line.chars();
                    let (index, tree) = (chars.next().unwrap_or(' '), chars.next().unwrap_or(' '));
                    if index == '?' {
                        untracked += 1;
                    } else {
                        staged += usize::from(index != ' ');
                        unstaged += usize::from(tree != ' ');
                    }
                }
                Some((lines.len(), staged, unstaged, untracked))
            }
            _ => None,
        });
        if let Some(files) = stash_files.take() {
            self.consequences.insert("stash", files);
        }
        if let Some(dialog) = &mut self.stash_dialog {
            let mut cancelled = false;
            egui::Window::new("Stash changes")
                .id(egui::Id::unique("stash-changes"))
                .modal(ctx, egui::Id::unique("stash-changes"))
                .open(&mut stash_open)
                .collapsible(false)
                .resizable(false)
                .default_width(420.0)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label("Record working changes in a new stash entry.");
                    ui.add(
                        egui::TextEdit::singleline(&mut dialog.message)
                            .hint_text("Message (optional)")
                            .desired_width(f32::INFINITY),
                    );
                    ui.checkbox(&mut dialog.include_untracked, "Include untracked files");
                    ui.checkbox(&mut dialog.keep_index, "Keep staged changes in the index");
                    // One fixed line: what will be stashed.
                    let summary = match stash_summary {
                        Some((files, staged, unstaged, untracked)) => {
                            let mut parts = Vec::new();
                            if staged > 0 {
                                parts.push(format!("{staged} staged"));
                            }
                            if unstaged > 0 {
                                parts.push(format!("{unstaged} unstaged"));
                            }
                            if untracked > 0 {
                                parts.push(format!("{untracked} untracked"));
                            }
                            format!(
                                "{}{}",
                                consequence::counted(files, "file", "files"),
                                if parts.is_empty() {
                                    String::new()
                                } else {
                                    format!(" · {}", parts.join(", "))
                                }
                            )
                        }
                        None => String::new(),
                    };
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 18.0),
                        egui::Sense::hover(),
                    );
                    bounded_text::paint(
                        ui,
                        rect,
                        &summary,
                        egui::FontId::proportional(12.0),
                        ui.visuals().weak_text_color(),
                    );
                    if let Some(error) = &dialog.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let files = stash_summary.map(|(files, ..)| files);
                            let ready = files != Some(0);
                            let label = match files {
                                Some(files) => format!(
                                    "Stash {}",
                                    consequence::counted(files, "file", "files")
                                ),
                                None => "Stash".into(),
                            };
                            let primary = egui::Button::new((
                                egui::RichText::new(label).color(egui::Color32::WHITE),
                                egui::RichText::new("Enter")
                                    .small()
                                    .color(egui::Color32::from_rgb(184, 212, 230)),
                            ))
                            .fill(egui::Color32::from_rgb(11, 85, 119));
                            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if ui.add_enabled(ready, primary).clicked() || (ready && enter) {
                                stash_submit = Some((
                                    dialog.message.clone(),
                                    dialog.include_untracked,
                                    dialog.keep_index,
                                ));
                            }
                            if ui
                                .button(("Cancel", egui::RichText::new("Esc").small().weak()))
                                .clicked()
                            {
                                cancelled = true;
                            }
                        });
                    });
                });
            if cancelled {
                stash_open = false;
            }
        }
        if let Some((message, include_untracked, keep_index)) = stash_submit {
            self.remember(|prefs| {
                prefs.stash_include_untracked = include_untracked;
                prefs.stash_keep_index = keep_index;
            });
            match self.push_stash(ctx, &message, include_untracked, keep_index) {
                Ok(()) => self.stash_dialog = None,
                Err(error) => {
                    if let Some(dialog) = &mut self.stash_dialog {
                        dialog.error = Some(error);
                    }
                }
            }
        } else if self.stash_dialog.is_some() && !stash_open {
            self.stash_dialog = None;
        }

        self.draw_submodule(ctx);
        self.draw_lfs(ctx);
        self.draw_signature(ctx);
        self.draw_tag_create(ctx);
        self.draw_force_push(ctx);
        self.draw_delete_remote(ctx);
        if self.conflict_view.is_some() {
            // The workspace's own footer: its keys, as in the conflict mockup.
            // The workspace's own footer: its keys and what Abort restores.
            let abort = match self.mutation.as_ref().map(|state| state.operation) {
                Some(gg_engine::mutation::Operation::Rebase) => match &self.rewrite_backup {
                    Some(backup) => format!("Abort restores {backup}"),
                    None => "Abort restores the branch as it was before the rebase".into(),
                },
                Some(operation) => {
                    operation_band::abort_note(operation, &self.current_branch_name())
                }
                None => String::new(),
            };
            egui::Panel::bottom("conflict-status")
                .exact_size(22.0)
                .show(ui, |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing.x = 14.0;
                        let hints = conflict_workspace::FOOTER_HINTS;
                        let abort_width = ui.fonts_mut(|fonts| {
                            fonts
                                .layout_no_wrap(
                                    abort.clone(),
                                    egui::FontId::proportional(11.0),
                                    egui::Color32::WHITE,
                                )
                                .size()
                                .x
                        });
                        for hint in hints {
                            if ui.available_width() < abort_width + 40.0 {
                                break;
                            }
                            ui.label(egui::RichText::new(hint).size(11.0).weak());
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add(
                                egui::Label::new(egui::RichText::new(&abort).size(11.0).weak())
                                    .truncate(),
                            );
                        });
                    });
                });
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(ui.visuals().panel_fill))
                .show(ui, |ui| self.draw_conflict_workspace(ui));
            return;
        }
        if self.working.is_some() {
            let external = self.external_operation_reason();
            let made = match self.rebase_progress().map(|progress| &progress.stop) {
                Some(gg_engine::mutation::RebaseStop::Reopened { made, .. }) => made
                    .iter()
                    .map(|step| (step.commit.clone(), step.subject.clone()))
                    .collect(),
                _ => Vec::new(),
            };
            let head = self
                .state
                .refs
                .head
                .as_ref()
                .and_then(|head| head.commit.clone())
                .map(|id| {
                    let subject = self
                        .state
                        .rows
                        .values()
                        .find(|commit| commit.id == id)
                        .map(|commit| commit.subject().to_owned())
                        .unwrap_or_default();
                    (id, subject)
                });
            let tracking = self.state.refs.current_tracking();
            let clean = status_view::CleanContext {
                head,
                ahead: tracking.and_then(|t| t.ahead).unwrap_or(0),
                behind: tracking.and_then(|t| t.behind).unwrap_or(0),
            };
            let reopened = matches!(
                self.rebase_progress().map(|progress| &progress.stop),
                Some(gg_engine::mutation::RebaseStop::Reopened { .. })
            );
            let backup = self.rewrite_backup.clone();
            if let Some(working) = &mut self.working {
                working.set_stop_commits(made);
                working.set_clean_context(clean);
                working.set_reopened(reopened, backup);
            }
            let close = self.working.as_mut().is_some_and(|working| {
                working.set_external_busy(external);
                working.draw_ui(ui)
            });
            match self
                .working
                .as_mut()
                .and_then(|working| working.take_clean_request())
            {
                Some(status_view::CleanAction::Pull) => {
                    let (mut local, mut history) = (false, false);
                    self.run_frame_command(ctx, FrameCommand::Pull, &mut local, &mut history);
                }
                Some(status_view::CleanAction::Push) => {
                    let (mut local, mut history) = (false, false);
                    self.run_frame_command(ctx, FrameCommand::Push, &mut local, &mut history);
                }
                None => {}
            }
            if let Some(request) = self
                .working
                .as_mut()
                .and_then(|working| working.take_submodule_request())
            {
                self.open_submodule(ctx, request);
            }
            if let Some(request) = self
                .working
                .as_mut()
                .and_then(|working| working.take_lfs_request())
            {
                self.open_lfs(ctx, request);
            }
            if let Some(path) = self
                .working
                .as_mut()
                .and_then(|working| working.take_editor_request())
            {
                self.open_in_editor(&path);
            }
            if let Some(receipt) = self
                .working
                .as_mut()
                .and_then(|working| working.take_export_receipt())
            {
                self.set_outcome(OutcomeSeverity::Info, receipt, None, None);
            }
            if let Some(stage) = self
                .working
                .as_mut()
                .and_then(|working| working.take_group_request())
            {
                if stage {
                    self.change_all(ctx, gg_git::worktree_changes::stage_all);
                } else {
                    self.change_all(ctx, gg_git::worktree_changes::unstage_all);
                }
            }
            let committed = self
                .working
                .as_mut()
                .and_then(|working| working.take_commit());
            if let Some(id) = committed {
                self.pending_commit = Some(id);
                // A commit at a rebase stop changes what remains to continue.
                self.refresh_local_summary(ctx);
                self.refresh_mutation();
            }
            if close {
                self.close_to_history(ctx);
            }
            return;
        }
        if self.review.is_some() {
            let close = self
                .review
                .as_mut()
                .is_some_and(|review| review.draw_ui(ui));
            if let Some(request) = self
                .review
                .as_mut()
                .and_then(|review| review.take_submodule_request())
            {
                self.open_submodule(ctx, request);
            }
            if let Some(change) = self
                .review
                .as_mut()
                .and_then(|review| review.take_lfs_request())
            {
                self.open_lfs(ctx, change);
            }
            if let Some(path) = self
                .review
                .as_mut()
                .and_then(|review| review.take_editor_request())
            {
                self.open_in_editor(&path);
            }
            if let Some(receipt) = self
                .review
                .as_mut()
                .and_then(|review| review.take_export_receipt())
            {
                self.set_outcome(OutcomeSeverity::Info, receipt, None, None);
            }
            if close {
                let location = self
                    .review
                    .as_mut()
                    .and_then(|review| review.take_return_location());
                self.review = None;
                if let Some(location) = location {
                    self.restore_return_location(ctx, location);
                } else {
                    self.restore_offset = Some(self.scroll_offset);
                }
                ctx.request_repaint();
            } else if let Some((revision, subject)) = self
                .review
                .as_mut()
                .and_then(|review| review.take_open_commit())
            {
                let location = self
                    .review
                    .as_ref()
                    .and_then(|review| review.return_location().cloned());
                self.review = None;
                self.open_review_at(ctx, revision, subject, location);
                ctx.request_repaint();
            }
            return;
        }

        // Receipts fade back to the key hints after 8 s.
        self.track_notice();
        if let Some(outcome) = &self.outcome
            && outcome.severity == OutcomeSeverity::Info
        {
            let elapsed = outcome.at.elapsed();
            if elapsed >= SUCCESS_FADE {
                self.outcome = None;
            } else {
                ctx.request_repaint_after(SUCCESS_FADE - elapsed);
            }
        }
        if let Some((_, at)) = &self.notice_seen
            && at.elapsed() < SUCCESS_FADE
        {
            ctx.request_repaint_after(SUCCESS_FADE - at.elapsed());
        }
        let message = self.footer_message(narrow);
        let mut act = None;
        egui::Panel::bottom("status")
            .exact_size(25.0)
            .show(ui, |ui| {
                act = footer::show(ui, &message);
            });
        match act {
            Some(footer::Act::Cancel) => {
                if let Some(worker) = &self.network {
                    worker.cancel();
                }
            }
            Some(footer::Act::Details) => self.outcome_details_open = true,
            Some(footer::Act::Recovery(action)) => self.run_recovery_action(ctx, action),
            Some(footer::Act::Dismiss) => self.dismiss_footer(),
            None => {}
        }
        if self.outcome_details_open {
            let shown = self.outcome.as_ref().and_then(|outcome| {
                outcome
                    .details
                    .clone()
                    .map(|details| (outcome.summary.clone(), details, outcome.recovery))
            });
            if let Some((summary, details, recovery)) = shown {
                match outcome_details::show(ctx, &summary, &details, recovery.map(recovery_label)) {
                    Some(outcome_details::Act::Close) => self.outcome_details_open = false,
                    Some(outcome_details::Act::Recovery) => {
                        self.outcome_details_open = false;
                        if let Some(action) = recovery {
                            self.run_recovery_action(ctx, action);
                        }
                    }
                    None => {}
                }
            } else {
                self.outcome_details_open = false;
            }
        }
        self.details_rect = egui::Rect::NOTHING;
        // Narrow windows keep the inspector under the list with a Details
        // button; Space replaces the list with the full details.
        let narrow_inspector = narrow && !self.narrow_details;
        if ((!narrow && self.details_visible) || narrow_inspector) && self.state.history.is_some() {
            // The commit inspector sits under the history list, as in the
            // History mockup; the refs sidebar keeps the full height.
            let mut open_details = false;
            let details = egui::Panel::bottom("details")
                .exact_size(details_view::INSPECTOR_HEIGHT)
                .resizable(false)
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    let stats = self.selected_commit_stats(ctx).map(|stats| stats.label());
                    let commit = self.state.displayed_details();
                    let menu = self.commit_menu_entries();
                    let command = details_view::inspector(
                        ui,
                        commit,
                        stats.as_deref(),
                        &mut self.avatars,
                        narrow.then_some(&mut open_details),
                        |ui| commit.and_then(|_| commit_action_menu(ui, &menu)),
                    );
                    if open_details {
                        self.narrow_details = true;
                    }
                    if let Some(command) = command
                        && let Some(commit) = commit
                    {
                        let id = commit.id.clone();
                        let subject = commit.subject().to_owned();
                        self.apply_commit_command(ctx, &id, &subject, command);
                    }
                });
            self.details_rect = details.response.rect;
        }
        let mut pending_commit_command: Option<(String, String, CommitCommand)> = None;
        // A double-clicked branch pill: its name and whether it is remote.
        let mut pill_switch: Option<(String, bool)> = None;
        let mut right_clicked_row: Option<usize> = None;
        let mut welcome_command: Option<welcome_view::Command> = None;
        egui::CentralPanel::default().show(ui, |ui| {
            if narrow && self.narrow_details {
                self.history_rect = egui::Rect::NOTHING;
                self.details_rect = ui.available_rect_before_wrap();
                let stats = self.selected_commit_stats(ctx);
                let commit = self.state.displayed_details();
                let mut refs: Vec<gg_engine::refs::RefItem> = commit
                    .map(|commit| self.state.refs.for_commit(&commit.id).cloned().collect())
                    .unwrap_or_default();
                let head_name = self.state.refs.head.as_ref().and_then(|h| h.name.clone());
                history_view::sort_labels(&mut refs, head_name.as_deref());
                let info = details_view::PageInfo {
                    loading: self.state.details.is_none(),
                    refs: &refs,
                    stats: stats.as_ref(),
                };
                let menu = self.commit_menu_entries();
                let action = details_view::page(ui, commit, &info, &mut self.details_file, |ui| {
                    commit.and_then(|_| commit_action_menu(ui, &menu))
                });
                let ids = commit.map(|commit| (commit.id.clone(), commit.subject().to_owned()));
                match action {
                    Some(details_view::PageAction::Back) => self.narrow_details = false,
                    Some(details_view::PageAction::Step(step)) => self.step_selection(ctx, step),
                    Some(details_view::PageAction::Review(path)) => {
                        self.review_from_details(ctx, path)
                    }
                    Some(details_view::PageAction::Command(command)) => {
                        if let Some((id, subject)) = ids {
                            self.apply_commit_command(ctx, &id, &subject, command);
                        }
                    }
                    None => {}
                }
                return;
            }
            if self.state.history.is_some() {
                // Empty states (proposal): blank while scanning; once the scan
                // finishes, either no commits yet or no filter matches.
                let empty = (self.state.count == 0 && self.state.finished)
                    .then_some(self.state.query.is_empty());
                self.history_rect = ui.available_rect_before_wrap();
                let row_menu = self.commit_menu_entries();
                // The columns scroll sideways only below their minimum width;
                // wider, the bar stays hidden so it never takes height and
                // moves the rows.
                let scroll_columns = ui.available_width() < history_view::MIN_WIDTH;
                egui::ScrollArea::horizontal()
                    .id_salt("history-columns")
                    .scroll_bar_visibility(if scroll_columns {
                        egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded
                    } else {
                        egui::scroll_area::ScrollBarVisibility::AlwaysHidden
                    })
                    .show(ui, |ui| {
                        ui.set_min_width(history_view::MIN_WIDTH);
                        let count = format!(
                            "{}{}{}",
                            self.state.count,
                            if self.state.finished { "" } else { "+" },
                            if self.rewrite_refresh.is_some() {
                                " · refreshing, showing previous history"
                            } else {
                                ""
                            }
                        );
                        if history_view::header(ui, &count, &self.state.query) {
                            self.state.query.clear();
                            self.filter_changed();
                        }
                        if let Some(unborn) = empty {
                            let changes = self
                                .local_summary
                                .as_ref()
                                .and_then(|summary| summary.changes)
                                .unwrap_or(0);
                            let branch = self.current_branch().unwrap_or_else(|| "HEAD".into());
                            let mut clear = false;
                            let mut go_changes = false;
                            ui.vertical_centered(|ui| {
                                ui.add_space((ui.available_height() * 0.3).max(24.0));
                                if unborn {
                                    ui.label(
                                        egui::RichText::new(format!("No commits yet on {branch}"))
                                            .size(18.0),
                                    );
                                    ui.add_space(8.0);
                                    if changes > 0 {
                                        go_changes = ui
                                            .button((
                                                "Go to Changes",
                                                egui::RichText::new("Ctrl+1").small().weak(),
                                            ))
                                            .clicked();
                                    } else {
                                        ui.weak(format!(
                                            "Create or copy files into {}, then commit them.",
                                            self.window_title.trim_start_matches("Git Good — ")
                                        ));
                                    }
                                } else {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "No commits match “{}”",
                                            self.state.query
                                        ))
                                        .size(18.0),
                                    );
                                    ui.weak("Searched messages, authors, and IDs.");
                                    ui.add_space(8.0);
                                    clear = ui
                                        .button((
                                            "Clear filter",
                                            egui::RichText::new("Esc").small().weak(),
                                        ))
                                        .clicked();
                                }
                            });
                            if clear {
                                self.state.query.clear();
                                self.filter_changed();
                            }
                            if go_changes {
                                self.open_working(ctx);
                            }
                            return;
                        }
                        let mut scroll = egui::ScrollArea::vertical().id_salt("history-rows");
                        if self.scroll_to_selection
                            && let Some(index) = self.state.selected_row
                        {
                            let height = ui.available_height();
                            let top = index as f32 * crate::density::row();
                            let bottom = top + crate::density::row();
                            let offset = self.scroll_offset;
                            let reveal = if top < offset {
                                top
                            } else if bottom > offset + height {
                                bottom - height
                            } else {
                                offset
                            };
                            let maximum =
                                (self.state.count as f32 * crate::density::row() - height).max(0.0);
                            scroll = scroll.vertical_scroll_offset(reveal.clamp(0.0, maximum));
                        }
                        if let Some(offset) = self.restore_offset.take() {
                            scroll = scroll.vertical_scroll_offset(offset);
                        }
                        // Scrolls, drags and keys are the user's own motion; the
                        // guard compares pointer positions itself.
                        let (now, moved, pressed, released) = ctx.input(|i| {
                            (
                                i.time,
                                i.smooth_scroll_delta != egui::Vec2::ZERO
                                    || i.pointer.is_decidedly_dragging()
                                    || i.events.iter().any(|event| {
                                        matches!(event, egui::Event::Key { pressed: true, .. })
                                    }),
                                i.pointer.primary_pressed(),
                                i.pointer.primary_released(),
                            )
                        });
                        let mut hovered = false;
                        let previous_spacing = ui.spacing().item_spacing.y;
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let scroll_output = scroll.show_rows(
                            ui,
                            crate::density::row(),
                            self.state.count,
                            |ui, rows| {
                                self.viewport =
                                    rows.start.saturating_sub(24)..rows.end.saturating_add(24);
                                self.row_rects.clear();
                                let head_name = self
                                    .state
                                    .refs
                                    .head
                                    .as_ref()
                                    .and_then(|head| head.name.clone());
                                for row in rows {
                                    let slot = egui::Rect::from_min_size(
                                        ui.cursor().min,
                                        egui::vec2(ui.available_width(), crate::density::row()),
                                    );
                                    let mut target = target_guard::Target::STILL;
                                    if let Some(point) = ui
                                        .ctx()
                                        .pointer_hover_pos()
                                        .filter(|_| ui.rect_contains_pointer(slot))
                                    {
                                        hovered = true;
                                        let key = self.state.rows.get(&row).map(|c| c.id.clone());
                                        target = self
                                            .history_guard
                                            .hover(now, &key, point, moved, pressed);
                                    }
                                    if let Some(commit) = self.state.rows.get(&row) {
                                        let mut refs = self
                                            .state
                                            .refs
                                            .for_commit(&commit.id)
                                            .cloned()
                                            .collect::<Vec<_>>();
                                        history_view::sort_labels(&mut refs, head_name.as_deref());
                                        if narrow {
                                            refs = history_view::merge_in_sync(
                                                refs,
                                                &self.state.refs.tracking,
                                            );
                                        }
                                        let graph = self.state.graphs.get(&row);
                                        let (response, ref_action) = ui
                                            .push_id(&commit.id, |ui| {
                                                history_view::row(
                                                    ui,
                                                    commit,
                                                    graph,
                                                    &refs,
                                                    head_name.as_deref(),
                                                    Some(&commit.id)
                                                        == self.state.selected.as_ref(),
                                                    &mut self.avatars,
                                                    target,
                                                )
                                            })
                                            .inner;
                                        self.row_rects.push((row, response.rect));
                                        // The menu's commands act on the selected
                                        // commit, so a right-click selects the row.
                                        if response.secondary_clicked() {
                                            right_clicked_row = Some(row);
                                        }
                                        response.context_menu(|ui| {
                                            let id = commit.id.clone();
                                            if let Some(command) = commit_action_menu(ui, &row_menu)
                                            {
                                                pending_commit_command = Some((
                                                    id.clone(),
                                                    commit.subject().to_owned(),
                                                    command,
                                                ));
                                                ui.close();
                                            }
                                        });
                                        if let Some(action) = ref_action {
                                            response.surrender_focus();
                                            self.state.select_row(row);
                                            if self.worker.is_none() {
                                                self.state.details =
                                                    self.state.rows.get(&row).cloned();
                                            }
                                            match action {
                                                history_view::RefAction::Select => {}
                                                history_view::RefAction::Switch(name) => {
                                                    pill_switch = Some((name, false));
                                                }
                                                history_view::RefAction::SwitchRemote(name) => {
                                                    pill_switch = Some((name, true));
                                                }
                                            }
                                        } else {
                                            if response.clicked() {
                                                response.surrender_focus();
                                                self.state.select_row(row);
                                                if self.worker.is_none() {
                                                    self.state.details =
                                                        self.state.rows.get(&row).cloned();
                                                }
                                            }
                                            if response.double_clicked() && narrow {
                                                self.narrow_details = true;
                                            }
                                        }
                                    } else {
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(ui.available_width(), crate::density::row()),
                                            egui::Sense::hover(),
                                        );
                                        // A row not read yet keeps its slot, empty.
                                        self.row_rects.push((row, rect));
                                    }
                                }
                            },
                        );
                        self.scroll_offset = scroll_output.state.clamped_offset().y;
                        self.page_rows = (scroll_output.inner_rect.height() / crate::density::row())
                            .floor()
                            .max(1.0) as usize;
                        ui.spacing_mut().item_spacing.y = previous_spacing;
                        if !hovered {
                            self.history_guard.leave();
                        } else if released {
                            self.history_guard.release();
                        }
                        if self.history_guard.animating(now) {
                            ctx.request_repaint();
                        }
                    });
            } else if !self.welcome_shown() {
                // The status line reports the read; the pane stays empty.
            } else {
                // The welcome view (proposal): logo and title, the
                // not-a-repository line, Open… / Clone… / New repository…
                // cards, and the recent repositories.
                self.welcome.read(ctx, &self.recent);
                let home = std::env::var("HOME").ok();
                let clone_folder = self.clone_start_folder();
                let shown = self.recent.len().min(settings::RECENT_LIMIT);
                let info = welcome_view::Info {
                    recent: &self.recent[..shown],
                    clone_folder: &clone_folder,
                    home: home.as_deref(),
                    now: settings::now(),
                    window_width: ctx.content_rect().width(),
                };
                welcome_command = self.welcome.show(ui, &info, &mut self.welcome_logo);
            }
        });
        if let Some(command) = welcome_command {
            self.run_welcome_command(ctx, command);
        }
        if let Some(row) = right_clicked_row {
            self.state.select_row(row);
            if self.worker.is_none() {
                self.state.details = self.state.rows.get(&row).cloned();
            }
        }
        if let Some((name, remote)) = pill_switch {
            ctx.request_repaint();
            if remote {
                self.switch_remote(ctx, name);
            } else {
                self.start_branch(ctx, name, false);
            }
        }
        if let Some((id, subject, command)) = pending_commit_command {
            self.apply_commit_command(ctx, &id, &subject, command);
        }
        if self.pending_filter_edit
            && let Some(pending) = &self.pending
            && !pending.request(stream::Request {
                revision: self.state.filter_revision,
                query: self.state.query.clone(),
                ..Default::default()
            })
        {
            ctx.request_repaint();
        }
        if self.pending.is_none()
            && let Some(worker) = &self.worker
            && !worker.request(stream::Request {
                revision: self.state.filter_revision,
                query: self.state.query.clone(),
                range: self.viewport.clone(),
                selected_id: self.state.selected.clone(),
                selected_row: self.state.selected_row,
                anchor_id: None,
            })
        {
            ctx.request_repaint();
        }
        bounded_text::show_disclosure(ctx);
        // gg's own title bar: the window's edges resize it, above everything.
        if self.chrome.custom {
            titlebar::resize_zones(ui);
        }
        if !self.deferred_input.is_empty() {
            ctx.request_repaint();
        }
        if self.network.is_none() && self.pending.is_none() {
            self.network_notice = None;
        }
        self.scroll_to_selection = false;
    }
}

impl eframe::App for Browser {
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        self.window_input(ctx, input);
        self.prepare_input(input);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        #[cfg(debug_assertions)]
        let ctx = ui.ctx().clone();
        self.draw_ui(ui);
        #[cfg(debug_assertions)]
        if let Some(path) = std::env::var_os("GG_SCREENSHOT") {
            let settle = std::env::var("GG_SCREENSHOT_DELAY_MS")
                .ok()
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or(500.0)
                / 1000.0;
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            if !self.screenshot_requested
                && self.pending.is_none()
                && ctx.input(|input| input.time) >= settle
            {
                self.screenshot_requested = true;
                let close = ctx.clone();
                ctx.request_screenshot(move |image| {
                    use std::io::Write;
                    let save = || -> std::io::Result<()> {
                        let mut file = std::fs::File::create(&path)?;
                        writeln!(file, "P6\n{} {}\n255", image.width(), image.height())?;
                        for pixel in &image.pixels {
                            file.write_all(&pixel.to_array()[..3])?;
                        }
                        Ok(())
                    };
                    if let Err(error) = save() {
                        eprintln!("Screenshot: {error}");
                    }
                    close.send_viewport_cmd(egui::ViewportCommand::Close);
                });
                ctx.request_repaint();
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn run_ui(
        ctx: &egui::Context,
        input: egui::RawInput,
        mut draw: impl FnMut(&mut egui::Ui),
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(input, |ui| draw(ui));
        output.textures_delta.clear();
        output
    }
    #[test]
    fn clone_destination_uses_the_repository_slug() {
        assert_eq!(
            gg_engine::sync::repository_name("https://example.org/team/project.git"),
            Some("project".into())
        );
        assert_eq!(
            gg_engine::sync::repository_name("git@example.org:team/project"),
            Some("project".into())
        );
    }

    #[test]
    fn clone_rejects_a_project_name_outside_the_root() {
        let mut dialog = CloneDialog::from_url(
            Some("https://example.org/team/project.git".into()),
            folders::starting_folder(),
        );
        assert_eq!(
            dialog.destination(),
            Some(folders::starting_folder().join("project"))
        );
        dialog.name = "../outside".into();
        assert!(dialog.destination().is_none());
        dialog.name = "project".into();
        dialog.folder = "relative".into();
        assert!(dialog.destination().is_none());
    }

    #[test]
    fn the_preferred_clone_folder_is_marked_and_kept_across_launches()
    -> Result<(), Box<dyn std::error::Error>> {
        let folder = tempfile::tempdir()?;
        let other = tempfile::tempdir()?;
        let config = tempfile::tempdir()?;
        let settings = config.path().join("roots.json");
        assert_eq!(
            clone_folder_label(folder.path(), folder.path()),
            format!("{} {}", phosphor::CHECK, folder.path().display())
        );
        assert_eq!(
            clone_folder_label(other.path(), folder.path()),
            other.path().to_string_lossy()
        );

        let mut app = layout_fixture();
        app.folders = folders::Roots::load_from(&settings);
        assert_eq!(app.clone_start_folder(), folders::starting_folder());
        app.folders.add(&other.path().to_string_lossy())?;
        app.folders.add(&folder.path().to_string_lossy())?;
        assert_eq!(app.clone_start_folder(), other.path());

        // Marking a saved folder makes it the default at every launch.
        app.clone_dialog = Some(CloneDialog::from_url(None, app.clone_start_folder()));
        assert_eq!(app.prefer_clone_folder(1), None);
        assert_eq!(app.clone_start_folder(), folder.path());
        assert_eq!(
            app.clone_dialog
                .as_ref()
                .map(|dialog| dialog.folder.clone()),
            Some(folder.path().to_string_lossy().into_owned())
        );
        let relaunched = folders::Roots::load_from(&settings);
        assert_eq!(relaunched.default_folder(), folder.path());

        // A folder picked in the clone dialog lasts for the session only.
        let picked = tempfile::tempdir()?;
        app.choose_clone_folder(picked.path().to_path_buf());
        assert_eq!(app.clone_start_folder(), picked.path());
        assert_eq!(
            folders::Roots::load_from(&settings).default_folder(),
            folder.path()
        );
        Ok(())
    }

    #[test]
    fn every_folder_field_offers_the_native_picker() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let rendered = |app: &mut Browser| {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        // Clone offers the picker as "Choose another folder…" in its one
        // folder control.
        app.clone_dialog = Some(CloneDialog::from_url(
            Some("https://example.org/gg.git".into()),
            folders::starting_folder(),
        ));
        assert!(rendered(&mut app).iter().any(|line| line == "INTO FOLDER"));
        app.clone_dialog = None;
        app.folders_dialog = Some(FoldersDialog::new(PrefSection::General));
        assert!(rendered(&mut app).iter().any(|line| line == "Browse…"));
        app.folders_dialog = None;
        app.edit_path = true;
        assert!(rendered(&mut app).iter().any(|line| line == "Browse…"));
        app.edit_path = false;
        app.worktree_window = Some(WorktreeWindow {
            open_created: None,
            snapshot: None,
            selected: None,
            error: None,
            notice: None,
            create: Some(CreateWorktreeDialog {
                destination: "/projects/next".into(),
                create: false,
                branch: String::new(),
                at: String::new(),
                error: None,
                destination_auto: false,
                open_after: true,
            }),
            confirm_remove: None,
            lock: None,
            busy: None,
            pending: None,
        });
        assert!(rendered(&mut app).iter().any(|line| line == "Browse…"));
    }

    #[test]
    fn the_clone_dialog_keeps_its_folder_controls_inside() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.clone_dialog = Some(CloneDialog::from_url(
            Some("https://example.org/team/project.git".into()),
            folders::starting_folder(),
        ));
        for _ in 0..3 {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
        }
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let window = ctx.memory(|memory| memory.area_rect(egui::Id::unique("clone-repository")));
        assert!(window.is_some(), "the clone dialog did not render");
        let name = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.job.text == "NAME" => {
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
            }
            _ => None,
        });
        assert!(name.is_some(), "the name field is missing");
        assert!(
            window.is_some_and(|window| name.is_some_and(|label| window.contains_rect(label))),
            "the folder and name controls are clipped: {name:?} in {window:?}"
        );
    }

    #[test]
    fn clone_enter_requires_a_focused_field() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let mut dialog = CloneDialog::from_url(
            Some("https://example.org/team/project.git".into()),
            folders::starting_folder(),
        );
        dialog.name = "../outside".into();
        app.clone_dialog = Some(dialog);
        crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ui| {
            app.draw_ui(ui)
        });
        crate::tests::run_ui(
            &ctx,
            frame_input(
                1120.0,
                740.0,
                vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
            ),
            |ui| app.draw_ui(ui),
        );
        assert!(app.network.is_none());
        assert!(app.clone_dialog.is_some());
    }
    #[test]
    fn native_graphics_backends_are_enabled() {
        assert!(!wgpu::Instance::enabled_backend_features().is_empty());
    }
    fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn resized_refs_and_details_keep_their_width_on_idle_frames() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let mut time = 1.0;
        let frame = |app: &mut Browser, time: &mut f64, events| {
            *time += 0.1;
            let mut input = frame_input(1600.0, 740.0, events);
            input.time = Some(*time);
            crate::tests::run_ui(&ctx, input, |ui| app.draw_ui(ui));
        };
        frame(&mut app, &mut time, vec![]);
        frame(&mut app, &mut time, vec![]);
        for left in [true, false] {
            let original = if left {
                app.refs_width
            } else {
                app.details_rect.width()
            };
            let start = egui::pos2(
                if left {
                    app.refs_width
                } else {
                    app.details_rect.left()
                },
                300.0,
            );
            let end = start + egui::vec2(if left { 60.0 } else { -60.0 }, 0.0);
            frame(&mut app, &mut time, vec![egui::Event::PointerMoved(start)]);
            frame(
                &mut app,
                &mut time,
                vec![egui::Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            frame(&mut app, &mut time, vec![egui::Event::PointerMoved(end)]);
            frame(
                &mut app,
                &mut time,
                vec![egui::Event::PointerButton {
                    pos: end,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            let resized = if left {
                app.refs_width
            } else {
                app.details_rect.width()
            };
            assert!(
                resized > original + 30.0,
                "divider did not resize: {original} -> {resized}"
            );
            let expected = (app.refs_width, app.details_rect, app.history_rect);
            for _ in 0..20 {
                frame(&mut app, &mut time, vec![]);
                assert_eq!(
                    (app.refs_width, app.details_rect, app.history_rect),
                    expected
                );
            }
        }
    }

    #[test]
    fn refs_sidebar_appears_only_when_there_is_room() {
        for (width, visible) in [(1119.0, false), (1120.0, true), (1600.0, true)] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                text.contains(&"BRANCHES"),
                visible,
                "width={width}: {text:?}"
            );
            assert!(app.history_rect.width() >= history_view::MIN_WIDTH);
        }
    }

    #[test]
    fn empty_history_explains_itself_only_after_the_scan() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.state.rows.clear();
        app.state.count = 0;
        app.state.finished = false;
        app.state.query = "zzz".into();
        let text = |app: &mut Browser| {
            let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            shown_text(&output)
        };
        let scanning = text(&mut app);
        assert!(
            !scanning
                .iter()
                .any(|line| line.contains("Searching") || line.contains("No commits")),
            "nothing while the scan runs: {scanning:?}"
        );
        app.state.finished = true;
        let filtered = text(&mut app);
        assert!(
            filtered
                .iter()
                .any(|line| line.contains("No commits match “zzz”"))
        );
        assert!(filtered.iter().any(|line| line == "Clear filter"));
        app.state.query.clear();
        let unborn = text(&mut app);
        assert!(
            unborn
                .iter()
                .any(|line| line.starts_with("No commits yet on")),
            "{unborn:?}"
        );
    }

    #[test]
    fn the_worktrees_window_rereads_when_gg_regains_focus() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.worktree_window = Some(WorktreeWindow {
            open_created: None,
            snapshot: None,
            selected: None,
            error: None,
            notice: None,
            create: None,
            confirm_remove: None,
            lock: None,
            busy: None,
            pending: None,
        });
        let input = |focused| egui::RawInput {
            focused,
            ..frame_input(1120.0, 740.0, vec![])
        };
        let _ = crate::tests::run_ui(&ctx, input(false), |ctx| app.draw_ui(ctx));
        let before = app.worktree_generation;
        let _ = crate::tests::run_ui(&ctx, input(true), |ctx| app.draw_ui(ctx));
        assert!(
            app.worktree_generation > before,
            "regaining focus requests a fresh worktree list"
        );
    }

    #[test]
    fn worktree_folders_default_to_a_sibling_named_after_the_branch() {
        let repo = Path::new("/home/ada/dev/gg");
        assert_eq!(
            worktree_default_destination(repo, "review/pr-42"),
            Path::new("/home/ada/dev")
                .join("gg-review-pr-42")
                .display()
                .to_string()
        );
        assert_eq!(worktree_default_destination(repo, "  "), "");
    }

    #[test]
    fn receipts_fade_after_eight_seconds() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let old = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(9))
            .unwrap_or_else(std::time::Instant::now);
        let outcome = |recovery| Outcome {
            severity: OutcomeSeverity::Info,
            summary: "Pushed".into(),
            details: None,
            recovery,
            at: old,
        };
        app.outcome = Some(outcome(None));
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.outcome.is_none(), "a plain success fades");
        app.outcome = Some(outcome(Some(RecoveryAction::Backups)));
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.outcome.is_none(), "a receipt with Recovery… fades too");
        // A fresh receipt stays, and so does a failure, however old.
        app.outcome = Some(Outcome {
            at: std::time::Instant::now(),
            ..outcome(None)
        });
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.outcome.is_some(), "a fresh receipt stays");
        app.outcome = Some(Outcome {
            severity: OutcomeSeverity::Error,
            ..outcome(None)
        });
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.outcome.is_some(), "a failure stays until dismissed");
        // A notice receipt fades the same way.
        app.outcome = None;
        app.network_notice = Some("Fetched origin".into());
        app.notice_seen = Some(("Fetched origin".into(), old));
        assert_eq!(app.footer_message(false).tone, footer::Tone::Idle);
        app.notice_seen = None;
        app.track_notice();
        let message = app.footer_message(false);
        assert_eq!(
            (message.tone, message.text.as_str()),
            (footer::Tone::Success, "Fetched origin")
        );
    }

    #[test]
    fn the_branch_and_its_tracking_show_only_on_the_branch_button() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            crate::phosphor::install(&ctx);
            let mut app = layout_fixture();
            app.state.refs.head = Some(gg_engine::refs::Head {
                name: Some("refs/heads/feature/a-long-branch-name".into()),
                detached: false,
                commit: Some(format!("{:040x}", 99)),
            });
            app.state.refs.tracking = vec![gg_engine::refs::Tracking {
                branch: "feature/a-long-branch-name".into(),
                full_name: "refs/heads/feature/a-long-branch-name".into(),
                upstream: "origin/feature/a-long-branch-name".into(),
                upstream_full_name: "refs/remotes/origin/feature/a-long-branch-name".into(),
                remote: "origin".into(),
                remote_branch: "feature/a-long-branch-name".into(),
                ahead: Some(2),
                behind: Some(1),
            }];
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(
                !text
                    .iter()
                    .any(|line| line.contains("origin/feature/a-long-branch-name")
                        || line.contains("tracking")
                        || line == &"Pull"),
                "width={width}: {text:?}"
            );
            let named = text
                .iter()
                .filter(|line| line.contains("feature/a-long-branch-name"))
                .count();
            assert!(named <= 1, "width={width}: {text:?}");
        }
    }

    #[test]
    fn status_hints_shorten_at_narrow_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let hint = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .find(|line| line.contains("Enter: review") && line.contains("Ctrl/Cmd+F: filter"));
            assert!(hint.is_some(), "missing status hint at {width}");
            if let Some(hint) = hint {
                if width < 900.0 {
                    assert!(hint.contains("j/k: select"), "{hint}");
                    assert!(!hint.contains("copy ID"), "{hint}");
                } else {
                    assert!(hint.contains("j k: select"), "{hint}");
                    assert!(hint.contains("R: rebase from here"), "{hint}");
                    assert!(hint.contains("copy ID"), "{hint}");
                }
            }
        }
    }

    #[test]
    fn clone_and_tracking_dialogs_render_at_supported_widths() {
        for width in [720.0, 1120.0] {
            for clone in [true, false] {
                let ctx = egui::Context::default();
                let mut app = layout_fixture();
                if clone {
                    app.clone_dialog = Some(CloneDialog::from_url(
                        Some("https://example.org/gg.git".into()),
                        folders::starting_folder(),
                    ));
                } else {
                    app.track_dialog = Some("origin/main".into());
                }
                let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                    app.draw_ui(ctx)
                });
                let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                    app.draw_ui(ctx)
                });
                let text = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if clone {
                    assert!(text.iter().any(|line| line.contains("Clone repository")));
                    assert!(text.iter().any(|line| line.contains("INTO FOLDER")));
                } else {
                    assert!(text.iter().any(|line| line.contains("Track remote branch")));
                    assert!(text.iter().any(|line| line.contains("Track branch")));
                }
            }
        }
    }

    #[test]
    fn preferences_list_sections_folders_and_save_changes_at_once()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let settings_file = dir.path().join("settings.json");
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            crate::phosphor::install(&ctx);
            let mut app = layout_fixture();
            app.prefs = settings::Prefs::load_from(&settings_file);
            app.folders = folders::Roots::load_from(&settings_file);
            let repos = dir.path().join("projects");
            std::fs::create_dir_all(repos.join("gg/.git"))?;
            app.folders.add(&repos.to_string_lossy())?;
            app.folders
                .add(&dir.path().join("gone").to_string_lossy())?;
            app.open_preferences(PrefSection::General);
            let frame = |app: &mut Browser, events| {
                crate::tests::run_ui(&ctx, frame_input(width, 740.0, events), |ctx| {
                    app.draw_ui(ctx)
                })
            };
            let _ = frame(&mut app, vec![]);
            let text = shown_text(&frame(&mut app, vec![]));
            for expected in [
                "Preferences",
                "General",
                "Appearance",
                "Rebase & recovery",
                "Keyboard",
                "Clone folders",
                "1 repository",
                "missing",
                "Reopen the last repository when launched outside one",
            ] {
                assert!(
                    text.iter().any(|line| line.contains(expected)),
                    "width {width}: missing {expected:?} in {text:?}"
                );
            }
            // Adding a folder already in the list names the reason.
            if let Some(dialog) = &mut app.folders_dialog {
                dialog.new_root = repos.to_string_lossy().into_owned();
            }
            let add = frame(&mut app, vec![])
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) if text.galley.job.text == "Add folder" => {
                        Some(text.visual_bounding_rect().center())
                    }
                    _ => None,
                })
                .ok_or("Add folder button missing")?;
            let _ = frame(&mut app, click_at(add));
            let _ = frame(&mut app, vec![]);
            assert_eq!(
                app.folders_dialog
                    .as_ref()
                    .and_then(|dialog| dialog.error.as_deref()),
                Some("Already in the list.")
            );

            // Appearance: choosing Light applies and saves immediately.
            app.open_preferences(PrefSection::Appearance);
            let light = frame(&mut app, vec![])
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) if text.galley.job.text == "Light" => {
                        Some(text.visual_bounding_rect().center())
                    }
                    _ => None,
                })
                .ok_or("Light theme option missing")?;
            let _ = frame(&mut app, click_at(light));
            assert_eq!(app.prefs.theme, settings::Theme::Light);
            assert_eq!(
                settings::Prefs::load_from(&settings_file).theme,
                settings::Theme::Light
            );
            assert!(app.folders_dialog.is_some(), "the window stays open");
            app.prefs.theme = settings::Theme::Dark;
            app.prefs.save()?;
        }
        Ok(())
    }

    #[test]
    fn dialogs_open_with_remembered_choices_and_save_them_on_submission()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let settings_file = dir.path().join("settings.json");
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.prefs = settings::Prefs::load_from(&settings_file);
        app.prefs.stash_include_untracked = true;
        app.prefs.stash_keep_index = true;
        app.prefs.merge_no_ff = true;
        app.prefs.branch_switch = false;
        // Remembered from an earlier run.
        app.prefs.save()?;

        app.open_stash_dialog();
        let stash = app.stash_dialog.as_ref().ok_or("stash dialog missing")?;
        assert!(stash.include_untracked && stash.keep_index);
        app.stash_dialog = None;
        app.open_new_branch();
        let branch = app.create_branch.as_ref().ok_or("branch dialog missing")?;
        assert!(
            !branch.switch,
            "switch after creation starts from the memory"
        );
        app.create_branch = None;

        // Opening alone saves nothing; submitting a changed choice does.
        app.open_merge_dialog();
        assert!(app.merge_dialog.as_ref().is_some_and(|dialog| dialog.no_ff));
        let frame = |app: &mut Browser, events| {
            crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, events), |ctx| {
                app.draw_ui(ctx)
            })
        };
        let _ = frame(&mut app, vec![]);
        assert_eq!(
            settings::read::<bool>(&settings_file, "merge_no_ff"),
            Some(true)
        );
        if let Some(dialog) = &mut app.merge_dialog {
            dialog.reference = "main".into();
            dialog.no_ff = false;
        }
        let output = frame(&mut app, vec![]);
        let merge = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == "Merge" => {
                    Some(text.visual_bounding_rect().center())
                }
                _ => None,
            })
            .ok_or("Merge button missing")?;
        let _ = frame(&mut app, click_at(merge));
        assert!(!app.prefs.merge_no_ff);
        assert_eq!(
            settings::read::<bool>(&settings_file, "merge_no_ff"),
            Some(false),
            "the submitted choice is saved"
        );
        assert!(
            settings::Prefs::load_from(&settings_file).stash_keep_index,
            "the other remembered values are saved with it"
        );
        Ok(())
    }

    #[test]
    fn preferences_show_diff_and_commit_sections() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let settings_file = dir.path().join("settings.json");
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.prefs = settings::Prefs::load_from(&settings_file);
        let frame = |app: &mut Browser| {
            crate::tests::run_ui(&ctx, frame_input(720.0, 520.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            })
        };
        app.open_preferences(PrefSection::Diff);
        let _ = frame(&mut app);
        let text = shown_text(&frame(&mut app));
        for expected in [
            "Diff",
            "Commit",
            "Open files in",
            "Full file",
            "Compare images",
            "Onion skin",
        ] {
            assert!(
                text.iter().any(|line| line == expected),
                "missing {expected:?} in {text:?}"
            );
        }
        app.open_preferences(PrefSection::Commit);
        let text = shown_text(&frame(&mut app));
        for expected in [
            "Sign commits",
            "Follow Git config (commit.gpgSign)",
            "Never",
        ] {
            assert!(
                text.iter().any(|line| line == expected),
                "missing {expected:?} in {text:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn preferences_commit_lists_the_signing_configuration_read_only() {
        let signing = SigningConfig {
            config: gg_engine::signature::Config {
                format: "ssh".into(),
                signing_key: Some("~/.ssh/id_ed25519.pub".into()),
                gpg_program: None,
                commit_signing: true,
                tag_signing: false,
                allowed_signers: Some("~/.config/git/allowed_signers".into()),
                tool_available: true,
                note: None,
            },
            file: Some(std::env::temp_dir().join("config")),
        };
        let ctx = egui::Context::default();
        let rows = |signing: Option<&SigningConfig>| {
            let mut height = 0.0;
            let output = crate::tests::run_ui(&ctx, frame_input(720.0, 480.0, vec![]), |ui| {
                let top = ui.cursor().top();
                let _ = signing_rows(ui, signing);
                height = ui.cursor().top() - top;
            });
            (shown_text(&output), height)
        };
        let (waiting, reserved) = rows(None);
        let (text, height) = rows(Some(&signing));
        for expected in [
            "Format",
            "ssh",
            "· gpg.format",
            "Signing key",
            "~/.ssh/id_ed25519.pub",
            "· user.signingkey",
            "Allowed signers",
            "~/.config/git/allowed_signers",
            "Read from Git config.",
            "Edit in repository config…",
        ] {
            assert!(
                text.iter().any(|line| line == expected),
                "missing {expected:?} in {text:?}"
            );
        }
        assert!(!waiting.iter().any(|line| line == "ssh"));
        assert_eq!(
            reserved, height,
            "the rows keep their place while Git is read"
        );
    }

    #[test]
    fn adding_a_clone_folder_does_not_move_the_dialog() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.folders_dialog = Some(FoldersDialog::new(PrefSection::General));
        app.folders = folders::Roots::default();
        let id = egui::Id::unique("preferences");
        let rect = |app: &mut Browser| {
            for _ in 0..3 {
                crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                    app.draw_ui(ctx)
                });
            }
            ctx.memory(|memory| memory.area_rect(id))
        };
        rect(&mut app);
        let empty = rect(&mut app);
        assert!(empty.is_some(), "the Preferences window did not render");
        let root = folders::starting_folder();
        assert!(app.folders.add(&root.to_string_lossy()).unwrap_or(false));
        let one = rect(&mut app);
        assert_eq!(one, empty, "the window moved after the first folder");
        assert!(
            app.folders
                .add(
                    &std::env::temp_dir()
                        .join("another-clone-folder")
                        .to_string_lossy()
                )
                .unwrap_or(false)
        );
        let two = rect(&mut app);
        assert_eq!(two, empty, "the window moved after the second folder");
        // Absolute on every platform: `/tmp/…` has no drive on Windows.
        let third = std::env::temp_dir().join("third-clone-folder");
        assert!(app.folders.add(&third.to_string_lossy()).unwrap_or(false));
        let three = rect(&mut app);
        assert_eq!(three, empty, "the window moved after the third folder");
    }

    #[test]
    fn stash_dialog_and_window_render_at_supported_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.stash_dialog = Some(StashDialog {
                message: "work in progress".into(),
                include_untracked: true,
                keep_index: false,
                error: None,
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(text.iter().any(|line| line.contains("Stash changes")));
            assert!(
                text.iter()
                    .any(|line| line.contains("Include untracked files"))
            );
        }
    }

    #[test]
    fn new_branch_without_a_commit_explains_the_unborn_head() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.state.refs.head = None;
        app.state.selected = None;
        app.open_new_branch();
        assert!(app.create_branch.is_none());
        assert_eq!(
            app.branch_error.as_deref(),
            Some("Create the first commit before creating a branch.")
        );
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text
                    .galley
                    .job
                    .text
                    .contains("Create the first commit before creating a branch.")
        )));
    }

    #[test]
    fn a_stash_shows_its_history_at_once_under_the_pointer()
    -> Result<(), Box<dyn std::error::Error>> {
        let git = |path: &std::path::Path, args: &[&str]| -> Result<(), String> {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(path)
                .args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env(
                    "GIT_CONFIG_GLOBAL",
                    if cfg!(windows) { "NUL" } else { "/dev/null" },
                )
                .output()
                .map_err(|error| error.to_string())?;
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            }
        };
        let dir = tempfile::tempdir()?;
        let path = std::fs::canonicalize(dir.path())?;
        git(&path, &["init", "-q", "-b", "main"])?;
        git(&path, &["config", "user.name", "Test"])?;
        git(&path, &["config", "user.email", "test@example.org"])?;
        git(&path, &["config", "core.autocrlf", "false"])?;
        std::fs::write(path.join("a"), b"base\n")?;
        git(&path, &["add", "a"])?;
        git(&path, &["commit", "-qm", "base"])?;

        let ctx = egui::Context::default();
        let mut app = Browser::new(path.clone());
        // The Stash dialog sits over History, so the pointer stays there.
        let over_history = egui::pos2(600.0, 300.0);
        let frame = |app: &mut Browser| {
            let _ = crate::tests::run_ui(
                &ctx,
                frame_input(1120.0, 740.0, vec![egui::Event::PointerMoved(over_history)]),
                |ui| app.draw_ui(ui),
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        app.load(&ctx);
        for _ in 0..400 {
            frame(&mut app);
            if app.pending.is_none() && app.state.history.is_some() {
                break;
            }
        }
        assert!(app.pending.is_none(), "the repository did not open");
        assert!(app.history_rect.contains(over_history));

        std::fs::write(path.join("a"), b"changed\n")?;
        app.push_stash(&ctx, "work", false, false)?;
        for _ in 0..400 {
            frame(&mut app);
            if app.pending.is_none() {
                break;
            }
        }
        assert!(
            app.pending.is_none(),
            "the stash's history waited for the pointer to leave History"
        );
        Ok(())
    }

    #[test]
    fn stash_restore_reports_the_restore_action() -> Result<(), Box<dyn std::error::Error>> {
        use std::process::Command;
        let run = |path: &std::path::Path, args: &[&str]| -> Result<(), String> {
            let output = Command::new("git")
                .arg("-C")
                .arg(path)
                .args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env(
                    "GIT_CONFIG_GLOBAL",
                    if cfg!(windows) { "NUL" } else { "/dev/null" },
                )
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.org")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.org")
                .output()
                .map_err(|error| error.to_string())?;
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            }
        };
        let dir = tempfile::tempdir()?;
        let path = dir.path();
        run(path, &["init", "-q", "-b", "main"])?;
        run(path, &["config", "user.name", "Test"])?;
        run(path, &["config", "user.email", "test@example.org"])?;
        run(path, &["config", "core.autocrlf", "false"])?;
        std::fs::write(path.join("a"), b"base\n")?;
        run(path, &["add", "a"])?;
        run(path, &["commit", "-qm", "base"])?;
        std::fs::write(path.join("a"), b"changed\n")?;
        run(path, &["stash", "push", "-qm", "work"])?;

        let ctx = egui::Context::default();
        let mut app = Browser::new(path.to_path_buf());
        app.state.history = Some(gg_engine::History {
            path: path.to_path_buf(),
            head: "main".into(),
            commits: Vec::new(),
            truncated: false,
            shallow: false,
        });
        app.restore_stash_inner(&ctx, "stash@{0}", false, false)?;
        let outcome = app.outcome.as_ref().ok_or("missing restore outcome")?;
        assert_eq!(outcome.summary, "Restored stash@{0}");
        assert!(matches!(outcome.severity, OutcomeSeverity::Info));
        assert_eq!(std::fs::read(path.join("a"))?, b"changed\n");
        Ok(())
    }

    #[test]
    fn branch_and_tag_delete_dialogs_render_at_supported_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.branch_delete = Some(BranchDeleteDialog {
                name: "feature".into(),
                error: None,
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(text.iter().any(|line| line.contains("Delete branch")));
            assert!(text.iter().any(|line| line.contains("feature")));
            assert!(
                text.iter()
                    .any(|line| line.contains("NOT IN THE CURRENT BRANCH"))
            );
            assert!(text.iter().any(|line| line.starts_with("Delete feature")));

            app.branch_delete = None;
            app.tag_delete = Some(TagDeleteDialog {
                name: "v1".into(),
                error: None,
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(text.iter().any(|line| line.contains("Delete tag")));
            assert!(text.iter().any(|line| line.contains("v1")));
        }
    }

    #[test]
    fn reset_confirmation_renders_at_supported_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.reset_confirm = Some(("abc123".into(), gg_engine::reset::Mode::Hard));
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(text.iter().any(|line| line.contains("Reset here")));
            assert!(text.iter().any(|line| line == &"Hard reset"));
            assert!(text.iter().any(|line| line.contains("CHANGES LOST")));
            assert!(text.iter().any(|line| line.contains("Reset")));
            assert!(text.iter().any(|line| line.contains("Cancel")));
        }
    }

    #[test]
    fn conflict_view_renders_at_supported_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.conflict_view = Some(ConflictView {
                paths: vec!["a".into()],
                selected: 0,
                previous_selected: 0,
                keep: None,
                content: Some(gg_engine::conflict::Content {
                    path: "a".into(),
                    base: Some("base\n".into()),
                    ours: Some("ours\n".into()),
                    theirs: Some("theirs\n".into()),
                    binary: false,
                    truncated: false,
                }),
                error: None,
                loading: false,
                worker: None,
                snapshot: Some(gg_engine::conflict::Snapshot {
                    base: Some("1".repeat(40)),
                    ours: Some("2".repeat(40)),
                    theirs: Some("3".repeat(40)),
                    working: Some(b"<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> x\n".to_vec()),
                    working_hash: None,
                    working_note: None,
                }),
                preview: gg_engine::conflict::Resolution::Working,
                blocks: None,
                blocks_note: None,
                whole_file: false,
                resolved: Vec::new(),
                edited: false,
                confirm_replace: false,
                names: None,
                ..Default::default()
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(text.iter().any(|line| line.contains("CONFLICTED")));
            assert!(text.iter().any(|line| line.contains("RESOLVED, STAGED")));
            assert!(text.iter().any(|line| line.contains("Base")));
            assert!(text.iter().any(|line| line.contains("Ours")));
            assert!(text.iter().any(|line| line.contains("Theirs")));
            assert!(text.iter().any(|line| line.contains("Keep working file")));
            assert!(text.iter().any(|line| line.contains("Stage resolution")));
        }
    }

    #[test]
    fn merge_dialog_and_banner_render_at_supported_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.merge_dialog = Some(MergeDialog {
                reference: "origin/main".into(),
                no_ff: true,
                error: None,
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(text.iter().any(|line| line.contains("Merge branch")));
            assert!(
                text.iter()
                    .any(|line| line.contains("Always create a merge commit"))
            );

            app.merge_dialog = None;
            app.mutation = Some(gg_engine::mutation::OperationState {
                operation: gg_engine::mutation::Operation::Merge,
                commit: Some("abc123".into()),
                conflicts: vec!["a".into()],
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(
                text.iter()
                    .any(|line| line.starts_with("Merge conflict · merging")),
                "{text:?}"
            );
            assert!(text.iter().any(|line| line.contains("Continue")));
            assert!(text.iter().any(|line| line.contains("Abort and restore")));
            assert!(!text.iter().any(|line| line.contains("Skip")), "{text:?}");
        }
    }

    #[test]
    fn rebase_dialog_and_banner_render_at_supported_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.rebase_dialog = Some(RebaseDialog {
                reference: "origin/main".into(),
                autostash: true,
                error: None,
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(text.iter().any(|line| line.contains("Rebase branch")));
            assert!(
                text.iter()
                    .any(|line| line.contains("Automatically stash working changes"))
            );
            assert!(text.iter().any(|line| line.contains("private backup ref")));
            assert!(text.iter().any(|line| line.contains("origin/main")));
            assert!(text.iter().any(|line| line.contains("Rebase")));

            app.rebase_dialog = None;
            app.mutation = Some(gg_engine::mutation::OperationState {
                operation: gg_engine::mutation::Operation::Rebase,
                commit: Some("abc123".into()),
                conflicts: vec!["a".into()],
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(
                text.iter().any(|line| line.starts_with("Rebase conflict")),
                "{text:?}"
            );
            assert!(text.iter().any(|line| line.contains("Continue")));
            assert!(text.iter().any(|line| line.contains("Skip commit")));
            assert!(text.iter().any(|line| line.contains("Abort and restore")));
        }
    }

    fn rebase_fixture(app: &Browser) -> Result<rebase_dialog::Dialog, String> {
        let commit = |id: &str, subject: &str| gg_engine::rewrite::RangeCommit {
            id: id.repeat(40),
            subject: subject.into(),
            author: "Test".into(),
            date: "2 days ago".into(),
            message: subject.into(),
        };
        rebase_dialog::Dialog::new(
            "/missing-rebase-fixture".into(),
            vec![commit("a", "first change"), commit("b", "second change")],
            Some("c".repeat(40)),
            &app.state.refs,
        )
    }

    #[test]
    fn rebase_dialog_names_the_branch_and_the_refs_left_behind() -> Result<(), String> {
        let mut app = layout_fixture();
        let item = |name: &str, kind, commit: &str| gg_engine::refs::RefItem {
            name: name.into(),
            full_name: name.into(),
            kind,
            commit: commit.repeat(40),
        };
        app.state.refs = gg_engine::refs::Snapshot {
            head: Some(gg_engine::refs::Head {
                name: Some("refs/heads/feature".into()),
                detached: false,
                commit: Some("b".repeat(40)),
            }),
            items: vec![
                item("feature", gg_engine::refs::RefKind::Branch, "b"),
                item("keep", gg_engine::refs::RefKind::Branch, "a"),
                item("v1", gg_engine::refs::RefKind::Tag, "a"),
                item("origin/feature", gg_engine::refs::RefKind::Remote, "a"),
            ],
            ..Default::default()
        };
        let dialog = rebase_fixture(&app)?;
        assert_eq!(dialog.branch, "feature");
        assert_eq!(dialog.stay, ["keep", "v1"]);
        assert_eq!(dialog.plan.rows[0].refs, ["feature"]);
        assert_eq!(dialog.plan.rows[1].refs, ["keep", "v1"]);
        Ok(())
    }

    fn click_at(pos: egui::Pos2) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }

    fn shown_text(output: &egui::FullOutput) -> Vec<String> {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect()
    }

    fn reopened_stop(made: usize, tracked_changes: usize) -> gg_engine::mutation::RebaseProgress {
        let step = |action: &str, id: char, subject: &str| gg_engine::mutation::RebaseStep {
            action: action.into(),
            commit: id.to_string().repeat(40),
            subject: subject.into(),
        };
        gg_engine::mutation::RebaseProgress {
            branch: Some("feature".into()),
            done: vec![
                step("pick", 'a', "first change"),
                step("pick", 'b', "second change"),
            ],
            todo: vec![step("pick", 'c', "third change")],
            stop: gg_engine::mutation::RebaseStop::Reopened {
                base: Some("a".repeat(40)),
                commit: "b".repeat(40),
                subject: "second change".into(),
                message: "second change\n\nwith a body".into(),
                made: (0..made)
                    .map(|index| step("commit", 'd', &format!("part {index}")))
                    .collect(),
            },
            tracked_changes,
        }
    }

    fn stopped_app(progress: gg_engine::mutation::RebaseProgress) -> Browser {
        let mut app = layout_fixture();
        app.mutation = Some(gg_engine::mutation::OperationState {
            operation: gg_engine::mutation::Operation::Rebase,
            commit: None,
            conflicts: Vec::new(),
        });
        app.local_summary = Some(LocalSummary {
            worktrees: Vec::new(),
            rebase: Some(progress),
            ..Default::default()
        });
        app
    }

    #[test]
    fn reopened_commit_opens_local_changes_with_its_message_and_waits_to_continue() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            crate::phosphor::install(&ctx);
            let mut app = stopped_app(reopened_stop(0, 2));
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let working = app.working.as_ref();
            assert!(working.is_some(), "Local Changes opens at the stop");
            assert_eq!(
                working.map(|working| working.draft()),
                Some("second change\n\nwith a body"),
                "the commit box holds the reopened commit's message"
            );
            let text = shown_text(&output);
            for expected in [
                "Rebase stopped · step 2 of 3 · reopened bbbbbbbb",
                "second change",
                "Continue",
                "Skip commit",
                "Abort and restore",
                "Commit or stash the 2 remaining changed files first.",
                "cccccccc pick",
            ] {
                assert!(
                    text.iter().any(|line| line.contains(expected)),
                    "width {width}: missing {expected:?} in {text:?}"
                );
            }
            assert!(
                text.iter()
                    .any(|line| line.contains("rebasing feature · step 2 of 3")),
                "the window identity names the rebase step: {text:?}"
            );
            // Ctrl+Enter does nothing while changes remain.
            let _ = crate::tests::run_ui(
                &ctx,
                frame_input(
                    width,
                    740.0,
                    vec![key(egui::Key::Enter, egui::Modifiers::COMMAND)],
                ),
                |ctx| app.draw_ui(ctx),
            );
            assert!(app.operation_worker.is_none());
        }
    }

    #[test]
    fn panes_hold_still_while_a_rebase_stop_changes() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let top = |app: &mut Browser| {
            for _ in 0..2 {
                let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                    app.draw_ui(ctx)
                });
            }
            app.history_rect.top()
        };
        let mut idle = layout_fixture();
        let mut stopped = stopped_app(reopened_stop(0, 2));
        stopped.reopen_seen = Some("b".repeat(40));
        let mut committed = stopped_app(reopened_stop(3, 0));
        committed.reopen_seen = Some("b".repeat(40));
        let idle = top(&mut idle);
        let stopped = top(&mut stopped);
        assert!(
            idle < stopped,
            "without an operation there is no band under the toolbar"
        );
        assert_eq!(
            top(&mut committed),
            stopped,
            "commits made at the stop do not add a line"
        );
    }

    #[test]
    fn commits_made_at_a_stop_are_listed_and_continue_is_ready() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let mut app = stopped_app(reopened_stop(2, 0));
        app.reopen_seen = Some("b".repeat(40));
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(
            app.working.is_none(),
            "a stop already seen does not reopen Local Changes"
        );
        let text = shown_text(&output);
        assert!(
            text.iter()
                .any(|line| line.starts_with("Commits made at this stop:")
                    && line.contains("part 0")
                    && line.contains("part 1")),
            "{text:?}"
        );
        assert!(!text.iter().any(|line| line.contains("remaining changed")));
    }

    #[test]
    fn rewrite_dialog_renders_at_supported_widths() -> Result<(), String> {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.rewrite_dialog = Some(rebase_fixture(&app)?);
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = shown_text(&output);
            for expected in [
                "Interactive rebase",
                "Start rebase",
                "Cancel",
                "aaaaaaa…HEAD onto ccccccc",
                "2 commits",
                "PLAN · NEWEST FIRST",
                "first change",
                "second change",
                "Pick",
                "Open in Review",
                "Stash working changes first",
                "private backup ref",
            ] {
                assert!(
                    text.iter().any(|line| line.contains(expected)),
                    "width {width}: missing {expected:?} in {text:?}"
                );
            }
            let newest = text
                .iter()
                .position(|line| line == "second change")
                .ok_or("newest row missing")?;
            let oldest = text
                .iter()
                .position(|line| line == "first change")
                .ok_or("oldest row missing")?;
            assert!(newest < oldest, "the plan lists the newest commit first");
        }
        Ok(())
    }

    #[test]
    fn rebase_dialog_keys_choose_actions_and_start_only_when_valid() -> Result<(), String> {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.rewrite_dialog = Some(rebase_fixture(&app)?);
        let frame = |app: &mut Browser, events| {
            crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, events), |ctx| {
                app.draw_ui(ctx)
            })
        };
        let _ = frame(&mut app, vec![]);
        // The oldest commit has nothing below to combine into.
        let _ = frame(
            &mut app,
            vec![
                key(egui::Key::ArrowDown, egui::Modifiers::NONE),
                key(egui::Key::F, egui::Modifiers::NONE),
            ],
        );
        let dialog = app.rewrite_dialog.as_ref().ok_or("dialog closed")?;
        assert_eq!(dialog.selected, 1);
        assert_eq!(dialog.plan.rows[1].action, gg_engine::rewrite::Action::Pick);
        assert!(
            dialog
                .error
                .as_deref()
                .is_some_and(|error| error.contains("No kept commit below")),
            "{:?}",
            dialog.error
        );
        // The newest squashes into the oldest; Alt+Up then moves the oldest up.
        let _ = frame(
            &mut app,
            vec![
                key(egui::Key::K, egui::Modifiers::NONE),
                key(egui::Key::S, egui::Modifiers::NONE),
            ],
        );
        let dialog = app.rewrite_dialog.as_ref().ok_or("dialog closed")?;
        assert_eq!(
            dialog.plan.rows[0].action,
            gg_engine::rewrite::Action::Squash
        );
        assert_eq!(dialog.error, None);
        let output = frame(&mut app, vec![]);
        let text = shown_text(&output);
        assert!(
            text.iter().any(|line| line.contains("squash into aaaaaaa")),
            "{text:?}"
        );
        assert!(text.iter().any(|line| line.contains("absorbs bbbbbbb")));
        // Esc cancels without starting anything.
        let _ = frame(
            &mut app,
            vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
        );
        assert!(app.rewrite_dialog.is_none());
        assert!(app.rewrite_job.is_none());
        Ok(())
    }

    #[test]
    fn rebase_dialog_layout_holds_still_when_actions_change() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let mut app = layout_fixture();
        app.rewrite_dialog = Some(rebase_fixture(&app)?);
        let place = |app: &mut Browser, events: Vec<egui::Event>| {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, events), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let find = |wanted: &str| {
                output.shapes.iter().find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) if text.galley.job.text.contains(wanted) => {
                        Some(text.pos)
                    }
                    _ => None,
                })
            };
            (find("Changed files").or(find("FILES ")), find("…HEAD onto"))
        };
        let (reader, range) = place(&mut app, vec![]);
        assert!(reader.is_some() && range.is_some(), "{reader:?} {range:?}");
        // Drop changes the result count; Reword swaps the message for a field.
        let (after_drop, range_after_drop) =
            place(&mut app, vec![key(egui::Key::D, egui::Modifiers::NONE)]);
        assert_eq!(
            range_after_drop, range,
            "the range does not move with the count"
        );
        assert_eq!(after_drop, reader);
        let (after_reword, _) = place(&mut app, vec![key(egui::Key::R, egui::Modifiers::NONE)]);
        assert_eq!(
            after_reword, reader,
            "the reader does not move for the reword field"
        );
        Ok(())
    }

    #[test]
    fn commit_menu_presets_select_the_commit_in_the_rebase_plan() -> Result<(), String> {
        let app = layout_fixture();
        let mut dialog = rebase_fixture(&app)?;
        dialog.preset(&"a".repeat(40), gg_engine::rewrite::Action::Drop);
        assert_eq!(dialog.selected, 1, "the oldest row is selected");
        assert_eq!(dialog.plan.rows[1].action, gg_engine::rewrite::Action::Drop);
        dialog.preset(&"b".repeat(40), gg_engine::rewrite::Action::Fixup);
        assert_eq!(dialog.plan.rows[0].action, gg_engine::rewrite::Action::Pick);
        assert!(
            dialog
                .error
                .as_deref()
                .is_some_and(|error| error.contains("No kept commit below")),
            "a fixup with only a dropped commit below is refused: {:?}",
            dialog.error
        );
        Ok(())
    }

    #[test]
    fn rebase_dialog_keys_still_work_after_a_button_took_focus() -> Result<(), String> {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.rewrite_dialog = Some(rebase_fixture(&app)?);
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        // Clicking a button gives it keyboard focus; only text fields should block keys.
        ctx.memory_mut(|m| m.request_focus(egui::Id::unique("a-clicked-button")));
        let _ = crate::tests::run_ui(
            &ctx,
            frame_input(
                1120.0,
                740.0,
                vec![key(egui::Key::D, egui::Modifiers::NONE)],
            ),
            |ctx| app.draw_ui(ctx),
        );
        let dialog = app.rewrite_dialog.as_ref().ok_or("dialog closed")?;
        assert_eq!(dialog.plan.rows[0].action, gg_engine::rewrite::Action::Drop);
        Ok(())
    }

    #[test]
    fn rebase_dialog_opens_review_and_returns_to_the_same_plan() -> Result<(), String> {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.rewrite_dialog = Some(rebase_fixture(&app)?);
        let frame = |app: &mut Browser, events| {
            crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, events), |ctx| {
                app.draw_ui(ctx)
            })
        };
        let _ = frame(&mut app, vec![]);
        let _ = frame(&mut app, vec![key(egui::Key::D, egui::Modifiers::NONE)]);
        let _ = frame(&mut app, vec![key(egui::Key::Enter, egui::Modifiers::NONE)]);
        assert!(
            app.rewrite_dialog
                .as_ref()
                .is_some_and(|dialog| dialog.held)
        );
        assert!(
            app.review.is_some(),
            "Enter opens the selected commit in Review"
        );
        let output = frame(&mut app, vec![]);
        let text = shown_text(&output);
        assert!(text.iter().any(|line| line == "‹ Rebase"), "{text:?}");
        assert!(!text.iter().any(|line| line == "Start rebase"));
        // Esc in Review returns to the plan with the choice kept.
        let _ = frame(
            &mut app,
            vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
        );
        let _ = frame(&mut app, vec![]);
        assert!(app.review.is_none());
        let dialog = app.rewrite_dialog.as_ref().ok_or("dialog closed")?;
        assert!(!dialog.held);
        assert_eq!(dialog.plan.rows[0].action, gg_engine::rewrite::Action::Drop);
        Ok(())
    }

    #[test]
    fn rewrite_progress_lives_once_in_the_operation_area() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let (_sender, receiver) = std::sync::mpsc::channel();
        let live = std::sync::Arc::new(std::sync::Mutex::new(RewriteLive {
            backup: Some("refs/gg/backup/main/2026-10-30T10-52".into()),
            progress: None,
        }));
        app.rewrite_job = Some(RewriteJob {
            receiver,
            restore_row: None,
            live,
        });
        let step = |commit: char, state| operation_band::Step {
            commit: commit.to_string().repeat(40),
            action: "pick".into(),
            subject: String::new(),
            state,
        };
        app.rewrite_steps = (
            "main".into(),
            vec![
                step('a', operation_band::StepState::Current),
                step('b', operation_band::StepState::Next),
            ],
        );
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.rewrite_dialog.is_none(), "configuration is closed");
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            text.iter()
                .filter(|line| line.starts_with("Rewriting main · step 1 of 2 · picking aaaaaaaa"))
                .count(),
            1,
            "one operation band reports the rewrite: {text:?}"
        );
        assert!(
            text.contains(&"Backup: refs/gg/backup/main/2026-10-30T10-52"),
            "the footer names the backup while Git writes: {text:?}"
        );
        assert!(
            !text
                .iter()
                .any(|line| line.contains("Abort") || *line == "Cancel"),
            "no cancel while Git writes: {text:?}"
        );
        assert!(
            text.iter()
                .any(|line| line.contains("Browsing stays available"))
        );
        assert!(!text.iter().any(|line| line.contains("Start rebase")));
    }

    #[test]
    fn rewrite_refresh_clears_the_operation_area() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.rewrite_refresh = Some(app.state.loaded_generation + 1);
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            text.iter()
                .filter(|line| line.contains("Refreshing history"))
                .count(),
            1,
            "one operation area reports the refresh: {text:?}"
        );
        assert!(
            text.iter()
                .any(|line| line.contains("showing previous history")),
            "the retained history is labeled: {text:?}"
        );

        app.state.loaded_generation = app.rewrite_refresh.unwrap_or_default();
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.rewrite_refresh.is_none());
        assert!(!output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.job.text.contains("Refreshing history")
        )));
    }

    #[test]
    fn refresh_failure_keeps_backup_recovery_reachable() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.rewrite_refresh = Some(app.state.loaded_generation + 1);
        app.state.error = Some("simulated refresh failure".into());
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.mutation_error.is_none());
        assert!(app.state.error.is_none());
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.iter().any(|line| line.contains("Refresh failed")));
        assert!(text.iter().any(|line| line.contains("Recovery…")));
    }

    #[test]
    fn continuation_worker_label_replaces_the_mutation_strip() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.mutation = Some(gg_engine::mutation::OperationState {
            operation: gg_engine::mutation::Operation::Rebase,
            commit: Some("abc123".into()),
            conflicts: vec!["a".into()],
        });
        let (_tx, rx) = std::sync::mpsc::channel();
        app.operation_worker = Some(rx);
        app.mutation_notice = Some("Continuing rebase…".into());
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            text.iter().any(|line| line.contains("Continuing rebase")),
            "the area names the running continuation: {text:?}"
        );
        assert!(!text.iter().any(|line| line.contains("rebase in progress")));
    }

    #[test]
    fn rewrite_failure_keeps_backup_recovery_reachable() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let path = app.stash_path().unwrap_or_default();
        assert!(!path.as_os_str().is_empty());
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = tx.send(Err("simulated rewrite failure".into()));
        app.operation_repo = Some(path);
        app.rewrite_job = Some(RewriteJob {
            receiver: rx,
            restore_row: None,
            live: Default::default(),
        });
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.mutation_error.is_none());
        assert!(
            app.outcome
                .as_ref()
                .is_some_and(|outcome| outcome.severity == OutcomeSeverity::Error)
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            text.iter()
                .any(|line| line.contains("simulated rewrite failure"))
        );
        assert!(text.iter().any(|line| line.contains("Recovery…")));
    }

    #[test]
    fn repository_switch_discards_foreign_operation_state() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let (_tx, rx) = std::sync::mpsc::channel();
        app.operation_worker = Some(rx);
        app.operation_repo = Some("/old".into());
        app.mutation_notice = Some("Merging old repository…".into());
        app.outcome = Some(Outcome {
            at: std::time::Instant::now(),
            severity: OutcomeSeverity::Info,
            summary: "Merged old repository".into(),
            details: None,
            recovery: None,
        });
        app.rewrite_refresh = Some(5);
        app.mutation_path = Some("/old".into());
        assert!(app.state.history.is_some());
        if let Some(history) = app.state.history.as_mut() {
            history.path = "/new".into();
        }
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.operation_worker.is_none());
        assert!(app.operation_repo.is_none());
        assert!(app.rewrite_refresh.is_none());
        assert!(app.mutation_notice.is_none());
        assert!(
            app.outcome.is_none(),
            "the old repository outcome is cleared"
        );
    }

    #[test]
    fn operation_worker_keeps_the_ui_responsive_and_rejects_duplicates() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let (gate_tx, gate_rx) = std::sync::mpsc::channel();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        assert!(
            app.start_operation(&ctx, "Testing operation…".into(), move || {
                let _ = started_tx.send(());
                let _ = gate_rx.recv();
                Ok(OperationOutcome::Rebase(
                    gg_engine::rebase::Receipt {
                        reference: "main".into(),
                        commit: None,
                        previous: None,
                        backup: None,
                        applied: true,
                        in_progress: false,
                        conflicts: Vec::new(),
                        warnings: Vec::new(),
                    },
                    "feature".into(),
                ))
            })
            .is_ok()
        );
        assert!(app.operation_busy());
        assert!(
            started_rx
                .recv_timeout(std::time::Duration::from_secs(5))
                .is_ok()
        );
        assert!(
            app.start_operation(&ctx, "Second…".into(), || unreachable!())
                .is_err()
        );
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text) if text.galley.job.text.contains("Testing operation")
        )));
        assert!(gate_tx.send(()).is_ok());
        for _ in 0..200 {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            if !app.operation_busy() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(!app.operation_busy());
        assert!(
            app.mutation_notice
                .as_deref()
                .is_some_and(|notice| notice.starts_with("Rebased ") && notice.contains(" onto "))
        );
    }

    #[cfg(unix)]
    #[test]
    fn rebase_worker_keeps_frames_rendering_while_a_hook_waits()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir()?;
        let repo = dir.path();
        let git = |args: &[&str]| -> Result<(), Box<dyn std::error::Error>> {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(repo)
                .args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.org")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.org")
                .output()?;
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(())
        };
        git(&["init", "-q", "-b", "main"])?;
        git(&["config", "user.name", "Test"])?;
        git(&["config", "user.email", "test@example.org"])?;
        std::fs::write(repo.join("a"), b"base\n")?;
        git(&["add", "a"])?;
        git(&["commit", "-qm", "base"])?;
        git(&["checkout", "-qb", "feature"])?;
        std::fs::write(repo.join("b"), b"feature\n")?;
        git(&["add", "b"])?;
        git(&["commit", "-qm", "feature"])?;
        git(&["checkout", "-q", "-"])?;
        std::fs::write(repo.join("c"), b"mainline\n")?;
        git(&["add", "c"])?;
        git(&["commit", "-qm", "mainline"])?;

        let marker = repo.join("release-pre-rebase");
        let hook = repo.join(".git/hooks/pre-rebase");
        std::fs::write(
            &hook,
            format!(
                "#!/bin/sh\nwhile [ ! -f '{}' ]; do sleep 0.05; done\n",
                marker.display()
            ),
        )?;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;

        let ctx = egui::Context::default();
        let mut app = Browser::new(repo.to_path_buf());
        app.state.history = Some(History {
            path: repo.to_path_buf(),
            head: "main".into(),
            truncated: false,
            shallow: false,
            commits: Vec::new(),
        });
        assert!(app.start_rebase(&ctx, "feature", false).is_ok());
        assert!(app.operation_busy());
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.job.text.starts_with("Rebasing ")
                    && text.galley.job.text.contains("onto feature")
        )));
        std::fs::write(&marker, b"go")?;
        for _ in 0..400 {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            if !app.operation_busy() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(!app.operation_busy());
        assert!(
            app.mutation_notice.as_deref().is_some_and(
                |notice| notice.starts_with("Rebased ") && notice.ends_with(" onto feature")
            ),
            "notice={:?} error={:?}",
            app.mutation_notice,
            app.mutation_error
        );
        Ok(())
    }

    #[test]
    fn conflict_selection_prefers_the_next_path() {
        let paths = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(next_conflict_selection(&paths, Some("b"), 0), 1);
        assert_eq!(next_conflict_selection(&paths, Some("missing"), 2), 2);
        assert_eq!(next_conflict_selection(&[], None, 5), 0);
    }

    #[test]
    fn conflict_worker_delivers_content_without_blocking() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        app.conflict_view = Some(ConflictView {
            paths: vec!["a".into()],
            selected: 0,
            previous_selected: 0,
            keep: None,
            content: None,
            error: None,
            loading: true,
            worker: Some(rx),
            snapshot: None,
            preview: gg_engine::conflict::Resolution::Working,
            blocks: None,
            blocks_note: None,
            whole_file: false,
            resolved: Vec::new(),
            edited: false,
            confirm_replace: false,
            names: None,
            ..Default::default()
        });
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.conflict_view.as_ref().is_some_and(|view| view.loading));
        assert!(
            tx.send(Ok(ConflictUpdate::Content {
                content: Box::new(gg_engine::conflict::Content {
                    path: "a".into(),
                    base: Some("base\n".into()),
                    ours: Some("ours\n".into()),
                    theirs: Some("theirs\n".into()),
                    binary: false,
                    truncated: false,
                }),
                snapshot: Box::new(gg_engine::conflict::Snapshot {
                    base: Some("1".repeat(40)),
                    ours: Some("2".repeat(40)),
                    theirs: Some("3".repeat(40)),
                    working: Some(b"ours\n".to_vec()),
                    working_hash: None,
                    working_note: None,
                }),
                blocks: Err("whole file only".into()),
                edited: false,
                names: None,
            }))
            .is_ok()
        );
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text) if text.galley.job.text.contains("ours")
        )));
        assert!(
            app.conflict_view
                .as_ref()
                .is_some_and(|view| !view.loading && view.content.is_some())
        );
    }

    #[test]
    fn conflict_preview_is_read_only_until_stage_resolution()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::process::Command;
        let run = |path: &std::path::Path, args: &[&str]| -> Result<(), String> {
            let output = Command::new("git")
                .arg("-C")
                .arg(path)
                .args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env(
                    "GIT_CONFIG_GLOBAL",
                    if cfg!(windows) { "NUL" } else { "/dev/null" },
                )
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.org")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.org")
                .output()
                .map_err(|error| error.to_string())?;
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            }
        };
        let dir = tempfile::tempdir()?;
        let path = dir.path();
        run(path, &["init", "-b", "main"])?;
        run(path, &["config", "user.name", "Test"])?;
        run(path, &["config", "user.email", "test@example.org"])?;
        run(path, &["config", "core.autocrlf", "false"])?;
        std::fs::write(path.join("a"), b"base\n")?;
        run(path, &["add", "a"])?;
        run(path, &["commit", "-qm", "base"])?;
        run(path, &["switch", "-qc", "feature"])?;
        std::fs::write(path.join("a"), b"feature\n")?;
        run(path, &["commit", "-qam", "feature"])?;
        run(path, &["switch", "-q", "main"])?;
        std::fs::write(path.join("a"), b"main\n")?;
        run(path, &["commit", "-qam", "main"])?;
        let _ = gg_git::merge::merge(path, "feature", false);
        let original = std::fs::read(path.join("a"))?;
        assert!(String::from_utf8_lossy(&original).contains("<<<<<<<"));

        let ctx = egui::Context::default();
        let mut app = Browser::new(path.to_path_buf());
        app.load(&ctx);
        for _ in 0..400 {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            if app.state.history.is_some() && app.mutation.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        app.open_conflict_view(&ctx);
        for _ in 0..400 {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            if app
                .conflict_view
                .as_ref()
                .is_some_and(|view| view.content.is_some() && view.snapshot.is_some())
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let (preview, working_text, snapshot) = {
            let view = app.conflict_view.as_ref().ok_or("missing conflict view")?;
            let snapshot = view.snapshot.clone().ok_or("missing snapshot")?;
            let content = view.content.as_ref().ok_or("missing content")?;
            (
                view.preview,
                conflict_preview_text(content, &snapshot, gg_engine::conflict::Resolution::Working),
                snapshot,
            )
        };
        assert_eq!(preview, gg_engine::conflict::Resolution::Working);
        assert!(working_text.contains("<<<<<<<"));
        assert_eq!(std::fs::read(path.join("a"))?, original);

        // Block by block is the default for text; key 2 chooses Theirs.
        let _ = crate::tests::run_ui(
            &ctx,
            frame_input(
                1120.0,
                740.0,
                vec![key(egui::Key::Num2, egui::Modifiers::NONE)],
            ),
            |ctx| app.draw_ui(ctx),
        );
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        {
            let view = app.conflict_view.as_ref().ok_or("missing conflict view")?;
            let blocks = view.blocks.as_ref().ok_or("missing blocks")?;
            assert_eq!(blocks.count(), 1);
            assert_eq!(
                blocks.choices[0],
                Some(gg_engine::conflict::BlockChoice::Theirs)
            );
            assert_eq!(blocks.text(), "feature\n");
        }
        let text = shown_text(&output);
        for expected in [
            "Block by block",
            "Conflict 1/1",
            "Theirs + ours",
            "working file preview, not staged",
            "ready to stage",
        ] {
            assert!(
                text.iter().any(|line| line.contains(expected)),
                "missing {expected:?} in {text:?}"
            );
        }
        assert_eq!(
            std::fs::read(path.join("a"))?,
            original,
            "choosing is read-only"
        );

        {
            let view = app.conflict_view.as_mut().ok_or("missing conflict view")?;
            view.whole_file = true;
            view.preview = gg_engine::conflict::Resolution::Ours;
        }
        let ours = {
            let view = app.conflict_view.as_ref().ok_or("missing conflict view")?;
            let content = view.content.as_ref().ok_or("missing content")?;
            conflict_preview_text(content, &snapshot, view.preview)
        };
        assert_eq!(ours, "main\n");
        assert_eq!(
            std::fs::read(path.join("a"))?,
            original,
            "choosing a preview source is read-only"
        );

        app.stage_conflict(&ctx, ConflictStage::Preview)?;
        for _ in 0..400 {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            if !app.operation_busy() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(!app.operation_busy());
        assert_eq!(std::fs::read(path.join("a"))?, b"main\n");
        assert!(gg_git::conflict::unmerged(path)?.is_empty());
        Ok(())
    }

    #[test]
    fn conflicts_open_in_the_main_workspace_and_hold_still() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let mut app = layout_fixture();
        let block = |n: &str| gg_engine::conflict::Segment::Block {
            ours: format!("ours {n}\n"),
            base: format!("base {n}\n"),
            theirs: format!("theirs {n}\n"),
        };
        app.conflict_view = Some(ConflictView {
            paths: vec!["a".into()],
            selected: 0,
            previous_selected: 0,
            keep: None,
            content: Some(gg_engine::conflict::Content {
                path: "a".into(),
                base: Some("base\n".into()),
                ours: Some("ours\n".into()),
                theirs: Some("theirs\n".into()),
                binary: false,
                truncated: false,
            }),
            error: None,
            loading: false,
            worker: None,
            blocks: Some(conflict_blocks::Blocks::new(vec![block("1"), block("2")])),
            blocks_note: None,
            whole_file: false,
            resolved: Vec::new(),
            edited: false,
            confirm_replace: false,
            names: None,
            snapshot: None,
            preview: gg_engine::conflict::Resolution::Working,
            ..Default::default()
        });
        let result_y = |app: &mut Browser, events: Vec<egui::Event>| {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, events), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = shown_text(&output);
            assert!(
                !text
                    .iter()
                    .any(|line| line.starts_with("Filter by message")),
                "the workspace replaces History: {text:?}"
            );
            output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == "Result" => {
                    Some(text.pos.y)
                }
                _ => None,
            })
        };
        let before = result_y(&mut app, vec![]).ok_or("no result line")?;
        let chosen = result_y(&mut app, vec![key(egui::Key::Num1, egui::Modifiers::NONE)]);
        assert_eq!(
            chosen,
            Some(before),
            "choosing a block does not move the result"
        );
        if let Some(view) = &mut app.conflict_view {
            view.error = Some("The conflict changed. Refresh external edits and retry.".into());
        }
        assert_eq!(
            result_y(&mut app, vec![]),
            Some(before),
            "an error has its own line"
        );
        let _ = crate::tests::run_ui(
            &ctx,
            frame_input(
                1120.0,
                740.0,
                vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
            ),
            |ctx| app.draw_ui(ctx),
        );
        assert!(app.conflict_view.is_none(), "Esc leaves the workspace");
        assert!(
            app.working.is_some(),
            "Esc returns to the plain changes list"
        );
        Ok(())
    }

    #[test]
    fn typing_in_the_result_edits_it_and_ctrl_z_undoes() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let mut app = layout_fixture();
        let block = gg_engine::conflict::Segment::Block {
            ours: "ours\n".into(),
            base: "base\n".into(),
            theirs: "theirs\n".into(),
        };
        app.conflict_view = Some(loaded_conflict(
            Some("ours\n"),
            Some("theirs\n"),
            b"<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> x\n",
            Some(conflict_blocks::Blocks::new(vec![
                gg_engine::conflict::Segment::Common("top\n".into()),
                block,
            ])),
        ));
        let mut time = 0.0;
        let mut frame = |app: &mut Browser, events: Vec<egui::Event>| {
            time += 0.1;
            let mut input = frame_input(1120.0, 740.0, events);
            input.time = Some(time);
            crate::tests::run_ui(&ctx, input, |ctx| app.draw_ui(ctx))
        };
        let _ = frame(&mut app, vec![]);
        let output = frame(&mut app, vec![]);
        // The Result's text: click at the end of its first line.
        let at = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text)
                    if text.galley.job.text.starts_with("top\n<<<<<<<") =>
                {
                    Some(text.pos + egui::vec2(40.0, 8.0))
                }
                _ => None,
            })
            .ok_or("the Result is painted")?;
        let press = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = frame(&mut app, vec![egui::Event::PointerMoved(at), press(true)]);
        let _ = frame(&mut app, vec![press(false)]);
        let _ = frame(&mut app, vec![egui::Event::Text("X".into())]);
        let doc = |app: &Browser| {
            app.conflict_view
                .as_ref()
                .and_then(|view| view.blocks.as_ref())
                .map(|blocks| blocks.doc.text.clone())
                .unwrap_or_default()
        };
        assert!(
            doc(&app).starts_with("topX\n"),
            "typed into the Result: {:?}",
            doc(&app)
        );
        assert_eq!(
            app.conflict_view
                .as_ref()
                .and_then(|view| view.blocks.as_ref())
                .map(|blocks| crate::conflict_result::line_origins(
                    &blocks.doc.text,
                    &blocks.doc.spans
                )[0]),
            Some(crate::conflict_result::Origin::Edited)
        );
        let _ = frame(&mut app, vec![key(egui::Key::Z, egui::Modifiers::COMMAND)]);
        assert!(
            doc(&app).starts_with("top\n<<<<<<<"),
            "Ctrl+Z undoes: {:?}",
            doc(&app)
        );
        Ok(())
    }

    #[test]
    fn conflict_staging_uses_the_operation_worker() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.conflict_view = Some(ConflictView {
            paths: vec!["a".into()],
            selected: 0,
            previous_selected: 0,
            keep: None,
            content: Some(gg_engine::conflict::Content {
                path: "a".into(),
                base: Some("base\n".into()),
                ours: Some("ours\n".into()),
                theirs: Some("theirs\n".into()),
                binary: false,
                truncated: false,
            }),
            error: None,
            loading: false,
            worker: None,
            blocks: None,
            blocks_note: None,
            whole_file: false,
            resolved: Vec::new(),
            edited: false,
            confirm_replace: false,
            names: None,
            snapshot: Some(gg_engine::conflict::Snapshot {
                base: Some("1".repeat(40)),
                ours: Some("2".repeat(40)),
                theirs: Some("3".repeat(40)),
                working: Some(b"ours\n".to_vec()),
                working_hash: None,
                working_note: None,
            }),
            preview: gg_engine::conflict::Resolution::Working,
            ..Default::default()
        });
        assert!(app.stage_conflict(&ctx, ConflictStage::Preview).is_ok());
        assert!(app.operation_busy());
        for _ in 0..200 {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            if !app.operation_busy() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(!app.operation_busy());
        assert!(app.mutation_error.is_some());
        assert!(
            app.outcome
                .as_ref()
                .is_some_and(|outcome| outcome.severity == OutcomeSeverity::Error)
        );
        assert!(
            app.conflict_view
                .as_ref()
                .and_then(|view| view.error.as_deref())
                .is_some(),
            "the conflict workspace shows the publication error"
        );
    }

    /// A loaded conflict view for `a` with the given sides and working bytes.
    fn loaded_conflict(
        ours: Option<&str>,
        theirs: Option<&str>,
        working: &[u8],
        blocks: Option<conflict_blocks::Blocks>,
    ) -> ConflictView {
        ConflictView {
            paths: vec!["a".into()],
            selected: 0,
            previous_selected: 0,
            keep: None,
            content: Some(gg_engine::conflict::Content {
                path: "a".into(),
                base: Some("base\n".into()),
                ours: ours.map(str::to_string),
                theirs: theirs.map(str::to_string),
                binary: false,
                truncated: false,
            }),
            error: None,
            loading: false,
            worker: None,
            whole_file: blocks.is_none(),
            blocks_note: blocks.is_none().then(|| "No text blocks.".to_string()),
            blocks,
            resolved: Vec::new(),
            edited: false,
            confirm_replace: false,
            names: None,
            snapshot: Some(gg_engine::conflict::Snapshot {
                base: Some("1".repeat(40)),
                ours: ours.map(|_| "2".repeat(40)),
                theirs: theirs.map(|_| "3".repeat(40)),
                working: Some(working.to_vec()),
                working_hash: None,
                working_note: None,
            }),
            preview: gg_engine::conflict::Resolution::Working,
            ..Default::default()
        }
    }

    #[test]
    fn conflict_workspace_shows_side_names() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let block = gg_engine::conflict::Segment::Block {
            ours: "ours\n".into(),
            base: "base\n".into(),
            theirs: "theirs\n".into(),
        };
        let mut view = loaded_conflict(
            Some("ours\n"),
            Some("theirs\n"),
            b"<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> x\n",
            Some(conflict_blocks::Blocks::new(vec![block])),
        );
        view.names = Some(gg_engine::conflict::SideNames {
            ours: "main".into(),
            theirs: "feature/login".into(),
        });
        app.conflict_view = Some(view);
        for whole_file in [false, true] {
            if let Some(view) = &mut app.conflict_view {
                view.whole_file = whole_file;
            }
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = shown_text(&output);
            for expected in ["Ours · main", "Theirs · feature/login"] {
                assert!(
                    text.iter().any(|line| line.contains(expected)),
                    "missing {expected:?} (whole file {whole_file}) in {text:?}"
                );
            }
        }
    }

    #[test]
    fn delete_modify_conflict_offers_keep_deletion() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.conflict_view = Some(loaded_conflict(Some("main\n"), None, b"main\n", None));
        let shown = |app: &mut Browser| {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            shown_text(&crate::tests::run_ui(
                &ctx,
                frame_input(1120.0, 740.0, vec![]),
                |ctx| app.draw_ui(ctx),
            ))
        };
        let text = shown(&mut app);
        assert!(text.iter().any(|line| line == "Keep deletion"), "{text:?}");
        assert!(
            text.iter()
                .any(|line| line.contains("(deleted on this side)")),
            "{text:?}"
        );
        // The deleted side points to Keep deletion.
        if let Some(view) = &mut app.conflict_view {
            view.preview = gg_engine::conflict::Resolution::Theirs;
        }
        let view = app.conflict_view.as_ref().map(|view| {
            (
                conflict_preview_available(view),
                conflict_preview_reason(view),
            )
        });
        assert_eq!(
            view,
            Some((
                false,
                "This side deleted the file. Choose Keep deletion to stage the removal."
            ))
        );
        // Keep deletion states the consequence and can be staged.
        if let Some(view) = &mut app.conflict_view {
            view.preview = gg_engine::conflict::Resolution::Deletion;
        }
        let text = shown(&mut app);
        assert!(
            text.iter().any(|line| line
                .contains("a will be deleted; the other side's changes to it are discarded.")),
            "{text:?}"
        );
        assert!(
            app.conflict_view
                .as_ref()
                .is_some_and(conflict_preview_available)
        );
        // A modify/modify conflict has no Keep deletion.
        app.conflict_view = Some(loaded_conflict(Some("o\n"), Some("t\n"), b"o\n", None));
        let text = shown(&mut app);
        assert!(!text.iter().any(|line| line == "Keep deletion"), "{text:?}");
    }

    #[test]
    fn an_edited_working_file_offers_use_working_file_and_confirms_replace() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let mut blocks = conflict_blocks::Blocks::new(vec![gg_engine::conflict::Segment::Block {
            ours: "ours\n".into(),
            base: "base\n".into(),
            theirs: "theirs\n".into(),
        }]);
        blocks.choose(0, gg_engine::conflict::BlockChoice::Ours);
        let mut view = loaded_conflict(
            Some("ours\n"),
            Some("theirs\n"),
            b"hand edit\n",
            Some(blocks),
        );
        view.edited = true;
        app.conflict_view = Some(view);
        let frame = |app: &mut Browser, events| {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, events), |ctx| {
                app.draw_ui(ctx)
            });
            shown_text(&crate::tests::run_ui(
                &ctx,
                frame_input(1120.0, 740.0, vec![]),
                |ctx| app.draw_ui(ctx),
            ))
        };
        let text = frame(&mut app, vec![]);
        for expected in [
            "This file was edited outside the conflict blocks.",
            "Use working file",
            "Replace with blocks…",
        ] {
            assert!(
                text.iter().any(|line| line.contains(expected)),
                "missing {expected:?} in {text:?}"
            );
        }
        assert!(!text.iter().any(|line| line == "Stage resolution"));
        // Ctrl+S asks before overwriting the edits; nothing is staged yet.
        let text = frame(&mut app, vec![key(egui::Key::S, egui::Modifiers::COMMAND)]);
        assert!(!app.operation_busy());
        assert!(
            app.conflict_view
                .as_ref()
                .is_some_and(|view| view.confirm_replace)
        );
        assert!(
            text.iter()
                .any(|line| line.contains("The edits made outside the conflict blocks are lost.")),
            "{text:?}"
        );
        assert!(
            text.iter().any(|line| line == "Replace and stage"),
            "{text:?}"
        );
        // Esc cancels the question, not the workspace.
        let _ = frame(
            &mut app,
            vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
        );
        assert!(
            app.conflict_view
                .as_ref()
                .is_some_and(|view| !view.confirm_replace)
        );
        assert!(!app.operation_busy());
    }

    #[test]
    fn hand_edits_in_the_result_are_what_stage_resolution_stages_and_minus_reopens()
    -> Result<(), Box<dyn std::error::Error>> {
        let run = |path: &std::path::Path, args: &[&str]| -> Result<(), String> {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(path)
                .args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env(
                    "GIT_CONFIG_GLOBAL",
                    if cfg!(windows) { "NUL" } else { "/dev/null" },
                )
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.org")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.org")
                .output()
                .map_err(|error| error.to_string())?;
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            }
        };
        let dir = tempfile::tempdir()?;
        let path = dir.path();
        run(path, &["init", "-b", "main"])?;
        run(path, &["config", "core.autocrlf", "false"])?;
        std::fs::write(path.join("a"), b"[deps]\neframe = \"0.36\"\nend\n")?;
        run(path, &["add", "a"])?;
        run(path, &["commit", "-qm", "base"])?;
        run(path, &["switch", "-qc", "feature"])?;
        std::fs::write(path.join("a"), b"[deps]\neframe = \"0.38\"\nend\n")?;
        run(path, &["commit", "-qam", "feature"])?;
        run(path, &["switch", "-q", "main"])?;
        std::fs::write(
            path.join("a"),
            b"[deps]\neframe = { version = \"0.37\", default-features = false }\nend\n",
        )?;
        run(path, &["commit", "-qam", "main"])?;
        let _ = run(path, &["merge", "-q", "feature"]);

        let ctx = egui::Context::default();
        let mut app = Browser::new(path.to_path_buf());
        app.load(&ctx);
        let settle = |app: &mut Browser, done: &dyn Fn(&Browser) -> bool| {
            for _ in 0..400 {
                let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                    app.draw_ui(ctx)
                });
                if done(app) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        };
        settle(&mut app, &|app| {
            app.state.history.is_some() && app.mutation.is_some()
        });
        app.open_conflict_view(&ctx);
        settle(&mut app, &|app| {
            app.conflict_view
                .as_ref()
                .is_some_and(|view| view.blocks.is_some())
        });
        {
            let view = app.conflict_view.as_mut().ok_or("missing conflict view")?;
            assert!(!view.edited);
            let blocks = view.blocks.as_mut().ok_or("missing blocks")?;
            assert!(
                blocks.doc.text.contains("<<<<<<< main\n"),
                "markers name the sides: {}",
                blocks.doc.text
            );
            blocks.choose(0, gg_engine::conflict::BlockChoice::Ours);
            let edited = blocks.doc.text.replace("end\n", "end by hand\n");
            blocks.doc.typed(edited, 0.0);
            assert!(blocks.complete());
        }
        app.stage_conflict(&ctx, ConflictStage::Preview)?;
        settle(&mut app, &|app| !app.operation_busy());
        assert_eq!(
            std::fs::read_to_string(path.join("a"))?,
            "[deps]\neframe = { version = \"0.37\", default-features = false }\nend by hand\n",
            "the edited Result is what is staged"
        );
        assert!(gg_git::conflict::unmerged(path)?.is_empty());
        settle(&mut app, &|app| {
            app.conflict_view
                .as_ref()
                .is_some_and(|view| !view.loading && view.resolved == ["a"])
        });
        // − reopens the file as conflicted.
        app.reopen_conflict(&ctx, "a".into())?;
        settle(&mut app, &|app| !app.operation_busy());
        assert_eq!(gg_git::conflict::unmerged(path)?, vec!["a".to_string()]);
        settle(&mut app, &|app| {
            app.conflict_view
                .as_ref()
                .is_some_and(|view| !view.loading && view.paths == ["a"])
        });
        let view = app.conflict_view.as_ref().ok_or("missing conflict view")?;
        assert!(view.resolved.is_empty(), "back under CONFLICTED");
        Ok(())
    }

    #[test]
    fn use_working_file_stages_a_hand_edited_file_as_is() -> Result<(), Box<dyn std::error::Error>>
    {
        let run = |path: &std::path::Path, args: &[&str]| -> Result<(), String> {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(path)
                .args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env(
                    "GIT_CONFIG_GLOBAL",
                    if cfg!(windows) { "NUL" } else { "/dev/null" },
                )
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.org")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.org")
                .output()
                .map_err(|error| error.to_string())?;
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            }
        };
        let dir = tempfile::tempdir()?;
        let path = dir.path();
        run(path, &["init", "-b", "main"])?;
        run(path, &["config", "core.autocrlf", "false"])?;
        std::fs::write(path.join("a"), b"base\n")?;
        run(path, &["add", "a"])?;
        run(path, &["commit", "-qm", "base"])?;
        run(path, &["switch", "-qc", "feature"])?;
        std::fs::write(path.join("a"), b"feature\n")?;
        run(path, &["commit", "-qam", "feature"])?;
        run(path, &["switch", "-q", "main"])?;
        std::fs::write(path.join("a"), b"main\n")?;
        run(path, &["commit", "-qam", "main"])?;
        // Merge with the test identity; it stops on the conflict.
        let _ = run(path, &["merge", "-q", "feature"]);
        std::fs::write(path.join("a"), b"resolved by hand\n")?;

        let ctx = egui::Context::default();
        let mut app = Browser::new(path.to_path_buf());
        app.load(&ctx);
        for _ in 0..400 {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            if app.state.history.is_some() && app.mutation.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        app.open_conflict_view(&ctx);
        for _ in 0..400 {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            if app
                .conflict_view
                .as_ref()
                .is_some_and(|view| view.content.is_some())
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        {
            let view = app.conflict_view.as_ref().ok_or("missing conflict view")?;
            assert!(view.edited, "the hand edit is detected");
            assert!(view.blocks.is_some(), "blocks still come from the stages");
            let names = view.names.as_ref().ok_or("missing side names")?;
            assert_eq!(
                (names.ours.as_str(), names.theirs.as_str()),
                ("main", "feature")
            );
        }
        app.stage_conflict(&ctx, ConflictStage::WorkingFile)?;
        for _ in 0..400 {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            if !app.operation_busy() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(std::fs::read(path.join("a"))?, b"resolved by hand\n");
        assert!(gg_git::conflict::unmerged(path)?.is_empty());
        Ok(())
    }

    #[test]
    fn scoped_commit_actions_menu_lists_grouped_actions() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.state.selected = Some("a".repeat(40));
            let _ = crate::tests::run_ui(
                &ctx,
                frame_input(width, 740.0, vec![key(egui::Key::A, egui::Modifiers::ALT)]),
                |ctx| app.draw_ui(ctx),
            );
            if width < 1040.0 {
                assert!(app.narrow_details, "narrow details must open first");
            }
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            // The popup mirrors the Commit menu with Copy subject and
            // Signature details… under Copy ID.
            for label in [
                "Review changes",
                "Compare with HEAD",
                "Copy ID",
                "Cherry-pick onto HEAD",
                "Revert",
                "Interactive rebase from here…",
                "Reword…",
                "Reopen for editing…",
                "Fixup into…",
                "Drop…",
                "Reset current branch to here",
                "Copy subject",
                "Signature details…",
            ] {
                assert!(text.contains(&label), "missing {label} at {width}");
            }
            assert!(
                !text.contains(&"Recovery…") || !text.contains(&"Copy subject"),
                "Recovery… moved to the app menu"
            );
            assert_eq!(
                text.iter().filter(|line| **line == "Actions").count(),
                1,
                "the inspector's or the narrow details' Actions button opens the menu at {width}"
            );
            assert!(!text.contains(&"Selected commit actions…"));
        }
    }

    #[test]
    fn actions_shortcut_opens_the_scoped_menu() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.state.selected = Some("a".repeat(40));
        let _ = crate::tests::run_ui(
            &ctx,
            frame_input(1120.0, 740.0, vec![key(egui::Key::A, egui::Modifiers::ALT)]),
            |ctx| app.draw_ui(ctx),
        );
        assert!(app.details_visible);
        assert!(!app.commit_actions, "the request is consumed once");
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text) if text.galley.job.text == "Cherry-pick onto HEAD"
        )));
    }

    #[test]
    fn commit_action_menu_matches_the_inspector_with_one_reason_band() {
        let ctx = egui::Context::default();
        let reason = "Rebase in progress. Continue or abort it to change commits.";
        let menu = vec![
            menus::item("Review changes", "Enter", FrameCommand::ReviewSelected).entry(),
            menus::item("Copy ID", "Ctrl+C", FrameCommand::CopyId).entry(),
            menus::Entry::Separator,
            menus::item("Cherry-pick onto HEAD", "", FrameCommand::CherryPick)
                .unless(Some(reason))
                .entry(),
            menus::item(
                "Drop…",
                "D",
                FrameCommand::RebasePreset(gg_engine::rewrite::Action::Drop),
            )
            .unless(Some(reason))
            .danger()
            .entry(),
            menus::Entry::Separator,
            menus::item("Reset current branch to here…", "", FrameCommand::ResetHere)
                .unless(Some(reason))
                .danger()
                .entry(),
        ];
        let output = crate::tests::run_ui(&ctx, frame_input(360.0, 600.0, vec![]), |ui| {
            commit_action_menu(ui, &menu);
        });
        let rows = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some((text.pos.y, text.galley.job.text.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let labels = rows
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>();
        let at = |label: &str| labels.iter().position(|text| *text == label);
        assert_eq!(
            [at("Copy ID"), at("Copy subject"), at("Signature details…")]
                .map(|index| index.is_some()),
            [true; 3]
        );
        assert!(
            at("Copy ID") < at("Copy subject") && at("Copy subject") < at("Signature details…")
        );
        assert!(at("Recovery…").is_none(), "Recovery… lives in the app menu");
        assert_eq!(
            labels.iter().filter(|text| **text == reason).count(),
            1,
            "one band names the reason: {labels:?}"
        );
        assert!(
            at(reason) < at("Cherry-pick onto HEAD"),
            "the band sits above the mutations"
        );
        assert!(
            at("Reset current branch to here").is_some(),
            "Reset opens Soft / Mixed / Hard"
        );
        assert!(
            labels
                .iter()
                .all(|text| !text.contains("8f0e") && !text.starts_with("COMMIT")),
            "no group headings or hashes in labels"
        );
    }

    #[test]
    fn commit_action_menu_returns_the_clicked_command() {
        let ctx = egui::Context::default();
        let app = layout_fixture();
        let menu = app.commit_menu_entries();
        let command = std::cell::RefCell::new(None);
        let draw = |events: Vec<egui::Event>| {
            crate::tests::run_ui(&ctx, frame_input(360.0, 600.0, events), |ui| {
                command.borrow_mut().replace(commit_action_menu(ui, &menu));
            })
        };
        let first = draw(vec![]);
        let copy = first
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == "Copy subject" => {
                    Some(text.pos)
                }
                _ => None,
            })
            .unwrap_or_default()
            + egui::vec2(5.0, 4.0);
        let button = |pressed| egui::Event::PointerButton {
            pos: copy,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        draw(vec![egui::Event::PointerMoved(copy), button(true)]);
        draw(vec![button(false)]);
        assert!(matches!(
            command.borrow().as_ref(),
            Some(Some(CommitCommand::CopySubject))
        ));
    }

    #[test]
    fn help_shortcuts_window_renders() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.help_shortcuts = true;
        // A new window is sized invisibly on its first frame.
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.contains(&"Keyboard shortcuts"));
        assert!(text.contains(&"REPOSITORY"));
        assert!(text.contains(&"Ctrl+O"));
        assert!(text.contains(&"Edit keys…"));
        assert!(
            text.iter()
                .all(|line| !line.to_lowercase().contains("refresh")),
            "no Refresh shortcut is advertised: {text:?}"
        );
    }

    #[test]
    fn keyboard_shortcuts_come_from_the_command_table() {
        let app = layout_fixture();
        let groups = shortcuts::groups(&app.menu_table(&app.frame_readiness()));
        let rows = groups
            .iter()
            .flat_map(|group| &group.rows)
            .map(|row| (row.key.as_str(), row.action.as_str()))
            .collect::<Vec<_>>();
        for key in [
            "Ctrl+O",
            "Ctrl+B",
            "Ctrl+Shift+B",
            "Ctrl+Shift+F",
            "Ctrl+Shift+L",
            "Ctrl+Shift+P",
            "Ctrl+Shift+S",
            "Ctrl+Shift+U",
            "Ctrl+Shift+Z",
            "Ctrl+1",
            "Ctrl+2",
            "Ctrl+\\",
            "F2",
            "Ctrl+Enter",
        ] {
            assert!(rows.iter().any(|(k, _)| *k == key), "missing {key}");
        }
        assert!(rows.contains(&("Ctrl+R", "Reword…")));
        let titles = groups.iter().map(|g| g.title.as_str()).collect::<Vec<_>>();
        for scope in [
            "Changes list",
            "Review",
            "Rebase list",
            "Conflicts",
            "Stashes",
        ] {
            assert!(titles.contains(&scope), "missing scope {scope}");
        }
        let reword = shortcuts::filter(&groups, "reword");
        assert!(reword.iter().all(|g| {
            g.rows
                .iter()
                .all(|r| r.action.to_lowercase().contains("reword"))
        }));
    }

    #[test]
    fn keyboard_shortcuts_close_on_escape_and_edit_keys_opens_preferences() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.help_shortcuts = true;
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let edit = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == "Edit keys…" => {
                    Some(text.pos + egui::vec2(4.0, 4.0))
                }
                _ => None,
            })
            .unwrap_or_default();
        let button = |pressed| egui::Event::PointerButton {
            pos: edit,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = crate::tests::run_ui(
            &ctx,
            frame_input(
                1120.0,
                740.0,
                vec![egui::Event::PointerMoved(edit), button(true)],
            ),
            |ctx| app.draw_ui(ctx),
        );
        let _ = crate::tests::run_ui(
            &ctx,
            frame_input(1120.0, 740.0, vec![button(false)]),
            |ctx| app.draw_ui(ctx),
        );
        assert!(!app.help_shortcuts);
        assert_eq!(
            app.folders_dialog.as_ref().map(|dialog| dialog.section),
            Some(PrefSection::Keyboard)
        );
        app.folders_dialog = None;
        app.help_shortcuts = true;
        let escape = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![escape]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(!app.help_shortcuts, "Esc closes the window");
    }

    #[test]
    fn operation_outcome_shows_details_and_recovery() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.outcome = Some(Outcome {
                at: std::time::Instant::now(),
                severity: OutcomeSeverity::Warning,
                summary: "Rebased onto feature · 1 conflict(s)".into(),
                details: Some("Conflicts: a.txt".into()),
                recovery: Some(RecoveryAction::ResolveConflicts),
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(
                text.iter()
                    .any(|line| line.contains("Rebased onto feature")),
                "missing outcome summary at {width}"
            );
            assert!(text.contains(&"Details…"), "missing details at {width}");
            assert!(
                text.contains(&"Resolve conflicts…"),
                "missing recovery at {width}"
            );
        }
    }

    #[test]
    fn operation_details_window_lists_full_text() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.outcome = Some(Outcome {
            at: std::time::Instant::now(),
            severity: OutcomeSeverity::Warning,
            summary: "Rewrote 2 commit(s)".into(),
            details: Some("Backup: refs/gg/backup/rewrite/1".into()),
            recovery: Some(RecoveryAction::Backups),
        });
        app.outcome_details_open = true;
        // A new window is sized invisibly on its first frame.
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.contains(&"Rewrote 2 commit(s) · details"));
        assert!(text.contains(&"Rewrote 2 commit(s)"));
        assert!(text.contains(&"Backup: refs/gg/backup/rewrite/1"));
        assert!(
            text.contains(&"Recovery…"),
            "the footer's recovery action is the window's primary"
        );
    }

    #[test]
    fn outcome_recovery_button_opens_backups() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.outcome = Some(Outcome {
            at: std::time::Instant::now(),
            severity: OutcomeSeverity::Info,
            summary: "Rewrote 1 commit(s)".into(),
            details: None,
            recovery: Some(RecoveryAction::Backups),
        });
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let backups = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.job.text == "Recovery…" => {
                Some(text.pos)
            }
            _ => None,
        });
        assert!(backups.is_some(), "missing backups button");
        let pos = backups.unwrap_or_default();
        let press = egui::Event::PointerButton {
            pos: pos + egui::vec2(5.0, 0.0),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        };
        let release = egui::Event::PointerButton {
            pos: pos + egui::vec2(5.0, 0.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = crate::tests::run_ui(
            &ctx,
            frame_input(1120.0, 740.0, vec![egui::Event::PointerMoved(pos), press]),
            |ctx| app.draw_ui(ctx),
        );
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![release]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(
            app.recovery
                .as_ref()
                .is_some_and(|view| { view.source == return_location::RecoverySource::Backups })
        );
    }

    #[test]
    fn mutation_banner_renders_at_supported_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.mutation = Some(gg_engine::mutation::OperationState {
                operation: gg_engine::mutation::Operation::CherryPick,
                commit: Some("abc123".into()),
                conflicts: vec!["a".into()],
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(
                text.iter()
                    .any(|line| line.starts_with("Cherry-pick conflict · applying")),
                "{text:?}"
            );
            assert!(text.contains(&"1 conflict left in 1 file"));
            assert!(text.iter().any(|line| line.contains("Continue")));
            assert!(text.iter().any(|line| line.contains("Skip commit")));
            assert!(text.iter().any(|line| line.contains("Abort and restore")));
            assert!(
                !text.iter().any(|line| line.contains("Recovery…")),
                "recovery destinations stay out of the continuation controls"
            );
        }
    }

    #[test]
    fn merge_operation_area_has_no_skip_at_either_width() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.mutation = Some(gg_engine::mutation::OperationState {
                operation: gg_engine::mutation::Operation::Merge,
                commit: Some("abc123".into()),
                conflicts: vec!["a".into()],
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                text.iter()
                    .filter(|line| line.starts_with("Merge conflict · merging"))
                    .count(),
                1,
                "one operation area at {width}: {text:?}"
            );
            assert!(text.iter().any(|line| line.contains("Continue")));
            assert!(text.iter().any(|line| line.contains("Abort")));
            assert!(!text.iter().any(|line| line.contains("Skip")));
            assert!(
                text.iter().any(|line| line.contains("abc123")),
                "the area names the operation target at {width}: {text:?}"
            );
        }
    }

    #[test]
    fn conflict_window_leaves_continuation_to_the_operation_area() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.mutation = Some(gg_engine::mutation::OperationState {
            operation: gg_engine::mutation::Operation::CherryPick,
            commit: Some("abc123".into()),
            conflicts: vec!["a".into()],
        });
        app.conflict_view = Some(ConflictView {
            paths: vec!["a".into()],
            selected: 0,
            previous_selected: 0,
            keep: None,
            content: None,
            error: None,
            loading: false,
            worker: None,
            snapshot: None,
            preview: gg_engine::conflict::Resolution::Working,
            blocks: None,
            blocks_note: None,
            whole_file: false,
            resolved: Vec::new(),
            edited: false,
            confirm_replace: false,
            names: None,
            ..Default::default()
        });
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            !text.iter().any(|line| line.contains("Resolve conflicts")),
            "the band hides Resolve conflicts while the workspace is open: {text:?}"
        );
        assert!(
            !text.contains(&"Close"),
            "the workspace has no Close: {text:?}"
        );
        assert_eq!(
            text.iter().filter(|line| line.contains("Continue")).count(),
            1,
            "only the operation area offers Continue: {text:?}"
        );
        assert_eq!(
            text.iter().filter(|line| **line == "Skip commit").count(),
            1,
            "only the operation band offers Skip commit: {text:?}"
        );
    }

    #[test]
    fn rebase_operation_area_keeps_recovery_out_of_continuation() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.mutation = Some(gg_engine::mutation::OperationState {
            operation: gg_engine::mutation::Operation::Rebase,
            commit: Some("abc123".into()),
            conflicts: vec!["a".into()],
        });
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.iter().any(|line| line.starts_with("Rebase conflict")));
        assert!(text.iter().any(|line| line.contains("Continue")));
        assert!(text.iter().any(|line| line.contains("Skip commit")));
        assert!(text.iter().any(|line| line.contains("Abort and restore")));
        assert!(!text.iter().any(|line| line.contains("Recovery…")));
    }

    #[test]
    fn network_cancel_button_stays_inside_the_operation_area() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.network = Some(network::Worker::idle(gg_engine::sync::Operation::Fetch));
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            assert!(
                output.shapes.iter().any(|shape| matches!(
                    &shape.shape,
                    egui::epaint::Shape::Text(text)
                        if text.galley.job.text == "Cancel"
                            && text.pos.x + text.galley.size().x <= width
                )),
                "cancel control missing or clipped at width {width}"
            );
        }
    }

    #[test]
    fn the_footer_shows_network_progress_then_cancelling() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let mut app = layout_fixture();
        app.network = Some(
            network::Worker::idle(gg_engine::sync::Operation::Fetch).labelled("Fetching origin"),
        );
        let message = app.footer_message(false);
        assert_eq!(message.text, "Starting fetch…", "until Git reports");
        app.network_progress = Some(gg_engine::sync::Progress {
            operation: gg_engine::sync::Operation::Fetch,
            phase: "Receiving objects".into(),
            percent: Some(42),
            message: "812/1930".into(),
        });
        let message = app.footer_message(false);
        assert_eq!(message.tone, footer::Tone::Progress);
        assert_eq!(
            message.full_text(),
            "Fetching origin · Receiving objects 42% · 812/1930"
        );
        assert_eq!(message.bar, Some(0.42));
        assert!(message.button(footer::Act::Cancel).is_some());
        // Drawn: the message, the bar, and Cancel inside the window.
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let text = shown_text(&output);
        assert!(
            text.iter()
                .any(|line| line == "Fetching origin · Receiving objects 42% · 812/1930"),
            "{text:?}"
        );
        assert!(!text.iter().any(|line| line.starts_with("Network · ")));
        if let Some(worker) = &app.network {
            worker.cancel();
        }
        let message = app.footer_message(false);
        assert_eq!(
            (message.text.as_str(), message.right.as_str()),
            ("Cancelling fetch…", "waits for Git to stop")
        );
        assert!(message.buttons.is_empty());
    }

    #[test]
    fn footer_labels_name_the_job_and_its_target() {
        let mut app = layout_fixture();
        app.branch_label = Some("Switching to feature/rebase-ui…".into());
        assert_eq!(
            app.footer_message(false).text,
            "Switching to feature/rebase-ui…"
        );
        app.branch_label = None;
        app.operation_worker = Some(std::sync::mpsc::sync_channel(1).1);
        app.mutation_notice = Some("Merging origin/main into main…".into());
        let message = app.footer_message(false);
        assert_eq!(
            (message.tone, message.text.as_str()),
            (footer::Tone::Progress, "Merging origin/main into main…")
        );
        // A band names its own Continue; the footer stays quiet about it.
        app.mutation_busy = Some(operation_band::Command::Continue);
        assert_ne!(app.footer_message(false).tone, footer::Tone::Progress);
    }

    #[test]
    fn receipts_and_failures_read_without_prefixes() {
        let mut app = layout_fixture();
        app.set_outcome(
            OutcomeSeverity::Info,
            "Rewrote 4 commits · backup refs/gg/backup/main/2026-10-30T10-52".into(),
            Some("Backup: refs/gg/backup/main/2026-10-30T10-52".into()),
            Some(RecoveryAction::Backups),
        );
        let message = app.footer_message(false);
        assert_eq!(message.tone, footer::Tone::Success);
        assert_eq!(message.text, "Rewrote 4 commits");
        assert_eq!(
            message.tail,
            " · backup refs/gg/backup/main/2026-10-30T10-52"
        );
        let labels: Vec<_> = message.buttons.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, ["Recovery…", "Details…"]);
        assert!(message.dismiss);

        app.set_outcome(
            OutcomeSeverity::Error,
            "Push rejected: origin/main has 1 commit you do not have · fetch first".into(),
            Some("! [rejected] main -> main (fetch first)".into()),
            Some(RecoveryAction::Fetch),
        );
        let message = app.footer_message(false);
        assert_eq!(message.tone, footer::Tone::Failed);
        let labels: Vec<_> = message
            .buttons
            .iter()
            .map(|b| (b.label.as_str(), b.primary))
            .collect();
        assert_eq!(labels, [("Fetch", true), ("Details…", false)]);
        assert!(message.dismiss);
        // A failure outranks a running job and stays until the next one.
        app.branch_label = Some("Switching to main…".into());
        assert_eq!(app.footer_message(false).tone, footer::Tone::Failed);
        app.clear_failure();
        assert_eq!(app.footer_message(false).tone, footer::Tone::Progress);
    }

    #[test]
    fn a_blocked_switch_offers_stash_and_switch_in_amber() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.local_summary = Some(LocalSummary {
            changes: Some(3),
            ..Default::default()
        });
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = tx.send(Err(
            "Working tree is not clean (a). Commit, stash, or discard changes before switching or creating a branch.".into(),
        ));
        app.branch = Some(rx);
        app.branch_target = Some(("feature/rebase-ui".into(), false));
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let message = app.footer_message(false);
        assert_eq!(message.tone, footer::Tone::Blocked);
        assert_eq!(
            message.text,
            "Can’t switch to feature/rebase-ui: 3 changed files would be overwritten"
        );
        let labels: Vec<_> = message
            .buttons
            .iter()
            .map(|b| (b.label.as_str(), b.primary))
            .collect();
        assert_eq!(labels, [("Stash and switch", true)]);
        assert!(message.dismiss);
    }

    #[test]
    fn cancelled_fetch_rereads_the_repository_before_offering_retry() -> Checked {
        let ctx = egui::Context::default();
        let (_dir, mut app) = two_commits()?;
        app.finish_network(
            &ctx,
            Err(format!("Fetch from origin{}", gg_git::sync::CANCELLED)),
            Some(gg_engine::sync::Operation::Fetch),
        );
        let outcome = app.outcome.as_ref().ok_or("missing cancel outcome")?;
        assert_eq!(
            outcome.summary,
            "Fetch from origin cancelled. Git Good re-read the repository; check what changed before retrying."
        );
        assert!(outcome.recovery == Some(RecoveryAction::RetryFetch));
        assert!(app.pending.is_some(), "refs and history are re-read");
        let text = frames_until(&ctx, &mut app, vec![], |app, _| app.pending.is_none())?;
        assert!(
            text.iter().any(|line| line == "Retry fetch"),
            "Retry is offered once the re-read finished: {text:?}"
        );
        Ok(())
    }

    #[test]
    fn finish_network_clone_opens_the_new_repository() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let mut app = Browser::new(".".into());
        app.outcome = Some(Outcome {
            at: std::time::Instant::now(),
            severity: OutcomeSeverity::Info,
            summary: "Old repository result".into(),
            details: None,
            recovery: None,
        });
        let path = std::env::current_dir()?;
        app.finish_network(
            &ctx,
            Ok(network::Outcome::Clone(gg_engine::sync::CloneReceipt {
                path: path.clone(),
                head: None,
                branch: Some("main".into()),
            })),
            None,
        );
        assert_eq!(app.path, path.to_string_lossy());
        assert!(app.network.is_none());
        assert!(
            app.network_notice
                .as_deref()
                .is_some_and(|notice| notice.starts_with("Cloned"))
        );
        assert!(app.state.requested.is_some());
        assert!(app.outcome.is_none(), "clone clears the old outcome");
        Ok(())
    }

    #[test]
    fn blocked_switch_keeps_intent_and_retries_explicitly() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = tx.send(Err("worktree has conflicting changes".into()));
        app.branch = Some(rx);
        app.branch_target = Some(("feature".into(), false));
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.branch.is_none());
        assert_eq!(app.pending_switch.as_deref(), Some("feature"));
        assert!(app.branch_error.is_none(), "a blocked switch is a warning");
        let severity = app.outcome.as_ref().map(|outcome| outcome.severity);
        let recovery = app.outcome.as_ref().and_then(|outcome| outcome.recovery);
        assert!(matches!(severity, Some(OutcomeSeverity::Warning)));
        assert!(matches!(recovery, Some(RecoveryAction::RetrySwitch)));

        // The retained intent never re-executes on its own.
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.branch.is_none());

        // Dismiss clears the retained intent without executing it.
        app.dismiss_outcome();
        assert!(app.pending_switch.is_none());
        assert!(app.outcome.is_none());
        app.outcome = Some(Outcome {
            at: std::time::Instant::now(),
            severity: OutcomeSeverity::Warning,
            summary: "Can’t switch to feature: 3 changed files would be overwritten".into(),
            details: None,
            recovery: Some(RecoveryAction::RetrySwitch),
        });
        app.pending_switch = Some("feature".into());

        // Retry revalidates by running the same explicit switch again.
        app.run_recovery_action(&ctx, RecoveryAction::RetrySwitch);
        assert!(app.branch.is_some(), "retry starts a fresh switch worker");

        // A successful revalidated switch clears the warning and the intent.
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = tx.send(Ok(gg_git::branch::Receipt {
            branch: "feature".into(),
            head: "a".repeat(40),
            created: false,
            warnings: vec![],
            stash: None,
        }));
        app.branch = Some(rx);
        app.branch_target = Some(("feature".into(), false));
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.pending_switch.is_none());
        assert!(app.outcome.is_none());
    }

    #[test]
    fn created_branch_is_reported_when_the_switch_is_blocked() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = tx.send(Err("worktree has conflicting changes".into()));
        app.branch = Some(rx);
        app.branch_target = Some(("feature".into(), false));
        app.branch_after_create = Some("feature".into());
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let summary = app.outcome.as_ref().map(|outcome| outcome.summary.as_str());
        assert!(
            summary.is_some_and(
                |summary| summary.starts_with("Created feature but can’t switch to it: ")
            ),
            "{summary:?}"
        );
        assert_eq!(app.pending_switch.as_deref(), Some("feature"));
    }

    #[test]
    fn branch_creation_window_renders_at_supported_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.create_branch = Some(CreateBranchDialog {
                name: "feature-x".into(),
                commit: "a".repeat(40),
                selector: "current HEAD".into(),
                switch: true,
                recovery_source: None,
                error: None,
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(text.iter().any(|line| line.contains("New branch")));
            assert!(
                text.iter()
                    .any(|line| line.contains("Switch after creation"))
            );
        }
    }

    #[test]
    fn recovery_return_location_reopens_the_same_source() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.restore_return_location(
            &ctx,
            return_location::ReturnLocation::Recovery {
                source: return_location::RecoverySource::Reflog,
                reference: Some("HEAD".into()),
                entry: "HEAD@{2}".into(),
            },
        );
        assert!(app.recovery.is_some());
        assert_eq!(
            app.recovery.as_ref().map(|view| view.source),
            Some(return_location::RecoverySource::Reflog)
        );
    }

    #[test]
    fn recovery_return_location_restores_the_reflog_reference_and_entry()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::process::Command;
        let run = |path: &std::path::Path, args: &[&str]| -> Result<(), String> {
            let output = Command::new("git")
                .arg("-C")
                .arg(path)
                .args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env(
                    "GIT_CONFIG_GLOBAL",
                    if cfg!(windows) { "NUL" } else { "/dev/null" },
                )
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.org")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.org")
                .output()
                .map_err(|error| error.to_string())?;
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            }
        };
        let dir = tempfile::tempdir()?;
        let path = dir.path();
        run(path, &["init", "-q", "-b", "main"])?;
        run(path, &["config", "user.name", "Test"])?;
        run(path, &["config", "user.email", "test@example.org"])?;
        std::fs::write(path.join("a"), b"base\n")?;
        run(path, &["add", "a"])?;
        run(path, &["commit", "-qm", "base"])?;
        std::fs::write(path.join("a"), b"next\n")?;
        run(path, &["commit", "-qam", "next"])?;
        run(path, &["branch", "feature"])?;

        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.state.history = Some(gg_engine::History {
            path: path.to_path_buf(),
            head: "main".into(),
            commits: Vec::new(),
            truncated: false,
            shallow: false,
        });
        app.restore_return_location(
            &ctx,
            return_location::ReturnLocation::Recovery {
                source: return_location::RecoverySource::Reflog,
                reference: Some("main".into()),
                entry: "main@{1}".into(),
            },
        );
        assert!(app.recovery.is_some(), "recovery stays open");
        if let Some(view) = app.recovery.as_mut() {
            assert_eq!(view.reference(), "main");
            assert_eq!(view.selected_selector(), Some("main@{1}"));
            assert_eq!(view.source, return_location::RecoverySource::Reflog);
            // The entry stays selected when the list arrives.
            view.wait();
            assert_eq!(view.selected_selector(), Some("main@{1}"));
        }
        Ok(())
    }

    #[test]
    fn recovery_window_switches_backups_and_reflog_in_its_header() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.recovery = Some(RecoveryView::new(
                std::env::temp_dir().join("a-very-long-repository-name"),
                &ctx,
            ));
            if let Some(view) = app.recovery.as_mut() {
                view.wait();
            }
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(text.iter().any(|line| line.contains("Recovery")));
            assert!(text.iter().any(|line| line.contains("Backups 0")));
            assert!(text.contains(&"Reflog"));
            assert!(!text.iter().any(|line| line.contains("Source")));
            assert!(!text.iter().any(|line| line.contains("Refresh")));
        }
    }

    #[test]
    fn recovery_window_stays_inside_the_narrow_viewport() -> Result<(), Box<dyn std::error::Error>>
    {
        use std::process::Command;
        let run = |path: &std::path::Path, args: &[&str]| -> Result<(), String> {
            let output = Command::new("git")
                .arg("-C")
                .arg(path)
                .args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env(
                    "GIT_CONFIG_GLOBAL",
                    if cfg!(windows) { "NUL" } else { "/dev/null" },
                )
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.org")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.org")
                .output()
                .map_err(|error| error.to_string())?;
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            }
        };
        let dir = tempfile::tempdir()?;
        let path = dir.path();
        run(path, &["init", "-q", "-b", "main"])?;
        run(path, &["config", "user.name", "Test"])?;
        run(path, &["config", "user.email", "test@example.org"])?;
        std::fs::write(path.join("a"), b"base\n")?;
        run(path, &["add", "a"])?;
        run(path, &["commit", "-qm", "base"])?;
        std::fs::write(path.join("a"), b"next\n")?;
        run(path, &["commit", "-qam", "next"])?;
        run(
            path,
            &[
                "update-ref",
                "refs/gg/backup/rewrite/1761490000000000000",
                "HEAD~1",
            ],
        )?;

        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.recovery = Some(RecoveryView::new(path.to_path_buf(), &ctx));
        if let Some(view) = app.recovery.as_mut() {
            view.wait();
        }
        let _ = crate::tests::run_ui(&ctx, frame_input(720.0, 480.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(720.0, 480.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for expected in ["Recovery", "Open commit", "Restore main…", "Delete backup…"] {
            assert!(
                text.iter().any(|line| line.contains(expected)),
                "missing {expected} at 720x480: {text:?}"
            );
        }
        let rect = ctx.memory(|memory| memory.area_rect(egui::Id::unique("recovery")));
        assert!(rect.is_some(), "the recovery window is laid out");
        if let Some(rect) = rect {
            assert!(
                rect.min.x >= 0.0 && rect.max.x <= 720.0,
                "recovery window leaves the viewport horizontally: {rect:?}"
            );
            assert!(
                rect.min.y >= 0.0 && rect.max.y <= 480.0,
                "recovery window leaves the viewport vertically: {rect:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn force_push_dialog_names_the_lease_and_never_offers_bare_force() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.force_push = Some(ForcePushDialog {
                branch: "main".into(),
                remote: "origin".into(),
                remote_branch: "main".into(),
                upstream: "origin/main".into(),
                lease: "a".repeat(40),
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            for expected in [
                "Force push with lease",
                "Rewrite origin/main with local main?",
                "Lease: origin/main at aaaaaaaaaaaa",
                "never uses bare --force",
            ] {
                assert!(
                    text.iter().any(|line| line.contains(expected)),
                    "missing {expected} at {width}: {text:?}"
                );
            }
            assert!(
                !text.iter().any(|line| line == &"Force push"),
                "bare force push control surfaced at {width}: {text:?}"
            );
        }
    }

    #[test]
    fn signature_window_shows_a_banner_and_links_the_signing_settings() {
        let verification = gg_engine::signature::Verification {
            reference: "a".repeat(40),
            kind: Some("commit".into()),
            state: gg_engine::signature::State::Good,
            format: Some("openpgp".into()),
            signer: Some("Test User <test@example.org>".into()),
            key: Some("F4545BDB96888825AA7CC8C7DFC666CDAB5C0CC8".into()),
            trust: Some("ultimate".into()),
            detail: None,
        };
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            let mut window = SignatureWindow::new(verification.reference.clone());
            window.verification = Some(verification.clone());
            app.signature_window = Some(window);
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            for expected in [
                "Signature · commit aaaaaaaa",
                "Good signature",
                "Signed by a key with ultimate trust.",
                "Test User <test@example.org>",
                "F4545BDB96888825AA7CC8C7DFC666CDAB5C0CC8",
                "TRUST",
                "ultimate",
                "Read-only check; nothing is changed.",
                "Signing settings…",
            ] {
                assert!(
                    text.iter().any(|line| line.contains(expected)),
                    "missing {expected} at {width}: {text:?}"
                );
            }
            if width <= 720.0 {
                let rect =
                    ctx.memory(|memory| memory.area_rect(egui::Id::unique("signature-details")));
                assert!(rect.is_some(), "the signature window is laid out");
                if let Some(rect) = rect {
                    assert!(
                        rect.min.x >= 0.0 && rect.max.x <= 720.0,
                        "signature window leaves the viewport: {rect:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn lfs_window_renders_pointer_identity_and_missing_object_state() {
        let oid = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let details = gg_engine::lfs::Details {
            path: "asset.bin".into(),
            path_bytes: b"asset.bin".to_vec(),
            before: None,
            after: Some(gg_engine::lfs::SidePointer {
                oid: oid.into(),
                size: 12345,
                available: false,
            }),
            lfs_installed: false,
            remote: Some("origin".into()),
            note: Some("git-lfs is not installed; fetch is unavailable.".into()),
        };
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            let mut window = LfsWindow::new(crate::diff_view::LfsRequest::Change(
                gg_engine::review::FileChange {
                    path: "asset.bin".into(),
                    path_bytes: b"asset.bin".to_vec(),
                    kind: gg_engine::review::ChangeKind::Modified,
                    old: None,
                    new: None,
                },
            ));
            window.details = Some(details.clone());
            window.content = Some(crate::binary_preview::decode(b"fetched content"));
            app.lfs_window = Some(window);
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            for expected in [
                "LFS",
                "asset.bin",
                "LFS unavailable",
                "01234567",
                "12 KiB",
                "not installed",
                "git-lfs is not installed.",
                "CONTENT PREVIEW · AFTER",
                "Hexadecimal preview",
                "Fetch object…",
            ] {
                assert!(
                    text.iter().any(|line| line.contains(expected)),
                    "missing {expected} at {width}: {text:?}"
                );
            }
            if width <= 720.0 {
                let rect = ctx.memory(|memory| memory.area_rect(egui::Id::unique("lfs-details")));
                assert!(rect.is_some(), "the LFS window is laid out");
                if let Some(rect) = rect {
                    assert!(
                        rect.min.x >= 0.0 && rect.max.x <= 720.0,
                        "LFS window leaves the viewport: {rect:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn submodule_details_window_renders_recorded_state_and_update() {
        let details = gg_engine::submodule::Details {
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
            status: Some("+".into()),
            note: None,
            subjects: Default::default(),
            behind: None,
        };
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            let mut window = SubmoduleWindow::new(crate::diff_view::SubmoduleRequest {
                path: "vendor/lib".into(),
                path_bytes: b"vendor/lib".to_vec(),
                before: Some("b".repeat(40)),
                after: Some("a".repeat(40)),
            });
            window.details = Some(details.clone());
            app.submodule_window = Some(window);
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            for expected in [
                "Submodule",
                "vendor/lib",
                "Different commit",
                "INDEX",
                "same as After",
                "CHECKED OUT",
                "Open nested repository",
                "Update to aaaaaaaa…",
            ] {
                assert!(
                    text.iter().any(|line| line.contains(expected)),
                    "missing {expected} at {width}: {text:?}"
                );
            }
            if width <= 720.0 {
                let rect =
                    ctx.memory(|memory| memory.area_rect(egui::Id::unique("submodule-details")));
                assert!(rect.is_some(), "the submodule window is laid out");
                if let Some(rect) = rect {
                    assert!(
                        rect.min.x >= 0.0 && rect.max.x <= 720.0,
                        "submodule window leaves the viewport: {rect:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn switching_worktree_restores_its_selection_and_filter() {
        let mut app = layout_fixture();
        app.state.selected = Some("a".repeat(40));
        app.state.query = "old".into();
        app.scroll_offset = 44.0;
        app.save_worktree_state();
        let feature = PathBuf::from("/projects/feature");
        app.worktree_states.insert(
            feature.clone(),
            WorktreeState {
                selected: Some("b".repeat(40)),
                query: "new".into(),
                scroll_offset: 88.0,
                narrow_details: false,
                details_visible: false,
            },
        );
        let ctx = egui::Context::default();
        app.switch_worktree(&ctx, feature);
        assert_eq!(app.path, "/projects/feature");
        assert_eq!(app.state.query, "new");
        assert_eq!(app.state.selected.as_deref(), Some("b".repeat(40).as_str()));
        assert_eq!(app.scroll_offset, 88.0);
        assert!(!app.details_visible);
    }

    #[test]
    fn worktree_switch_waits_for_an_active_operation() {
        let mut app = layout_fixture();
        let original = app.path.clone();
        let (_tx, rx) = std::sync::mpsc::channel();
        app.branch = Some(rx);
        let ctx = egui::Context::default();

        app.switch_worktree(&ctx, PathBuf::from("/projects/feature"));

        assert_eq!(app.path, original);
        assert!(app.branch.is_some());
    }

    #[test]
    fn worktree_window_lists_state_and_fits_the_narrow_viewport() {
        let snapshot = gg_engine::worktree::Snapshot {
            worktrees: vec![
                gg_engine::worktree::Worktree {
                    path: "/projects/main".into(),
                    path_bytes: b"/projects/main".to_vec(),
                    head: Some("a".repeat(40)),
                    branch: Some("main".into()),
                    detached: false,
                    bare: false,
                    locked: None,
                    prunable: None,
                    current: true,
                    main: true,
                    dirty: Some(false),
                },
                gg_engine::worktree::Worktree {
                    path: "/projects/feature".into(),
                    path_bytes: b"/projects/feature".to_vec(),
                    head: Some("b".repeat(40)),
                    branch: Some("feature".into()),
                    detached: false,
                    bare: false,
                    locked: None,
                    prunable: None,
                    current: false,
                    main: false,
                    dirty: Some(true),
                },
            ],
        };
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.worktree_window = Some(WorktreeWindow {
                open_created: None,
                snapshot: Some(snapshot.clone()),
                selected: Some(1),
                error: None,
                notice: None,
                create: None,
                confirm_remove: None,
                lock: None,
                busy: None,
                pending: None,
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            for expected in [
                "Worktrees",
                "main",
                "feature",
                "Current",
                "Modified",
                "Open",
                "Remove…",
            ] {
                assert!(
                    text.iter().any(|line| line.contains(expected)),
                    "missing {expected} at {width}: {text:?}"
                );
            }
            if width <= 720.0 {
                let rect = ctx.memory(|memory| memory.area_rect(egui::Id::unique("worktrees")));
                assert!(rect.is_some(), "the worktree window is laid out");
                if let Some(rect) = rect {
                    assert!(
                        rect.min.x >= 0.0 && rect.max.x <= 720.0,
                        "worktree window leaves the viewport: {rect:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn worktree_state_round_trips_selection_filters_and_place() {
        let mut app = layout_fixture();
        app.state.selected = Some("a".repeat(40));
        app.state.query = "needle".into();
        app.scroll_offset = 88.0;
        app.narrow_details = true;
        app.details_visible = false;
        app.save_worktree_state();
        let key = app.worktree_key();
        app.state.selected = None;
        app.state.query.clear();
        app.scroll_offset = 0.0;
        app.narrow_details = false;
        app.details_visible = true;
        app.restore_worktree_state(&key);
        assert_eq!(app.state.selected.as_deref(), Some("a".repeat(40).as_str()));
        assert_eq!(app.state.query, "needle");
        assert_eq!(app.scroll_offset, 88.0);
        assert!(app.narrow_details);
        assert!(!app.details_visible);
    }

    #[test]
    fn reflog_branch_creation_dialog_renders_at_supported_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.create_branch = Some(CreateBranchDialog {
                name: "recovered".into(),
                commit: "b".repeat(40),
                selector: "HEAD@{2}".into(),
                switch: false,
                recovery_source: Some(return_location::RecoverySource::Reflog),
                error: None,
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(text.iter().any(|line| line.contains("Create branch here")));
            assert!(text.iter().any(|line| line.contains("HEAD@{2}")));
            assert!(text.iter().any(|line| line.contains("recovered")));
            assert!(text.iter().any(|line| line == &"Create"));
            assert!(text.iter().any(|line| line == &"Cancel"));
        }
    }

    #[test]
    fn ref_jump_clears_filters_and_retargets_selection() {
        let mut app = layout_fixture();
        app.state.query = "filter".into();
        app.state.selected = Some("old".into());
        app.jump_to_ref("new".into());
        assert!(app.state.query.is_empty());
        assert_eq!(app.state.selected.as_deref(), Some("new"));
        assert!(app.state.selected_row.is_none());
    }

    #[test]
    fn keyboard_selects_offscreen_commit_and_copies_exact_id() {
        let ctx = egui::Context::default();
        let mut app = Browser::new(".".into());
        app.state.history = Some(History {
            path: ".".into(),
            head: "main".into(),
            truncated: false,
            shallow: false,
            commits: (0..100)
                .map(|i| gg_engine::Commit {
                    id: format!("commit-{i}"),
                    parents: vec![],
                    author: "Ada".into(),
                    email: String::new(),
                    time: String::new(),
                    message: format!("Commit {i}"),
                })
                .collect(),
        });
        if let Some(history) = app.state.history.take() {
            app.state.complete(0, Ok(history));
        }
        app.state.selected = Some("commit-98".into());
        app.state.reconcile_selection();
        let _ = crate::tests::run_ui(&ctx, egui::RawInput::default(), |ctx| app.draw_ui(ctx));
        let output = crate::tests::run_ui(
            &ctx,
            egui::RawInput {
                events: vec![
                    key(egui::Key::ArrowDown, egui::Modifiers::NONE),
                    key(egui::Key::C, egui::Modifiers::COMMAND),
                ],
                ..Default::default()
            },
            |ctx| app.draw_ui(ctx),
        );
        assert_eq!(app.state.selected.as_deref(), Some("commit-99"));
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::epaint::Shape::Text(text) if text.galley.job.text == "Commit 99")));

        assert!(
            output.platform_output.commands.iter().any(
                |cmd| matches!(cmd, egui::OutputCommand::CopyText(text) if text == "commit-99")
            )
        );
        assert!(!output.shapes.is_empty());
    }

    #[test]
    fn ctrl_f_opens_search_with_the_history_filter() {
        let ctx = egui::Context::default();
        let mut app = Browser::new(".".into());
        app.state.query = "alpha".into();
        let input = |events| frame_input(1120.0, 740.0, events);
        let _ = crate::tests::run_ui(&ctx, input(vec![]), |ctx| app.draw_ui(ctx));
        let _ = crate::tests::run_ui(
            &ctx,
            input(vec![key(egui::Key::F, egui::Modifiers::COMMAND)]),
            |ctx| app.draw_ui(ctx),
        );
        assert!(ctx.memory(|m| m.has_focus(search_field_id())));
        assert_eq!(app.search_text, "alpha", "search starts from the filter");
        let _ = crate::tests::run_ui(
            &ctx,
            input(vec![key(egui::Key::Escape, egui::Modifiers::NONE)]),
            |ctx| app.draw_ui(ctx),
        );
        assert!(app.search.is_none());
    }

    #[test]
    fn renders_empty_error_and_filtered_snapshots() {
        let ctx = egui::Context::default();
        let mut app = Browser::new(PathBuf::from("."));
        for state in 0..3 {
            app.state.history = Some(History {
                path: ".".into(),
                head: "main".into(),
                commits: if state == 2 {
                    vec![gg_engine::Commit {
                        id: "abc123".into(),
                        parents: vec![],
                        author: "Ada".into(),
                        email: "ada@example.org".into(),
                        time: "2026-01-01".into(),
                        message: "First commit".into(),
                    }]
                } else {
                    vec![]
                },
                truncated: false,
                shallow: false,
            });
            app.state.error =
                (state == 1).then(|| "Could not read repository. Open again to retry.".into());
            app.state.selected = Some("abc123".into());
            app.state.query = "missing".into();
            let output =
                crate::tests::run_ui(&ctx, egui::RawInput::default(), |ctx| app.draw_ui(ctx));
            assert!(!output.shapes.is_empty());
        }
    }
    fn layout_fixture() -> Browser {
        let mut app = Browser::new("/projects/a-very-long-repository-name".into());
        app.state.history = Some(History { path: "/projects/a-very-long-repository-name".into(), head: "feature/a-long-branch-name".into(), truncated: false, shallow: false,
            commits: (0..100).map(|i| gg_engine::Commit { id: format!("{i:040x}"), parents: vec!["b".repeat(40)],
                author: "An author with a very long display name".into(), email: "a-long-address@example.org".into(),
                time: "2026-10-17 12:30:00 +0200".into(), message: format!("Subject {i}\n\nA detailed commit message that wraps within the details pane.") }).collect() });
        if let Some(history) = app.state.history.take() {
            app.state.complete(0, Ok(history));
        }
        app.state.selected = Some(format!("{:040x}", 50));
        app.state.reconcile_selection();
        app
    }

    pub(super) fn frame_input(width: f32, height: f32, events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, height),
            )),
            events,
            ..Default::default()
        }
    }

    #[test]
    fn switching_between_branches_moves_head_without_rereading_history() {
        // History holds HEAD and every ref, so a switch between branches
        // changes no row: HEAD moves, the new tip is selected, and the list
        // is not read again.
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let tip = format!("{:040x}", 7);
        app.state.refs.head = Some(gg_engine::refs::Head {
            name: Some("refs/heads/main".into()),
            detached: false,
            commit: Some(format!("{:040x}", 0)),
        });
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let _ = tx.send(Ok(gg_git::branch::Receipt {
            branch: "feature/login".into(),
            head: tip.clone(),
            created: false,
            warnings: Vec::new(),
            stash: None,
        }));
        app.branch = Some(rx);
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.pending.is_none(), "no history re-read");
        assert!(app.last_load.is_some(), "the watcher sees it as read");
        assert_eq!(
            app.state.refs.head,
            Some(gg_engine::refs::Head {
                name: Some("refs/heads/feature/login".into()),
                detached: false,
                commit: Some(tip.clone()),
            })
        );
        assert_eq!(app.state.selected.as_deref(), Some(tip.as_str()));
        assert_eq!(app.current_branch().as_deref(), Some("feature/login"));
    }

    #[test]
    fn narrow_refs_drawer_opens_over_the_list_jumps_and_closes() -> Result<(), String> {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let branch = |name: &str, commit: usize| gg_engine::refs::RefItem {
            name: name.into(),
            full_name: format!("refs/heads/{name}"),
            kind: gg_engine::refs::RefKind::Branch,
            commit: format!("{commit:040x}"),
        };
        app.state.refs.items = vec![branch("main", 3), branch("topic", 7)];
        let run = |app: &mut Browser, events| {
            crate::tests::run_ui(&ctx, frame_input(720.0, 480.0, events), |ctx| {
                app.draw_ui(ctx)
            })
        };
        let _ = run(&mut app, vec![]);
        let _ = run(
            &mut app,
            vec![key(egui::Key::Backslash, egui::Modifiers::COMMAND)],
        );
        assert!(app.refs_drawer, "Ctrl+\\ opens the drawer below 1120 px");
        let _ = run(&mut app, vec![]);
        let output = run(&mut app, vec![]);
        assert_eq!(
            ctx.memory(|m| m.focused()),
            Some(refs_view::filter_id()),
            "focus starts in the filter"
        );
        let drawer = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == "topic" => {
                    Some(text.pos.x)
                }
                _ => None,
            })
            .ok_or("the drawer lists the branches")?;
        assert!(drawer < REFS_DRAWER_WIDTH, "the drawer is at the left");
        let _ = run(
            &mut app,
            vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
        );
        assert!(!app.refs_drawer, "Esc closes the drawer");
        // Enter shows the chosen ref in History and closes the drawer.
        let _ = run(
            &mut app,
            vec![key(egui::Key::Backslash, egui::Modifiers::COMMAND)],
        );
        let _ = run(&mut app, vec![]);
        let _ = run(
            &mut app,
            vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE)],
        );
        let _ = run(&mut app, vec![key(egui::Key::Enter, egui::Modifiers::NONE)]);
        assert!(!app.refs_drawer);
        assert_eq!(app.state.selected, Some(format!("{:040x}", 7)));
        Ok(())
    }

    #[test]
    fn narrow_details_review_opens_on_the_chosen_file() -> Result<(), String> {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let id = format!("{:040x}", 50);
        let file = |path: &str| commit_stats::FileStat {
            path: path.into(),
            kind: gg_engine::review::ChangeKind::Modified,
            lines: Some((1, 1)),
        };
        app.commit_stats = Some((
            id.clone(),
            commit_stats::Stats {
                files: 2,
                added: 2,
                deleted: 2,
                complete: true,
                rows: vec![file("a.rs"), file("b.rs")],
            },
        ));
        let run = |app: &mut Browser, events| {
            crate::tests::run_ui(&ctx, frame_input(720.0, 480.0, events), |ctx| {
                app.draw_ui(ctx)
            })
        };
        let _ = run(&mut app, vec![]);
        let _ = run(&mut app, vec![key(egui::Key::Space, egui::Modifiers::NONE)]);
        assert!(app.narrow_details);
        let output = run(&mut app, vec![]);
        let texts = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(texts.iter().any(|text| text == "FILES 2"), "{texts:?}");
        let _ = run(
            &mut app,
            vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE)],
        );
        assert_eq!(app.details_file, 1, "Down chooses the next file");
        let _ = run(&mut app, vec![key(egui::Key::Enter, egui::Modifiers::NONE)]);
        let review = app.review.as_ref().ok_or("Enter opens Review")?;
        assert_eq!(review.pending_path(), Some("b.rs"), "on the chosen file");
        assert!(matches!(
            review.return_location(),
            Some(return_location::ReturnLocation::Details(_))
        ));
        Ok(())
    }

    #[test]
    fn narrow_history_drops_author_and_merges_in_sync_pills() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let id = format!("{:040x}", 0);
        app.state.selected = Some(id.clone());
        app.state.reconcile_selection();
        app.state.refs.items = vec![
            gg_engine::refs::RefItem {
                name: "main".into(),
                full_name: "refs/heads/main".into(),
                kind: gg_engine::refs::RefKind::Branch,
                commit: id.clone(),
            },
            gg_engine::refs::RefItem {
                name: "origin/main".into(),
                full_name: "refs/remotes/origin/main".into(),
                kind: gg_engine::refs::RefKind::Remote,
                commit: id,
            },
        ];
        app.state.refs.tracking = vec![gg_engine::refs::Tracking {
            branch: "main".into(),
            full_name: "refs/heads/main".into(),
            upstream: "origin/main".into(),
            upstream_full_name: "refs/remotes/origin/main".into(),
            remote: "origin".into(),
            remote_branch: "main".into(),
            ahead: Some(0),
            behind: Some(0),
        }];
        for (width, narrow) in [(720.0, true), (1120.0, false)] {
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let texts = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let has = |label: &str| texts.iter().any(|text| text == label);
            assert_eq!(has("Author"), !narrow, "Author column at {width}");
            assert!(has("Date") && has("Hash"), "Hash and Date stay at {width}");
            assert_eq!(has("main ⇄ origin"), narrow, "one pill at {width}");
            assert_eq!(has("origin/main"), !narrow, "two pills at {width}");
            assert_eq!(has("Details"), narrow, "Details button at {width}");
        }
    }

    #[test]
    fn compact_layout_preserves_table_width_and_disjoint_panes() {
        for (width, height) in [(720.0, 480.0), (1120.0, 740.0), (1600.0, 900.0)] {
            for scale in [1.0, 2.0] {
                let ctx = egui::Context::default();
                ctx.set_pixels_per_point(scale);
                let mut app = layout_fixture();
                // Settle panel geometry and font layout before checking the complete frame.
                let _ = crate::tests::run_ui(&ctx, frame_input(width, height, vec![]), |ctx| {
                    app.draw_ui(ctx)
                });
                let output =
                    crate::tests::run_ui(&ctx, frame_input(width, height, vec![]), |ctx| {
                        app.draw_ui(ctx)
                    });
                assert!(
                    app.history_rect.width() >= history_view::MIN_WIDTH,
                    "width={width} scale={scale} history={:?} details={:?}",
                    app.history_rect,
                    app.details_rect
                );
                // Menu bar, toolbar, the fixed operation area, and the filter row.
                assert!(
                    app.history_rect.top() <= MENU_BAR_HEIGHT + 34.0 + OPERATION_AREA_HEIGHT + 50.0
                );
                assert!(app.history_rect.bottom() <= height - 20.0);
                // The inspector sits under the list at every width.
                assert!(!app.history_rect.intersects(app.details_rect));
                assert!(app.details_rect.right() <= width);
                for label in ["Repository", "View", "Changes"] {
                    let shape = output.shapes.iter().find_map(|shape| match &shape.shape {
                        egui::epaint::Shape::Text(text) if text.galley.job.text == label => {
                            Some(text)
                        }
                        _ => None,
                    });
                    assert!(shape.is_some_and(
                        |text| text.pos.x >= 0.0 && text.pos.x + text.galley.size().x <= width
                    ));
                }
            }
        }
    }

    #[test]
    fn commits_are_named_with_subject_author_and_date_beside_the_hash() {
        let app = layout_fixture();
        let commit = app.state.rows.values().next().cloned();
        let Some(commit) = commit else {
            unreachable!("the fixture has history rows");
        };
        let summary = app.commit_summary(&commit.id);
        assert!(summary.starts_with(&commit.id[..8]), "{summary}");
        assert!(summary.contains(commit.subject()), "{summary}");
        assert!(summary.contains(&commit.author), "{summary}");
        assert_eq!(
            app.commit_summary("ffffffffffffffff"),
            "ffffffff",
            "unknown: the ID alone"
        );
    }

    type Checked = Result<(), Box<dyn std::error::Error>>;

    /// Run Git in `path` with a fixed identity; its trimmed output.
    fn git_in(path: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.org")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.org")
            .output()?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    /// A repository with commits `one` and `two` of file `a` (and `b` in
    /// `one`), opened in gg.
    fn two_commits() -> Result<(tempfile::TempDir, Browser), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path();
        git_in(path, &["init", "-q", "-b", "main"])?;
        git_in(path, &["config", "core.autocrlf", "false"])?;
        std::fs::write(path.join("a"), b"one\n")?;
        std::fs::write(path.join("b"), b"one\n")?;
        git_in(path, &["add", "a", "b"])?;
        git_in(path, &["commit", "-qm", "one"])?;
        std::fs::write(path.join("a"), b"two\n")?;
        git_in(path, &["commit", "-qam", "two"])?;
        let mut app = Browser::new(path.to_path_buf());
        app.state.history = Some(gg_engine::History {
            path: path.to_path_buf(),
            head: "main".into(),
            commits: Vec::new(),
            truncated: false,
            shallow: false,
        });
        Ok((dir, app))
    }

    /// Draw frames until `done` holds for the shown text (background reads
    /// arrive between frames); the last frame's text.
    fn frames_until(
        ctx: &egui::Context,
        app: &mut Browser,
        events: Vec<egui::Event>,
        mut done: impl FnMut(&mut Browser, &[String]) -> bool,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut events = Some(events);
        loop {
            let output = crate::tests::run_ui(
                ctx,
                frame_input(1120.0, 740.0, events.take().unwrap_or_default()),
                |ctx| app.draw_ui(ctx),
            );
            let text = shown_text(&output);
            if done(app, &text) {
                return Ok(text);
            }
            if std::time::Instant::now() > until {
                return Err(format!("timed out; shown: {text:?}").into());
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    /// Every one of a dialog's background lists has arrived.
    fn lists_ready(app: &mut Browser, slots: &[&'static str]) -> bool {
        slots.iter().all(|slot| {
            app.consequences
                .get_mut(slot)
                .is_some_and(|lines| lines.get().is_some())
        })
    }

    /// Click the shown text `label` (a button's label).
    fn click_label(ctx: &egui::Context, app: &mut Browser, label: &str) -> Checked {
        let output = crate::tests::run_ui(ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == label => {
                    Some(text.visual_bounding_rect().center())
                }
                _ => None,
            })
            .ok_or(format!("{label} not shown"))?;
        let _ = crate::tests::run_ui(ctx, frame_input(1120.0, 740.0, click_at(pos)), |ctx| {
            app.draw_ui(ctx)
        });
        Ok(())
    }

    #[test]
    fn new_branch_switches_after_creation() -> Checked {
        let ctx = egui::Context::default();
        let (dir, mut app) = two_commits()?;
        let path = dir.path();
        let head = git_in(path, &["rev-parse", "HEAD"])?;
        app.load(&ctx);
        frames_until(&ctx, &mut app, vec![], |app, _| {
            app.pending.is_none() && app.state.refs.head.is_some()
        })?;
        app.create_branch = Some(CreateBranchDialog {
            name: "feature".into(),
            commit: head,
            selector: "current HEAD".into(),
            switch: true,
            recovery_source: None,
            error: None,
        });
        frames_until(&ctx, &mut app, vec![], |_, text| {
            text.iter().any(|line| line == "Create")
        })?;
        click_label(&ctx, &mut app, "Create")?;
        // The pointer rests over the history, holding back any reread until
        // the switch has ended; then it leaves and the reread lands.
        frames_until(&ctx, &mut app, vec![], |app, _| app.branch.is_none())?;
        frames_until(&ctx, &mut app, vec![egui::Event::PointerGone], |app, _| {
            app.create_branch.is_none() && app.branch.is_none() && app.pending.is_none()
        })?;
        assert_eq!(
            git_in(path, &["symbolic-ref", "HEAD"])?,
            "refs/heads/feature"
        );
        assert_eq!(
            app.state
                .refs
                .head
                .as_ref()
                .and_then(|head| head.name.as_deref()),
            Some("refs/heads/feature"),
            "the shown HEAD follows the switch"
        );
        assert!(
            app.state
                .refs
                .items
                .iter()
                .any(|item| item.full_name == "refs/heads/feature"),
            "the new branch is listed"
        );
        Ok(())
    }

    #[test]
    fn git_reason_reads_the_last_fatal_or_error_line() {
        assert_eq!(
            git_reason(
                "Branch switch failed (exit status: 128). Review the current branch and hook output before retrying.\nfatal: 'x' is already used by worktree at '/w'"
            ),
            Some("'x' is already used by worktree at '/w'")
        );
        assert_eq!(
            git_reason("hint: a\nerror: first\nfatal: last"),
            Some("last")
        );
        assert_eq!(git_reason("Branch switch failed (exit status: 1)."), None);
        assert_eq!(git_reason("fatal:   "), None);
    }

    #[test]
    fn a_refused_switch_shows_gits_reason() -> Checked {
        let ctx = egui::Context::default();
        let (dir, mut app) = two_commits()?;
        let path = dir.path();
        let other = tempfile::tempdir()?;
        let worktree = other.path().join("onion");
        git_in(
            path,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "onion",
                &worktree.to_string_lossy(),
            ],
        )?;
        app.load(&ctx);
        frames_until(&ctx, &mut app, vec![], |app, _| app.pending.is_none())?;
        app.start_branch(&ctx, "onion".into(), false);
        frames_until(&ctx, &mut app, vec![], |app, _| app.branch.is_none())?;
        let outcome = app.outcome.as_ref().ok_or("no outcome")?;
        assert!(
            outcome.summary.starts_with("Can’t switch to onion: ")
                && outcome.summary.contains("worktree"),
            "the summary gives Git's reason: {}",
            outcome.summary
        );
        assert!(
            !outcome.summary.contains("exit status"),
            "{}",
            outcome.summary
        );
        assert!(
            outcome
                .details
                .as_deref()
                .is_some_and(|details| details.contains("fatal:")),
            "Git's output is behind Details…"
        );
        assert_eq!(git_in(path, &["symbolic-ref", "HEAD"])?, "refs/heads/main");
        Ok(())
    }

    /// The Worktrees window's row for `branch`.
    fn worktree_index(app: &Browser, branch: &str) -> Option<usize> {
        app.worktree_window
            .as_ref()
            .and_then(|window| window.snapshot.as_ref())
            .and_then(|snapshot| {
                snapshot
                    .worktrees
                    .iter()
                    .position(|worktree| worktree.branch.as_deref() == Some(branch))
            })
    }

    #[test]
    fn a_click_on_a_worktree_rows_text_selects_the_row() -> Checked {
        let ctx = egui::Context::default();
        let (dir, mut app) = two_commits()?;
        let path = dir.path();
        let other = tempfile::tempdir()?;
        let linked = other.path().join("linked");
        git_in(
            path,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "linked",
                &linked.to_string_lossy(),
            ],
        )?;
        app.load(&ctx);
        frames_until(&ctx, &mut app, vec![], |app, _| app.pending.is_none())?;
        app.open_worktrees(&ctx);
        frames_until(&ctx, &mut app, vec![], |app, _| {
            app.worktree_window
                .as_ref()
                .is_some_and(|window| window.snapshot.is_some() && window.busy.is_none())
        })?;
        let target = worktree_index(&app, "linked");
        assert_ne!(
            app.worktree_window
                .as_ref()
                .and_then(|window| window.selected),
            target,
            "another row starts selected"
        );
        // Click right on the row's path and branch text, not the gaps.
        let main_index = app
            .worktree_window
            .as_ref()
            .and_then(|window| window.snapshot.as_ref())
            .and_then(|snapshot| snapshot.worktrees.iter().position(|worktree| worktree.main));
        // The path cell, the branch cell and the status pill, each once (a
        // second click on one spot would be a double click, which opens).
        for ends in ["/linked", "linked", "Clean"] {
            if let Some(window) = &mut app.worktree_window {
                window.selected = main_index;
            }
            let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            // The window draws over the refs sidebar, which also lists
            // `linked`: take the last match.
            let pos = output
                .shapes
                .iter()
                .rev()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text)
                        if (ends.starts_with('/') && text.galley.job.text.ends_with(ends))
                            || text.galley.job.text == ends =>
                    {
                        Some(text.visual_bounding_rect().center())
                    }
                    _ => None,
                })
                .ok_or(format!("{ends} not shown"))?;
            // Resting on the text lights the row.
            let hover = vec![egui::Event::PointerMoved(pos)];
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, hover), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            // A filled 22 px row under the pointer (the window may still be
            // fading in, so the colour is not compared).
            let lit = output.shapes.iter().any(|shape| match &shape.shape {
                egui::epaint::Shape::Rect(rect) => {
                    rect.fill.a() > 0
                        && rect.rect.contains(pos)
                        && (rect.rect.height() - inspect_ui::ROW).abs() < 0.5
                }
                _ => false,
            });
            assert!(lit, "resting on {ends:?} lights its row");
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, click_at(pos)), |ctx| {
                app.draw_ui(ctx)
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            assert_eq!(
                app.worktree_window
                    .as_ref()
                    .and_then(|window| window.selected),
                target,
                "a click on {ends:?} selects its row"
            );
        }
        Ok(())
    }

    #[test]
    fn worktrees_window_prunes_missing_and_locks_with_a_reason() -> Checked {
        let ctx = egui::Context::default();
        let (dir, mut app) = two_commits()?;
        let path = dir.path();
        let other = tempfile::tempdir()?;
        let linked = other.path().join("linked");
        let gone = other.path().join("gone");
        for (folder, branch) in [(&linked, "linked"), (&gone, "gone")] {
            git_in(
                path,
                &[
                    "worktree",
                    "add",
                    "-q",
                    "-b",
                    branch,
                    &folder.to_string_lossy(),
                ],
            )?;
        }
        std::fs::remove_dir_all(&gone)?;
        app.load(&ctx);
        frames_until(&ctx, &mut app, vec![], |app, _| app.pending.is_none())?;
        app.open_worktrees(&ctx);
        let listed = |app: &mut Browser, _: &[String]| {
            app.worktree_window
                .as_ref()
                .is_some_and(|window| window.snapshot.is_some() && window.busy.is_none())
        };
        let text = frames_until(&ctx, &mut app, vec![], listed)?;
        for pill in ["Current", "Clean", "Missing"] {
            assert!(text.iter().any(|line| line == pill), "{pill}: {text:?}");
        }
        assert!(
            text.iter().any(|line| line.ends_with("/linked")),
            "the path keeps its own folder: {text:?}"
        );

        // A missing worktree's primary is Prune; Git forgets it, the branch stays.
        let missing = worktree_index(&app, "gone");
        if let Some(window) = &mut app.worktree_window {
            window.selected = missing;
        }
        frames_until(&ctx, &mut app, vec![], |_, text| {
            text.iter().any(|line| line == "Prune 1 missing")
        })?;
        click_label(&ctx, &mut app, "Prune 1 missing")?;
        frames_until(&ctx, &mut app, vec![], |app, _| {
            worktree_index(app, "gone").is_none() && listed(app, &[])
        })?;
        git_in(path, &["rev-parse", "--verify", "refs/heads/gone"])?;

        // Lock… asks for a reason, and Del cannot remove a locked worktree.
        let linked_index = worktree_index(&app, "linked");
        if let Some(window) = &mut app.worktree_window {
            window.selected = linked_index;
            window.lock = Some((linked.clone(), "on a USB disk".into()));
        }
        frames_until(&ctx, &mut app, vec![], |_, text| {
            text.iter().any(|line| line == "Lock worktree")
        })?;
        click_label(&ctx, &mut app, "Lock")?;
        let text = frames_until(&ctx, &mut app, vec![], |app, _| {
            listed(app, &[])
                && app
                    .worktree_window
                    .as_ref()
                    .and_then(|window| window.snapshot.as_ref())
                    .is_some_and(|snapshot| {
                        snapshot
                            .worktrees
                            .iter()
                            .any(|worktree| worktree.locked.as_deref() == Some("on a USB disk"))
                    })
        })?;
        assert!(text.iter().any(|line| line == "Locked"), "{text:?}");
        let _ = frames_until(
            &ctx,
            &mut app,
            vec![egui::Event::Key {
                key: egui::Key::Delete,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            |_, _| true,
        )?;
        assert!(
            app.worktree_window
                .as_ref()
                .is_some_and(|window| window.confirm_remove.is_none()),
            "a locked worktree is not offered for removal"
        );
        Ok(())
    }

    #[test]
    fn reset_runs_as_chosen_and_hard_always_asks() -> Checked {
        let ctx = egui::Context::default();
        let (dir, mut app) = two_commits()?;
        let path = dir.path();
        let first = git_in(path, &["rev-parse", "HEAD^"])?;
        let second = git_in(path, &["rev-parse", "HEAD"])?;
        app.run_commit_action(
            &ctx,
            first.clone(),
            CommitAction::Reset(gg_engine::reset::Mode::Mixed),
        );
        assert_eq!(
            app.reset_confirm,
            Some((first.clone(), gg_engine::reset::Mode::Mixed)),
            "the question comes first by default"
        );
        app.reset_confirm = None;
        app.prefs.confirm_destructive = false;
        app.run_commit_action(
            &ctx,
            first.clone(),
            CommitAction::Reset(gg_engine::reset::Mode::Hard),
        );
        assert_eq!(
            app.reset_confirm,
            Some((first.clone(), gg_engine::reset::Mode::Hard)),
            "Hard asks even when confirmation is off"
        );
        assert_eq!(git_in(path, &["rev-parse", "HEAD"])?, second, "not run");
        app.reset_confirm = None;
        app.run_commit_action(
            &ctx,
            first.clone(),
            CommitAction::Reset(gg_engine::reset::Mode::Soft),
        );
        assert_eq!(app.reset_confirm, None, "Soft runs without the question");
        assert_eq!(git_in(path, &["rev-parse", "HEAD"])?, first);
        assert_eq!(
            git_in(path, &["diff", "--cached", "--name-only"])?,
            "a",
            "Soft, as chosen, keeps the index"
        );
        git_in(path, &["reset", "-q", "--soft", &second])?;
        app.run_commit_action(
            &ctx,
            first.clone(),
            CommitAction::Reset(gg_engine::reset::Mode::Mixed),
        );
        assert_eq!(app.reset_confirm, None, "Mixed runs without the question");
        assert_eq!(git_in(path, &["rev-parse", "HEAD"])?, first);
        assert_eq!(git_in(path, &["diff", "--cached", "--name-only"])?, "");
        assert_eq!(
            std::fs::read(path.join("a"))?,
            b"two\n",
            "working files stay"
        );
        Ok(())
    }

    #[test]
    fn hard_reset_lists_untracked_files_it_would_overwrite_and_refuses() -> Checked {
        let ctx = egui::Context::default();
        let (dir, mut app) = two_commits()?;
        let path = dir.path();
        std::fs::write(path.join("notes"), b"tracked\n")?;
        git_in(path, &["add", "notes"])?;
        git_in(path, &["commit", "-qm", "notes"])?;
        let target = git_in(path, &["rev-parse", "HEAD"])?;
        git_in(path, &["rm", "-q", "--cached", "notes"])?;
        git_in(path, &["commit", "-qm", "untrack notes"])?;
        std::fs::write(path.join("notes"), b"mine\n")?;
        let before = git_in(path, &["rev-parse", "HEAD"])?;
        app.reset_confirm = Some((target, gg_engine::reset::Mode::Hard));
        // The notice follows the list by at most a frame.
        let text = frames_until(&ctx, &mut app, vec![], |_, text| {
            text.iter().any(|line| line == "notes")
                && text
                    .iter()
                    .any(|line| line.contains("Move or delete the untracked files"))
        })?;
        assert!(
            text.iter()
                .any(|line| line.contains("UNTRACKED FILES OVERWRITTEN"))
        );
        assert!(
            text.iter()
                .any(|line| line.contains("Move or delete the untracked files")),
            "{text:?}"
        );
        click_label(&ctx, &mut app, "Hard reset")?;
        let _ = frames_until(&ctx, &mut app, vec![], |_, _| true)?;
        assert!(app.reset_confirm.is_some(), "the dialog stays open");
        assert_eq!(git_in(path, &["rev-parse", "HEAD"])?, before, "refused");
        assert_eq!(std::fs::read(path.join("notes"))?, b"mine\n");
        Ok(())
    }

    #[test]
    fn reset_confirmation_rereads_and_asks_again_after_an_outside_change() -> Checked {
        let ctx = egui::Context::default();
        let (dir, mut app) = two_commits()?;
        let path = dir.path();
        let first = git_in(path, &["rev-parse", "HEAD^"])?;
        app.reset_confirm = Some((first.clone(), gg_engine::reset::Mode::Mixed));
        let _ = frames_until(&ctx, &mut app, vec![], |app, _| {
            lists_ready(app, &["reset-commits", "reset-head"])
        })?;
        // Someone commits while the dialog is open.
        git_in(path, &["commit", "-q", "--allow-empty", "-m", "outside"])?;
        let outside = git_in(path, &["rev-parse", "HEAD"])?;
        click_label(&ctx, &mut app, "Reset")?;
        let text = frames_until(&ctx, &mut app, vec![], |_, text| {
            text.iter().any(|line| line == consequence::CHANGED)
        })?;
        assert!(
            text.iter().any(|line| line.ends_with("outside")),
            "{text:?}"
        );
        assert!(app.reset_confirm.is_some(), "the dialog stays open");
        assert_eq!(git_in(path, &["rev-parse", "HEAD"])?, outside, "not run");
        click_label(&ctx, &mut app, "Reset")?;
        let _ = frames_until(&ctx, &mut app, vec![], |app, _| app.reset_confirm.is_none())?;
        assert_eq!(
            git_in(path, &["rev-parse", "HEAD"])?,
            first,
            "confirmed again"
        );
        Ok(())
    }

    #[test]
    fn discard_all_rereads_and_asks_again_after_an_outside_change() -> Checked {
        let ctx = egui::Context::default();
        let (dir, mut app) = two_commits()?;
        let path = dir.path();
        std::fs::write(path.join("a"), b"mine\n")?;
        app.discard_confirm = true;
        let _ = frames_until(&ctx, &mut app, vec![], |app, _| {
            lists_ready(app, &["discard", "discard-head"])
        })?;
        std::fs::write(path.join("b"), b"also mine\n")?;
        click_label(&ctx, &mut app, "Discard 1 file, staged and unstaged")?;
        let text = frames_until(&ctx, &mut app, vec![], |_, text| {
            text.iter().any(|line| line == consequence::CHANGED)
        })?;
        assert!(
            text.iter()
                .any(|line| line == "Discard 2 files, staged and unstaged"),
            "{text:?}"
        );
        assert!(app.discard_confirm, "the dialog stays open");
        assert_eq!(std::fs::read(path.join("a"))?, b"mine\n", "not run");
        click_label(&ctx, &mut app, "Discard 2 files, staged and unstaged")?;
        let _ = frames_until(&ctx, &mut app, vec![], |app, _| !app.discard_confirm)?;
        assert_eq!(std::fs::read(path.join("a"))?, b"two\n");
        assert_eq!(std::fs::read(path.join("b"))?, b"one\n");
        Ok(())
    }

    #[test]
    fn launch_opens_the_working_folder_or_the_last_repository() {
        let repos = [PathBuf::from("/gone"), PathBuf::from("/repo")];
        let is_repo = |path: &Path| path == Path::new("/repo") || path == Path::new("/here");
        assert_eq!(
            launch_folder("/here".into(), true, &repos, is_repo),
            PathBuf::from("/here"),
            "a repository working folder wins"
        );
        assert_eq!(
            launch_folder("/tmp".into(), true, &repos, is_repo),
            PathBuf::from("/repo"),
            "outside a repository the last one that still is one opens"
        );
        assert_eq!(
            launch_folder("/tmp".into(), false, &repos, is_repo),
            PathBuf::from("/tmp")
        );
    }

    #[test]
    fn preferences_search_keeps_sections_with_a_match() {
        let shown = |query: &str| {
            PrefSection::ALL
                .into_iter()
                .filter(|section| section.matches(query))
                .collect::<Vec<_>>()
        };
        assert_eq!(shown(""), PrefSection::ALL.to_vec());
        assert_eq!(shown("zoom"), [PrefSection::Appearance]);
        assert_eq!(shown("title bar"), [PrefSection::Appearance]);
        assert_eq!(shown("SIGN"), [PrefSection::Commit]);
        assert_eq!(shown("side by"), [PrefSection::Diff]);
        assert!(shown("nothing like this").is_empty());
    }

    #[test]
    fn window_title_names_the_folder_and_a_stopped_rebase() {
        assert_eq!(
            window_title("/home/me/src/gg", Some("/home/me"), None),
            "Git Good — ~/src/gg"
        );
        assert_eq!(
            window_title("/home/meow/gg", Some("/home/me"), None),
            "Git Good — /home/meow/gg"
        );
        let step = |id: char| gg_engine::mutation::RebaseStep {
            action: "pick".into(),
            commit: id.to_string().repeat(40),
            subject: "s".into(),
        };
        let mut progress = gg_engine::mutation::RebaseProgress {
            branch: Some("feature".into()),
            done: vec![step('a'), step('b')],
            todo: vec![step('c'), step('d')],
            stop: gg_engine::mutation::RebaseStop::Other,
            tracked_changes: 0,
        };
        assert_eq!(
            window_title("/r", None, Some(&progress)),
            "Git Good — /r — rebasing feature · step 2 of 4"
        );
        progress.stop = gg_engine::mutation::RebaseStop::Conflict;
        assert_eq!(
            window_title("/r", None, Some(&progress)),
            "Git Good — /r — rebase conflict · step 2 of 4"
        );
    }

    fn welcome_texts(
        ctx: &egui::Context,
        app: &mut Browser,
        events: Vec<egui::Event>,
    ) -> Vec<String> {
        let output = crate::tests::run_ui(ctx, frame_input(1120.0, 740.0, events), |ui| {
            app.draw_ui(ui)
        });
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect()
    }

    /// Frames until `done`, for background reads.
    fn welcome_until(
        ctx: &egui::Context,
        app: &mut Browser,
        done: impl Fn(&Browser, &[String]) -> bool,
    ) -> Vec<String> {
        let mut texts = Vec::new();
        for _ in 0..300 {
            texts = welcome_texts(ctx, app, vec![]);
            if done(app, &texts) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        texts
    }

    fn welcome_key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn recent_list(paths: &[PathBuf]) -> Vec<settings::Recent> {
        paths
            .iter()
            .map(|path| settings::Recent {
                path: path.clone(),
                opened: Some(settings::now() - 7_200),
            })
            .collect()
    }

    #[test]
    fn the_welcome_frame_shows_only_search_and_the_actions() {
        let ctx = egui::Context::default();
        let mut app = Browser::new("/nowhere".into());
        welcome_texts(&ctx, &mut app, vec![]);
        let texts = welcome_texts(&ctx, &mut app, vec![]);
        // Changes is only the menu title; the toolbar tab is gone.
        let count = |label: &str| texts.iter().filter(|text| *text == label).count();
        assert_eq!(count("Changes"), 1, "{texts:?}");
        for hidden in ["History", "no branch", "COMMITS"] {
            assert_eq!(count(hidden), 0, "{hidden} shown: {texts:?}");
        }
        assert!(
            texts
                .iter()
                .any(|text| text.ends_with("Search repositories and actions")),
            "{texts:?}"
        );
        for shown in ["Open…", "Clone…", "New repository…", "git init in a folder"] {
            assert!(
                texts.iter().any(|text| text == shown),
                "{shown} missing: {texts:?}"
            );
        }
        assert!(
            texts
                .iter()
                .any(|text| text == "A folder on this computer · Ctrl+O"),
            "{texts:?}"
        );
    }

    #[test]
    fn the_narrow_welcome_keeps_three_cards_and_drops_logo_version_and_branch() {
        let ctx = egui::Context::default();
        let mut app = Browser::new("/nowhere".into());
        app.recent = recent_list(&[PathBuf::from("/nowhere/narrow")]);
        let texts = |app: &mut Browser, width: f32| {
            crate::tests::run_ui(&ctx, frame_input(width, 480.0, vec![]), |ui| {
                app.draw_ui(ui)
            })
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
        };
        let version = format!("Git Good {}", env!("CARGO_PKG_VERSION"));
        let wide = texts(&mut app, 1120.0);
        assert!(
            wide.iter().any(|text| text.starts_with(&version)),
            "{wide:?}"
        );
        let narrow = texts(&mut app, 720.0);
        assert!(
            !narrow.iter().any(|text| text.starts_with(&version)),
            "{narrow:?}"
        );
        assert!(
            !narrow
                .iter()
                .any(|text| text.contains("Search repositories"))
        );
        for card in ["Open…", "Clone…", "New repository…", "Ctrl+O", "git init"] {
            assert!(
                narrow.iter().any(|text| text == card),
                "{card} missing: {narrow:?}"
            );
        }
    }

    #[test]
    fn first_run_explains_where_repositories_will_appear() {
        let ctx = egui::Context::default();
        let mut app = Browser::new("/nowhere".into());
        let texts = welcome_texts(&ctx, &mut app, vec![]);
        for shown in [
            "Welcome to Git Good",
            "Open a repository to see its history and changes.",
            "Repositories you open are listed here. Drop a folder on this window to open it.",
            "Clone folder:",
            "Change in Preferences…",
        ] {
            assert!(
                texts.iter().any(|text| text == shown),
                "{shown} missing: {texts:?}"
            );
        }
        assert!(!texts.iter().any(|text| text == "RECENT"));
        assert!(!texts.iter().any(|text| text == "Open a repository"));
    }

    #[test]
    fn recent_rows_take_arrows_enter_and_delete() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir()?;
        let paths = ["one", "two", "three"].map(|name| dir.path().join(name));
        for path in &paths {
            std::fs::create_dir(path)?;
        }
        let mut app = Browser::new("/nowhere".into());
        app.recent = recent_list(&paths);
        let texts = welcome_texts(&ctx, &mut app, vec![]);
        assert!(texts.iter().any(|text| text == "Open a repository"));
        assert!(texts.iter().any(|text| text == "RECENT"));
        assert!(texts.iter().any(|text| text == "2 h ago"), "{texts:?}");
        assert_eq!(app.welcome.selected, 0, "the first row is selected");
        welcome_texts(&ctx, &mut app, vec![welcome_key(egui::Key::ArrowDown)]);
        welcome_texts(&ctx, &mut app, vec![welcome_key(egui::Key::ArrowDown)]);
        welcome_texts(&ctx, &mut app, vec![welcome_key(egui::Key::ArrowDown)]);
        assert_eq!(app.welcome.selected, 2);
        welcome_texts(&ctx, &mut app, vec![welcome_key(egui::Key::ArrowUp)]);
        welcome_texts(&ctx, &mut app, vec![welcome_key(egui::Key::Delete)]);
        assert_eq!(
            app.recent
                .iter()
                .map(|entry| entry.path.clone())
                .collect::<Vec<_>>(),
            [paths[0].clone(), paths[2].clone()]
        );
        assert!(paths[1].is_dir(), "removing keeps the folder");
        welcome_texts(&ctx, &mut app, vec![welcome_key(egui::Key::Enter)]);
        assert_eq!(app.welcome.opening.as_ref(), Some(&paths[2]));
        assert_eq!(app.state.requested.as_ref(), Some(&paths[2]));
        // Opening keeps the row selected and the view in place.
        let texts = welcome_texts(&ctx, &mut app, vec![]);
        if app.pending.is_some() {
            assert!(texts.iter().any(|text| text == "RECENT"));
            assert!(
                texts.iter().any(|text| text.starts_with("Reading ")),
                "{texts:?}"
            );
        }
        let texts = welcome_until(&ctx, &mut app, |app, _| app.welcome.failed.is_some());
        let shown = format!("{} is no longer a Git repository.", paths[2].display());
        assert!(texts.contains(&shown), "{texts:?}");
        assert!(texts.iter().any(|text| text == "Remove from recent"));
        assert!(!texts.iter().any(|text| text == "Locate…"));
        assert!(
            app.welcome.launch.is_none(),
            "a row failure stays in its row"
        );
        Ok(())
    }

    #[test]
    fn a_missing_recent_folder_is_marked_and_offers_remove_and_locate()
    -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir()?;
        let gone = dir.path().join("old-site");
        let mut app = Browser::new("/nowhere".into());
        app.recent = recent_list(std::slice::from_ref(&gone));
        welcome_until(&ctx, &mut app, |_, texts| {
            texts.iter().any(|text| text == "missing")
        });
        app.run_welcome_command(&ctx, welcome_view::Command::OpenRecent(gone.clone()));
        let texts = welcome_until(&ctx, &mut app, |app, _| app.welcome.failed.is_some());
        assert_eq!(
            app.welcome.failed,
            Some((gone.clone(), welcome_view::Failure::Missing))
        );
        let shown = format!("{} no longer exists.", gone.display());
        assert!(texts.contains(&shown), "{texts:?}");
        assert!(texts.iter().any(|text| text == "Locate…"));
        click_welcome(&ctx, &mut app, "Remove from recent")?;
        assert!(app.recent.is_empty());
        assert!(app.welcome.failed.is_none());
        // Locate… replaces the entry in place.
        let moved = dir.path().join("moved");
        app.recent = recent_list(&[dir.path().join("a"), gone.clone()]);
        app.relocate_recent(&gone, moved.clone());
        assert_eq!(app.recent[1].path, moved);
        assert_eq!(app.recent.len(), 2);
        Ok(())
    }

    fn click_welcome(ctx: &egui::Context, app: &mut Browser, label: &str) -> Result<(), String> {
        let output = crate::tests::run_ui(ctx, frame_input(1120.0, 740.0, vec![]), |ui| {
            app.draw_ui(ui)
        });
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == label => {
                    Some(text.visual_bounding_rect().center())
                }
                _ => None,
            })
            .ok_or(format!("{label} not shown"))?;
        let _ = crate::tests::run_ui(ctx, frame_input(1120.0, 740.0, click_at(pos)), |ui| {
            app.draw_ui(ui)
        });
        Ok(())
    }

    #[test]
    fn typing_filters_many_recent_repositories_and_esc_clears() {
        let ctx = egui::Context::default();
        let mut app = Browser::new("/nowhere".into());
        let paths = (0..12)
            .map(|index| PathBuf::from(format!("/nowhere/repo-{index}")))
            .collect::<Vec<_>>();
        app.recent = recent_list(&paths);
        let texts = welcome_texts(&ctx, &mut app, vec![]);
        assert!(texts.iter().any(|text| text == "10"), "{texts:?}");
        welcome_texts(&ctx, &mut app, vec![egui::Event::Text("repo-1".into())]);
        assert_eq!(app.welcome.filter, "repo-1");
        let texts = welcome_texts(&ctx, &mut app, vec![]);
        // repo-1 only: repo-10 and repo-11 are past the limit of ten.
        assert!(texts.iter().any(|text| text == "1 of 10"), "{texts:?}");
        welcome_texts(&ctx, &mut app, vec![welcome_key(egui::Key::Escape)]);
        assert!(app.welcome.filter.is_empty());
        // Six or fewer: typing does not filter.
        app.recent.truncate(6);
        welcome_texts(&ctx, &mut app, vec![egui::Event::Text("x".into())]);
        assert!(app.welcome.filter.is_empty());
    }

    #[test]
    fn one_recent_limit_for_the_welcome_list_open_recent_and_open() {
        let ctx = egui::Context::default();
        let mut app = Browser::new("/nowhere".into());
        // Absolute on every platform (Windows paths need a drive).
        let paths = (0..12)
            .map(|index| std::env::temp_dir().join(format!("limit-{index}")))
            .collect::<Vec<_>>();
        app.recent = recent_list(&paths);
        let rows = |texts: &[String]| {
            texts
                .iter()
                .filter(|text| text.starts_with("limit-"))
                .count()
        };
        assert_eq!(
            rows(&welcome_texts(&ctx, &mut app, vec![])),
            settings::RECENT_LIMIT
        );
        let ready = app.frame_readiness();
        let recent = menus::flatten(&app.menu_table(&ready))
            .into_iter()
            .filter(|(_, item)| matches!(item.command, FrameCommand::OpenRecent(_)))
            .count();
        assert_eq!(recent, settings::RECENT_LIMIT);
        app.edit_path = true;
        app.path.clear();
        welcome_texts(&ctx, &mut app, vec![]);
        let texts = welcome_texts(&ctx, &mut app, vec![]);
        let dialog = texts
            .iter()
            .filter(|text| text.starts_with("limit-") && !text.contains('/'))
            .count();
        assert!(dialog >= settings::RECENT_LIMIT, "{texts:?}");
    }

    #[test]
    fn a_launch_folder_that_is_not_a_repository_offers_initialize_here()
    -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir()?;
        let mut app = Browser::new(dir.path().to_path_buf());
        app.load(&ctx);
        let texts = welcome_until(&ctx, &mut app, |app, _| app.welcome.launch.is_some());
        let shown = format!("{} is not a Git repository.", dir.path().display());
        assert!(texts.contains(&shown), "{texts:?}");
        click_welcome(&ctx, &mut app, "Initialize here…")?;
        assert_eq!(
            app.new_repository
                .as_ref()
                .map(|dialog| PathBuf::from(&dialog.folder)),
            Some(dir.path().to_path_buf())
        );
        Ok(())
    }

    #[test]
    fn the_new_repository_dialog_creates_and_opens_the_repository()
    -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir()?;
        let folder = dir.path().join("notes");
        std::fs::create_dir(&folder)?;
        std::fs::write(folder.join("a.md"), b"a")?;
        let mut app = Browser::new("/nowhere".into());
        app.run_welcome_command(
            &ctx,
            welcome_view::Command::NewRepository(Some(folder.clone())),
        );
        if let Some(dialog) = &mut app.new_repository {
            dialog.commit = false;
        }
        let texts = welcome_until(&ctx, &mut app, |app, _| {
            app.new_repository
                .as_ref()
                .is_some_and(|dialog| dialog.inspection().is_some())
        });
        let line = format!("{} has 1 file.", folder.display());
        assert!(texts.contains(&line), "{texts:?}");
        assert!(texts.iter().any(|text| text == "FIRST BRANCH"));
        click_welcome(&ctx, &mut app, "Create repository")?;
        welcome_until(&ctx, &mut app, |app, _| app.new_repository.is_none());
        assert!(folder.join(".git").is_dir());
        assert_eq!(app.state.requested, Some(std::fs::canonicalize(&folder)?));
        assert!(
            app.open_changes_after_load,
            "no commit: Local Changes opens"
        );
        // Inside a repository the dialog refuses and offers to open it.
        let nested = folder.join("docs");
        app.new_repository = Some(new_repository::Dialog::new(Some(nested)));
        let texts = welcome_until(&ctx, &mut app, |app, _| {
            app.new_repository
                .as_ref()
                .is_some_and(|dialog| dialog.inspection().is_some())
        });
        let top = std::fs::canonicalize(&folder)?;
        let open = format!("Open {}", top.display());
        assert!(texts.contains(&open), "{texts:?}");
        assert!(
            texts.iter().any(|text| text.ends_with("Open it instead?")),
            "{texts:?}"
        );
        Ok(())
    }

    #[test]
    fn without_a_repository_menus_stay_complete_and_repository_commands_say_why() {
        let app = Browser::new("/nowhere".into());
        let ready = app.frame_readiness();
        let open = [
            FrameCommand::Open,
            FrameCommand::Clone,
            FrameCommand::NewRepository(None),
            FrameCommand::Preferences,
            FrameCommand::Quit,
            FrameCommand::Shortcuts,
            FrameCommand::Documentation,
            FrameCommand::About,
        ];
        let items = menus::flatten(&app.menu_table(&ready));
        let with_repo =
            menus::flatten(&layout_fixture().menu_table(&layout_fixture().frame_readiness()));
        assert_eq!(
            items.len(),
            with_repo.len(),
            "the menus look the same before and after"
        );
        for (path, item) in items {
            if open.contains(&item.command)
                || matches!(
                    item.command,
                    FrameCommand::SetDensity(_)
                        | FrameCommand::SetDiffLayout(_)
                        | FrameCommand::SetTheme(_)
                        | FrameCommand::Unavailable
                        | FrameCommand::ClearRecent
                )
            {
                continue;
            }
            assert_eq!(
                item.disabled.as_deref(),
                Some("Open a repository first"),
                "{path} › {}",
                item.label
            );
        }
    }

    #[test]
    fn menus_follow_the_final_design_mockup() {
        let app = layout_fixture();
        let ready = app.frame_readiness();
        let table = app.menu_table(&ready);
        let labels = |title: &str| {
            table
                .iter()
                .find(|menu| menu.title == title)
                .map(|menu| {
                    menu.entries
                        .iter()
                        .map(|entry| match entry {
                            menus::Entry::Item(item) => item.label.clone(),
                            menus::Entry::Separator => "-".into(),
                            menus::Entry::Submenu(title, _) => format!("{title} ▸"),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        };
        let titles = table.iter().map(|menu| menu.title).collect::<Vec<_>>();
        if cfg!(target_os = "macos") {
            assert_eq!(
                titles,
                [
                    "Git Good",
                    "Repository",
                    "Branch",
                    "Commit",
                    "Changes",
                    "View",
                    "Help"
                ]
            );
        } else {
            assert_eq!(
                titles,
                ["Repository", "Branch", "Commit", "Changes", "View", "Help"]
            );
            // No Refresh: the window refreshes on file changes and before commands.
            assert_eq!(
                labels("Repository"),
                [
                    "Open…",
                    "Clone…",
                    "New repository…",
                    "Open recent ▸",
                    "-",
                    "Fetch",
                    "Pull (rebase)",
                    "Push",
                    "Push with force-with-lease…",
                    "-",
                    "Worktrees…",
                    "Submodules…",
                    "LFS…",
                    "Recovery…",
                    "-",
                    "Preferences…",
                    "Quit"
                ]
            );
        }
        // Track remote branch… is kept from before the mockup; see the review board.
        assert_eq!(
            labels("Branch"),
            [
                "New branch…",
                "New tag…",
                "Switch to…",
                "-",
                "Merge into current…",
                "Rebase current onto…",
                "Interactive rebase of current branch…",
                "Set upstream…",
                "Track remote branch…",
                "-",
                "Rename…",
                "Delete…"
            ]
        );
        assert_eq!(
            labels("Commit"),
            [
                "Review changes",
                "Compare with HEAD",
                "Copy ID",
                "-",
                "Cherry-pick onto HEAD",
                "Revert",
                "-",
                "Interactive rebase from here…",
                "Reword…",
                "Reopen for editing…",
                "Fixup into…",
                "Drop…",
                "-",
                "Reset current branch to here…"
            ]
        );
        assert_eq!(
            labels("Changes"),
            [
                "Stage all",
                "Unstage all",
                "Discard all tracked changes…",
                "-",
                "Commit",
                "Amend HEAD",
                "-",
                "Stash changes…",
                "Stashes…",
                "-",
                "Resolve conflicts…"
            ]
        );
        assert_eq!(
            labels("View"),
            [
                "Local Changes",
                "History",
                "-",
                "Refs sidebar",
                "Commit inspector",
                "-",
                "Inline diff",
                "Side-by-side diff",
                "-",
                "Compact density",
                "Theme: Dark ▸"
            ]
        );
        assert_eq!(
            labels("Help"),
            ["Keyboard shortcuts", "Documentation", "About Git Good"]
        );
    }

    #[test]
    fn ctrl_shift_b_opens_the_branch_switcher_before_other_keys() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let mut app = layout_fixture();
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let _ = crate::tests::run_ui(
            &ctx,
            frame_input(
                1120.0,
                740.0,
                vec![key(
                    egui::Key::B,
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                )],
            ),
            |ctx| app.draw_ui(ctx),
        );
        let ready = app.frame_readiness();
        let switch = menus::flatten(&app.menu_table(&ready))
            .into_iter()
            .find(|(_, item)| item.command == FrameCommand::SwitchTo)
            .and_then(|(_, item)| item.disabled);
        assert_eq!(switch, None, "Switch to… is enabled");
        assert!(egui::Popup::is_id_open(&ctx, branch_switcher_popup_id()));
        assert!(app.create_branch.is_none(), "Ctrl+Shift+B is not Ctrl+B");
    }

    #[test]
    fn the_system_title_bar_preference_keeps_the_window_decorations() -> Result<(), String> {
        let mut prefs = settings::Prefs::default();
        assert_eq!(custom_titlebar(&prefs), !cfg!(target_os = "macos"));
        prefs.system_title_bar = true;
        assert!(!custom_titlebar(&prefs));
        let icon = branding::icon()?.clone();
        let size = egui::vec2(1120.0, 740.0);
        assert_eq!(
            native_options(false, size, icon.clone())
                .viewport
                .decorations,
            Some(true)
        );
        let custom = native_options(true, size, icon).viewport;
        assert_eq!(custom.decorations, Some(false));
        assert_eq!(custom.min_inner_size, Some(egui::vec2(720.0, 480.0)));
        Ok(())
    }

    /// The macOS menu bar is gg's own table laid out the Mac way; built and
    /// checked on every platform.
    #[test]
    fn the_mac_menu_bar_comes_from_the_command_table() {
        let app = layout_fixture();
        let table = app.menu_table(&app.frame_readiness());
        let (bar, commands) = native_menu::from_table(&table, native_menu_role);
        let layout = native_menu::mac_layout(&bar);
        assert!(layout.clashes.is_empty(), "{:?}", layout.clashes);
        let titles: Vec<&str> = layout.bar.menus.iter().map(|m| m.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "Git Good",
                "Repository",
                "Branch",
                "Commit",
                "Changes",
                "View",
                "Window",
                "Help"
            ]
        );
        let app_menu: Vec<(String, Option<String>)> = layout.bar.menus[0]
            .children
            .iter()
            .filter_map(|node| match node {
                native_menu::Node::Item(item) => Some((item.label.clone(), item.shortcut.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(
            app_menu,
            [
                ("About Git Good".to_string(), None),
                ("Settings…".to_string(), Some("Cmd+,".to_string())),
                ("Hide Git Good".to_string(), Some("Cmd+H".to_string())),
                ("Quit Git Good".to_string(), Some("Cmd+Q".to_string())),
            ]
        );
        let command = |label: &str| {
            layout
                .bar
                .items()
                .into_iter()
                .find(|item| item.label == label)
                .and_then(|item| commands.get(&item.id))
                .cloned()
        };
        assert_eq!(command("Settings…"), Some(FrameCommand::Preferences));
        assert_eq!(
            command("Quit Git Good"),
            Some(FrameCommand::Quit),
            "Git Good's own Quit"
        );
        assert_eq!(command("About Git Good"), Some(FrameCommand::About));
        // Every in-window item is there with its label, enablement and check.
        let native = layout.bar.items();
        for (_, item) in menus::flatten(&table) {
            if native_menu_role(&item.command).is_some() {
                continue;
            }
            let found = native.iter().find(|n| n.label == item.label);
            assert!(
                found.is_some_and(|n| n.enabled == item.disabled.is_none()
                    && n.checked == item.checked
                    && n.shortcut.as_deref() == (!item.key.is_empty()).then_some(item.key)),
                "{} differs: {found:?}",
                item.label
            );
        }
        // Every shortcut is the key press gg's handling already reads.
        for item in &native {
            let Some(shortcut) = item.shortcut.as_deref() else {
                continue;
            };
            let chord = native_menu::Chord::parse(shortcut);
            assert!(
                chord.and_then(|c| c.key_event(true)).is_some(),
                "{shortcut} on {}",
                item.label
            );
        }
        let copy = native_menu::Chord::parse("Ctrl+C");
        assert!(
            copy.is_some_and(|c| !c.native_ok()),
            "⌘C stays the window's, for text fields"
        );
    }

    #[test]
    fn selection_changes_update_the_mac_menu_in_place() {
        let mut app = layout_fixture();
        let key = |app: &Browser| {
            let table = app.menu_table(&app.frame_readiness());
            native_menu::structure_key(
                &native_menu::mac_layout(&native_menu::from_table(&table, native_menu_role).0).bar,
            )
        };
        let selected = key(&app);
        app.state.selected = None;
        assert_eq!(key(&app), selected, "enablement changes in place");
        app.recent = vec![settings::Recent {
            path: "/projects/other".into(),
            opened: None,
        }];
        assert_ne!(key(&app), selected, "a new Open recent entry rebuilds");
    }

    /// Windows and Linux: the menu bar is the title bar.
    #[test]
    fn gg_draws_the_title_bar_with_menus_identity_and_caption_buttons() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let mut app = layout_fixture();
        app.chrome.custom = true;
        let frame = |app: &mut Browser, events| {
            crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, events), |ui| {
                app.draw_ui(ui)
            })
        };
        let _ = frame(&mut app, vec![]);
        let output = frame(&mut app, vec![]);
        let close = ctx
            .read_response(titlebar::caption_id(titlebar::Caption::Close))
            .map(|r| r.rect)
            .ok_or("no Close button")?;
        let minimize = ctx
            .read_response(titlebar::caption_id(titlebar::Caption::Minimize))
            .map(|r| r.rect)
            .ok_or("no Minimize button")?;
        assert_eq!((close.right(), close.top()), (1120.0, 0.0));
        assert!(close.bottom() <= MENU_BAR_HEIGHT);
        let text = |name: &str| {
            output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == name => {
                    Some(text.visual_bounding_rect())
                }
                _ => None,
            })
        };
        let help = text("Help").ok_or("no Help menu")?;
        let identity = text(&app.window_title).ok_or("no identity")?;
        assert!(identity.left() > help.right() && identity.right() < minimize.left());
        assert!(
            (identity.center().x - 560.0).abs() < 2.0,
            "centred: {identity:?}"
        );
        // Close runs gg's Quit, which closes the window.
        let mut commands = vec![];
        for events in [
            vec![egui::Event::PointerMoved(close.center())],
            vec![egui::Event::PointerButton {
                pos: close.center(),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            vec![egui::Event::PointerButton {
                pos: close.center(),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        ] {
            let mut out = frame(&mut app, events);
            commands.extend(
                out.viewport_output
                    .remove(&egui::ViewportId::ROOT)
                    .map(|v| v.commands)
                    .unwrap_or_default(),
            );
        }
        assert!(
            commands.contains(&egui::ViewportCommand::Close),
            "{commands:?}"
        );
        Ok(())
    }

    /// macOS: with the native menu bar the in-window menus are hidden, the
    /// toolbar takes the top row, and native clicks run gg's commands.
    #[test]
    fn the_native_menu_bar_replaces_the_in_window_menus() -> Result<(), String> {
        use std::cell::RefCell;
        use std::rc::Rc;
        #[derive(Default)]
        struct Fake {
            bars: Vec<native_menu::MenuBar>,
            events: Vec<native_menu::Event>,
        }
        struct Shared(Rc<RefCell<Fake>>);
        impl native_menu::Backend for Shared {
            fn sync(&mut self, bar: &native_menu::MenuBar) {
                self.0.borrow_mut().bars.push(bar.clone());
            }
            fn drain(&mut self) -> Vec<native_menu::Event> {
                std::mem::take(&mut self.0.borrow_mut().events)
            }
        }
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let fake = Rc::new(RefCell::new(Fake::default()));
        let mut app = layout_fixture();
        app.native_menu = Some(native_menu::NativeMenu::new(Box::new(Shared(Rc::clone(
            &fake,
        )))));
        let frame = |app: &mut Browser| {
            let mut input = frame_input(1120.0, 740.0, vec![]);
            app.window_input(&ctx, &mut input);
            crate::tests::run_ui(&ctx, input, |ui| app.draw_ui(ui))
        };
        let _ = frame(&mut app);
        let output = frame(&mut app);
        assert!(
            !fake.borrow().bars.is_empty(),
            "the menus went to the backend"
        );
        let in_window = output.shapes.iter().any(|shape| {
            matches!(&shape.shape, egui::epaint::Shape::Text(text) if text.galley.job.text == "Repository")
        });
        assert!(!in_window, "no in-window menu bar");
        let about = fake
            .borrow()
            .bars
            .last()
            .and_then(|bar| {
                bar.items()
                    .into_iter()
                    .find(|i| i.label == "About Git Good")
                    .map(|i| i.id.clone())
            })
            .ok_or("no About Git Good")?;
        fake.borrow_mut().events = vec![native_menu::Event::Click(about)];
        let _ = frame(&mut app);
        assert!(app.about_open, "the click ran Help › About Git Good");
        Ok(())
    }

    #[test]
    fn menu_bar_is_one_row_and_view_shortcuts_switch_views() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let mut app = layout_fixture();
        let frame = |app: &mut Browser, events| {
            crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, events), |ctx| {
                app.draw_ui(ctx)
            })
        };
        let _ = frame(&mut app, vec![]);
        let output = frame(&mut app, vec![]);
        for title in ["Repository", "Branch", "Commit", "Changes", "View", "Help"] {
            let y = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) if text.galley.job.text == title => {
                        Some(text.visual_bounding_rect().center().y)
                    }
                    _ => None,
                })
                .ok_or(format!("menu {title} missing"))?;
            assert!(
                y < MENU_BAR_HEIGHT,
                "{title} sits in the 26 px menu bar: {y}"
            );
        }
        let _ = frame(
            &mut app,
            vec![key(egui::Key::Num1, egui::Modifiers::COMMAND)],
        );
        assert!(app.working.is_some(), "Ctrl+1 opens Local Changes");
        let has_sidebar = |output: &egui::FullOutput| {
            output.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::epaint::Shape::Text(text) if text.galley.job.text == "BRANCHES")
            })
        };
        let changes = frame(&mut app, vec![]);
        assert!(
            !has_sidebar(&changes),
            "Local Changes hides the refs sidebar"
        );
        let _ = frame(
            &mut app,
            vec![key(egui::Key::Backslash, egui::Modifiers::COMMAND)],
        );
        let changes = frame(&mut app, vec![]);
        assert!(
            has_sidebar(&changes),
            "the toggle shows it in Local Changes"
        );
        let _ = frame(
            &mut app,
            vec![key(egui::Key::Backslash, egui::Modifiers::COMMAND)],
        );
        let _ = frame(
            &mut app,
            vec![key(egui::Key::Num2, egui::Modifiers::COMMAND)],
        );
        assert!(app.working.is_none(), "Ctrl+2 returns to History");
        let history = frame(&mut app, vec![]);
        assert!(
            has_sidebar(&history),
            "History keeps its own sidebar choice"
        );
        let shown = app.refs_visible;
        let _ = frame(
            &mut app,
            vec![key(egui::Key::Backslash, egui::Modifiers::COMMAND)],
        );
        assert_eq!(app.refs_visible, !shown, "Ctrl+\\ toggles the refs sidebar");
        Ok(())
    }

    #[test]
    fn local_changes_and_history_share_one_navigation_strip_at_every_width()
    -> Result<(), Box<dyn std::error::Error>> {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.state.query = "Subject".into();
            app.restore_offset = Some(123.0);
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let selected = app.state.selected.clone();
            let query = app.state.query.clone();
            let offset = app.scroll_offset;
            app.open_working(&ctx);
            let working = app.working.as_mut().ok_or("missing working view")?;
            working.set_draft("strip draft".into());
            app.close_to_history(&ctx);
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let nav = |label: &str| {
                output.shapes.iter().find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text)
                        if text.galley.job.text == label
                            && (MENU_BAR_HEIGHT..MENU_BAR_HEIGHT + 34.0).contains(&text.pos.y) =>
                    {
                        Some(text.pos)
                    }
                    _ => None,
                })
            };
            let local = nav("Changes").ok_or("missing Local Changes")?;
            let history = nav("History").ok_or("missing History")?;
            assert!(
                local.x < history.x,
                "Local Changes must stay first at {width}: {local:?} {history:?}"
            );
            assert!(
                (local.y - history.y).abs() < 1.0,
                "destinations share one strip at {width}: {local:?} {history:?}"
            );
            assert!(
                output.shapes.iter().any(|shape| matches!(
                    &shape.shape,
                    egui::epaint::Shape::Text(text) if text.galley.job.text == "Branch"
                )),
                "ref browsing stays reachable at {width}"
            );
            let click = |ctx: &egui::Context, app: &mut Browser, pos: egui::Pos2| {
                let press = egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                };
                let release = egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                };
                crate::tests::run_ui(
                    ctx,
                    frame_input(width, 740.0, vec![press, release]),
                    |ctx| app.draw_ui(ctx),
                )
            };
            click(&ctx, &mut app, local + egui::vec2(8.0, 6.0));
            assert!(app.working.is_some(), "width {width}");
            assert_eq!(app.state.selected, selected);
            assert_eq!(app.state.query, query);

            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            for label in ["Repository", "Changes", "Commit", "Amend HEAD"] {
                assert!(
                    output.shapes.iter().any(|shape| matches!(
                        &shape.shape,
                        egui::epaint::Shape::Text(text) if text.galley.job.text == label
                    )),
                    "missing {label} at width {width}"
                );
            }
            if width >= 1120.0 {
                let positions = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::epaint::Shape::Text(text)
                            if text.galley.job.text == "Changes"
                                && text.pos.y > MENU_BAR_HEIGHT =>
                        {
                            Some(text.pos.x)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    positions.len(),
                    1,
                    "one stable strip entry without a duplicate workspace heading at {width}: {positions:?}"
                );
                assert!(positions.iter().any(|x| *x < 190.0), "{positions:?}");
            }
            let history = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text)
                        if text.galley.job.text == "History" && text.pos.y < 80.0 =>
                    {
                        Some(text.pos)
                    }
                    _ => None,
                })
                .ok_or("missing History strip entry")?;
            click(&ctx, &mut app, history + egui::vec2(8.0, 6.0));
            assert!(app.working.is_none(), "width {width}");
            assert_eq!(app.state.selected, selected);
            assert_eq!(app.state.query, query);
            assert!(
                (app.scroll_offset - offset).abs() < 1.0,
                "offset {} != {offset} at {width}",
                app.scroll_offset
            );
            app.open_working(&ctx);
            assert_eq!(
                app.working
                    .as_ref()
                    .map(status_view::StatusView::draft)
                    .unwrap_or_default(),
                "strip draft"
            );
        }
        Ok(())
    }

    #[test]
    fn review_close_restores_the_recorded_history_location() {
        for action in [CommitAction::Review, CommitAction::Compare] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.state.query = "Subject".into();
            app.restore_offset = Some(321.0);
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let selected = app.state.selected.clone();
            let query = app.state.query.clone();
            let offset = app.scroll_offset;
            let target = format!("{:040x}", 10);
            app.run_commit_action(&ctx, target.clone(), action);
            assert!(app.review.is_some());
            assert_eq!(app.state.selected.as_deref(), Some(target.as_str()));
            let _ = crate::tests::run_ui(
                &ctx,
                frame_input(
                    1120.0,
                    740.0,
                    vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
                ),
                |ctx| app.draw_ui(ctx),
            );
            assert!(app.review.is_none());
            assert_eq!(app.state.selected, selected);
            assert_eq!(app.state.query, query);
            assert!((app.scroll_offset - offset).abs() < 1.0);
        }
    }

    fn click_navigation(label: &str, app: &mut Browser, ctx: &egui::Context, width: f32) {
        let output = crate::tests::run_ui(ctx, frame_input(width, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let Some(position) = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text)
                if text.galley.job.text == label && text.pos.y < 80.0 =>
            {
                Some(text.pos + egui::vec2(8.0, 6.0))
            }
            _ => None,
        }) else {
            return;
        };
        for event in [
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ] {
            let _ = crate::tests::run_ui(ctx, frame_input(width, 740.0, vec![event]), |ctx| {
                app.draw_ui(ctx)
            });
        }
        let _ = crate::tests::run_ui(
            ctx,
            frame_input(
                width,
                740.0,
                vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            ),
            |ctx| app.draw_ui(ctx),
        );
    }

    #[test]
    fn window_toolbar_keeps_the_owners_buttons_in_place_across_views_and_widths()
    -> Result<(), Box<dyn std::error::Error>> {
        // Sync, new branch, stash and its picker, rebase from here, and
        // recovery; New tag and Merge live in the menus.
        let order = [
            crate::phosphor::SIDEBAR_SIMPLE,
            crate::phosphor::DOWNLOAD_SIMPLE,
            crate::phosphor::ARROW_LINE_DOWN,
            crate::phosphor::ARROW_LINE_UP,
            crate::phosphor::GIT_BRANCH,
            crate::phosphor::ARCHIVE,
            crate::phosphor::BOX_ARROW_UP,
            crate::phosphor::LIST_NUMBERS,
            crate::phosphor::CLOCK_COUNTER_CLOCKWISE,
            crate::phosphor::MAGNIFYING_GLASS,
        ];
        // The branch button adds its own glyph and caret; nothing else.
        let allowed = [
            crate::phosphor::SIDEBAR_SIMPLE,
            crate::phosphor::DOWNLOAD_SIMPLE,
            crate::phosphor::ARROW_LINE_DOWN,
            crate::phosphor::ARROW_LINE_UP,
            crate::phosphor::GIT_BRANCH,
            crate::phosphor::ARCHIVE,
            crate::phosphor::BOX_ARROW_UP,
            crate::phosphor::LIST_NUMBERS,
            crate::phosphor::CLOCK_COUNTER_CLOCKWISE,
            crate::phosphor::CARET_DOWN,
            crate::phosphor::MAGNIFYING_GLASS,
        ];
        let icon_glyph = |text: &str| {
            text.chars().count() == 1
                && text.chars().all(|c| ('\u{E000}'..='\u{F8FF}').contains(&c))
        };
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            crate::phosphor::install(&ctx);
            let mut app = layout_fixture();
            let mut positions = Vec::new();
            for local in [false, true] {
                if local {
                    app.open_working(&ctx);
                }
                let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                    app.draw_ui(ctx)
                });
                let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                    app.draw_ui(ctx)
                });
                let xs = order
                    .iter()
                    .map(|glyph| {
                        output
                            .shapes
                            .iter()
                            .find_map(|shape| match &shape.shape {
                                // The search field's hint starts with its glyph.
                                egui::epaint::Shape::Text(text)
                                    if (text.galley.job.text == *glyph
                                        || text
                                            .galley
                                            .job
                                            .text
                                            .starts_with(&format!("{glyph} ")))
                                        && text.pos.y < 90.0 =>
                                {
                                    Some(text.pos.x)
                                }
                                _ => None,
                            })
                            .ok_or(format!("missing toolbar glyph {glyph:?} at {width}"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let extra = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::epaint::Shape::Text(text)
                            if (MENU_BAR_HEIGHT..MENU_BAR_HEIGHT + TOOLBAR_HEIGHT)
                                .contains(&text.pos.y)
                                && icon_glyph(&text.galley.job.text)
                                && !allowed.contains(&text.galley.job.text.as_str()) =>
                        {
                            Some(text.galley.job.text.clone())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert!(
                    extra.is_empty(),
                    "only navigation and sync stay on the toolbar at {width}: {extra:?}"
                );
                assert!(
                    xs.windows(2).all(|pair| pair[0] + 17.0 <= pair[1]),
                    "toolbar icons keep their order without overlapping at {width}: {xs:?}"
                );
                assert!(xs.iter().all(|x| *x + 17.0 <= width), "{xs:?} at {width}");
                positions.push(xs);
            }
            assert_eq!(
                positions[0], positions[1],
                "History and Local Changes keep one toolbar at {width}"
            );
        }
        Ok(())
    }

    #[test]
    fn branch_menu_rewrite_covers_the_branch_and_keeps_the_selection()
    -> Result<(), Box<dyn std::error::Error>> {
        let git = |path: &std::path::Path, args: &[&str]| -> Result<String, String> {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(path)
                .args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env(
                    "GIT_CONFIG_GLOBAL",
                    if cfg!(windows) { "NUL" } else { "/dev/null" },
                )
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.org")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.org")
                .output()
                .map_err(|error| error.to_string())?;
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).into_owned())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            }
        };
        let dir = tempfile::tempdir()?;
        let path = dir.path();
        git(path, &["init", "-q", "-b", "main"])?;
        for i in 0..4 {
            if i == 2 {
                git(path, &["switch", "-q", "-c", "feature"])?;
            }
            std::fs::write(path.join("a"), format!("{i}\n"))?;
            git(path, &["add", "a"])?;
            git(path, &["commit", "-qm", &format!("change {i}")])?;
        }
        let ids = git(path, &["rev-list", "HEAD"])?
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let mut app = Browser::new(path.to_path_buf());
        app.state.complete(
            0,
            Ok(gg_engine::History {
                path: path.to_path_buf(),
                head: "main".into(),
                commits: ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| gg_engine::Commit {
                        id: id.clone(),
                        parents: ids.get(i + 1).cloned().into_iter().collect(),
                        author: "Test".into(),
                        email: "test@example.org".into(),
                        time: "2026-10-30 12:00:00 +0000".into(),
                        message: format!("change {}", 3 - i),
                    })
                    .collect(),
                truncated: false,
                shallow: false,
            }),
        );
        // Select an older commit on main: the branch rewrite must not depend on it.
        let selected = ids[3].clone();
        app.state.selected = Some(selected.clone());
        app.state.reconcile_selection();
        app.local_summary = Some(LocalSummary::read(path.to_path_buf()));
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let ready = app.frame_readiness();
        let (menu, item) = menus::flatten(&app.menu_table(&ready))
            .into_iter()
            .find(|(_, item)| item.command == FrameCommand::Rewrite)
            .ok_or("Interactive rebase of current branch… has no menu home")?;
        assert_eq!(menu, "Branch");
        assert_eq!(item.disabled, None, "the branch has own commits");
        let (mut local, mut history) = (false, false);
        app.run_frame_command(&ctx, item.command, &mut local, &mut history);
        let dialog = app
            .rewrite_dialog
            .as_ref()
            .ok_or("rewrite dialog did not open")?;
        assert_eq!(
            app.state.selected.as_deref(),
            Some(selected.as_str()),
            "the branch rewrite leaves the history selection alone"
        );
        assert_eq!(
            dialog.from, ids[1],
            "starts at the branch's first own commit"
        );
        assert_eq!(
            dialog.plan.rows.len(),
            2,
            "the branch's own commits through HEAD"
        );
        Ok(())
    }

    #[test]
    fn stash_picker_keys_select_ask_to_drop_and_apply() {
        let ctx = egui::Context::default();
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let mut picker = StashPicker::default();
        let press = |picker: &mut StashPicker, keys: Vec<egui::Key>| {
            let mut result = None;
            let input = egui::RawInput {
                events: keys.into_iter().map(key).collect(),
                ..Default::default()
            };
            let _ = crate::tests::run_ui(&ctx, input, |ui| {
                result = stash_picker_keys(ui.ctx(), picker, 3);
            });
            result
        };
        assert_eq!(
            press(
                &mut picker,
                vec![
                    egui::Key::ArrowDown,
                    egui::Key::ArrowDown,
                    egui::Key::ArrowDown
                ]
            ),
            None
        );
        assert_eq!(picker.selected, 2, "stops at the last entry");
        press(&mut picker, vec![egui::Key::ArrowUp, egui::Key::Delete]);
        assert_eq!(picker.selected, 1);
        assert!(picker.confirm_drop);
        press(&mut picker, vec![egui::Key::Escape]);
        assert!(!picker.confirm_drop);
        assert_eq!(press(&mut picker, vec![egui::Key::Enter]), Some(true));
    }

    #[test]
    fn stash_picker_lists_entries_with_restore_show_and_drop()
    -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let mut app = layout_fixture();
        app.local_summary = Some(LocalSummary {
            worktrees: Vec::new(),
            changes: Some(2),
            stashes: (0..2)
                .map(|index| gg_engine::stash::StashEntry {
                    index,
                    reference: format!("stash@{{{index}}}"),
                    commit: format!("{index:040x}"),
                    message: format!("WIP on main: abc123 work {index}"),
                    branch: (index == 0).then(|| "main".to_string()),
                    age: format!("{} hours ago", index + 2),
                    files: Some(index as usize * 2 + 1),
                })
                .collect(),
            branch_start: None,
            rebase: None,
            files: Vec::new(),
        });
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        // Changes › Stashes… opens the picker; there is no toolbar button.
        let ready = app.frame_readiness();
        let (menu, item) = menus::flatten(&app.menu_table(&ready))
            .into_iter()
            .find(|(_, item)| item.command == FrameCommand::Stashes)
            .ok_or("Stashes… has no menu home")?;
        assert_eq!(
            (menu.as_str(), item.label.as_str()),
            ("Changes", "Stashes…")
        );
        assert_eq!(item.disabled, None);
        let (mut local, mut history) = (false, false);
        app.run_frame_command(&ctx, item.command, &mut local, &mut history);
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let texts = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for expected in [
            "Apply stash@{0}",
            "Pop",
            "Show changes",
            "Drop…",
            // Ref, message without the branch prefix, then branch, age, and
            // file count on the second line.
            "stash@{0}",
            "abc123 work 0",
            "on main · 2 hours ago · 1 file",
            "on detached · 3 hours ago · 3 files",
        ] {
            assert!(
                texts.iter().any(|text| text == expected),
                "missing {expected:?} in {texts:?}"
            );
        }
        assert!(
            texts.iter().any(|text| text.contains("stash@{1}")),
            "both entries are listed"
        );

        // The popup keeps its size when Drop… swaps in the confirmation.
        let before = ctx
            .memory(|memory| memory.area_rect(stash_picker_popup_id()))
            .ok_or("the picker popup has no area")?;
        let drop = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == "Drop…" => {
                    Some(text.pos + text.galley.size() / 2.0)
                }
                _ => None,
            })
            .ok_or("missing Drop…")?;
        let click = |pos| {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, click(drop)), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.stash_picker.confirm_drop, "Drop… asks for confirmation");
        assert!(
            output.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::epaint::Shape::Text(text) if text.galley.job.text.starts_with("Drop stash@{0}?")
            )),
            "the confirmation names the entry"
        );
        let after = ctx
            .memory(|memory| memory.area_rect(stash_picker_popup_id()))
            .ok_or("the picker popup closed")?;
        assert_eq!(before, after, "the popup does not resize");
        Ok(())
    }

    #[test]
    fn decision_dialogs_block_the_window_behind_them() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let frame = |app: &mut Browser, events: Vec<egui::Event>| {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, events), |ctx| {
                app.draw_ui(ctx)
            });
        };
        let click = |pos: egui::Pos2| {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        frame(&mut app, vec![]);
        let selected = app.state.selected.clone();
        let row = app
            .row_rects
            .iter()
            .find(|(row, _)| *row == 3)
            .map(|(_, rect)| rect.left_center() + egui::vec2(40.0, 0.0));
        let Some(row) = row else {
            unreachable!("row 3 is not painted");
        };
        app.open_merge_dialog();
        frame(&mut app, vec![]);
        frame(&mut app, vec![]);
        frame(&mut app, click(row));
        assert_eq!(
            app.state.selected, selected,
            "clicks behind a modal dialog are ignored"
        );
        frame(
            &mut app,
            vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE)],
        );
        assert_eq!(
            app.state.selected, selected,
            "navigation keys behind a modal dialog are ignored"
        );
        app.merge_dialog = None;
        frame(&mut app, vec![]);
        frame(&mut app, vec![]);
        frame(&mut app, click(row));
        assert_ne!(app.state.selected, selected, "the window responds again");
    }

    fn switcher_fixture(app: &mut Browser) {
        let branch = |name: &str, kind| gg_engine::refs::RefItem {
            name: name.into(),
            full_name: match kind {
                gg_engine::refs::RefKind::Branch => format!("refs/heads/{name}"),
                gg_engine::refs::RefKind::Remote => format!("refs/remotes/{name}"),
                gg_engine::refs::RefKind::Tag => format!("refs/tags/{name}"),
            },
            kind,
            commit: "a".repeat(40),
        };
        app.state.refs = gg_engine::refs::Snapshot {
            head: Some(gg_engine::refs::Head {
                name: Some("refs/heads/main".into()),
                detached: false,
                commit: Some("a".repeat(40)),
            }),
            items: vec![
                branch("feature/search", gg_engine::refs::RefKind::Branch),
                branch("main", gg_engine::refs::RefKind::Branch),
                branch("origin/main", gg_engine::refs::RefKind::Remote),
                branch("v1", gg_engine::refs::RefKind::Tag),
            ],
            ..Default::default()
        };
    }

    #[test]
    fn switcher_lists_local_branches_current_first_and_filters() {
        let mut app = layout_fixture();
        switcher_fixture(&mut app);
        assert_eq!(
            switcher_branches(&app.state.refs, ""),
            vec![
                ("main".to_string(), true),
                ("feature/search".to_string(), false)
            ]
        );
        assert_eq!(
            switcher_branches(&app.state.refs, "FEAT sea"),
            vec![("feature/search".to_string(), false)]
        );
    }

    #[test]
    fn the_branch_button_opens_the_switcher_at_both_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            crate::phosphor::install(&ctx);
            let mut app = layout_fixture();
            switcher_fixture(&mut app);
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            // The switcher's branch glyph is the right-most one in the toolbar.
            let glyph = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text)
                        if text.galley.job.text == crate::phosphor::GIT_BRANCH
                            && text.pos.y < 80.0 =>
                    {
                        Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
                    }
                    _ => None,
                })
                .max_by(|a, b| a.left().total_cmp(&b.left()));
            let Some(glyph) = glyph else {
                unreachable!("branch button at {width}")
            };
            assert!(glyph.right() <= width, "{glyph:?} at {width}");
            let pos = glyph.center();
            let click = vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ];
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, click), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let texts = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            for expected in [
                "main  ·  current",
                "feature/search",
                "New branch from HEAD…",
            ] {
                assert!(
                    texts.iter().any(|text| text == expected),
                    "missing {expected:?} at {width}"
                );
            }
            assert!(!texts.iter().any(|text| text == "origin/main"));
        }
    }

    #[test]
    fn search_filters_history_or_runs_the_chosen_action() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let frame = |app: &mut Browser, events: Vec<egui::Event>| {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, events), |ctx| {
                app.draw_ui(ctx)
            });
        };
        frame(&mut app, vec![]);
        frame(&mut app, vec![key(egui::Key::K, egui::Modifiers::COMMAND)]);
        assert!(app.search.is_some(), "Ctrl+K opens search");
        frame(&mut app, vec![]);
        frame(&mut app, vec![egui::Event::Text("Subject 7".into())]);
        frame(&mut app, vec![key(egui::Key::Enter, egui::Modifiers::NONE)]);
        assert!(app.search.is_none(), "running a result closes search");
        assert_eq!(
            app.state.query, "Subject 7",
            "the first result filters history"
        );

        let sidebar = app.refs_visible;
        frame(&mut app, vec![key(egui::Key::K, egui::Modifiers::COMMAND)]);
        frame(&mut app, vec![]);
        frame(&mut app, vec![egui::Event::Text("refs sidebar".into())]);
        frame(
            &mut app,
            vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE)],
        );
        frame(&mut app, vec![key(egui::Key::Enter, egui::Modifiers::NONE)]);
        assert!(app.search.is_none());
        assert_eq!(
            app.refs_visible, !sidebar,
            "Down then Enter runs the action"
        );

        frame(&mut app, vec![key(egui::Key::K, egui::Modifiers::COMMAND)]);
        frame(&mut app, vec![key(egui::Key::K, egui::Modifiers::COMMAND)]);
        assert!(app.search.is_none(), "Ctrl+K toggles search closed");
    }

    #[test]
    fn switching_to_a_remote_branch_uses_its_local_branch_or_tracks_it() {
        let item = |name: &str, kind| gg_engine::refs::RefItem {
            name: name.into(),
            full_name: String::new(),
            kind,
            commit: "a".repeat(40),
        };
        let refs = [
            item("main", gg_engine::refs::RefKind::Branch),
            item("origin/main", gg_engine::refs::RefKind::Remote),
            item("origin/feature", gg_engine::refs::RefKind::Remote),
            item("feature", gg_engine::refs::RefKind::Tag),
        ];
        assert_eq!(
            remote_switch("origin/main", &refs),
            RemoteSwitch::Local("main".into())
        );
        // A tag of the same name is not a branch to switch to.
        assert_eq!(
            remote_switch("origin/feature", &refs),
            RemoteSwitch::Track("origin/feature".into())
        );
        assert_eq!(
            remote_switch("upstream/team/feature", &refs),
            RemoteSwitch::Track("upstream/team/feature".into())
        );
    }

    #[test]
    fn search_field_lists_files_in_a_popup_of_one_height() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let mut app = layout_fixture();
        app.local_summary = Some(LocalSummary {
            worktrees: Vec::new(),
            files: vec!["docs/plan.md".into(), "src/plan_view.rs".into()],
            ..Default::default()
        });
        let frame = |app: &mut Browser, events: Vec<egui::Event>| {
            crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, events), |ctx| {
                app.draw_ui(ctx)
            })
        };
        let _ = frame(&mut app, vec![]);
        let _ = frame(&mut app, vec![key(egui::Key::K, egui::Modifiers::COMMAND)]);
        assert!(
            ctx.memory(|m| m.has_focus(search_field_id())),
            "Ctrl+K focuses the toolbar field"
        );
        let _ = frame(&mut app, vec![egui::Event::Text("p".into())]);
        let _ = frame(&mut app, vec![]);
        let popup = egui::Id::unique("frame-search");
        let many = ctx
            .memory(|m| m.area_rect(popup))
            .ok_or("no results popup")?;
        let output = frame(&mut app, vec![egui::Event::Text("lan_view".into())]);
        let _ = frame(&mut app, vec![]);
        let few = ctx
            .memory(|m| m.area_rect(popup))
            .ok_or("no results popup")?;
        assert_eq!(few, many, "the popup keeps its size as results change");
        let text = shown_text(&output);
        assert!(text.iter().any(|line| line == "FILES"), "{text:?}");
        assert!(text.iter().any(|line| line.starts_with("src/plan_view.rs")));
        let _ = frame(
            &mut app,
            vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
        );
        assert!(app.search.is_none());
        assert!(app.search_text.is_empty(), "Esc clears the field");
        Ok(())
    }

    #[test]
    fn search_offers_frame_actions_with_their_readiness_and_refs() {
        let mut app = layout_fixture();
        app.state.refs.items = vec![gg_engine::refs::RefItem {
            name: "release".into(),
            full_name: "refs/tags/release".into(),
            kind: gg_engine::refs::RefKind::Tag,
            commit: "c".repeat(40),
        }];
        let ready = app.frame_readiness();
        let items = app.search_items(&ready);
        let pull = items
            .iter()
            .find(|item| item.label == "Pull (rebase)")
            .map(|item| item.disabled.clone());
        assert_eq!(
            pull,
            Some(Some(
                "The current branch has no upstream to pull from".to_string()
            )),
            "search shares the toolbar's readiness"
        );
        let results = search::results("release", &items, FrameCommand::Filter("release".into()));
        assert_eq!(results[0].group, search::Group::Filter);
        assert!(results.iter().any(|item| item.group == search::Group::Tags
            && item.command == FrameCommand::Inspect("c".repeat(40))));
    }

    #[test]
    fn welcome_search_lists_recent_repositories_and_new_repository() {
        let mut app = Browser::new("/nowhere".into());
        // Absolute on every platform (Windows paths need a drive).
        let blog = std::env::temp_dir().join("blog");
        app.recent = recent_list(std::slice::from_ref(&blog));
        let ready = app.frame_readiness();
        let items = app.search_items(&ready);
        let results = search::results("blog", &items, FrameCommand::Filter("blog".into()));
        assert!(
            results
                .iter()
                .any(|item| item.group == search::Group::Repositories
                    && item.label == "blog"
                    && item.command == FrameCommand::OpenRecent(blog.clone()))
        );
        let results = search::results("new repo", &items, FrameCommand::Filter("x".into()));
        assert!(
            results
                .iter()
                .any(|item| item.group == search::Group::Actions
                    && item.command == FrameCommand::NewRepository(None)
                    && item.disabled.is_none())
        );
    }

    #[test]
    fn stash_commands_follow_local_changes_and_entries() {
        let mut app = layout_fixture();
        app.local_summary = Some(LocalSummary {
            worktrees: Vec::new(),
            changes: Some(0),
            stashes: Vec::new(),
            branch_start: None,
            rebase: None,
            files: Vec::new(),
        });
        let ready = app.frame_readiness();
        assert_eq!(ready.stash_reason, Some("No local changes to stash"));
        assert_eq!(ready.stashes, 0);
        assert!(ready.branch_rewrite_reason.is_some());
        app.local_summary = Some(LocalSummary {
            worktrees: Vec::new(),
            changes: Some(2),
            stashes: vec![gg_engine::stash::StashEntry {
                index: 0,
                reference: "stash@{0}".into(),
                commit: "f".repeat(40),
                message: "WIP on main".into(),
                ..Default::default()
            }],
            branch_start: Some(("e".repeat(40), 2)),
            rebase: None,
            files: Vec::new(),
        });
        let ready = app.frame_readiness();
        assert_eq!(ready.stash_reason, None);
        assert_eq!(ready.stashes, 1);
        assert_eq!(ready.branch_rewrite_reason, None);
    }

    #[test]
    fn commands_off_the_toolbar_keep_their_menu_home_and_search_entry() -> Result<(), String> {
        let mut app = layout_fixture();
        app.local_summary = Some(LocalSummary {
            worktrees: Vec::new(),
            changes: Some(0),
            stashes: Vec::new(),
            branch_start: None,
            rebase: None,
            files: Vec::new(),
        });
        let ready = app.frame_readiness();
        let menu = menus::flatten(&app.menu_table(&ready));
        let search = app.search_items(&ready);
        for (command, home) in [
            (FrameCommand::NewBranch, "Branch"),
            (FrameCommand::NewTag, "Branch"),
            (FrameCommand::Merge, "Branch"),
            (FrameCommand::Rewrite, "Branch"),
            (FrameCommand::Stash, "Changes"),
            (FrameCommand::Stashes, "Changes"),
            (FrameCommand::Recovery, "Repository"),
            (FrameCommand::Submodules, "Repository"),
            (FrameCommand::Lfs, "Repository"),
        ] {
            let found = menu
                .iter()
                .filter(|(_, item)| item.command == command)
                .collect::<Vec<_>>();
            assert_eq!(found.len(), 1, "{command:?} has one menu home");
            assert_eq!(found[0].0, home, "{command:?}");
            let entry = search
                .iter()
                .find(|item| item.command == command)
                .ok_or(format!("{command:?} is missing from search"))?;
            assert_eq!(entry.context, format!("{home} menu"));
            assert_eq!(entry.disabled, found[0].1.disabled, "{command:?}");
        }
        // One Recovery entry point holds both sources.
        for label in ["Backups…", "Reflog…"] {
            assert!(
                !menu.iter().any(|(_, item)| item.label == label),
                "{label} is gone"
            );
        }
        // Disabled items explain the repository state, never a missing feature.
        assert!(
            menu.iter()
                .filter_map(|(_, item)| item.disabled.as_deref())
                .all(|reason| !reason.contains("Not built")),
        );
        let stashes = menu
            .iter()
            .find(|(_, item)| item.command == FrameCommand::Stashes)
            .and_then(|(_, item)| item.disabled.clone());
        assert_eq!(stashes.as_deref(), Some("No stashes"));
        Ok(())
    }

    #[test]
    fn discard_all_names_staged_changes_and_keeps_untracked_files() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let (mut local, mut history) = (false, false);
        app.run_frame_command(&ctx, FrameCommand::DiscardAll, &mut local, &mut history);
        let _ = crate::tests::run_ui(&ctx, frame_input(720.0, 480.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(720.0, 480.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for expected in [
            "Discard all tracked changes",
            "Restore every tracked file to HEAD? Staged and unstaged changes are both lost.",
            "This cannot be undone. Untracked files are kept; files only added to the index become untracked.",
            "Discard tracked changes",
        ] {
            assert!(text.contains(&expected), "missing {expected:?} in {text:?}");
        }
    }

    #[test]
    fn repository_recovery_opens_one_window_on_the_last_source() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let (mut local, mut history) = (false, false);
        app.run_frame_command(&ctx, FrameCommand::Recovery, &mut local, &mut history);
        let view = app.recovery.as_mut().ok_or("Recovery… opens the window")?;
        assert!(view.open);
        assert_eq!(view.source, return_location::RecoverySource::Backups);
        view.source = return_location::RecoverySource::Reflog;
        view.open = false;
        app.run_frame_command(&ctx, FrameCommand::Recovery, &mut local, &mut history);
        let view = app.recovery.as_ref().ok_or("the window reopens")?;
        assert!(view.open);
        assert_eq!(view.source, return_location::RecoverySource::Reflog);
        Ok(())
    }

    #[test]
    fn changes_stashes_opens_the_picker_inside_the_narrow_window()
    -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let mut app = layout_fixture();
        app.local_summary = Some(LocalSummary {
            worktrees: Vec::new(),
            changes: Some(0),
            stashes: vec![gg_engine::stash::StashEntry {
                index: 0,
                reference: "stash@{0}".into(),
                commit: "f".repeat(40),
                message: "WIP on main: work".into(),
                branch: Some("main".into()),
                age: "2 hours ago".into(),
                files: Some(1),
            }],
            branch_start: None,
            rebase: None,
            files: Vec::new(),
        });
        let _ = crate::tests::run_ui(&ctx, frame_input(720.0, 480.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let (mut local, mut history) = (false, false);
        app.run_frame_command(&ctx, FrameCommand::Stashes, &mut local, &mut history);
        for _ in 0..2 {
            let _ = crate::tests::run_ui(&ctx, frame_input(720.0, 480.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
        }
        assert!(egui::Popup::is_id_open(&ctx, stash_picker_popup_id()));
        let rect = ctx
            .memory(|memory| memory.area_rect(stash_picker_popup_id()))
            .ok_or("the picker popup has no area")?;
        assert!(
            rect.min.x >= 0.0 && rect.max.x <= 720.0 && rect.max.y <= 480.0,
            "the picker stays inside 720x480: {rect:?}"
        );
        assert!(
            rect.min.y >= MENU_BAR_HEIGHT + 34.0,
            "the picker opens below the toolbar: {rect:?}"
        );
        Ok(())
    }

    fn tracking_fixture(app: &mut Browser, ahead: u64, behind: u64) {
        app.state.refs.head = Some(gg_engine::refs::Head {
            name: Some("refs/heads/feature".into()),
            detached: false,
            commit: Some("a".repeat(40)),
        });
        app.state.refs.tracking = vec![gg_engine::refs::Tracking {
            branch: "feature".into(),
            full_name: "refs/heads/feature".into(),
            upstream: "origin/feature".into(),
            upstream_full_name: "refs/remotes/origin/feature".into(),
            remote: "origin".into(),
            remote_branch: "feature".into(),
            ahead: Some(ahead),
            behind: Some(behind),
        }];
    }

    #[test]
    fn pull_is_ready_whenever_the_current_branch_has_an_upstream() {
        let mut app = layout_fixture();
        assert_eq!(
            app.frame_readiness().pull_reason,
            Some("The current branch has no upstream to pull from")
        );
        // Cached tracking may be stale: Pull fetches before deciding.
        tracking_fixture(&mut app, 0, 0);
        let ready = app.frame_readiness();
        assert_eq!(ready.behind, 0);
        assert_eq!(ready.pull_reason, None);
        tracking_fixture(&mut app, 0, 3);
        let ready = app.frame_readiness();
        assert_eq!(ready.behind, 3, "the badge keeps the cached count");
        assert_eq!(ready.pull_reason, None);
        app.state.refs.head = Some(gg_engine::refs::Head {
            name: None,
            detached: true,
            commit: Some("a".repeat(40)),
        });
        assert_eq!(
            app.frame_readiness().pull_reason,
            Some("The current branch has no upstream to pull from")
        );
    }

    #[test]
    fn pull_reports_already_up_to_date_after_fetching() {
        let ctx = egui::Context::default();
        for relation in [
            gg_engine::sync::UpstreamRelation::UpToDate,
            gg_engine::sync::UpstreamRelation::Ahead,
        ] {
            let mut app = layout_fixture();
            tracking_fixture(&mut app, 0, 0);
            app.pull_rebase = app.pull_rebase_target("refs/heads/feature");
            assert!(app.pull_rebase.is_some());
            app.finish_network(
                &ctx,
                Ok(network::Outcome::Pull {
                    fetch: gg_engine::sync::FetchReceipt {
                        remote: "origin".into(),
                        pruned: false,
                    },
                    relation: Some(relation),
                }),
                Some(gg_engine::sync::Operation::Fetch),
            );
            assert!(app.pull_rebase.is_none());
            assert!(app.operation_worker.is_none(), "nothing to integrate");
            assert_eq!(
                app.outcome.as_ref().map(|outcome| outcome.summary.as_str()),
                Some("Already up to date with origin/feature.")
            );
        }
    }

    #[test]
    fn push_failures_offer_the_follow_up_that_can_help() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        tracking_fixture(&mut app, 1, 1);
        let push = Some(gg_engine::sync::Operation::Push);
        app.finish_network(
            &ctx,
            Err("Push rejected: origin/feature has commits you don't have. Fetch to see them.\n ! [rejected]        feature -> feature (fetch first)".into()),
            push,
        );
        let outcome = app.outcome.as_ref();
        assert_eq!(
            outcome.map(|outcome| outcome.summary.as_str()),
            Some("Push rejected: origin/feature has commits you don't have. Fetch to see them.")
        );
        assert!(
            outcome
                .and_then(|outcome| outcome.details.as_deref())
                .is_some_and(|details| details.contains("[rejected]")),
            "Git's output is behind Details…"
        );
        let recovery = outcome.and_then(|outcome| outcome.recovery);
        assert!(matches!(recovery, Some(RecoveryAction::Fetch)));
        assert_eq!(recovery.map(recovery_label), Some("Fetch"));

        app.finish_network(
            &ctx,
            Err("Push feature to origin/feature failed (exit status: 128)\nfatal: unable to access 'https://example.org/gg.git/': Could not resolve host: example.org".into()),
            push,
        );
        let recovery = app.outcome.as_ref().and_then(|outcome| outcome.recovery);
        assert_eq!(recovery.map(recovery_label), Some("Retry push"));

        app.finish_network(
            &ctx,
            Err("Push feature to origin/feature failed (exit status: 1)\n ! [remote rejected] feature -> feature (pre-receive hook declined)".into()),
            push,
        );
        let recovery = app.outcome.as_ref().and_then(|outcome| outcome.recovery);
        assert_eq!(recovery.map(recovery_label), None);
    }

    #[test]
    fn rewrite_follow_up_offers_the_push_the_result_needs() {
        use gg_engine::sync::UpstreamRelation as R;
        let label = |conflicts, offer, relation| {
            recovery_label(rewrite_follow_up(conflicts, offer, relation))
        };
        assert_eq!(label(false, true, Some(R::Ahead)), "Push");
        assert_eq!(label(false, false, Some(R::Ahead)), "Push");
        assert_eq!(
            label(false, true, Some(R::Diverged)),
            "Push with force-with-lease…"
        );
        assert_eq!(label(false, false, Some(R::Diverged)), "Recovery…");
        assert_eq!(label(false, true, Some(R::UpToDate)), "Recovery…");
        assert_eq!(label(false, true, None), "Recovery…");
        assert_eq!(label(true, true, Some(R::Diverged)), "Resolve conflicts…");
    }

    #[test]
    fn history_strip_leaves_the_narrow_details_overlay() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.state.selected = Some(format!("{:040x}", 10));
        let _ = crate::tests::run_ui(
            &ctx,
            frame_input(
                720.0,
                740.0,
                vec![key(egui::Key::Space, egui::Modifiers::NONE)],
            ),
            |ctx| app.draw_ui(ctx),
        );
        assert!(app.narrow_details);
        click_navigation("History", &mut app, &ctx, 720.0);
        assert!(
            !app.narrow_details,
            "the History destination must leave the details overlay"
        );
    }

    #[test]
    fn closing_review_from_the_history_strip_restores_the_caller() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.state.query = "Subject".into();
        app.restore_offset = Some(321.0);
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let selected = app.state.selected.clone();
        let query = app.state.query.clone();
        let offset = app.scroll_offset;
        app.run_commit_action(&ctx, format!("{:040x}", 10), CommitAction::Review);
        assert!(app.review.is_some());
        click_navigation("History", &mut app, &ctx, 1120.0);
        assert!(app.review.is_none());
        assert_eq!(app.state.selected, selected);
        assert_eq!(app.state.query, query);
        assert!((app.scroll_offset - offset).abs() < 1.0);
    }

    fn switcher_refs() -> gg_engine::refs::Snapshot {
        let item = |name: &str, full: &str, kind| gg_engine::refs::RefItem {
            name: name.into(),
            full_name: full.into(),
            kind,
            commit: "a".repeat(40),
        };
        gg_engine::refs::Snapshot {
            head: Some(gg_engine::refs::Head {
                name: Some("refs/heads/main".into()),
                detached: false,
                commit: Some("a".repeat(40)),
            }),
            items: vec![
                item("main", "refs/heads/main", gg_engine::refs::RefKind::Branch),
                item(
                    "feature",
                    "refs/heads/feature",
                    gg_engine::refs::RefKind::Branch,
                ),
                item(
                    "origin/main",
                    "refs/remotes/origin/main",
                    gg_engine::refs::RefKind::Remote,
                ),
                item(
                    "origin/other",
                    "refs/remotes/origin/other",
                    gg_engine::refs::RefKind::Remote,
                ),
                item(
                    "origin/HEAD",
                    "refs/remotes/origin/HEAD",
                    gg_engine::refs::RefKind::Remote,
                ),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn the_switcher_lists_remote_branches_without_a_local_one() {
        assert_eq!(
            switcher_remotes(&switcher_refs(), ""),
            vec!["origin/other".to_string()]
        );
        assert!(switcher_remotes(&switcher_refs(), "zzz").is_empty());
    }

    #[test]
    fn the_switcher_runs_the_row_under_the_keyboard_cursor() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let mut app = layout_fixture();
        app.state.refs = switcher_refs();
        let frame = |app: &mut Browser, events| {
            crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, events), |ctx| {
                app.draw_ui(ctx)
            })
        };
        let _ = frame(&mut app, vec![]);
        let _ = frame(
            &mut app,
            vec![key(
                egui::Key::B,
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            )],
        );
        assert!(egui::Popup::is_id_open(&ctx, branch_switcher_popup_id()));
        // Rows that can run: feature, then origin/other.
        let _ = frame(
            &mut app,
            vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE)],
        );
        assert_eq!(app.branch_cursor, 1);
        let _ = frame(&mut app, vec![key(egui::Key::Enter, egui::Modifiers::NONE)]);
        assert!(
            app.track_dialog.is_none(),
            "no dialog: it tracks and switches"
        );
        assert_eq!(
            app.branch_label.as_deref(),
            Some("Creating other from origin/other…")
        );
    }

    #[test]
    fn refs_menu_offers_the_explicit_switch_command_at_both_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.state.refs = gg_engine::refs::Snapshot {
                head: Some(gg_engine::refs::Head {
                    name: Some("refs/heads/main".into()),
                    detached: false,
                    commit: Some("a".repeat(40)),
                }),
                items: vec![
                    gg_engine::refs::RefItem {
                        name: "main".into(),
                        full_name: "refs/heads/main".into(),
                        kind: gg_engine::refs::RefKind::Branch,
                        commit: "a".repeat(40),
                    },
                    gg_engine::refs::RefItem {
                        name: "feature".into(),
                        full_name: "refs/heads/feature".into(),
                        kind: gg_engine::refs::RefKind::Branch,
                        commit: "c".repeat(40),
                    },
                ],
                ..Default::default()
            };
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let refs = output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text)
                    if text.galley.job.text == "Branch" && text.pos.y < 42.0 =>
                {
                    Some(text.pos + egui::vec2(8.0, text.galley.size().y / 2.0))
                }
                _ => None,
            });
            let Some(refs) = refs else {
                continue;
            };
            for event in [
                egui::Event::PointerMoved(refs),
                egui::Event::PointerButton {
                    pos: refs,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos: refs,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ] {
                let _ = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![event]), |ctx| {
                    app.draw_ui(ctx)
                });
            }
            let mut text = Vec::new();
            for _ in 0..3 {
                let output = crate::tests::run_ui(&ctx, frame_input(width, 740.0, vec![]), |ctx| {
                    app.draw_ui(ctx)
                });
                text = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                        _ => None,
                    })
                    .collect();
                if text.iter().any(|line| line == "Switch to…") {
                    break;
                }
            }
            assert!(
                text.iter().any(|line| line == "Switch to…"),
                "missing explicit switch route at {width}: {text:?}"
            );
            // Read-only browsing lives in the refs sidebar and search.
        }
    }

    #[test]
    fn ref_sidebar_click_preserves_head_index_worktree_and_refs()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let repo = dir.path();
        let git = |args: &[&str]| -> Result<String, Box<dyn std::error::Error>> {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(repo)
                .args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.org")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.org")
                .output()?;
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        };
        git(&["init", "-q", "-b", "main"])?;
        git(&["config", "user.name", "Test"])?;
        git(&["config", "user.email", "test@example.org"])?;
        std::fs::write(repo.join("a"), b"base\n")?;
        git(&["add", "a"])?;
        git(&["commit", "-qm", "base"])?;
        git(&["checkout", "-qb", "feature"])?;
        std::fs::write(repo.join("b"), b"feature\n")?;
        git(&["add", "b"])?;
        git(&["commit", "-qm", "feature"])?;
        git(&["checkout", "-q", "main"])?;

        let head = git(&["rev-parse", "HEAD"])?;
        let feature = git(&["rev-parse", "feature"])?;
        let status = git(&["status", "--porcelain=v1"])?;
        let index = std::fs::read(repo.join(".git/index"))?;
        let branches = git(&[
            "for-each-ref",
            "--format=%(refname) %(objectname)",
            "refs/heads",
        ])?;

        let ctx = egui::Context::default();
        let mut app = Browser::new(repo.to_path_buf());
        app.load(&ctx);
        for _ in 0..500 {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            if app.state.finished && app.state.error.is_none() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            app.state.finished,
            "history did not load: {:?}",
            app.state.error
        );
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let feature_pos = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text)
                    if text.galley.job.text == "feature" && text.pos.x < 190.0 =>
                {
                    Some(text.pos + egui::vec2(8.0, text.galley.size().y / 2.0))
                }
                _ => None,
            })
            .ok_or("feature branch is not visible in the sidebar")?;
        for event in [
            egui::Event::PointerMoved(feature_pos),
            egui::Event::PointerButton {
                pos: feature_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: feature_pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ] {
            let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![event]), |ctx| {
                app.draw_ui(ctx)
            });
        }
        assert_eq!(app.state.selected.as_deref(), Some(feature.as_str()));
        assert_eq!(git(&["rev-parse", "HEAD"])?, head);
        assert_eq!(git(&["status", "--porcelain=v1"])?, status);
        assert_eq!(std::fs::read(repo.join(".git/index"))?, index);
        assert_eq!(
            git(&[
                "for-each-ref",
                "--format=%(refname) %(objectname)",
                "refs/heads"
            ])?,
            branches
        );
        Ok(())
    }

    #[test]
    fn enter_opens_selected_commit_at_both_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            let _ = crate::tests::run_ui(&ctx, frame_input(width, 480.0, vec![]), |ctx| {
                app.draw_ui(ctx)
            });
            let _ = crate::tests::run_ui(
                &ctx,
                frame_input(
                    width,
                    480.0,
                    vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
                ),
                |ctx| app.draw_ui(ctx),
            );
            assert!(app.review.is_some());
        }
    }
    #[test]
    fn narrow_details_returns_to_the_same_selected_history_row() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.scroll_to_selection = true;
        let _ = crate::tests::run_ui(&ctx, frame_input(720.0, 480.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let selected = app.state.selected.clone();
        let details = crate::tests::run_ui(
            &ctx,
            frame_input(
                720.0,
                480.0,
                vec![key(egui::Key::Space, egui::Modifiers::NONE)],
            ),
            |ctx| app.draw_ui(ctx),
        );
        assert!(app.narrow_details);
        assert_eq!(app.history_rect, egui::Rect::NOTHING);
        // Subject appears once in details; the message body does not repeat it.
        assert_eq!(details.shapes.iter().filter(|shape| matches!(&shape.shape, egui::epaint::Shape::Text(text) if text.galley.job.text == "Subject 50")).count(), 1);
        let history = crate::tests::run_ui(
            &ctx,
            frame_input(
                720.0,
                480.0,
                vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
            ),
            |ctx| app.draw_ui(ctx),
        );
        assert!(!app.narrow_details);
        assert_eq!(app.state.selected, selected);
        assert!(history.shapes.iter().any(|shape| matches!(&shape.shape, egui::epaint::Shape::Text(text) if text.galley.job.text == "Subject 50")));
    }
}

#[cfg(test)]
mod performance {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    #[ignore = "set GG_TEST_REPOSITORY to a real repository and GG_OFFLINE=1"]
    fn real_repository_history_first_frame_and_deep_navigation()
    -> Result<(), Box<dyn std::error::Error>> {
        let path =
            std::env::var_os("GG_TEST_REPOSITORY").ok_or("GG_TEST_REPOSITORY is required")?;
        let ctx = egui::Context::default();
        let mut app = Browser::new(path.into());
        let started = Instant::now();
        app.load(&ctx);
        let mut first = None;
        let mut frames = Vec::new();
        while !app.state.finished {
            let before = Instant::now();
            let _ = crate::tests::run_ui(
                &ctx,
                super::tests::frame_input(1120.0, 740.0, vec![]),
                |ctx| app.draw_ui(ctx),
            );
            frames.push(before.elapsed());
            if app.state.count > 0 && first.is_none() {
                first = Some(started.elapsed());
            }
            assert!(app.state.error.is_none(), "{:?}", app.state.error);
            assert!(started.elapsed() < Duration::from_secs(120));
            std::thread::sleep(Duration::from_millis(1));
        }
        let complete = started.elapsed();
        let mut jumps = Vec::new();
        for row in [app.state.count.saturating_sub(1), 0] {
            app.state.select_row(row);
            app.scroll_to_selection = true;
            let requested = Instant::now();
            loop {
                let _ = crate::tests::run_ui(
                    &ctx,
                    super::tests::frame_input(1120.0, 740.0, vec![]),
                    |ctx| app.draw_ui(ctx),
                );
                if app.state.rows.contains_key(&row)
                    && app
                        .state
                        .details
                        .as_ref()
                        .is_some_and(|commit| Some(&commit.id) == app.state.selected.as_ref())
                {
                    break;
                }
                assert!(
                    requested.elapsed() < Duration::from_secs(30),
                    "deep jump to {row} did not settle"
                );
                std::thread::sleep(Duration::from_millis(1));
            }
            jumps.push(requested.elapsed());
        }
        frames.sort();
        eprintln!(
            "Real repository: commits={}; first rows={:?}; complete={:?}; frame p95={:?}; deep jumps={:?}; retained rows={}",
            app.state.count,
            first,
            complete,
            frames[frames.len() * 95 / 100],
            jumps,
            app.state.rows.len()
        );
        assert!(app.state.rows.len() <= stream::CACHE_ROWS);
        Ok(())
    }

    #[test]
    #[ignore = "set GG_TEST_REPOSITORY to a packed 100,000-commit fixture and GG_OFFLINE=1"]
    fn packed_history_first_frame_and_deep_navigation() -> Result<(), Box<dyn std::error::Error>> {
        let path =
            std::env::var_os("GG_TEST_REPOSITORY").ok_or("GG_TEST_REPOSITORY is required")?;
        let ctx = egui::Context::default();
        let mut app = Browser::new(path.into());
        let started = Instant::now();
        app.load(&ctx);
        let mut first = None;
        let mut frames = Vec::new();
        while !app.state.finished {
            let before = Instant::now();
            let _ = crate::tests::run_ui(
                &ctx,
                super::tests::frame_input(1120.0, 740.0, vec![]),
                |ctx| app.draw_ui(ctx),
            );
            frames.push(before.elapsed());
            if app.state.count > 0 && first.is_none() {
                first = Some(started.elapsed());
            }
            assert!(app.state.error.is_none(), "{:?}", app.state.error);
            assert!(started.elapsed() < Duration::from_secs(60));
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(app.state.count, 100_000);
        let complete = started.elapsed();
        for row in [99_999, 0] {
            app.state.select_row(row);
            app.scroll_to_selection = true;
            let requested = Instant::now();
            loop {
                let _ = crate::tests::run_ui(
                    &ctx,
                    super::tests::frame_input(1120.0, 740.0, vec![]),
                    |ctx| app.draw_ui(ctx),
                );
                if app.state.rows.contains_key(&row)
                    && app
                        .state
                        .details
                        .as_ref()
                        .is_some_and(|c| Some(&c.id) == app.state.selected.as_ref())
                {
                    break;
                }
                assert!(requested.elapsed() < Duration::from_secs(10));
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(app.state.rows.len() <= stream::CACHE_ROWS);
        }
        frames.sort();
        eprintln!(
            "First rows: {:?}; complete: {:?}; in-memory frame p95: {:?}; retained rows: {}",
            first,
            complete,
            frames[frames.len() * 95 / 100],
            app.state.rows.len()
        );
        Ok(())
    }
}
