mod avatars;
mod backups_view;
mod binary_preview;
mod blame_cli;
mod blame_worker;
mod bounded_text;
mod branch_cli;
mod branding;
mod clipboard;
mod colors;
mod commit_cli;
mod conflict_blocks;
mod conflict_cli;
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
mod graph;
mod history_view;
#[cfg(test)]
mod interaction_tests;
mod lfs_cli;
mod lfs_worker;
mod line_map_worker;
mod menus;
mod merge_cli;
mod mutation_cli;
mod navigation;
mod network;
mod phosphor;
mod reader;
mod rebase_cli;
mod rebase_dialog;
mod rebase_plan;
mod rebase_stop;
mod records;
mod reflog_cli;
mod reflog_view;
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
mod signature_cli;
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
mod sync_cli;
mod tag_cli;
mod thumbnails;
mod watcher;
mod worktree_cli;
mod worktree_worker;
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
            "gg · good git\n\ngit-good [PATH]\ngit-good --version        Print version\ngit-good history [PATH]   Print history as JSON\ngit-good refs [PATH]      Print branches, remotes, tags, and HEAD as JSON\ngit-good status [PATH] [--file PATH --area AREA]   Print working status or diff as JSON\ngit-good stage --file PATH [--hunk HEADER]... [--repo PATH]   Stage one file or selected hunks\ngit-good unstage --file PATH [--repo PATH]   Unstage one file\ngit-good commit [--amend] [--sign|--no-sign] -m MESSAGE [--repo PATH]   Commit the index or amend HEAD after Git hooks\ngit-good reflog [REF] [--repo PATH]   Print reflog entries as JSON\ngit-good blame PATH [--revision REF] [--lines START,END] [--repo PATH]   Print line attribution as JSON\ngit-good worktree <list|add|remove> [OPTIONS] [--repo PATH]   List, add, or remove worktrees\ngit-good submodule <status|update> PATH [OPTIONS] [--repo PATH]   Inspect or update a gitlink submodule\ngit-good lfs <status|fetch> PATH [--area staged|unstaged|untracked|conflict] [--revision REF] [--before REF] [--repo PATH]   Inspect or fetch an LFS pointer\ngit-good signature <config|verify> [REF] [--repo PATH]   Read signing config or verify a commit/tag\ngit-good switch [--create] [--stash [--include-untracked]] BRANCH [--repo PATH]   Switch to or create a local branch, optionally stashing changes first\ngit-good fetch [REMOTE] [--prune] [--repo PATH]   Fetch and update remote-tracking refs\ngit-good push [REMOTE] [BRANCH] [--set-upstream] [--force-with-lease] [--repo PATH]   Push a local branch, with an explicit lease when rewriting\ngit-good clone URL [PATH]   Clone a repository and open it\ngit-good stash <list|push|apply|pop|drop> [OPTIONS] [--repo PATH]   List or mutate stashes\ngit-good cherry-pick COMMIT [--repo PATH]   Apply a commit\ngit-good revert COMMIT [--repo PATH]   Invert a commit\ngit-good merge REF [--no-ff] [--repo PATH]   Merge a branch, remote, or tag\ngit-good rebase REF [--autostash] [--repo PATH]   Rebase the current branch onto a branch, remote, or tag\ngit-good rebase --continue|--skip|--abort [--repo PATH]   Recover an in-progress rebase\ngit-good rewrite --from COMMIT STEP... [--autostash] [--repo PATH]   Pick, reword, edit, squash, fixup, drop, or reorder a linear range (STEP: --pick|--edit|--squash|--fixup|--drop COMMIT, --reword COMMIT MESSAGE)\ngit-good rewrite --continue|--skip|--abort [--repo PATH]   Recover an in-progress rewrite\ngit-good conflicts [--repo PATH]   List unmerged paths\ngit-good conflict --file PATH [--use ours|theirs|--stage] [--repo PATH]   Read or resolve a conflict\ngit-good reset --soft|--mixed|--hard COMMIT [--repo PATH]   Reset the current branch\ngit-good branch --delete BRANCH [--force] [--repo PATH]   Delete a local branch\ngit-good branch --create NAME [--at REF] [--repo PATH]   Create a local branch without switching\ngit-good tag --delete TAG [--repo PATH]   Delete a local tag
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
    let headless = first.as_deref() == Some(std::ffi::OsStr::new("history"));
    let given = if headless { args.next() } else { first }.map(PathBuf::from);
    let path = match given {
        Some(path) => path,
        None if headless => std::env::current_dir()?,
        None => launch_folder(
            std::env::current_dir()?,
            settings::Prefs::load().reopen_last,
            &settings::recent(),
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
    eframe::run_native(
        "gg · good git",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(branding::icon()?.clone())
                .with_inner_size(window_size)
                .with_min_inner_size([720.0, 480.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            phosphor::install(&cc.egui_ctx);
            let mut app = Browser::new(path);
            app.apply_prefs(&cc.egui_ctx);
            app.details_visible = app.prefs.details_visible;
            app.load(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )?;
    Ok(())
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
                "appearance theme dark light system density compact comfortable zoom font size code"
            }
            Self::Diff => "diff open files full file layout inline side by side images compare",
            Self::Commit => {
                "commit identity name email signing sign tags subject length guide hooks verify"
            }
            Self::Rebase => {
                "rebase recovery stash autostash backup pull strategy merge fast-forward push force lease"
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
            new_root: String::new(),
            error: None,
            counts: Vec::new(),
        }
    }
}

struct StashSwitchDialog {
    target: String,
    include_untracked: bool,
    error: Option<String>,
}

struct PullRebase {
    path: PathBuf,
    remote: String,
    upstream: String,
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

#[derive(Clone)]
struct RestoreStashDialog {
    reference: String,
    restore_index: bool,
    remove: bool,
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
    New,
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

/// The toolbar stash picker's popup, also opened from the sidebar.
fn stash_picker_popup_id() -> egui::Id {
    egui::Id::unique("toolbar-stash-picker")
}

/// A command chosen in the toolbar stash picker.
#[derive(Clone, Debug, PartialEq, Eq)]
enum StashPick {
    Apply(String),
    Pop(String),
    Show { commit: String, message: String },
    Drop(String),
}

/// Toolbar stash picker: selected entry and pending drop confirmation.
#[derive(Clone, Debug, Default)]
struct StashPicker {
    selected: usize,
    confirm_drop: bool,
    error: Option<String>,
}

struct StashWindow {
    entries: Vec<gg_engine::stash::StashEntry>,
    error: Option<String>,
    notice: Option<String>,
    confirm_drop: Option<String>,
}

struct RecoveryView {
    source: return_location::RecoverySource,
    reflog: reflog_view::ReflogView,
    backups: backups_view::BackupsView,
    open: bool,
}

enum RecoveryToolAction {
    Reflog(reflog_view::Action),
    Backups(backups_view::Action),
}

fn recovery_source_label(source: return_location::RecoverySource) -> &'static str {
    match source {
        return_location::RecoverySource::Backups => "Backups",
        return_location::RecoverySource::Reflog => "Reflog",
    }
}

impl RecoveryView {
    fn new(path: PathBuf) -> Self {
        Self {
            source: return_location::RecoverySource::Backups,
            reflog: reflog_view::ReflogView::new(path.clone(), "HEAD".into()),
            backups: backups_view::BackupsView::new(path),
            open: true,
        }
    }

    fn select(&mut self, source: return_location::RecoverySource, entry: &str) {
        self.source = source;
        match source {
            return_location::RecoverySource::Backups => self.backups.select_entry(entry),
            return_location::RecoverySource::Reflog => self.reflog.select_entry(entry),
        }
    }

    fn draw(&mut self, ctx: &egui::Context, references: &[String]) -> Option<RecoveryToolAction> {
        let mut action = None;
        let mut open = self.open;
        let max_width = (ctx.content_rect().width() - 24.0).max(320.0);
        egui::Window::new("Recovery")
            .id(egui::Id::unique("recovery"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(max_width.min(760.0))
            .max_width(max_width)
            .default_height(500.0)
            .default_pos(ctx.content_rect().center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Source");
                    let mut source = self.source;
                    egui::ComboBox::from_id_salt("recovery-source")
                        .selected_text(recovery_source_label(source))
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for candidate in [
                                return_location::RecoverySource::Backups,
                                return_location::RecoverySource::Reflog,
                            ] {
                                ui.selectable_value(
                                    &mut source,
                                    candidate,
                                    recovery_source_label(candidate),
                                );
                            }
                        });
                    if source != self.source {
                        self.source = source;
                    }
                });
                ui.separator();
                match self.source {
                    return_location::RecoverySource::Backups => {
                        if let Some(next) = self.backups.content(ui) {
                            action = Some(RecoveryToolAction::Backups(next));
                        }
                    }
                    return_location::RecoverySource::Reflog => {
                        if let Some(next) = self.reflog.content(ui, references) {
                            action = Some(RecoveryToolAction::Reflog(next));
                        }
                    }
                }
            });
        self.open = open;
        action
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
    /// Commits in the current branch's own range for the toolbar rewrite.
    branch_rewrite_count: usize,
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
    Recovery,
    Worktrees,
    Open,
    Clone,
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
    Backups,
    Reflog,
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

/// Width of the conflicted-paths list in the conflict workspace.
const CONFLICT_PATHS_WIDTH: f32 = 220.0;

/// Fixed height of the operation area: two lines.
const OPERATION_AREA_HEIGHT: f32 = 44.0;

/// Height of the menu bar row (final design: 26 px).
const MENU_BAR_HEIGHT: f32 = 26.0;

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

/// The OS window title, as in the mockups: `gg — ~/path`, and the rebase
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
    let mut title = format!("gg — {shown}");
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

/// `Name <email>` from the repository's Git config, for Preferences ›
/// Commit; `None` when Git has no identity.
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
    receiver: std::sync::mpsc::Receiver<Result<gg_engine::rewrite::Receipt, String>>,
    restore_row: Option<usize>,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConflictSide {
    Base,
    Ours,
    Theirs,
}

enum ConflictUpdate {
    Paths(Vec<String>),
    Content {
        content: Box<gg_engine::conflict::Content>,
        snapshot: Box<gg_engine::conflict::Snapshot>,
        blocks: Result<Vec<gg_engine::conflict::Segment>, String>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConflictTab {
    Base,
    Ours,
    Theirs,
    Result,
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
    tab: ConflictTab,
    /// Per-block choices for a text conflict.
    blocks: Option<conflict_blocks::Blocks>,
    /// Why this file has no text blocks.
    blocks_note: Option<String>,
    /// Resolve the whole file from one source instead of block by block.
    whole_file: bool,
}

fn next_conflict_selection(paths: &[String], keep: Option<&str>, previous: usize) -> usize {
    keep.and_then(|path| paths.iter().position(|candidate| candidate == path))
        .unwrap_or(previous)
        .min(paths.len().saturating_sub(1))
}

fn conflict_side_hint(operation: Option<gg_engine::mutation::Operation>) -> &'static str {
    match operation {
        Some(gg_engine::mutation::Operation::Merge) => {
            "Ours is the current branch. Theirs is the branch being merged."
        }
        Some(gg_engine::mutation::Operation::Rebase) => {
            "Ours is the rewritten branch so far. Theirs is the commit being replayed."
        }
        Some(gg_engine::mutation::Operation::CherryPick) => {
            "Ours is the current branch. Theirs is the cherry-picked commit."
        }
        Some(gg_engine::mutation::Operation::Revert) => {
            "Ours is the current branch. Theirs is the reverted change."
        }
        None => "Ours is stage 2. Theirs is stage 3.",
    }
}

fn conflict_resolution_label(resolution: gg_engine::conflict::Resolution) -> &'static str {
    match resolution {
        gg_engine::conflict::Resolution::Base => "Base",
        gg_engine::conflict::Resolution::Ours => "Ours",
        gg_engine::conflict::Resolution::Theirs => "Theirs",
        gg_engine::conflict::Resolution::Working => "Working file",
    }
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
    }
}

fn conflict_preview_reason(view: &ConflictView) -> &'static str {
    if view.content.is_none() || view.snapshot.is_none() {
        "Load the conflict before staging a resolution."
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

fn conflict_side_label(
    operation: Option<gg_engine::mutation::Operation>,
    side: ConflictSide,
) -> &'static str {
    match (operation, side) {
        (_, ConflictSide::Base) => "Base",
        (Some(gg_engine::mutation::Operation::Rebase), ConflictSide::Ours) => {
            "Ours · rewritten branch (stage 2)"
        }
        (Some(gg_engine::mutation::Operation::Rebase), ConflictSide::Theirs) => {
            "Theirs · replayed commit (stage 3)"
        }
        (Some(gg_engine::mutation::Operation::Merge), ConflictSide::Ours) => {
            "Ours · current branch (stage 2)"
        }
        (Some(gg_engine::mutation::Operation::Merge), ConflictSide::Theirs) => {
            "Theirs · incoming branch (stage 3)"
        }
        (_, ConflictSide::Ours) => "Ours · current branch (stage 2)",
        (_, ConflictSide::Theirs) => "Theirs · incoming change (stage 3)",
    }
}

enum StashAction {
    Drop(String),
}

#[derive(Clone, Copy)]
enum MutationAction {
    Continue,
    Skip,
    Abort,
}

enum OperationArea {
    Mutation {
        operation: gg_engine::mutation::Operation,
        commit: Option<String>,
        conflicts: usize,
        pending: bool,
    },
    Rewrite {
        refreshing: bool,
    },
    Worker {
        label: String,
    },
    Network {
        operation: gg_engine::sync::Operation,
        progress: Option<gg_engine::sync::Progress>,
        cancelling: bool,
    },
    Branch {
        label: String,
    },
}

enum OperationOutcome {
    Merge(gg_engine::merge::Receipt),
    Rebase(gg_engine::rebase::Receipt),
    Recovery {
        operation: gg_engine::mutation::Operation,
        action: MutationAction,
        receipt: gg_engine::mutation::MutationReceipt,
    },
    Conflict {
        file: String,
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
    OpenRecovery,
    OpenSignature,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OutcomeSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq)]
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
}

struct Outcome {
    severity: OutcomeSeverity,
    summary: String,
    details: Option<String>,
    recovery: Option<RecoveryAction>,
}

fn recovery_label(action: RecoveryAction) -> &'static str {
    match action {
        RecoveryAction::Backups => "Recovery…",
        RecoveryAction::ResolveConflicts => "Resolve conflicts…",
        RecoveryAction::RetryFetch => "Retry fetch",
        RecoveryAction::RetryPush => "Retry push",
        RecoveryAction::ManageStashes => "Stashes…",
        RecoveryAction::RetrySwitch => "Retry switch",
        RecoveryAction::StashAndSwitch => "Stash and switch…",
        RecoveryAction::PullRebase => "Pull",
        RecoveryAction::ForcePush => "Push with force-with-lease…",
    }
}

/// The Commit actions popup: the Commit menu's own entries (one command
/// table), then the popup-only extras.
fn commit_action_menu(
    ui: &mut egui::Ui,
    menu: &[menus::Entry<FrameCommand>],
) -> Option<CommitCommand> {
    let mut chosen = None;
    menus::entries(ui, menu, &mut chosen);
    let mut command = chosen.map(CommitCommand::Menu);
    ui.separator();
    if ui.button("Copy subject").clicked() {
        command = Some(CommitCommand::CopySubject);
    }
    if ui.button("Signature details…").clicked() {
        command = Some(CommitCommand::OpenSignature);
    }
    if ui.button("Recovery…").clicked() {
        command = Some(CommitCommand::OpenRecovery);
    }
    command
}

fn short_id(reference: &str) -> String {
    reference.chars().take(12).collect()
}

fn lfs_bytes(value: u64) -> String {
    if value < 1024 {
        format!("{value} B")
    } else if value < 1024 * 1024 {
        format!("{:.1} KiB", value as f64 / 1024.0)
    } else {
        format!("{:.1} MiB", value as f64 / (1024.0 * 1024.0))
    }
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
    snapshot: Option<gg_engine::worktree::Snapshot>,
    selected: Option<usize>,
    error: Option<String>,
    notice: Option<String>,
    create: Option<CreateWorktreeDialog>,
    confirm_remove: Option<usize>,
    busy: Option<String>,
    pending: Option<worktree_worker::Action>,
}

struct CreateWorktreeDialog {
    destination: String,
    create: bool,
    branch: String,
    at: String,
    error: Option<String>,
}

struct SubmoduleWindow {
    request: crate::diff_view::SubmoduleRequest,
    details: Option<gg_engine::submodule::Details>,
    error: Option<String>,
    notice: Option<String>,
    busy: Option<String>,
    confirm_update: bool,
    refresh_parent: bool,
}

struct LfsWindow {
    request: crate::diff_view::LfsRequest,
    details: Option<gg_engine::lfs::Details>,
    content: Option<crate::binary_preview::Preview>,
    preview_note: Option<String>,
    texture: Option<egui::TextureHandle>,
    error: Option<String>,
    notice: Option<String>,
    busy: Option<String>,
    confirm_fetch: bool,
}

struct SignatureWindow {
    reference: String,
    verification: Option<gg_engine::signature::Verification>,
    config: Option<gg_engine::signature::Config>,
    error: Option<String>,
    busy: Option<String>,
}

struct Browser {
    logo: Option<egui::TextureHandle>,
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
    recent: Vec<PathBuf>,
    about_open: bool,
    /// The larger logo drawn in About gg.
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
    stash_window: Option<StashWindow>,
    restore_stash: Option<RestoreStashDialog>,
    stash_notice: Option<String>,
    mutation: Option<gg_engine::mutation::OperationState>,
    mutation_notice: Option<String>,
    mutation_error: Option<String>,
    mutation_path: Option<PathBuf>,
    outcome: Option<Outcome>,
    outcome_details_open: bool,
    #[cfg(debug_assertions)]
    screenshot_requested: bool,
    #[cfg(debug_assertions)]
    open_local: bool,
    /// Debug: `GG_REBASE=<commit>` opens the interactive rebase from there.
    open_rebase: Option<String>,
    /// Debug: `GG_REVIEW=<commit>` opens that commit in Review.
    open_review: Option<String>,
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
    search: Option<SearchPalette>,
    branch_filter: String,
    help_shortcuts: bool,
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
    submodule_worker: Option<submodule_worker::Worker>,
    submodule_generation: u64,
    lfs_window: Option<LfsWindow>,
    lfs_worker: Option<lfs_worker::Worker>,
    lfs_generation: u64,
    signature_window: Option<SignatureWindow>,
    signature_worker: Option<signature_worker::Worker>,
    signature_generation: u64,
    refs_visible: bool,
    refs_query: String,
    refs_width: f32,
    state: state::State,
    pending: Option<stream::Worker>,
    /// Watches the open repository; settled changes refresh what they touch.
    watcher: Option<watcher::Watcher>,
    /// When the history last started loading, so a ref change it already
    /// saw does not load it again.
    last_load: Option<std::time::Instant>,
    pending_filter_edit: bool,
    worker: Option<stream::Worker>,
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
    last_scroll: f64,
    edit_path: bool,
    details_visible: bool,
    narrow_details: bool,
}

impl Browser {
    fn new(path: PathBuf) -> Self {
        Self {
            logo: None,
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
            recent: if cfg!(test) {
                Vec::new()
            } else {
                settings::recent()
            },
            // Debug builds open About gg with `GG_ABOUT=1` for native review.
            about_open: cfg!(debug_assertions) && std::env::var_os("GG_ABOUT").is_some(),
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
            stash_window: None,
            restore_stash: None,
            stash_notice: None,
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
            submodule_worker: None,
            submodule_generation: 0,
            lfs_window: None,
            lfs_worker: None,
            lfs_generation: 0,
            signature_window: None,
            signature_worker: None,
            signature_generation: 0,
            refs_visible: true,
            refs_query: String::new(),
            refs_width: 0.0,
            state: state::State::default(),
            pending: None,
            watcher: None,
            last_load: None,
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
            last_scroll: f64::NEG_INFINITY,
            edit_path: false,
            details_visible: true,
            narrow_details: cfg!(debug_assertions) && std::env::var_os("GG_DETAILS").is_some(),
        }
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
                self.branch_target = Some((target_name, create));
                self.branch_label = Some(if create {
                    "Creating branch…".into()
                } else {
                    "Switching branch…".into()
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
                self.branch_target = Some((target_name, false));
                self.branch_label = Some("Stashing changes and switching branch…".into());
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
            remote: tracking.remote.clone(),
            upstream: tracking.upstream_full_name.clone(),
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
        let remote = target.remote.clone();
        self.pull_rebase = Some(target);
        self.start_fetch(ctx, Some(remote), self.prefs.fetch_prune);
        if self.network.is_none() {
            self.pull_rebase = None;
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
                self.branch_label = Some("Creating tracking branch…".into());
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
        match network::Worker::fetch(ctx, path, remote, prune) {
            Ok(worker) => {
                self.network = Some(worker);
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
        match network::Worker::push(ctx, path, None, None, set_upstream, None) {
            Ok(worker) => {
                self.network = Some(worker);
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
        match network::Worker::push(
            ctx,
            path,
            Some(dialog.remote),
            Some(dialog.branch),
            false,
            Some(dialog.lease),
        ) {
            Ok(worker) => {
                self.network = Some(worker);
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

    fn draw_force_push(&mut self, ctx: &egui::Context) {
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
                ui.weak("Git rejects this push if the remote branch no longer points at that lease, leaving the remote unchanged. This never uses bare --force.");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            self.network.is_none() && self.branch.is_none(),
                            egui::Button::new("Force push with lease"),
                        )
                        .clicked()
                    {
                        submit = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
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
        settings::set_reader_mode(ctx, self.prefs.reader_mode);
        settings::set_image_layout(ctx, self.prefs.image_layout);
        settings::set_diff_layout(ctx, self.prefs.diff_layout);
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
        let default_folder = self.folders.default_folder();
        let offline_override = std::env::var_os("GG_OFFLINE").is_some();
        // Modal, as in the mockup: a fixed size keeps it in place while
        // folders come and go; Esc, a click outside, × or Close dismiss it.
        let modal = egui::Modal::new(egui::Id::unique("preferences")).show(ctx, |ui| {
                ui.set_width(620.0);
                // A fixed header row: title, search, and ×.
                ui.allocate_ui_with_layout(
                    egui::vec2(620.0, 30.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                    ui.set_min_size(egui::vec2(620.0, 30.0));
                    ui.heading("Preferences");
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
                    egui::vec2(620.0, 430.0),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                    ui.set_min_size(egui::vec2(620.0, 430.0));
                    ui.set_max_height(430.0);
                    ui.vertical(|ui| {
                        ui.set_width(130.0);
                        for section in PrefSection::ALL {
                            // Sections without a match keep their slot, empty.
                            let shown = visible.contains(&section);
                            let response = ui.add_visible(
                                shown,
                                egui::Button::selectable(dialog.section == section, section.label()),
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
                        .max_height(370.0)
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
                            ui.weak("Without a path gg opens the current folder; when that is not a repository, the most recent one opens instead.");
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
                                                ui.selectable_value(&mut prefs.zoom, zoom, format!("{zoom}%"));
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
                                                ui.selectable_value(&mut prefs.code_font, size, px(size));
                                            }
                                        });
                                    ui.end_row();
                                });
                        }
                        PrefSection::Diff => {
                            ui.strong("Default layout");
                            ui.horizontal(|ui| {
                                ui.selectable_value(&mut prefs.diff_layout, settings::DiffLayout::Inline, "Inline");
                                ui.selectable_value(
                                    &mut prefs.diff_layout,
                                    settings::DiffLayout::SideBySide,
                                    "Side by side",
                                );
                            });
                            ui.weak("Also View › Inline diff / Side-by-side diff and the reader's toggle.");
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
                            if ui.button("Show keyboard shortcuts window…").clicked() {
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
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "no configuration folder".into());
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!("Changes apply now · saved to {saved}"))
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
        match network::Worker::clone(ctx, url, destination) {
            Ok(worker) => {
                self.network = Some(worker);
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
                let pull = self
                    .pull_rebase
                    .take()
                    .filter(|pull| self.stash_path().as_ref() == Some(&pull.path));
                self.load(ctx);
                if let Some(pull) = pull {
                    self.network_notice = None;
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
                let error =
                    if force_push && matches!(operation, Some(gg_engine::sync::Operation::Push)) {
                        format!("Force push with lease failed: {error}")
                    } else {
                        error
                    };
                let recovery = match operation {
                    Some(gg_engine::sync::Operation::Push) if !force_push => {
                        Some(RecoveryAction::RetryPush)
                    }
                    Some(gg_engine::sync::Operation::Fetch) => Some(RecoveryAction::RetryFetch),
                    _ => None,
                };
                self.set_outcome(OutcomeSeverity::Error, error.clone(), None, recovery);
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

    fn refresh_stashes(&mut self) {
        let Some(path) = self.stash_path() else {
            return;
        };
        let result = gg_git::stash::list(&path);
        if let Some(window) = &mut self.stash_window {
            match result {
                Ok(list) => {
                    window.entries = list.entries;
                    window.error = None;
                }
                Err(error) => window.error = Some(error),
            }
        }
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
        self.refresh_stashes();
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
                "Restored {reference} · {} conflict(s)",
                receipt.conflicts.len()
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
        self.refresh_stashes();
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
        self.refresh_stashes();
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
            view.reflog.load();
        }
    }

    fn refresh_backups(&mut self) {
        if let Some(view) = &mut self.recovery {
            view.backups.load();
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
            FrameCommand::Stashes => self.open_stash_window(),
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
            FrameCommand::Recovery => self.open_recovery_window(),
            FrameCommand::Worktrees => self.open_worktrees(ctx),
            FrameCommand::Open => self.edit_path = true,
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
            FrameCommand::ToggleRefs => self.refs_visible = !self.refs_visible,
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
            FrameCommand::OpenRecent(path) => {
                self.path = path.to_string_lossy().into_owned();
                self.load(ctx);
            }
            FrameCommand::Backups => {
                self.open_recovery(ctx, return_location::RecoverySource::Backups)
            }
            FrameCommand::Reflog => {
                self.open_recovery(ctx, return_location::RecoverySource::Reflog)
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
            FrameCommand::RebasePreset(action) => self.rebase_preset(action),
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
                            ui.selectable_value(&mut dialog.upstream, String::new(), "None");
                            for name in &remotes {
                                ui.selectable_value(&mut dialog.upstream, name.clone(), name);
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
                                ui.selectable_value(&mut pick.name, name.clone(), name);
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
        if self.discard_confirm {
            let mut open = true;
            let mut cancel = false;
            let mut confirm = false;
            egui::Window::new("Discard all changes")
                .id(egui::Id::unique("discard-all"))
                .modal(ctx, egui::Id::unique("discard-all"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label("Restore every tracked file, staged and unstaged, to HEAD?");
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        "This cannot be undone. Untracked files are kept.",
                    );
                    ui.horizontal(|ui| {
                        confirm |= ui
                            .button(
                                egui::RichText::new("Discard all")
                                    .color(ui.visuals().error_fg_color),
                            )
                            .clicked();
                        cancel |= ui.button("Cancel").clicked();
                    });
                });
            if cancel {
                open = false;
            }
            if confirm {
                self.discard_confirm = false;
                self.change_all(ctx, gg_git::worktree_changes::discard_all);
            } else if !open {
                self.discard_confirm = false;
            }
        }
        if self.about_open {
            let mut open = true;
            egui::Window::new("About gg")
                .id(egui::Id::unique("about-gg"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    let about = branding::about();
                    ui.set_max_width(320.0);
                    ui.vertical_centered(|ui| {
                        branding::logo(ui, &mut self.about_logo, 96.0);
                        ui.heading("gg · good git");
                        ui.weak(format!("Version {}", about.version));
                    });
                    ui.add_space(8.0);
                    egui::Grid::new("about-gg-facts")
                        .num_columns(2)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            ui.weak("Author");
                            ui.label(about.authors.join(", "));
                            ui.end_row();
                            ui.weak("License");
                            ui.hyperlink_to(
                                about.license,
                                "https://www.apache.org/licenses/LICENSE-2.0",
                            );
                            ui.end_row();
                            ui.weak("Source");
                            ui.hyperlink_to("github.com/baol/gg", "https://github.com/baol/gg");
                            ui.end_row();
                        });
                    ui.add_space(8.0);
                    ui.add(egui::Label::new(egui::RichText::new(about.note).weak()).wrap());
                });
            self.about_open = open;
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
    fn open_in_editor(&mut self, path: &str) {
        let Some(workdir) = self.stash_path() else {
            return;
        };
        let file = workdir.join(path);
        let result = if file.exists() {
            editor::open(&self.prefs.editor, &file)
        } else {
            Err(format!("{path} is not in the working tree."))
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
    fn rebase_preset(&mut self, action: gg_engine::rewrite::Action) {
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
        let selected = if self.state.selected.is_some() {
            idle(ready.has_repo)
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
        let delete = if other_branches {
            idle(ready.branch_idle)
        } else {
            Some("No other local branch to delete")
        };
        let force = if self.force_push_target().is_some() {
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
            self.recent
                .iter()
                .map(|path| {
                    item(
                        path.display().to_string(),
                        "",
                        FrameCommand::OpenRecent(path.clone()),
                    )
                    .unless(self.repository_switch_pending().then_some(busy))
                    .entry()
                })
                .collect()
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
            item("Submodules…", "", FrameCommand::Unavailable)
                .unless(Some(
                    "Not built yet: open a submodule from a changed file in Local Changes or Review",
                ))
                .entry(),
            item("LFS…", "", FrameCommand::Unavailable)
                .unless(Some(
                    "Not built yet: inspect an LFS file from Local Changes or Review",
                ))
                .entry(),
            Entry::Separator,
            item("Backups…", "", FrameCommand::Backups)
                .unless(idle(ready.has_repo))
                .hint("Private backups taken before every rewrite")
                .entry(),
            item("Reflog…", "", FrameCommand::Reflog)
                .unless(idle(ready.has_repo))
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
                title: "gg",
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
                item("Discard all…", "", FrameCommand::DiscardAll)
                    .unless(changes)
                    .hint("Restore tracked files to HEAD; untracked files are kept")
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
                item("Stash changes", "Ctrl+Shift+Z", FrameCommand::Stash)
                    .unless(ready.stash_reason)
                    .entry(),
                item("Stashes…", "", FrameCommand::Stashes)
                    .unless((!ready.stash_idle).then_some(busy))
                    .entry(),
                Entry::Separator,
                item("Resolve conflicts…", "", FrameCommand::ResolveConflicts)
                    .unless((!conflicts).then_some("No conflicts to resolve"))
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
                    .checked(self.working.is_none() && self.review.is_none())
                    .entry(),
                Entry::Separator,
                item("Refs sidebar", "Ctrl+\\", FrameCommand::ToggleRefs)
                    .unless(
                        (!sidebar)
                            .then_some("Resize the window to at least 1120 px to show the sidebar"),
                    )
                    .checked(self.refs_visible && sidebar)
                    .entry(),
                item("Commit inspector", "Space", FrameCommand::ToggleDetails)
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
                    .hint("Open the gg README in the browser")
                    .entry(),
                item("About gg", "", FrameCommand::About).entry(),
            ],
        });
        table
    }

    fn search_items(&self, ready: &FrameReadiness) -> Vec<search::Item<FrameCommand>> {
        use search::Group;
        // Every menu item, named with its menu, plus the toolbar's own rebase.
        let mut items = menus::flatten(&self.menu_table(ready))
            .into_iter()
            .filter(|(_, item)| item.command != FrameCommand::Unavailable)
            .map(|(path, item)| search::Item {
                group: Group::Actions,
                label: item.label,
                context: format!("{path} menu"),
                key: item.key,
                disabled: item.disabled,
                command: item.command,
            })
            .collect::<Vec<_>>();
        items.push(search::Item {
            group: Group::Actions,
            label: "Interactive rebase of this branch".into(),
            context: "toolbar · the branch's own commits".into(),
            key: "",
            disabled: ready.branch_rewrite_reason.map(str::to_string),
            command: FrameCommand::Rewrite,
        });
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
        let results =
            |ui: &mut egui::Ui, palette: &mut SearchPalette, chosen: &mut Option<usize>| {
                let mut group = None;
                egui::ScrollArea::vertical()
                    .id_salt("search-results")
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
                ui.weak(
                    "Enter: run · Up/Down: choose · Esc: close · every action also lives in a menu",
                );
            };
        let mut close = escape;
        if palette.in_field {
            let anchor = palette.anchor;
            let left = (anchor.right() - width).max(ctx.content_rect().left() + 8.0);
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
                egui::vec2(-8.0, MENU_BAR_HEIGHT + 34.0),
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

    /// What the operation area shows when nothing runs: the branch, its
    /// tracking state, local changes, and stashes.
    fn idle_summary(&self) -> String {
        if self.state.history.is_none() {
            return String::new();
        }
        let mut parts = Vec::new();
        let head = self.state.refs.head.as_ref();
        parts.push(match head.and_then(|head| head.name.as_deref()) {
            Some(full) => format!("On {}", full.strip_prefix("refs/heads/").unwrap_or(full)),
            None => "Detached HEAD".into(),
        });
        if let Some(tracking) = self.state.refs.current_tracking() {
            parts.push(format!("tracking {}", refs_view::tracking_label(tracking)));
        }
        if let Some(summary) = &self.local_summary {
            if let Some(changes) = summary.changes {
                parts.push(match changes {
                    0 => "no local changes".into(),
                    1 => "1 changed file".into(),
                    n => format!("{n} changed files"),
                });
            }
            match summary.stashes.len() {
                0 => {}
                1 => parts.push("1 stash".into()),
                n => parts.push(format!("{n} stashes")),
            }
        }
        parts.join(" · ")
    }

    /// `feature · rebase 3/6` while an interactive rebase is stopped.
    fn rebase_step_label(&self) -> Option<String> {
        let progress = self.rebase_progress()?;
        let (step, total) = progress.position();
        Some(format!(
            "{} · rebase {step}/{total}",
            progress.branch.as_deref().unwrap_or("detached HEAD")
        ))
    }

    /// The toolbar current-branch button and its filterable switcher popup.
    fn branch_switcher_button(
        &mut self,
        ui: &mut egui::Ui,
        ready: &FrameReadiness,
        compact: bool,
    ) -> Option<BranchPick> {
        let head = self.state.refs.head.as_ref();
        let name = match (
            self.rebase_step_label(),
            head.and_then(|head| head.name.as_deref()),
        ) {
            (Some(step), _) => step,
            (None, Some(full)) => full.strip_prefix("refs/heads/").unwrap_or(full).to_string(),
            (None, None) if head.is_some_and(|head| head.detached) => "detached HEAD".into(),
            (None, None) => "no branch".into(),
        };
        let tracking = self
            .state
            .refs
            .current_tracking()
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
        let icon = crate::phosphor::icon(crate::phosphor::GIT_BRANCH).size(15.0);
        let caret = crate::phosphor::icon(crate::phosphor::CARET_DOWN).size(10.0);
        let button = if compact {
            egui::Button::new((icon, caret))
        } else {
            egui::Button::new((
                icon,
                egui::RichText::new(&name),
                egui::RichText::new(&tracking).small().weak(),
                caret,
            ))
            .truncate()
        }
        .min_size(egui::vec2(28.0, 24.0));
        let response = ui
            .add_enabled(ready.has_repo, button)
            .on_hover_text(format!("{name} {tracking}\nSwitch branch"));
        if response.clicked() {
            self.branch_filter.clear();
        }
        let branches = switcher_branches(&self.state.refs, &self.branch_filter);
        let filter = &mut self.branch_filter;
        let mut pick = None;
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
                        let label = if *current {
                            format!("{branch}  ·  current")
                        } else {
                            branch.clone()
                        };
                        let response = ui.add_enabled(
                            reason.is_none(),
                            egui::Button::selectable(*current, label)
                                .truncate()
                                .min_size(egui::vec2(ui.available_width(), 24.0)),
                        );
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
                });
                ui.separator();
                if ui
                    .add_enabled(
                        ready.branch_idle,
                        egui::Button::new("New branch from HEAD…")
                            .min_size(egui::vec2(ui.available_width(), 24.0)),
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

    /// The toolbar **Stashes** button and its picker popup.
    fn stash_picker_button(
        &mut self,
        ui: &mut egui::Ui,
        ready: &FrameReadiness,
    ) -> Option<StashPick> {
        let reason = if !ready.stash_idle {
            Some(if ready.has_repo {
                "Wait for the current operation to finish"
            } else {
                "Open a repository first"
            })
        } else if ready.stashes == 0 {
            Some("No stashes")
        } else {
            None
        };
        let button = egui::Button::new((
            crate::phosphor::icon(crate::phosphor::BOX_ARROW_UP).size(17.0),
            egui::RichText::new(ready.stashes.to_string()).size(11.0),
            crate::phosphor::icon(crate::phosphor::CARET_DOWN).size(10.0),
        ))
        .min_size(egui::vec2(28.0, 26.0))
        .frame_when_inactive(false);
        let response = ui.add_enabled(reason.is_none(), button);
        let tip = format!("Stashes ({}) · apply, pop, show, or drop", ready.stashes);
        let response = match reason {
            Some(reason) => response.on_disabled_hover_text(format!("{tip}\n{reason}")),
            None => response.on_hover_text(tip),
        };
        if response.clicked() {
            self.stash_picker.confirm_drop = false;
            self.stash_picker.error = None;
        }
        let entries = self
            .local_summary
            .as_ref()
            .map(|summary| summary.stashes.clone())
            .unwrap_or_default();
        let picker = &mut self.stash_picker;
        let mut pick = None;
        egui::Popup::from_toggle_button_response(&response)
            .id(stash_picker_popup_id())
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
                            egui::RichText::new("Up/Down: select · Enter: apply · Del: drop")
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
            egui::Popup::close_all(ui.ctx());
        }
        pick
    }
}

/// Width of the Stashes picker popup.
const STASH_PICKER_WIDTH: f32 = 560.0;
/// Height of the picker's entry list: six rows.
const STASH_PICKER_LIST: f32 = 6.0 * 24.0;
/// Height reserved for the picker's actions, its drop confirmation, and an error.
const STASH_PICKER_ACTIONS: f32 = 64.0;

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
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::click());
    if selected {
        ui.painter()
            .rect_filled(rect, 3.0, colors::selection(ui.visuals()));
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, 3.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    let font = egui::FontId::proportional(12.5);
    let weak = ui.visuals().weak_text_color();
    let strong = ui.visuals().strong_text_color();
    let inner = rect.shrink2(egui::vec2(6.0, 0.0));
    // Slots from the right: files 64, age 96, branch 110; ref 72 on the left.
    let files = egui::Rect::from_min_max(egui::pos2(inner.right() - 64.0, inner.top()), inner.max);
    let age = egui::Rect::from_min_max(
        egui::pos2(files.left() - 100.0, inner.top()),
        egui::pos2(files.left() - 4.0, inner.bottom()),
    );
    let branch = egui::Rect::from_min_max(
        egui::pos2(age.left() - 114.0, inner.top()),
        egui::pos2(age.left() - 4.0, inner.bottom()),
    );
    let reference =
        egui::Rect::from_min_max(inner.min, egui::pos2(inner.left() + 72.0, inner.bottom()));
    let message = egui::Rect::from_min_max(
        egui::pos2(reference.right() + 4.0, inner.top()),
        egui::pos2(branch.left() - 6.0, inner.bottom()),
    );
    bounded_text::paint(
        ui,
        reference,
        &entry.reference,
        egui::FontId::monospace(11.5),
        weak,
    );
    bounded_text::paint(
        ui,
        message,
        stash_display_message(entry),
        font.clone(),
        strong,
    );
    bounded_text::paint(
        ui,
        branch,
        entry.branch.as_deref().unwrap_or("detached"),
        font.clone(),
        weak,
    );
    bounded_text::paint(ui, age, &entry.age, font.clone(), weak);
    let count = match entry.files {
        Some(1) => "1 file".to_string(),
        Some(count) => format!("{count} files"),
        None => String::new(),
    };
    bounded_text::paint(ui, files, &count, font, weak);
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
            if ui.button("Drop").clicked() {
                *pick = Some(StashPick::Drop(entry.reference.clone()));
                picker.confirm_drop = false;
            }
            if ui.button("Cancel").clicked() {
                picker.confirm_drop = false;
            }
        });
    } else {
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
            if ui.button("Drop…").clicked() {
                picker.confirm_drop = true;
            }
        });
    }
    if let Some(error) = &picker.error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
}

impl Browser {
    fn apply_stash_pick(&mut self, ctx: &egui::Context, pick: StashPick) {
        let result = match pick {
            StashPick::Apply(reference) => self.restore_stash_inner(ctx, &reference, false, false),
            StashPick::Pop(reference) => self.restore_stash_inner(ctx, &reference, true, false),
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
            settings::push_recent(&mut self.recent, path.clone());
            if let Err(error) = settings::save_recent(&self.recent) {
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
            receipt.commit.chars().take(8).collect::<String>(),
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

    /// The one operation currently owning the strip beneath navigation.
    fn operation_area(&self) -> Option<OperationArea> {
        if self.operation_worker.is_some() {
            return Some(OperationArea::Worker {
                label: self
                    .mutation_notice
                    .clone()
                    .unwrap_or_else(|| "Working…".into()),
            });
        }
        if let Some(state) = &self.mutation {
            return Some(OperationArea::Mutation {
                operation: state.operation,
                commit: state.commit.clone(),
                conflicts: state.conflicts.len(),
                pending: false,
            });
        }
        if self.rewrite_job.is_some() {
            return Some(OperationArea::Rewrite { refreshing: false });
        }
        if self.rewrite_refresh.is_some() {
            return Some(OperationArea::Rewrite { refreshing: true });
        }
        if let Some(worker) = &self.network {
            return Some(OperationArea::Network {
                operation: worker.operation,
                progress: self.network_progress.clone(),
                cancelling: worker.is_cancelled(),
            });
        }
        self.branch_label
            .as_ref()
            .map(|label| OperationArea::Branch {
                label: label.clone(),
            })
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
        });
    }

    fn run_recovery_action(&mut self, ctx: &egui::Context, action: RecoveryAction) {
        match action {
            RecoveryAction::Backups => {
                if let Some(history) = &self.state.history {
                    self.recovery = Some(RecoveryView::new(history.path.clone()));
                }
            }
            RecoveryAction::ResolveConflicts => self.open_conflict_view(ctx),
            RecoveryAction::RetryFetch => self.start_fetch(ctx, None, self.prefs.fetch_prune),
            RecoveryAction::RetryPush => self.start_push(ctx),
            RecoveryAction::RetrySwitch => {
                if let Some(target) = self.pending_switch.clone() {
                    self.start_branch(ctx, target, false);
                }
            }
            RecoveryAction::ManageStashes => {
                self.stash_window = Some(StashWindow {
                    entries: Vec::new(),
                    error: None,
                    notice: None,
                    confirm_drop: None,
                });
                self.refresh_stashes();
            }
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
            CommitAction::Reset(mode) => self.reset_confirm = Some((id, mode)),
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
            CommitCommand::OpenRecovery => {
                if let Some(history) = &self.state.history {
                    self.recovery = Some(RecoveryView::new(history.path.clone()));
                }
            }
            CommitCommand::OpenSignature => self.open_signature(ctx, id.to_string()),
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
            OperationOutcome::Merge(receipt) => {
                let summary = match receipt.kind {
                    Some(gg_engine::merge::Kind::UpToDate) => {
                        format!("Merge {} · already up to date", receipt.reference)
                    }
                    Some(gg_engine::merge::Kind::FastForward) => {
                        format!("Fast-forwarded to {}", receipt.reference)
                    }
                    Some(gg_engine::merge::Kind::MergeCommit) => {
                        format!("Merged {} as a merge commit", receipt.reference)
                    }
                    None => format!(
                        "Merged {}{}",
                        receipt.reference,
                        if receipt.conflicts.is_empty() {
                            String::new()
                        } else {
                            format!(" · {} conflict(s)", receipt.conflicts.len())
                        }
                    ),
                };
                self.mutation_notice = Some(summary.clone());
                let conflicts = !receipt.conflicts.is_empty();
                self.set_outcome(
                    if conflicts {
                        OutcomeSeverity::Warning
                    } else {
                        OutcomeSeverity::Info
                    },
                    summary,
                    conflicts.then(|| receipt.conflicts.join("\n")),
                    conflicts.then_some(RecoveryAction::ResolveConflicts),
                );
                self.conflict_view = None;
                self.restore_row = restore_row;
                self.refresh_mutation();
                self.load(ctx);
                self.refresh_working(ctx);
            }
            OperationOutcome::Rebase(receipt) => {
                self.rebase_target = None;
                let summary = if receipt.in_progress {
                    format!(
                        "Rebasing onto {}{}",
                        receipt.reference,
                        if receipt.conflicts.is_empty() {
                            String::new()
                        } else {
                            format!(" · {} conflict(s)", receipt.conflicts.len())
                        }
                    )
                } else {
                    format!("Rebased onto {}", receipt.reference)
                };
                self.mutation_notice = Some(summary.clone());
                let conflicts = !receipt.conflicts.is_empty();
                self.set_outcome(
                    if conflicts {
                        OutcomeSeverity::Warning
                    } else {
                        OutcomeSeverity::Info
                    },
                    summary,
                    conflicts.then(|| receipt.conflicts.join("\n")),
                    conflicts.then_some(RecoveryAction::ResolveConflicts),
                );
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
                self.refresh_conflict_view(ctx, Some(&file));
                self.load(ctx);
                self.refresh_working(ctx);
            }
            OperationOutcome::Recovery {
                operation,
                action,
                receipt,
            } => {
                let summary = if receipt.conflicts.is_empty() {
                    match action {
                        MutationAction::Continue => format!("Continued {}", operation.as_str()),
                        MutationAction::Skip => format!("Skipped {}", operation.as_str()),
                        MutationAction::Abort => format!("Aborted {}", operation.as_str()),
                    }
                } else {
                    format!(
                        "{} · {} conflict(s)",
                        operation.as_str(),
                        receipt.conflicts.len()
                    )
                };
                self.mutation_notice = Some(summary.clone());
                let conflicts = !receipt.conflicts.is_empty();
                self.set_outcome(
                    if conflicts {
                        OutcomeSeverity::Warning
                    } else {
                        OutcomeSeverity::Info
                    },
                    summary,
                    conflicts.then(|| receipt.conflicts.join("\n")),
                    conflicts.then_some(RecoveryAction::ResolveConflicts),
                );
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
        let label = match action {
            MutationAction::Continue => format!("Continuing {}…", operation.as_str()),
            MutationAction::Skip => format!("Skipping {}…", operation.as_str()),
            MutationAction::Abort => format!("Aborting {}…", operation.as_str()),
        };
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
        })
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
        let label = format!("Merging {reference}…");
        self.start_operation(ctx, label, move || {
            gg_git::merge::merge(&path, &reference, no_ff).map(OperationOutcome::Merge)
        })
    }

    /// Pull with the fast-forward-only strategy: move the branch to the
    /// upstream, or refuse when they have diverged.
    fn start_fast_forward(&mut self, ctx: &egui::Context, reference: &str) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before pulling.")?;
        let reference = reference.to_string();
        let label = format!("Fast-forwarding to {reference}…");
        self.start_operation(ctx, label, move || {
            gg_git::merge::fast_forward(&path, &reference).map(OperationOutcome::Merge)
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
        let label = format!("Rebasing onto {reference}…");
        self.rebase_target = Some(reference.clone());
        let started = self.start_operation(ctx, label, move || {
            gg_git::rebase::rebase(&path, &reference, autostash).map(OperationOutcome::Rebase)
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
        let (sender, receiver) = std::sync::mpsc::channel();
        let repaint = ctx.clone();
        std::thread::Builder::new()
            .name("gg-rewrite".into())
            .spawn(move || {
                let result = gg_git::rewrite::rewrite(&path, &from, &steps, autostash);
                let _ = sender.send(result);
                repaint.request_repaint();
            })
            .map_err(|error| format!("Start rewrite: {error}"))?;
        self.rewrite_job = Some(RewriteJob {
            receiver,
            restore_row,
        });
        self.operation_repo = Some(operation_path);
        self.rewrite_dialog = None;
        Ok(())
    }

    fn finish_rewrite(
        &mut self,
        ctx: &egui::Context,
        receipt: gg_engine::rewrite::Receipt,
        restore_row: Option<usize>,
    ) -> u64 {
        self.refresh_local_summary(ctx);
        self.mutation_error = None;
        let conflicts = receipt.in_progress || !receipt.conflicts.is_empty();
        let summary = if receipt.in_progress {
            format!(
                "Rewriting {} commit(s){}",
                receipt.steps,
                if receipt.conflicts.is_empty() {
                    String::new()
                } else {
                    format!(" · {} conflict(s)", receipt.conflicts.len())
                }
            )
        } else {
            format!("Rewrote {} commit(s)", receipt.steps)
        };
        self.mutation_notice = Some(summary.clone());
        let mut details = Vec::new();
        if let Some(backup) = &receipt.backup {
            details.push(format!("Backup: {backup}"));
        }
        if !receipt.conflicts.is_empty() {
            details.push(format!("Conflicts: {}", receipt.conflicts.join(", ")));
        }
        self.set_outcome(
            if conflicts {
                OutcomeSeverity::Warning
            } else {
                OutcomeSeverity::Info
            },
            summary,
            (!details.is_empty()).then(|| details.join("\n")),
            Some(if conflicts {
                RecoveryAction::ResolveConflicts
            } else if self.prefs.offer_force_lease && self.state.refs.current_tracking().is_some() {
                // The branch was published: the rewrite needs a forced push.
                RecoveryAction::ForcePush
            } else {
                RecoveryAction::Backups
            }),
        );
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

    fn open_stash_window(&mut self) {
        self.stash_window = Some(StashWindow {
            entries: Vec::new(),
            error: None,
            notice: None,
            confirm_drop: None,
        });
        self.refresh_stashes();
    }

    fn open_merge_dialog(&mut self) {
        self.merge_dialog = Some(MergeDialog {
            reference: String::new(),
            no_ff: self.prefs.merge_no_ff,
            error: None,
        });
    }

    fn open_recovery_window(&mut self) {
        if let Some(history) = &self.state.history {
            self.recovery = Some(RecoveryView::new(history.path.clone()));
        }
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
        } else if behind == 0 {
            Some("Up to date with the upstream; fetch to check again")
        } else {
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
            branch_rewrite_count: branch_rewrite.map_or(0, |(_, count)| *count),
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

    /// Resolve conflicts in the main workspace, under the operation area.
    /// The workspace fills the content area at a fixed size; only a window
    /// resize changes it.
    fn draw_conflict_workspace(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        let conflict_mutation = self.mutation.clone();
        let conflict_pending =
            self.operation_busy() || self.network.is_some() || self.branch.is_some();
        let mut conflict_select = None;
        let mut stage_resolution = false;
        let mut refresh_content = false;
        let mut conflict_open = true;
        let workdir = self.stash_path();
        let editor_command = self.prefs.editor.clone();
        if let Some(view) = &mut self.conflict_view
            && !view.whole_file
            && let Some(blocks) = &mut view.blocks
            && crate::navigation::available(ctx)
            && !ctx.text_edit_focused()
        {
            blocks.keys(ctx);
        }
        if let Some(view) = &mut self.conflict_view {
            let operation = conflict_mutation.as_ref().map(|state| state.operation);
            let unresolved = conflict_mutation
                .as_ref()
                .map(|state| state.conflicts.len())
                .unwrap_or(view.paths.len());
            ui.horizontal(|ui| {
                ui.heading("Resolve conflicts");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button("Close")
                        .on_hover_text("Return to the previous view (Esc)")
                        .clicked()
                    {
                        conflict_open = false;
                    }
                });
            });
            ui.separator();
            ui.scope(|ui| {
                    ui.horizontal(|ui| {
                        ui.strong(
                            operation
                                .map(gg_engine::mutation::Operation::as_str)
                                .unwrap_or("operation"),
                        );
                        ui.label(format!("· {unresolved} unresolved"));
                        if let Some(commit) = conflict_mutation
                            .as_ref()
                            .and_then(|state| state.commit.as_deref())
                        {
                            ui.label(format!("· {}", commit.chars().take(8).collect::<String>()));
                        }
                    });
                    ui.label(conflict_side_hint(operation));
                    // One fixed line for the error or the empty state.
                    let note = match (&view.error, view.paths.is_empty() && !view.loading) {
                        (Some(error), _) => (error.clone(), ui.visuals().error_fg_color),
                        (None, true) => ("No unmerged paths.".into(), ui.visuals().text_color()),
                        (None, false) => (String::new(), ui.visuals().text_color()),
                    };
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 20.0),
                        egui::Sense::hover(),
                    );
                    bounded_text::paint(ui, rect, &note.0, egui::FontId::proportional(13.0), note.1);
                    ui.separator();
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(CONFLICT_PATHS_WIDTH);
                            ui.label("Paths");
                            let height = ui.available_height();
                            egui::ScrollArea::vertical()
                                .id_salt("conflict-paths")
                                .max_height(height)
                                .min_scrolled_height(height)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    for (index, path) in view.paths.iter().enumerate() {
                                        if ui
                                            .selectable_label(index == view.selected, path)
                                            .clicked()
                                        {
                                            conflict_select = Some(index);
                                        }
                                    }
                                });
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            let Some(content) = &view.content else {
                                if view.loading {
                                    ui.spinner();
                                }
                                return;
                            };
                            ui.horizontal(|ui| {
                                ui.weak("Resolve");
                                let blocks = view.blocks.is_some();
                                if ui
                                    .add_enabled(
                                        blocks,
                                        egui::Button::selectable(
                                            !view.whole_file && blocks,
                                            "Block by block",
                                        ),
                                    )
                                    .on_disabled_hover_text(
                                        view.blocks_note.as_deref().unwrap_or("No text blocks."),
                                    )
                                    .clicked()
                                {
                                    view.whole_file = false;
                                }
                                if ui
                                    .add(egui::Button::selectable(
                                        view.whole_file || !blocks,
                                        "Whole file",
                                    ))
                                    .clicked()
                                {
                                    view.whole_file = true;
                                }
                                if let Some(note) = &view.blocks_note {
                                    ui.weak(note);
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let file = workdir.as_ref().zip(view.paths.get(view.selected));
                                        if ui
                                            .add_enabled(file.is_some(), egui::Button::new("Open in editor"))
                                            .on_hover_text(if editor_command.trim().is_empty() {
                                                "Open the working file with the system's default app (set an editor in Preferences)"
                                            } else {
                                                "Open the working file with the editor from Preferences"
                                            })
                                            .clicked()
                                            && let Some((workdir, path)) = file
                                        {
                                            view.error = editor::open(&editor_command, &workdir.join(path)).err();
                                        }
                                    },
                                );
                            });
                            if !view.whole_file
                                && let Some(blocks) = &mut view.blocks
                            {
                                let labels = conflict_blocks::Labels {
                                    base: conflict_side_label(operation, ConflictSide::Base),
                                    ours: conflict_side_label(operation, ConflictSide::Ours),
                                    theirs: conflict_side_label(operation, ConflictSide::Theirs),
                                };
                                blocks.show(ui, &labels);
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.strong(format!(
                                        "Result · {}/{} blocks chosen · preview, not staged",
                                        blocks.chosen(),
                                        blocks.count()
                                    ));
                                    let enabled = !conflict_pending && blocks.complete();
                                    let response = ui.add_enabled(
                                        enabled,
                                        egui::Button::new("Stage resolution"),
                                    );
                                    let response = if conflict_pending {
                                        response.on_disabled_hover_text(
                                            "Wait for the current operation to finish.",
                                        )
                                    } else {
                                        response.on_disabled_hover_text(
                                            "Choose a resolution for every block first.",
                                        )
                                    };
                                    if response.clicked() {
                                        stage_resolution = true;
                                    }
                                });
                                let text = blocks.text();
                                egui::ScrollArea::both()
                                    .id_salt("conflict-block-result")
                                    .max_height(150.0)
                                    .min_scrolled_height(150.0)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.monospace(text);
                                    });
                                return;
                            }
                            let snapshot = view.snapshot.clone();
                            let preview_text = snapshot
                                .as_ref()
                                .map(|snapshot| conflict_preview_text(content, snapshot, view.preview))
                                .unwrap_or_default();
                            ui.horizontal_wrapped(|ui| {
                                ui.weak("Preview source");
                                for (resolution, label) in [
                                    (gg_engine::conflict::Resolution::Base, "Base"),
                                    (gg_engine::conflict::Resolution::Ours, "Ours"),
                                    (gg_engine::conflict::Resolution::Theirs, "Theirs"),
                                    (gg_engine::conflict::Resolution::Working, "Working file"),
                                ] {
                                    if ui
                                        .selectable_label(view.preview == resolution, label)
                                        .clicked()
                                    {
                                        view.preview = resolution;
                                        view.tab = ConflictTab::Result;
                                    }
                                }
                                if ui.button("Refresh external edits").clicked() {
                                    refresh_content = true;
                                }
                            });
                            ui.weak(
                                "Choosing a source only changes this preview. Stage resolution publishes it once.",
                            );
                            if content.binary {
                                ui.colored_label(ui.visuals().warn_fg_color, "Binary content");
                            }
                            if content.truncated {
                                ui.colored_label(ui.visuals().warn_fg_color, "Content truncated");
                            }
                            if ui.available_width() >= 720.0 {
                                ui.columns(3, |columns| {
                                    for (column, (title, text)) in columns.iter_mut().zip([
                                        (conflict_side_label(operation, ConflictSide::Base), &content.base),
                                        (conflict_side_label(operation, ConflictSide::Ours), &content.ours),
                                        (conflict_side_label(operation, ConflictSide::Theirs), &content.theirs),
                                    ]) {
                                        column.label(title);
                                        egui::ScrollArea::both()
                                            .id_salt(title)
                                            .max_height(240.0)
                                            .min_scrolled_height(240.0)
                                            .auto_shrink([false, false])
                                    .min_scrolled_height(240.0)
                                    .auto_shrink([false, false])
                                            .show(column, |ui| {
                                                ui.monospace(text.as_deref().unwrap_or("(absent)"));
                                            });
                                    }
                                });
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.strong(format!(
                                        "Result · {} · preview, not staged",
                                        conflict_resolution_label(view.preview)
                                    ));
                                    let enabled = !conflict_pending && conflict_preview_available(view);
                                    let response = ui.add_enabled(
                                        enabled,
                                        egui::Button::new("Stage resolution"),
                                    );
                                    let response = if enabled {
                                        response
                                    } else {
                                        response.on_disabled_hover_text(conflict_preview_reason(view))
                                    };
                                    if response.clicked() {
                                        stage_resolution = true;
                                    }
                                });
                                egui::ScrollArea::both()
                                    .id_salt("conflict-result")
                                    .max_height(150.0)
                                    .min_scrolled_height(150.0)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.monospace(&preview_text);
                                    });
                            } else {
                                ui.horizontal(|ui| {
                                    ui.selectable_value(&mut view.tab, ConflictTab::Base, "Base");
                                    ui.selectable_value(&mut view.tab, ConflictTab::Ours, "Ours");
                                    ui.selectable_value(
                                        &mut view.tab,
                                        ConflictTab::Theirs,
                                        "Theirs",
                                    );
                                    ui.selectable_value(
                                        &mut view.tab,
                                        ConflictTab::Result,
                                        "Result",
                                    );
                                });
                                let text = match view.tab {
                                    ConflictTab::Base => content.base.as_deref().unwrap_or("(absent)"),
                                    ConflictTab::Ours => content.ours.as_deref().unwrap_or("(absent)"),
                                    ConflictTab::Theirs => {
                                        content.theirs.as_deref().unwrap_or("(absent)")
                                    }
                                    ConflictTab::Result => &preview_text,
                                };
                                egui::ScrollArea::both()
                                    .id_salt("conflict-side")
                                    .max_height(280.0)
                                    .min_scrolled_height(280.0)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.monospace(text);
                                    });
                                ui.horizontal(|ui| {
                                    ui.weak(format!(
                                        "Result · {} · preview, not staged",
                                        conflict_resolution_label(view.preview)
                                    ));
                                    let enabled = !conflict_pending && conflict_preview_available(view);
                                    let response = ui.add_enabled(
                                        enabled,
                                        egui::Button::new("Stage resolution"),
                                    );
                                    let response = if enabled {
                                        response
                                    } else {
                                        response.on_disabled_hover_text(conflict_preview_reason(view))
                                    };
                                    if response.clicked() {
                                        stage_resolution = true;
                                    }
                                });
                            }
                        });
                    });
                });
        }
        if let Some(index) = conflict_select
            && let Some(view) = &mut self.conflict_view
        {
            view.selected = index;
        }
        if conflict_select.is_some() || refresh_content {
            self.load_conflict_content(ctx);
        }
        if stage_resolution
            && let Err(error) = self.stage_conflict_resolution(ctx)
            && let Some(view) = &mut self.conflict_view
        {
            view.error = Some(error);
        }
        if self.conflict_view.is_some() && !conflict_open {
            self.conflict_view = None;
        }
    }

    fn open_conflict_view(&mut self, ctx: &egui::Context) {
        self.refresh_conflict_view(ctx, None);
    }

    fn refresh_conflict_view(&mut self, ctx: &egui::Context, keep: Option<&str>) {
        let Some(path) = self.stash_path() else {
            return;
        };
        let view = self.conflict_view.get_or_insert_with(|| ConflictView {
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
            tab: ConflictTab::Ours,
            blocks: None,
            blocks_note: None,
            whole_file: false,
        });
        view.keep = keep.map(str::to_string);
        view.previous_selected = view.selected;
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let repaint = ctx.clone();
        if std::thread::Builder::new()
            .name("gg-conflicts".into())
            .spawn(move || {
                let result = gg_git::conflict::unmerged(&path).map(ConflictUpdate::Paths);
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
        if let Some(view) = &mut self.conflict_view {
            view.worker = None;
            view.loading = false;
            match message {
                Ok(ConflictUpdate::Paths(paths)) => {
                    let keep = view.keep.take();
                    view.selected =
                        next_conflict_selection(&paths, keep.as_deref(), view.previous_selected);
                    view.paths = paths;
                    view.content = None;
                    view.snapshot = None;
                    view.blocks = None;
                    view.blocks_note = None;
                    view.error = None;
                    load_content = !view.paths.is_empty();
                }
                Ok(ConflictUpdate::Content {
                    content,
                    snapshot,
                    blocks,
                }) => {
                    view.content = Some(*content);
                    view.snapshot = Some(*snapshot);
                    match blocks {
                        Ok(segments) => {
                            view.blocks = Some(conflict_blocks::Blocks::new(segments));
                            view.blocks_note = None;
                        }
                        Err(note) => {
                            view.blocks = None;
                            view.blocks_note = Some(note);
                        }
                    }
                    view.preview = gg_engine::conflict::Resolution::Working;
                    view.tab = ConflictTab::Result;
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
                    Ok(ConflictUpdate::Content {
                        content: Box::new(content),
                        snapshot: Box::new(snapshot),
                        blocks,
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

    fn stage_conflict_resolution(&mut self, ctx: &egui::Context) -> Result<(), String> {
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
        let resolution = self
            .conflict_view
            .as_ref()
            .map(|view| view.preview)
            .unwrap_or(gg_engine::conflict::Resolution::Working);
        let composed = match self.conflict_view.as_ref() {
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

    fn load(&mut self, ctx: &egui::Context) {
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

    fn open_recovery(&mut self, ctx: &egui::Context, source: return_location::RecoverySource) {
        if let Some(path) = self.stash_path() {
            let view = self.recovery.get_or_insert_with(|| RecoveryView::new(path));
            view.source = source;
            view.open = true;
            ctx.request_repaint();
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
                view.reflog.set_reference(reference);
            }
            view.select(source, commit);
        }
    }

    fn open_worktrees(&mut self, ctx: &egui::Context) {
        if self.worktree_window.is_none() {
            self.worktree_window = Some(WorktreeWindow {
                snapshot: None,
                selected: None,
                error: None,
                notice: None,
                create: None,
                confirm_remove: None,
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

    fn poll_worktrees(&mut self) {
        let Some(update) = self
            .worktree_worker
            .as_ref()
            .and_then(worktree_worker::Worker::poll)
        else {
            return;
        };
        if update.generation != self.worktree_generation {
            return;
        }
        let Some(window) = &mut self.worktree_window else {
            return;
        };
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
        self.stash_window = None;
        self.worktree_window = None;
        self.edit_path = false;
        self.path = path.to_string_lossy().into_owned();
        self.restore_worktree_state(&path);
        self.load(ctx);
    }

    fn draw_worktrees(&mut self, ctx: &egui::Context) {
        self.poll_worktrees();
        let branches = self
            .state
            .refs
            .items
            .iter()
            .filter(|item| item.kind == gg_engine::refs::RefKind::Branch)
            .map(|item| item.name.clone())
            .collect::<Vec<_>>();
        let repository_switch_pending = self.repository_switch_pending();
        let Some(window) = &mut self.worktree_window else {
            return;
        };
        let mut open = true;
        let mut refresh = false;
        let mut create_open = false;
        let mut open_path = None;
        let mut remove_open = None;
        let mut submit_create = None;
        let mut cancel_create = false;
        let mut submit_remove = None;
        let mut cancel_remove = false;
        let max_width = (ctx.content_rect().width() - 24.0).max(360.0);
        egui::Window::new("Worktrees")
            .id(egui::Id::unique("worktrees"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(max_width.min(760.0))
            .max_width(max_width)
            .default_height(420.0)
            .default_pos(ctx.content_rect().center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(window.busy.is_none(), egui::Button::new("Refresh"))
                        .clicked()
                    {
                        refresh = true;
                    }
                    if ui
                        .add_enabled(window.busy.is_none(), egui::Button::new("Create worktree…"))
                        .clicked()
                    {
                        create_open = true;
                    }
                    if let Some(busy) = window.busy.as_deref().filter(|busy| !busy.is_empty()) {
                        ui.spinner();
                        ui.weak(busy);
                    }
                });
                if let Some(error) = &window.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                if let Some(notice) = &window.notice {
                    ui.colored_label(ui.visuals().hyperlink_color, notice);
                }
                ui.separator();
                let Some(snapshot) = &window.snapshot else {
                    // Until the first list arrives the window stays empty.
                    if window.busy.is_none() {
                        ui.weak("Worktrees unavailable.");
                    }
                    return;
                };
                for (index, worktree) in snapshot.worktrees.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let selected = window.selected == Some(index);
                        if ui
                            .selectable_label(
                                selected,
                                format!("{}  {}", worktree.branch_label(), worktree.path),
                            )
                            .clicked()
                        {
                            window.selected = Some(index);
                        }
                        ui.weak(worktree.status_label());
                        if worktree.main {
                            ui.weak("main");
                        }
                        let path = worktree_path(&worktree.path_bytes);
                        let can_open = worktree.prunable.is_none() && path.exists();
                        if ui
                            .add_enabled(
                                !worktree.current && can_open && !repository_switch_pending,
                                egui::Button::new("Open"),
                            )
                            .on_disabled_hover_text(if worktree.current {
                                "This is the current worktree"
                            } else if repository_switch_pending {
                                "Wait for the current operation to finish"
                            } else {
                                "The worktree path is missing"
                            })
                            .clicked()
                        {
                            open_path = Some(path.clone());
                        }
                        let removable = worktree.can_remove();
                        if ui
                            .add_enabled(removable.is_ok(), egui::Button::new("Remove…"))
                            .on_disabled_hover_text(
                                removable
                                    .err()
                                    .unwrap_or_else(|| "Remove this worktree".into()),
                            )
                            .clicked()
                        {
                            remove_open = Some(index);
                        }
                    });
                    if let Some(lock) = &worktree.locked {
                        ui.weak(format!("Locked: {lock}"));
                    }
                    if let Some(reason) = &worktree.prunable {
                        ui.weak(format!("Missing: {reason}"));
                    }
                }
                if snapshot.worktrees.is_empty() {
                    ui.weak("No worktrees registered.");
                }
            });
        if let Some(dialog) = &mut window.create {
            let mut dialog_open = true;
            egui::Window::new("Create worktree")
                .id(egui::Id::unique("create-worktree"))
                .modal(ctx, egui::Id::unique("create-worktree"))
                .open(&mut dialog_open)
                .collapsible(false)
                .resizable(false)
                .default_width(460.0)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label("Destination");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut dialog.destination)
                                .desired_width((ui.available_width() - 84.0).max(160.0)),
                        );
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
                        }
                    });
                    ui.radio_value(&mut dialog.create, false, "Existing branch");
                    ui.radio_value(&mut dialog.create, true, "New branch");
                    if dialog.create {
                        ui.label("New branch");
                        ui.add(egui::TextEdit::singleline(&mut dialog.branch).desired_width(260.0));
                        ui.label("Start at");
                        ui.add(egui::TextEdit::singleline(&mut dialog.at).desired_width(260.0));
                    } else if branches.is_empty() {
                        ui.weak("No local branches are available.");
                    } else {
                        egui::ComboBox::from_id_salt("worktree-branch")
                            .selected_text(if dialog.branch.is_empty() {
                                "Choose a branch"
                            } else {
                                &dialog.branch
                            })
                            .width(260.0)
                            .show_ui(ui, |ui| {
                                for branch in &branches {
                                    ui.selectable_value(&mut dialog.branch, branch.clone(), branch);
                                }
                            });
                    }
                    if let Some(error) = &dialog.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.horizontal(|ui| {
                        let ready = !dialog.destination.trim().is_empty()
                            && !dialog.branch.trim().is_empty()
                            && window.busy.is_none();
                        if ui.add_enabled(ready, egui::Button::new("Create")).clicked() {
                            submit_create = Some((
                                PathBuf::from(dialog.destination.trim()),
                                dialog.create,
                                dialog.branch.trim().to_string(),
                                dialog.at.trim().to_string(),
                            ));
                        }
                        if ui.button("Cancel").clicked() {
                            cancel_create = true;
                        }
                    });
                });
            if !dialog_open {
                cancel_create = true;
            }
        }
        if let Some(index) = window.confirm_remove
            && let Some(snapshot) = &window.snapshot
            && let Some(worktree) = snapshot.worktrees.get(index)
        {
            let mut confirm = true;
            egui::Window::new("Remove worktree")
                .id(egui::Id::unique("remove-worktree")).modal(ctx, egui::Id::unique("remove-worktree"))
                .open(&mut confirm)
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Remove {} at {}?",
                        worktree.branch_label(),
                        worktree.path
                    ));
                    ui.weak("Worktree removal is refused for dirty, locked, missing, main, and current worktrees. No force option is offered.");
                    if let Some(error) = &window.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(window.busy.is_none(), egui::Button::new("Remove"))
                            .clicked()
                        {
                            submit_remove = Some(worktree_path(&worktree.path_bytes));
                        }
                        if ui.button("Cancel").clicked() {
                            cancel_remove = true;
                        }
                    });
                });
            if !confirm {
                cancel_remove = true;
            }
        }
        if open_path.is_some() {
            self.worktree_window = None;
        }
        if refresh {
            self.request_worktrees(ctx, worktree_worker::Action::List);
        }
        if create_open && let Some(window) = &mut self.worktree_window {
            window.create = Some(CreateWorktreeDialog {
                destination: String::new(),
                create: true,
                branch: String::new(),
                at: "HEAD".into(),
                error: None,
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
        self.submodule_window = Some(SubmoduleWindow {
            request,
            details: None,
            error: None,
            notice: None,
            busy: None,
            confirm_update: false,
            refresh_parent: false,
        });
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
        let mut open = true;
        let mut open_nested = false;
        let mut ask_update = false;
        let mut submit_update = false;
        let mut cancel_update = false;
        {
            let Some(window) = &mut self.submodule_window else {
                return;
            };
            let max_width = (ctx.content_rect().width() - 24.0).max(360.0);
            egui::Window::new("Submodule details")
                .id(egui::Id::unique("submodule-details"))
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_width(max_width.min(680.0))
                .max_width(max_width)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new(&request.path).monospace().strong());
                    if let Some(details) = &window.details {
                        ui.weak(details.status_label());
                        if let Some(before) = request.before.as_deref() {
                            ui.label(format!("Before {before}"));
                        }
                        if let Some(after) = request.after.as_deref() {
                            ui.label(format!("After {after}"));
                        }
                        ui.label(format!(
                            "Index {}",
                            details.index.as_deref().unwrap_or("absent")
                        ));
                        ui.label(format!(
                            "Checked out {}",
                            details.checked_out.as_deref().unwrap_or("not initialized")
                        ));
                        ui.horizontal_wrapped(|ui| {
                            if let Some(url) = &details.url {
                                ui.weak(url);
                            }
                            if let Some(branch) = &details.branch {
                                ui.weak(format!("branch {branch}"));
                            }
                        });
                        if let Some(configured) = &details.configured_url
                            && Some(configured) != details.url.as_ref()
                        {
                            ui.colored_label(
                                ui.visuals().warn_fg_color,
                                format!("Configured URL differs: {configured}"),
                            );
                        }
                        if let Some(note) = &details.note {
                            ui.weak(note);
                        }
                        let update = details.can_update();
                        ui.horizontal_wrapped(|ui| {
                            let can_open = details.initialized
                                && nested.as_ref().is_some_and(|path| path.exists());
                            if ui
                                .add_enabled(can_open, egui::Button::new("Open nested repository"))
                                .on_disabled_hover_text(
                                    "Initialize the submodule before opening it",
                                )
                                .clicked()
                            {
                                open_nested = true;
                            }
                            if ui
                                .add_enabled(
                                    update.is_ok() && window.busy.is_none(),
                                    egui::Button::new("Update…"),
                                )
                                .on_disabled_hover_text(
                                    update
                                        .err()
                                        .unwrap_or_else(|| "Update the nested checkout".into()),
                                )
                                .clicked()
                            {
                                ask_update = true;
                            }
                        });
                    }
                    if let Some(error) = &window.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    if let Some(notice) = &window.notice {
                        ui.colored_label(ui.visuals().hyperlink_color, notice);
                    }
                });
            if window.confirm_update
                && let Some(details) = &window.details
            {
                let mut confirm = true;
                egui::Window::new("Update submodule")
                    .id(egui::Id::unique("update-submodule"))
                    .modal(ctx, egui::Id::unique("update-submodule"))
                    .open(&mut confirm)
                    .collapsible(false)
                    .resizable(false)
                    .default_pos(ctx.content_rect().center())
                    .pivot(egui::Align2::CENTER_CENTER)
                    .show(ctx, |ui| {
                        ui.label(format!(
                            "Update {} to {}?",
                            request.path,
                            details
                                .target()
                                .map(|target| target.chars().take(8).collect::<String>())
                                .unwrap_or_else(|| "the index commit".into())
                        ));
                        if let Some(url) = &details.url {
                            ui.weak(format!("Git may fetch from {url}."));
                        }
                        ui.weak("The parent index is not staged by this action.");
                        if ui
                            .add_enabled(window.busy.is_none(), egui::Button::new("Update"))
                            .clicked()
                        {
                            submit_update = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel_update = true;
                        }
                    });
                if !confirm {
                    cancel_update = true;
                }
            }
        }
        if ask_update && let Some(window) = &mut self.submodule_window {
            window.confirm_update = true;
        }
        if cancel_update && let Some(window) = &mut self.submodule_window {
            window.confirm_update = false;
        }
        if submit_update {
            if let Some(window) = &mut self.submodule_window {
                window.confirm_update = false;
            }
            self.request_submodule(ctx, submodule_worker::Action::Update);
        }
        if open_nested && let Some(path) = nested {
            self.submodule_window = None;
            self.switch_worktree(ctx, path);
            return;
        }
        let refresh = self
            .submodule_window
            .as_mut()
            .is_some_and(|window| std::mem::take(&mut window.refresh_parent));
        if refresh {
            self.refresh_working(ctx);
        }
        if !open {
            self.submodule_window = None;
        }
    }

    fn open_signature(&mut self, ctx: &egui::Context, reference: String) {
        self.signature_window = Some(SignatureWindow {
            reference: reference.clone(),
            verification: None,
            config: None,
            error: None,
            busy: None,
        });
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
            Ok((config, verification)) => {
                window.config = Some(config);
                window.verification = Some(verification);
                window.error = None;
            }
            Err(error) => window.error = Some(error),
        }
    }

    fn draw_signature(&mut self, ctx: &egui::Context) {
        self.poll_signature();
        let Some(reference) = self.signature_window.as_ref().map(|w| w.reference.clone()) else {
            return;
        };
        let mut open = true;
        {
            let Some(window) = &mut self.signature_window else {
                return;
            };
            let max_width = (ctx.content_rect().width() - 24.0).max(360.0);
            egui::Window::new("Signature details")
                .id(egui::Id::unique("signature-details"))
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_width(max_width.min(680.0))
                .max_width(max_width)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new(short_id(&reference)).monospace().strong());
                    if window.busy.is_some() {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.weak("Verifying signature…");
                        });
                    }
                    if let Some(verification) = &window.verification {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(verification.state.label()).strong());
                        egui::Grid::new("signature-verification")
                            .num_columns(2)
                            .spacing([12.0, 4.0])
                            .show(ui, |ui| {
                                if let Some(kind) = &verification.kind {
                                    ui.weak("Object");
                                    ui.label(kind);
                                    ui.end_row();
                                }
                                if let Some(signer) = &verification.signer {
                                    ui.weak("Signer");
                                    ui.label(signer);
                                    ui.end_row();
                                }
                                if let Some(key) = &verification.key {
                                    ui.weak("Key");
                                    ui.label(egui::RichText::new(key).monospace());
                                    ui.end_row();
                                }
                                if let Some(trust) = &verification.trust {
                                    ui.weak("Trust");
                                    ui.label(trust);
                                    ui.end_row();
                                }
                                if let Some(format) = &verification.format {
                                    ui.weak("Signature format");
                                    ui.label(format);
                                    ui.end_row();
                                }
                            });
                        if let Some(detail) = &verification.detail {
                            ui.add_space(4.0);
                            ui.weak(detail);
                        }
                    }
                    if let Some(config) = &window.config {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.label(egui::RichText::new("Signing configuration").strong());
                        egui::Grid::new("signature-config")
                            .num_columns(2)
                            .spacing([12.0, 4.0])
                            .show(ui, |ui| {
                                ui.weak("Format");
                                ui.label(&config.format);
                                ui.end_row();
                                ui.weak("Signing key");
                                ui.label(
                                    config
                                        .signing_key
                                        .as_deref()
                                        .unwrap_or("Not configured"),
                                );
                                ui.end_row();
                                ui.weak("Sign commits");
                                ui.label(if config.commit_signing {
                                    "Enabled"
                                } else {
                                    "Disabled"
                                });
                                ui.end_row();
                                ui.weak("Sign tags");
                                ui.label(if config.tag_signing {
                                    "Enabled"
                                } else {
                                    "Disabled"
                                });
                                ui.end_row();
                                if let Some(program) = &config.gpg_program {
                                    ui.weak("Program");
                                    ui.label(program);
                                    ui.end_row();
                                }
                                if let Some(allowed) = &config.allowed_signers {
                                    ui.weak("Allowed signers");
                                    ui.label(allowed);
                                    ui.end_row();
                                }
                            });
                        if let Some(note) = &config.note {
                            ui.weak(note);
                        }
                    }
                    if let Some(error) = &window.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.add_space(4.0);
                    ui.weak("Verification is read-only: trust stores and configuration are not changed.");
                });
        }
        if !open {
            self.signature_window = None;
        }
    }

    fn open_lfs(&mut self, ctx: &egui::Context, request: crate::diff_view::LfsRequest) {
        self.lfs_window = Some(LfsWindow {
            request: request.clone(),
            details: None,
            content: None,
            preview_note: None,
            texture: None,
            error: None,
            notice: None,
            busy: None,
            confirm_fetch: false,
        });
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
                        format!("Fetched {} ({}).", receipt.oid, lfs_bytes(receipt.size))
                    } else {
                        format!("Fetch completed but {} is still unavailable.", receipt.oid)
                    });
                }
            }
            Err(error) => window.error = Some(error),
        }
    }

    fn draw_lfs(&mut self, ctx: &egui::Context) {
        self.poll_lfs();
        let Some(request) = self
            .lfs_window
            .as_ref()
            .map(|window| window.request.clone())
        else {
            return;
        };
        let mut open = true;
        let mut ask_fetch = false;
        let mut submit_fetch = false;
        let mut cancel_fetch = false;
        {
            let Some(window) = &mut self.lfs_window else {
                return;
            };
            let max_width = (ctx.content_rect().width() - 24.0).max(360.0);
            egui::Window::new("LFS pointer")
                .id(egui::Id::unique("lfs-details"))
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_width(max_width.min(680.0))
                .max_width(max_width)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new(request.path()).monospace().strong());
                    if let Some(details) = &window.details {
                        ui.weak(details.status_label());
                        for (label, pointer) in [
                            ("Before", details.before.as_ref()),
                            ("After", details.after.as_ref()),
                        ] {
                            if let Some(pointer) = pointer {
                                ui.label(format!(
                                    "{label} {} · {} · {}",
                                    pointer.oid,
                                    lfs_bytes(pointer.size),
                                    if pointer.available {
                                        "available"
                                    } else {
                                        "missing"
                                    }
                                ));
                            }
                        }
                        ui.weak(if details.lfs_installed {
                            "git-lfs available"
                        } else {
                            "git-lfs is not installed"
                        });
                        if let Some(remote) = &details.remote {
                            ui.weak(format!("Remote {remote}"));
                        }
                        if let Some(note) = &details.note {
                            ui.weak(note);
                        }
                        if window.content.is_some() || window.preview_note.is_some() {
                            ui.add_space(6.0);
                            ui.separator();
                            ui.label(egui::RichText::new("Content preview").strong());
                            if let Some(content) = &window.content {
                                crate::binary_preview::show_single(
                                    ui,
                                    content,
                                    &mut window.texture,
                                );
                            }
                            if let Some(note) = &window.preview_note {
                                ui.weak(note);
                            }
                            ui.weak("Bounded preview; export keeps the complete original bytes.");
                        }
                        let fetch = details.can_fetch();
                        if ui
                            .add_enabled(
                                fetch.is_ok() && window.busy.is_none(),
                                egui::Button::new("Fetch object…"),
                            )
                            .on_disabled_hover_text(
                                fetch
                                    .err()
                                    .unwrap_or_else(|| "Fetch the missing object".into()),
                            )
                            .clicked()
                        {
                            ask_fetch = true;
                        }
                        ui.weak("Fetching does not change or smudge working files.");
                    }
                    if let Some(error) = &window.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    if let Some(notice) = &window.notice {
                        ui.colored_label(ui.visuals().hyperlink_color, notice);
                    }
                });
            if window.confirm_fetch
                && let Some(details) = &window.details
                && let Some(target) = details.target()
            {
                let mut confirm = true;
                egui::Window::new("Fetch LFS object")
                    .id(egui::Id::unique("fetch-lfs"))
                    .modal(ctx, egui::Id::unique("fetch-lfs"))
                    .open(&mut confirm)
                    .collapsible(false)
                    .resizable(false)
                    .default_pos(ctx.content_rect().center())
                    .pivot(egui::Align2::CENTER_CENTER)
                    .show(ctx, |ui| {
                        ui.label(format!(
                            "Fetch {} ({})?",
                            target.oid,
                            lfs_bytes(target.size)
                        ));
                        if let Some(remote) = &details.remote {
                            ui.weak(format!("Remote {remote}."));
                        }
                        ui.weak("This may download data. Working files are not changed.");
                        if ui
                            .add_enabled(window.busy.is_none(), egui::Button::new("Fetch"))
                            .clicked()
                        {
                            submit_fetch = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel_fetch = true;
                        }
                    });
                if !confirm {
                    cancel_fetch = true;
                }
            }
        }
        if ask_fetch && let Some(window) = &mut self.lfs_window {
            window.confirm_fetch = true;
        }
        if cancel_fetch && let Some(window) = &mut self.lfs_window {
            window.confirm_fetch = false;
        }
        if submit_fetch {
            if let Some(window) = &mut self.lfs_window {
                window.confirm_fetch = false;
            }
            self.request_lfs(ctx, lfs_worker::Action::Fetch(request));
        }
        if !open {
            self.lfs_window = None;
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
        let title = window_title(
            &self.path,
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
            // Esc leaves the conflict workspace before any view can take it.
            self.conflict_view = None;
        }
        self.ctx_width = ctx.content_rect().width();
        if let Some((changes, first)) = self.watcher.as_mut().and_then(|watcher| watcher.poll()) {
            self.apply_changes(ctx, changes, first);
        }
        self.menu_shortcuts(ctx);
        self.poll_local_summary();
        let focused = ctx.input(|i| i.focused);
        if focused && !self.window_focused {
            self.refresh_local_summary(ctx);
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
        self.open_reopened_commit(ctx);
        self.remember_layout(ctx);
        #[cfg(debug_assertions)]
        if self.state.history.is_some()
            && let Some(revision) = self.open_review.take()
        {
            self.open_review_at(ctx, revision, String::new(), None);
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
                    self.branch_after_create = None;
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
                    self.state.query.clear();
                    self.state.start_filter();
                    self.state.selected = Some(receipt.head);
                    self.state.selected_row = None;
                    self.state.details = None;
                    self.scroll_offset = 0.0;
                    self.scroll_to_selection = true;
                    self.load(ctx);
                }
                Err(error) => {
                    self.branch_warning = None;
                    if let Some((name, false)) = target {
                        self.pending_switch = Some(name.clone());
                        let created = self.branch_after_create.take();
                        let summary = match created {
                            Some(branch) => {
                                format!("Created {branch} but the switch was blocked: {error}")
                            }
                            None => format!("Switch to {name} blocked: {error}"),
                        };
                        let recovery = if gg_git::dirty_tree_error(&error) {
                            RecoveryAction::StashAndSwitch
                        } else {
                            RecoveryAction::RetrySwitch
                        };
                        self.set_outcome(OutcomeSeverity::Warning, summary, None, Some(recovery));
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
                            Some(Err("Network worker stopped. Refresh and retry.".into()));
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
                    Ok(receipt) => {
                        let generation = self.finish_rewrite(ctx, receipt, restore_row);
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
        let wide_refs = !narrow && self.refs_visible && ctx.content_rect().width() >= 1120.0;
        if self.working.is_none() && self.review.is_none() {
            self.state.viewport = self.viewport.clone();
            let (hovering_history, interacting, now) = ctx.input(|i| {
                (
                    i.pointer.hover_pos().is_some_and(|p| {
                        self.history_rect.contains(p) || self.details_rect.contains(p)
                    }),
                    i.pointer.any_down() || i.smooth_scroll_delta != egui::Vec2::ZERO,
                    i.time,
                )
            });
            if interacting {
                self.last_scroll = now;
            }
            if now - self.last_scroll <= 0.2 {
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
            let safe = !hovering_history && !interacting && now - self.last_scroll > 0.2;
            let hovering_scrollbar = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| {
                self.history_rect.contains(p) && p.x >= self.history_rect.right() - 18.0
            });
            let extent_safe = !interacting && !hovering_scrollbar && now - self.last_scroll > 0.2;
            let pinned = ctx.input(|i| i.pointer.hover_pos()).and_then(|p| {
                self.row_rects
                    .iter()
                    .find(|(_, rect)| rect.contains(p))
                    .map(|(row, _)| *row)
            });
            if let Some(result) = self.pending.as_ref().and_then(stream::Worker::poll) {
                match result {
                    Ok(update) if self.pending_filter_edit && !self.state.is_current(&update) => {}
                    Ok(update) => self.staged = Some(update),
                    Err(error) => {
                        self.state.error = Some(error);
                        self.pending = None;
                        self.staged = None;
                    }
                }
            }
            if self.pending.is_some()
                && (safe || self.state.history.is_none())
                && (self.rewrite_refresh.is_some()
                    || self.staged.as_ref().is_some_and(|u| {
                        u.request.anchor_id.is_none()
                            || u.anchor_row.is_some()
                            || u.complete
                            || u.error.is_some()
                    }))
                && let Some(update) = self.staged.take()
            {
                self.restore_offset = Some(
                    update
                        .anchor_row
                        .map(|row| {
                            row as f32 * crate::density::row()
                                + self.scroll_offset % crate::density::row()
                        })
                        .unwrap_or(0.0),
                );
                self.state.accept(update, true, true, pinned);
                self.worker = self.pending.take();
                self.pending_filter_edit = false;
                self.viewport = 0..64;
                self.scroll_to_selection = false;
            }
            if self.pending.is_none() {
                if let Some(result) = self.worker.as_ref().and_then(stream::Worker::poll) {
                    match result {
                        Ok(update) if !self.state.is_current(&update) => {}
                        Ok(update) => {
                            if safe {
                                self.staged = None;
                                if self.state.selected_row.is_none() && update.selection.is_some() {
                                    self.scroll_to_selection = true;
                                }
                                self.state.accept(update, false, true, pinned);
                            } else if self.state.is_current(&update) {
                                // Rows can fill off-pointer; counts and selection wait for a safe boundary.
                                self.staged = Some(update);
                            }
                        }
                        Err(error) => self.state.error = Some(error),
                    }
                }
                if !safe && let Some(update) = &self.staged {
                    self.state.accept(update.clone(), false, false, pinned);
                }
                if extent_safe
                    && let Some(update) = &self.staged
                    && self.state.is_current(update)
                {
                    self.state.count = update.count;
                    self.state.scanned = update.scanned;
                    self.state.finished = update.complete;
                    if self.state.requested.is_none() {
                        self.state.error = update.error.clone();
                    }
                }
                if safe && let Some(update) = self.staged.take() {
                    if self.state.selected_row.is_none() && update.selection.is_some() {
                        self.scroll_to_selection = true;
                    }
                    self.state.accept(update, false, true, pinned);
                }
            }
            self.avatars.poll(
                ctx,
                !hovering_history && !interacting && now - self.last_scroll > 0.2,
            );
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
                    let location = self.capture_return_location();
                    self.open_review(ctx, location);
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
                        self.rebase_preset(action);
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
                    if !(narrow && self.narrow_details)
                        && ctx.input_mut(|i| i.consume_key(modifiers, key))
                        && self.state.count > 0
                    {
                        if self.worker.is_some() {
                            let row = self.state.selected_row.unwrap_or(0);
                            let next = row
                                .saturating_add_signed(distance as isize)
                                .min(self.state.count - 1);
                            self.state.select_row(next);
                        } else {
                            self.state.move_selection(distance);
                        }
                        crate::navigation::focus_content(ctx);
                        self.scroll_to_selection = true;
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
            && ctx.content_rect().width() >= 1120.0
        {
            self.refs_visible = !self.refs_visible;
        }
        let table = self.menu_table(&ready);
        let mut menu_command = None;
        // egui's in-window menu bar: frameless titles, hover moves between open menus.
        egui::Panel::top("menu-bar")
            .exact_size(MENU_BAR_HEIGHT)
            .show(ui, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    menu_command = menus::show(ui, &table);
                    ui.separator();
                    let identity_width = ui.available_width().max(80.0);
                    let (identity, path) = if let Some(history) = &self.state.history {
                        let tracking = match self.rebase_step_label() {
                            Some(step) => format!("  ·  {step}"),
                            None => self
                                .state
                                .refs
                                .current_tracking()
                                .map(|tracking| {
                                    format!("  ·  {}", refs_view::tracking_label(tracking))
                                })
                                .unwrap_or_default(),
                        };
                        (
                            format!(
                                "{}  ·  {}{}",
                                history
                                    .path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy(),
                                if history.shallow {
                                    format!("{} · shallow", history.head)
                                } else {
                                    history.head.clone()
                                },
                                tracking
                            ),
                            history.path.display().to_string(),
                        )
                    } else {
                        ("Open a repository".into(), self.path.clone())
                    };
                    ui.allocate_ui_with_layout(
                        egui::vec2(identity_width, 26.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.set_min_width(identity_width);
                            bounded_text::field(
                                ui,
                                &format!("{identity}  ·  {path}"),
                                egui::FontId::proportional(13.0),
                                26.0,
                            );
                        },
                    );
                });
            });
        let mut tool = menu_command;
        let mut stash_pick = None;
        let mut open_search = false;
        let mut search_field: Option<(egui::Rect, bool)> = None;
        let mut branch_pick = None;
        egui::Panel::top("navigation")
            .exact_size(34.0)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    // Narrow windows give the logo's slot to the tools.
                    if ctx.content_rect().width() >= 900.0 {
                        branding::toolbar(ui, &mut self.logo);
                        ui.add_space(4.0);
                    }
                    let has_repo = self.state.history.is_some();
                    let local_selected = self.working.is_some();
                    let sidebar_available = ctx.content_rect().width() >= 1120.0;
                    let sidebar = ui
                        .add_enabled(
                            sidebar_available,
                            egui::Button::selectable(
                                self.refs_visible && sidebar_available,
                                crate::phosphor::icon(crate::phosphor::SIDEBAR_SIMPLE).size(17.0),
                            )
                            .min_size(egui::vec2(28.0, 26.0)),
                        )
                        .on_hover_text("Refs sidebar")
                        .on_disabled_hover_text(
                            "Refs sidebar\nResize the window to at least 1120 px to show the sidebar",
                        );
                    if sidebar.clicked() {
                        self.refs_visible = !self.refs_visible;
                    }
                    ui.add_space(6.0);
                    let changes = ui
                        .add_enabled(
                            has_repo,
                            egui::Button::selectable(local_selected, "Changes")
                                .min_size(egui::vec2(96.0, 24.0)),
                        )
                        .on_hover_text(match ready.changes {
                            0 => "Local Changes: stage, unstage, commit, and amend".to_string(),
                            1 => "Local Changes: 1 changed file".to_string(),
                            count => format!("Local Changes: {count} changed files"),
                        });
                    paint_badge(ui, changes.rect, ready.changes as u64);
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
                    let separator = |ui: &mut egui::Ui| {
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                    };
                    use crate::phosphor as icon;
                    if toolbar_button(ui, icon::DOWNLOAD_SIMPLE, None, "Fetch  Ctrl+Shift+F", idle(ready.network_idle)) {
                        tool = Some(FrameCommand::Fetch);
                    }
                    let strategy = self.prefs.pull_strategy.menu_label();
                    let pull_tip = if ready.behind > 0 {
                        format!("Pull {} ({strategy})  Ctrl+Shift+L", ready.behind)
                    } else {
                        format!("Pull ({strategy})  Ctrl+Shift+L")
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
                    separator(ui);
                    if toolbar_button(ui, icon::GIT_BRANCH, None, "New branch…  Ctrl+B", idle(ready.branch_idle)) {
                        tool = Some(FrameCommand::NewBranch);
                    }
                    if toolbar_button(ui, icon::TAG, None, "New tag…", idle(ready.branch_idle)) {
                        tool = Some(FrameCommand::NewTag);
                    }
                    if toolbar_button(
                        ui,
                        icon::GIT_MERGE,
                        None,
                        "Merge into current…",
                        idle(ready.branch_idle),
                    ) {
                        tool = Some(FrameCommand::Merge);
                    }
                    separator(ui);
                    if toolbar_button(ui, icon::ARCHIVE, None, "Stash changes…  Ctrl+Shift+Z", ready.stash_reason) {
                        tool = Some(FrameCommand::Stash);
                    }
                    if let Some(pick) = self.stash_picker_button(ui, &ready) {
                        stash_pick = Some(pick);
                    }
                    separator(ui);
                    let rewrite_tip = if ready.branch_rewrite_count > 0 {
                        format!(
                            "Interactive rebase of this branch · {} commit{} not on the upstream or default branch",
                            ready.branch_rewrite_count,
                            if ready.branch_rewrite_count == 1 { "" } else { "s" }
                        )
                    } else {
                        "Interactive rebase of this branch".into()
                    };
                    if toolbar_button(
                        ui,
                        icon::LIST_NUMBERS,
                        None,
                        &rewrite_tip,
                        ready.branch_rewrite_reason,
                    ) {
                        tool = Some(FrameCommand::Rewrite);
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
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let icon = crate::phosphor::icon(crate::phosphor::MAGNIFYING_GLASS);
                        // A search field when there is room; narrow windows keep
                        // an icon that opens the same search with its own field.
                        let wide = ui.available_width() >= 150.0;
                        if wide {
                            let roomy = ui.available_width() >= 560.0;
                            let width = if roomy {
                                330.0
                            } else if ui.available_width() >= 420.0 {
                                220.0
                            } else {
                                140.0
                            };
                            let hint = if roomy {
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
                        let compact = !wide || ui.available_width() < 230.0;
                        if let Some(pick) = self.branch_switcher_button(ui, &ready, compact) {
                            branch_pick = Some(pick);
                        }
                    });
                });
            });
        if let Some(pick) = stash_pick {
            self.apply_stash_pick(ctx, pick);
        }
        match branch_pick {
            Some(BranchPick::Switch(name)) => self.start_branch(ctx, name, false),
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
        let mut cancel_network = false;
        let rebase_progress = self.rebase_progress().cloned();
        // The operation area is always present at a fixed height, so starting
        // or finishing an operation never moves the panes below it.
        let idle_summary = self.idle_summary();
        {
            let area = self.operation_area();
            egui::Panel::top("operation-area").exact_size(OPERATION_AREA_HEIGHT).show(ui, |ui| {
                let Some(area) = area else {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Label::new(egui::RichText::new(&idle_summary).weak())
                                .truncate(),
                        );
                    });
                    return;
                };
                ui.set_clip_rect(ui.max_rect());
                if let (
                    OperationArea::Mutation {
                        operation: gg_engine::mutation::Operation::Rebase,
                        conflicts,
                        pending,
                        ..
                    },
                    Some(progress),
                ) = (&area, &rebase_progress)
                {
                    let keyed = !pending
                        && rebase_stop::continue_reason(progress, *conflicts).is_none()
                        && crate::navigation::available(ui.ctx())
                        && !ui.ctx().text_edit_focused()
                        && ui.ctx().input_mut(|i| {
                            i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter)
                        });
                    match rebase_stop::show(ui, progress, *conflicts, *pending) {
                        Some(rebase_stop::Command::Continue) => {
                            mutation_action = Some(MutationAction::Continue);
                        }
                        Some(rebase_stop::Command::Skip) => {
                            mutation_action = Some(MutationAction::Skip);
                        }
                        Some(rebase_stop::Command::Abort) => {
                            mutation_action = Some(MutationAction::Abort);
                        }
                        Some(rebase_stop::Command::ResolveConflicts) => resolve_conflicts = true,
                        None if keyed => mutation_action = Some(MutationAction::Continue),
                        None => {}
                    }
                    return;
                }
                ui.horizontal(|ui| {
                    match &area {
                        OperationArea::Mutation {
                            operation,
                            commit,
                            conflicts,
                            pending,
                        } => {
                            ui.strong(format!("{} in progress", operation.as_str()));
                            if let Some(commit) = commit {
                                ui.weak(format!(
                                    "· {}",
                                    commit.chars().take(8).collect::<String>()
                                ));
                            }
                            if *conflicts > 0 {
                                ui.label(format!(
                                    "· {conflicts} conflict{}",
                                    if *conflicts == 1 { "" } else { "s" }
                                ));
                                if ui
                                    .add_enabled(
                                        !pending,
                                        egui::Button::new("Resolve conflicts…"),
                                    )
                                    .clicked()
                                {
                                    resolve_conflicts = true;
                                }
                            }
                            let reason = if *conflicts > 0 {
                                "Resolve every unmerged path first."
                            } else {
                                "Continue the in-progress operation."
                            };
                            if ui
                                .add_enabled(
                                    !pending && *conflicts == 0,
                                    egui::Button::new("Continue"),
                                )
                                .on_hover_text(reason)
                                .clicked()
                            {
                                mutation_action = Some(MutationAction::Continue);
                            }
                            if *operation != gg_engine::mutation::Operation::Merge
                                && ui
                                    .add_enabled(!pending, egui::Button::new("Skip"))
                                    .on_hover_text("Skip the current commit and continue.")
                                    .clicked()
                            {
                                mutation_action = Some(MutationAction::Skip);
                            }
                            if ui
                                .add_enabled(!pending, egui::Button::new("Abort"))
                                .on_hover_text("Abort the operation and restore the previous tip.")
                                .clicked()
                            {
                                mutation_action = Some(MutationAction::Abort);
                            }
                        }
                        OperationArea::Rewrite { refreshing } => {
                            ui.spinner();
                            ui.strong(if *refreshing {
                                "Refreshing history…"
                            } else {
                                "Rewriting commits…"
                            });
                            ui.weak(
                                "Browsing stays available; the old history is labeled until the replacement is accepted.",
                            );
                        }
                        OperationArea::Worker { label } => {
                            ui.spinner();
                            ui.strong(label);
                        }
                        OperationArea::Network {
                            operation,
                            progress,
                            cancelling,
                        } => {
                            ui.spinner();
                            if let Some(progress) = progress {
                                ui.strong(progress.operation.as_str());
                                ui.label(format!(
                                    "· {} {}{}",
                                    progress.phase,
                                    progress
                                        .percent
                                        .map(|percent| format!("{percent}% "))
                                        .unwrap_or_default(),
                                    progress.message
                                ));
                            } else {
                                ui.strong(format!("Starting {}…", operation.as_str()));
                            }
                            if *cancelling {
                                ui.weak("Cancelling…");
                            } else if ui.button("Cancel").clicked() {
                                cancel_network = true;
                            }
                        }
                        OperationArea::Branch { label } => {
                            ui.spinner();
                            ui.strong(label);
                        }
                    }
                });
            });
        }
        if resolve_conflicts {
            self.open_conflict_view(ctx);
        }
        if cancel_network && let Some(worker) = &self.network {
            worker.cancel();
        }
        if let Some(action) = mutation_action
            && let Some(operation) = self.mutation.as_ref().map(|state| state.operation)
            && let Err(error) = self.recover_mutation(ctx, operation, action)
        {
            self.mutation_error = Some(error);
        }
        if self.edit_path {
            egui::Panel::top("open-repository").show(ui, |ui| {
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.path)
                            .desired_width((ui.available_width() - 156.0).max(100.0))
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
                    let open_ready = self.pending.is_none() && !self.repository_switch_pending();
                    let enter =
                        response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui
                        .add_enabled(open_ready, egui::Button::new("Open"))
                        .clicked()
                        || (open_ready && enter)
                    {
                        self.recovery = None;
                        if let Some(working) = &self.working {
                            self.working_drafts
                                .insert(working.repository_key(), working.capture_draft());
                        }
                        self.working = None;
                        self.review = None;
                        self.load(ctx);
                        self.edit_path = false;
                    }
                });
            });
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
                .default_size(230.0)
                .min_size(200.0)
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
            match action {
                Some(refs_view::Action::OpenWorktree(path)) => {
                    let (mut local, mut history) = (false, false);
                    self.run_frame_command(
                        ctx,
                        FrameCommand::OpenRecent(PathBuf::from(path)),
                        &mut local,
                        &mut history,
                    );
                }
                Some(refs_view::Action::Switch(name)) => self.start_branch(ctx, name, false),
                Some(refs_view::Action::Jump(id)) => self.jump_to_ref(id),
                Some(refs_view::Action::Track(name)) => {
                    self.track_dialog = Some(name);
                }
                Some(refs_view::Action::PullRebase) => self.start_pull_rebase(ctx),
                Some(refs_view::Action::OpenStash(index)) => {
                    self.stash_picker.selected = index;
                    self.stash_picker.confirm_drop = false;
                    self.stash_picker.error = None;
                    egui::Popup::open_id(ctx, stash_picker_popup_id());
                }
                Some(refs_view::Action::Merge(name)) => {
                    self.merge_dialog = Some(MergeDialog {
                        reference: name,
                        no_ff: self.prefs.merge_no_ff,
                        error: None,
                    });
                }
                Some(refs_view::Action::Rebase(name)) => {
                    self.rebase_dialog = Some(RebaseDialog {
                        reference: name,
                        autostash: self.prefs.rebase_autostash,
                        error: None,
                    });
                }
                Some(refs_view::Action::DeleteBranch(name)) => {
                    self.branch_delete = Some(BranchDeleteDialog { name, error: None });
                }
                Some(refs_view::Action::DeleteTag(name)) => {
                    self.tag_delete = Some(TagDeleteDialog { name, error: None });
                }
                Some(refs_view::Action::VerifyTag(name)) => self.open_signature(ctx, name),
                None => {}
            }
        }
        if go_local {
            self.open_working(ctx);
        }
        if go_history {
            self.close_to_history(ctx);
            self.narrow_details = false;
        }
        self.draw_folders_dialog(ctx);
        self.draw_menu_dialogs(ctx);
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
                    ui.label("Destination");
                    ui.horizontal(|ui| {
                        let selected = if candidates
                            .iter()
                            .any(|candidate| candidate.to_string_lossy() == dialog.folder.trim())
                        {
                            dialog.folder.trim().to_owned()
                        } else {
                            "Custom folder".to_owned()
                        };
                        egui::ComboBox::from_id_salt("clone-folder")
                            .selected_text(selected)
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for candidate in &candidates {
                                    let label = candidate.to_string_lossy().into_owned();
                                    let text = clone_folder_label(candidate, &default_folder);
                                    if ui
                                        .selectable_label(dialog.folder.trim() == label, text)
                                        .clicked()
                                    {
                                        dialog.folder = label;
                                        chosen_clone_folder = Some(candidate.clone());
                                    }
                                }
                            });
                        let folder = ui.add(
                            egui::TextEdit::singleline(&mut dialog.folder)
                                .hint_text("Parent folder")
                                .desired_width(170.0),
                        );
                        if folder.lost_focus() {
                            field_lost_focus = true;
                            let typed = PathBuf::from(dialog.folder.trim());
                            if typed.is_absolute() {
                                chosen_clone_folder = Some(typed);
                            }
                        }
                        if ui
                            .button("Browse…")
                            .on_hover_text("Choose the parent folder with the desktop dialog")
                            .clicked()
                            && let Some(path) = folder_picker::pick_folder(
                                "Clone destination",
                                Some(Path::new(dialog.folder.trim())),
                            )
                        {
                            dialog.folder = path.to_string_lossy().into_owned();
                            chosen_clone_folder = Some(path);
                        }
                        if ui
                            .button("Folders…")
                            .on_hover_text("Edit the development base folders and the default")
                            .clicked()
                        {
                            open_folders = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        let name = ui.add(
                            egui::TextEdit::singleline(&mut dialog.name)
                                .hint_text("Project folder")
                                .desired_width(220.0),
                        );
                        if name.changed() {
                            dialog.name_from_url = false;
                        }
                        field_lost_focus |= name.lost_focus();
                    });
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
                RecoveryToolAction::Reflog(action) => match action {
                    reflog_view::Action::Open {
                        commit,
                        subject,
                        selector,
                    } => {
                        self.open_recovery_review(
                            ctx,
                            return_location::RecoverySource::Reflog,
                            self.recovery
                                .as_ref()
                                .map(|view| view.reflog.reference().to_owned()),
                            commit,
                            subject,
                            selector,
                        );
                    }
                    reflog_view::Action::Reset { commit, mode } => {
                        self.reset_confirm = Some((commit, mode));
                    }
                    reflog_view::Action::CreateBranch { commit, selector } => {
                        self.create_branch = Some(CreateBranchDialog {
                            name: String::new(),
                            commit,
                            selector,
                            switch: false,
                            recovery_source: Some(return_location::RecoverySource::Reflog),
                            error: None,
                        });
                    }
                },
                RecoveryToolAction::Backups(action) => match action {
                    backups_view::Action::Open {
                        commit,
                        subject,
                        reference,
                    } => {
                        self.open_recovery_review(
                            ctx,
                            return_location::RecoverySource::Backups,
                            None,
                            commit,
                            subject,
                            reference,
                        );
                    }
                    backups_view::Action::Reset { commit, mode } => {
                        self.reset_confirm = Some((commit, mode));
                    }
                    backups_view::Action::CreateBranch { commit, selector } => {
                        self.create_branch = Some(CreateBranchDialog {
                            name: String::new(),
                            commit,
                            selector,
                            switch: false,
                            recovery_source: Some(return_location::RecoverySource::Backups),
                            error: None,
                        });
                    }
                },
            }
        }

        let mut create_branch_submit = None;
        let mut create_branch_cancel = false;
        if let Some(dialog) = &mut self.create_branch {
            egui::Window::new(dialog.title())
                .id(egui::Id::unique("create-branch-dialog"))
                .modal(ctx, egui::Id::unique("create-branch-dialog"))
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Create a branch at {} ({})",
                        dialog.commit.chars().take(8).collect::<String>(),
                        dialog.selector
                    ));
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
                            receipt.commit.chars().take(8).collect::<String>()
                        );
                        self.create_branch = None;
                        self.load(ctx);
                        if let Some(view) = &mut self.recovery {
                            match source {
                                Some(return_location::RecoverySource::Reflog) => {
                                    view.reflog.notice(notice)
                                }
                                Some(return_location::RecoverySource::Backups) => {
                                    view.backups.notice(notice)
                                }
                                None => {}
                            }
                        }
                        if switch {
                            self.branch_after_create = Some(receipt.branch.clone());
                            self.start_branch(ctx, receipt.branch.clone(), false);
                            if self.branch.is_none() {
                                self.branch_after_create = None;
                                if let Some(error) = self.branch_error.take() {
                                    let summary = format!(
                                        "Created {} but the switch could not start: {error}",
                                        receipt.branch
                                    );
                                    self.set_outcome(OutcomeSeverity::Warning, summary, None, None);
                                }
                            }
                        } else if source.is_none() {
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
        let mut branch_delete_open = true;
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
                    ui.label(format!("Delete local branch {}?", dialog.name));
                    if let Some(error) = &dialog.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            branch_delete_action = Some((dialog.name.clone(), false));
                        }
                        if ui.button("Force delete").clicked() {
                            branch_delete_action = Some((dialog.name.clone(), true));
                        }
                        if ui.button("Cancel").clicked() {
                            cancelled = true;
                        }
                    });
                });
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
        if let Some((commit, mode)) = self.reset_confirm.clone() {
            egui::Window::new("Reset here")
                .id(egui::Id::unique("reset-here"))
                .modal(ctx, egui::Id::unique("reset-here"))
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Reset the current branch to {} using {} reset?",
                        commit.chars().take(8).collect::<String>(),
                        mode.as_str()
                    ));
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
                    if mode == gg_engine::reset::Mode::Hard {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            "Hard reset discards staged and working changes.",
                        );
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Reset").clicked() {
                            reset_submit = Some((commit.clone(), mode));
                        }
                        if ui.button("Cancel").clicked() {
                            reset_cancel = true;
                        }
                    });
                });
        }
        if let (Some(mode), Some((_, current))) = (reset_mode, &mut self.reset_confirm) {
            *current = mode;
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
            let mut open = true;
            egui::Window::new("Keyboard shortcuts")
                .id(egui::Id::unique("keyboard-shortcuts"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(420.0)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    for (group, keys) in [
                        (
                            "Navigation",
                            "Up/Down or j/k: move · PgUp/PgDn: page · Enter: review · Space: details · Esc: back",
                        ),
                        (
                            "Repository",
                            "Ctrl/Cmd+K: search · Ctrl/Cmd+R: refresh · Ctrl/Cmd+F: filter · Ctrl/Cmd+C: copy ID",
                        ),
                        (
                            "Commit actions",
                            "Alt+A: actions for the selected commit · R: interactive rebase · menus: Repository, Branch, Commit, Changes, View, Help",
                        ),
                    ] {
                        ui.strong(group);
                        ui.label(keys);
                        ui.add_space(6.0);
                    }
                });
            if !open {
                self.help_shortcuts = false;
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
                    if let Some(error) = &dialog.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Stash").clicked() {
                            stash_submit = Some((
                                dialog.message.clone(),
                                dialog.include_untracked,
                                dialog.keep_index,
                            ));
                        }
                        if ui.button("Cancel").clicked() {
                            cancelled = true;
                        }
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

        let mut stash_action = None;
        let mut stash_window_open = true;
        let mut stash_refresh = false;
        let mut restore_reference = None;
        if let Some(window) = &mut self.stash_window {
            egui::Window::new("Stashes")
                .id(egui::Id::unique("stashes"))
                .open(&mut stash_window_open)
                .collapsible(false)
                .resizable(true)
                .default_width(520.0)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    if let Some(error) = &window.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    if let Some(notice) = &window.notice {
                        ui.label(notice);
                    }
                    if ui.button("Refresh").clicked() {
                        stash_refresh = true;
                    }
                    ui.separator();
                    if window.entries.is_empty() {
                        ui.label("No stashes.");
                    }
                    for entry in &window.entries {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&entry.reference).strong());
                            ui.label(egui::RichText::new(&entry.message).size(11.0));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("Drop").clicked() {
                                        window.confirm_drop = Some(entry.reference.clone());
                                    }
                                    if ui.button("Restore…").clicked() {
                                        restore_reference = Some(entry.reference.clone());
                                    }
                                },
                            );
                        });
                    }
                });
            if window.confirm_drop.is_some() {
                let reference = window.confirm_drop.clone().unwrap_or_default();
                egui::Window::new("Drop stash")
                    .id(egui::Id::unique("drop-stash"))
                    .modal(ctx, egui::Id::unique("drop-stash"))
                    .collapsible(false)
                    .resizable(false)
                    .default_pos(ctx.content_rect().center())
                    .pivot(egui::Align2::CENTER_CENTER)
                    .show(ctx, |ui| {
                        ui.label(format!(
                            "Drop {reference}? This removes the stash entry, not working files."
                        ));
                        ui.horizontal(|ui| {
                            if ui.button("Drop").clicked() {
                                stash_action = Some(StashAction::Drop(reference.clone()));
                                window.confirm_drop = None;
                            }
                            if ui.button("Cancel").clicked() {
                                window.confirm_drop = None;
                            }
                        });
                    });
            }
        }
        if let Some(reference) = restore_reference {
            self.restore_stash = Some(RestoreStashDialog {
                reference,
                restore_index: false,
                remove: false,
                error: None,
            });
        }
        let mut restore_submit = false;
        if let Some(dialog) = &mut self.restore_stash {
            let mut open = true;
            let mut cancelled = false;
            egui::Window::new("Restore stash")
                .id(egui::Id::unique("restore-stash"))
                .modal(ctx, egui::Id::unique("restore-stash"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(420.0)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Restore {} into the working tree.",
                        dialog.reference
                    ));
                    ui.checkbox(&mut dialog.restore_index, "Restore staged state");
                    ui.checkbox(&mut dialog.remove, "Remove stash after successful restore");
                    if let Some(error) = &dialog.error {
                        egui::ScrollArea::vertical()
                            .id_salt("restore-stash-error")
                            .max_height(80.0)
                            .show(ui, |ui| {
                                ui.colored_label(ui.visuals().error_fg_color, error);
                            });
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Restore stash").clicked() {
                            restore_submit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancelled = true;
                        }
                    });
                });
            if cancelled || !open {
                self.restore_stash = None;
            }
        }
        if restore_submit && let Some(dialog) = self.restore_stash.clone() {
            match self.restore_stash_inner(
                ctx,
                &dialog.reference,
                dialog.remove,
                dialog.restore_index,
            ) {
                Ok(()) => self.restore_stash = None,
                Err(error) => {
                    if let Some(dialog) = &mut self.restore_stash {
                        dialog.error = Some(error);
                    }
                }
            }
        }
        if stash_refresh {
            self.refresh_stashes();
        }
        if let Some(action) = stash_action {
            let result = match action {
                StashAction::Drop(reference) => self.drop_stash(ctx, &reference),
            };
            if let Err(error) = result
                && let Some(window) = &mut self.stash_window
            {
                window.error = Some(error);
            }
        } else if self.stash_window.is_some() && !stash_window_open {
            self.stash_window = None;
        }

        self.draw_submodule(ctx);
        self.draw_lfs(ctx);
        self.draw_signature(ctx);
        self.draw_tag_create(ctx);
        self.draw_force_push(ctx);
        if self.conflict_view.is_some() {
            egui::CentralPanel::default().show(ui, |ui| self.draw_conflict_workspace(ui));
            return;
        }
        if self.working.is_some() {
            let external = self.external_operation_reason();
            let close = self.working.as_mut().is_some_and(|working| {
                working.set_external_busy(external);
                working.draw_ui(ui)
            });
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

        let operation_active = self.operation_area().is_some();
        egui::Panel::bottom("status").exact_size(25.0).show(ui, |ui| {
            let (message, color) = if let Some(error) = &self.network_error {
                (format!("Network · {error}"), ui.visuals().error_fg_color)
            } else if let Some(error) = &self.state.error {
                (format!("{}{}", if self.state.history.is_some() { "Previous results · " } else { "" }, error), ui.visuals().error_fg_color)
            } else if let Some(error) = &self.branch_error {
                (format!("Branch · {error}"), ui.visuals().error_fg_color)
            } else if let Some(error) = &self.mutation_error {
                (format!("History · {error}"), ui.visuals().error_fg_color)
            } else if !operation_active && let Some(outcome) = &self.outcome {
                let color = match outcome.severity {
                    OutcomeSeverity::Info => ui.visuals().text_color(),
                    OutcomeSeverity::Warning => ui.visuals().warn_fg_color,
                    OutcomeSeverity::Error => ui.visuals().error_fg_color,
                };
                (
                    format!(
                        "{} · {}",
                        if matches!(
                            outcome.recovery,
                            Some(RecoveryAction::RetrySwitch | RecoveryAction::StashAndSwitch)
                        ) {
                            "Branch"
                        } else {
                            "History"
                        },
                        outcome.summary
                    ),
                    color,
                )
            } else if !operation_active && let Some(worker) = &self.network {
                if worker.is_cancelled() {
                    ("Cancelling network operation…".into(), ui.visuals().text_color())
                } else if let Some(progress) = &self.network_progress {
                    (format!(
                        "{} · {}{} · {}",
                        progress.operation.as_str(),
                        progress.phase,
                        progress.percent.map(|percent| format!(" {percent}%")).unwrap_or_default(),
                        progress.message
                    ), ui.visuals().text_color())
                } else {
                    (format!("Starting {}…", worker.operation.as_str()), ui.visuals().text_color())
                }
            } else if !operation_active && let Some(notice) = &self.network_notice {
                (format!("Network · {notice}"), ui.visuals().text_color())
            } else if !operation_active && let Some(warning) = &self.branch_warning {
                (format!("Branch · {warning}"), ui.visuals().text_color())
            } else if !operation_active && let Some(label) = &self.branch_label {
                (label.clone(), ui.visuals().text_color())
            } else if !operation_active && let Some(notice) = &self.stash_notice {
                (format!("Stash · {notice}"), ui.visuals().text_color())
            } else if !operation_active && let Some(notice) = &self.mutation_notice {
                (format!("History · {notice}"), ui.visuals().text_color())
            } else if self.pending.is_some() {
                (format!("Reading {}{}", self.state.requested.as_deref().unwrap_or(std::path::Path::new(".")).display(), if self.state.history.is_some() { " · showing previous results" } else { "" }), ui.visuals().text_color())
            } else if !self.state.finished && self.worker.is_some() {
                (format!("{} · {} commits examined", if self.state.query.is_empty() { "Counting history" } else { "Searching history" }, self.state.scanned), ui.visuals().text_color())
            } else {
                let tracking = self
                    .state
                    .refs
                    .current_tracking()
                    .map(|tracking| format!("{}   ·   ", refs_view::tracking_label(tracking)))
                    .unwrap_or_default();
                let hints = if narrow {
                    "j/k: select · Enter: review · Space: details · Esc: history · Ctrl/Cmd+F: filter"
                } else {
                    "j k: select   ·   Enter: review   ·   Space: details   ·   R: rebase from here   ·   Ctrl+K: search   ·   Ctrl/Cmd+F: filter   ·   Ctrl/Cmd+C: copy ID"
                };
                (format!("{tracking}{hints}"), ui.visuals().weak_text_color())
            };
            let outcome_active = !operation_active
                && self.network_error.is_none()
                && self.state.error.is_none()
                && self.branch_error.is_none()
                && self.mutation_error.is_none()
                && self.outcome.is_some();
            let details = outcome_active
                .then(|| self.outcome.as_ref().and_then(|outcome| outcome.details.clone()))
                .flatten();
            let recovery = outcome_active
                .then(|| self.outcome.as_ref().and_then(|outcome| outcome.recovery))
                .flatten();
            let mut details_clicked = false;
            let mut recovery_clicked = None;
            let mut dismiss_clicked = false;
            ui.horizontal(|ui| {
                let reserved = 70.0
                    + if recovery.is_some() {
                        130.0
                    } else if details.is_some() {
                        76.0
                    } else {
                        0.0
                    };
                let width = (ui.available_width() - reserved).max(100.0);
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(width, 18.0),
                    egui::Sense::hover(),
                );
                bounded_text::paint(ui, rect, &message, egui::FontId::proportional(11.0), color);
                if details.is_some() && ui.small_button("Details…").clicked() {
                    details_clicked = true;
                }
                if let Some(action) = recovery
                    && ui.small_button(recovery_label(action)).clicked()
                {
                    recovery_clicked = Some(action);
                }
                if outcome_active && ui.small_button("Dismiss").clicked() {
                    dismiss_clicked = true;
                }
            });
            if details_clicked {
                self.outcome_details_open = true;
            }
            if let Some(action) = recovery_clicked {
                self.run_recovery_action(ctx, action);
            }
            if dismiss_clicked {
                self.dismiss_outcome();
            }
        });
        if self.outcome_details_open {
            let details = self
                .outcome
                .as_ref()
                .and_then(|outcome| outcome.details.clone());
            if let Some(details) = details {
                let mut open = true;
                egui::Window::new("Operation details")
                    .id(egui::Id::unique("operation-details"))
                    .open(&mut open)
                    .collapsible(false)
                    .resizable(true)
                    .default_width(560.0)
                    .default_height(240.0)
                    .default_pos(ctx.content_rect().center())
                    .pivot(egui::Align2::CENTER_CENTER)
                    .show(ctx, |ui| {
                        if ui.button("Copy details").clicked() {
                            ui.ctx().copy_text(details.clone());
                        }
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.add(egui::Label::new(&details).wrap().selectable(true));
                        });
                    });
                if !open {
                    self.outcome_details_open = false;
                }
            } else {
                self.outcome_details_open = false;
            }
        }
        self.details_rect = egui::Rect::NOTHING;
        if !narrow && self.details_visible {
            // The commit inspector sits under the history list, as in the
            // History mockup; the refs sidebar keeps the full height.
            let details = egui::Panel::bottom("details")
                .exact_size(details_view::INSPECTOR_HEIGHT)
                .resizable(false)
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    let commit = self.state.displayed_details();
                    let menu = self.commit_menu_entries();
                    let command = details_view::inspector(ui, commit, &mut self.avatars, |ui| {
                        commit.and_then(|_| commit_action_menu(ui, &menu))
                    });
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
        let mut right_clicked_row: Option<usize> = None;
        egui::CentralPanel::default().show(ui, |ui| {
            if narrow && self.narrow_details {
                self.history_rect = egui::Rect::NOTHING;
                self.details_rect = ui.available_rect_before_wrap();
                if ui.button("Back to history").clicked() {
                    self.narrow_details = false;
                }
                ui.add_space(8.0);
                let commit = self.state.displayed_details();
                let loading = self.state.details.is_none();
                let menu = self.commit_menu_entries();
                let command = details_view::show(ui, commit, loading, &mut self.avatars, |ui| {
                    commit.and_then(|_| commit_action_menu(ui, &menu))
                });
                if let Some(command) = command
                    && let Some(commit) = commit
                {
                    let id = commit.id.clone();
                    let subject = commit.subject().to_owned();
                    self.apply_commit_command(ctx, &id, &subject, command);
                }
                return;
            }
            if self.state.history.is_some() {
                if self.state.count == 0 {
                    ui.label(if !self.state.finished {
                        "Searching history..."
                    } else if self.state.query.is_empty() {
                        "Ready for the first commit"
                    } else {
                        "No commits match this filter."
                    });
                }
                self.history_rect = ui.available_rect_before_wrap();
                let row_menu = self.commit_menu_entries();
                egui::ScrollArea::horizontal()
                    .id_salt("history-columns")
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
                                    if let Some(commit) = self.state.rows.get(&row) {
                                        let mut refs = self
                                            .state
                                            .refs
                                            .for_commit(&commit.id)
                                            .cloned()
                                            .collect::<Vec<_>>();
                                        history_view::sort_labels(&mut refs, head_name.as_deref());
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
                    });
            } else if self.pending.is_some() {
                // The status line reports the read; the pane stays empty.
            } else {
                ui.label("Choose Open… to enter a repository path.");
            }
        });
        if let Some(row) = right_clicked_row {
            self.state.select_row(row);
            if self.worker.is_none() {
                self.state.details = self.state.rows.get(&row).cloned();
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
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
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
        app.clone_dialog = Some(CloneDialog::from_url(
            Some("https://example.org/gg.git".into()),
            folders::starting_folder(),
        ));
        assert!(rendered(&mut app).iter().any(|line| line == "Browse…"));
        app.clone_dialog = None;
        app.folders_dialog = Some(FoldersDialog::new(PrefSection::General));
        assert!(rendered(&mut app).iter().any(|line| line == "Browse…"));
        app.folders_dialog = None;
        app.edit_path = true;
        assert!(rendered(&mut app).iter().any(|line| line == "Browse…"));
        app.edit_path = false;
        app.worktree_window = Some(WorktreeWindow {
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
            }),
            confirm_remove: None,
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
        let browse = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.job.text == "Browse…" => {
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
            }
            _ => None,
        });
        assert!(browse.is_some(), "the folder picker button is missing");
        assert!(
            window.is_some_and(|window| browse.is_some_and(|button| window.contains_rect(button))),
            "the folder picker button is clipped: {browse:?} in {window:?}"
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
                text.iter().any(|line| line.contains("BRANCHES  0")),
                visible,
                "width={width}: {text:?}"
            );
            assert!(app.history_rect.width() >= history_view::MIN_WIDTH);
        }
    }

    #[test]
    fn current_tracking_appears_in_sidebar_and_status_line() {
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
            let labels = text
                .iter()
                .filter(|line| line.contains("origin/feature/a-long-branch-name"))
                .count();
            assert!(labels >= 1, "width={width}: {text:?}");
            if width >= 1120.0 {
                assert!(labels >= 2, "width={width}: {text:?}");
            }
        }
    }

    #[test]
    fn tracking_pull_button_stays_inside_the_refs_pane() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let mut app = layout_fixture();
        app.state.refs.head = Some(gg_engine::refs::Head {
            name: Some("refs/heads/repo-and-branch-ux".into()),
            detached: false,
            commit: Some(format!("{:040x}", 99)),
        });
        app.state.refs.tracking = vec![gg_engine::refs::Tracking {
            branch: "repo-and-branch-ux".into(),
            full_name: "refs/heads/repo-and-branch-ux".into(),
            upstream: "origin/repo-and-branch-ux".into(),
            upstream_full_name: "refs/remotes/origin/repo-and-branch-ux".into(),
            remote: "origin".into(),
            remote_branch: "repo-and-branch-ux".into(),
            ahead: Some(0),
            behind: Some(0),
        }];
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.refs_width > 0.0, "the refs pane did not render");
        let button = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.job.text == "Pull" => {
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
            }
            _ => None,
        });
        assert!(button.is_some(), "the Pull button is missing");
        assert!(
            button.is_some_and(|rect| rect.max.x <= app.refs_width + 0.5),
            "the Pull button is clipped by the {} point pane: {button:?}",
            app.refs_width
        );
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
                    assert!(text.iter().any(|line| line.contains("Destination")));
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
        assert_eq!(settings::read::<bool>(&settings_file, "merge_no_ff"), None);
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
                .add("/tmp/another-clone-folder")
                .unwrap_or(false)
        );
        let two = rect(&mut app);
        assert_eq!(two, empty, "the window moved after the second folder");
        assert!(app.folders.add("/tmp/third-clone-folder").unwrap_or(false));
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

            app.stash_dialog = None;
            app.stash_window = Some(StashWindow {
                entries: vec![gg_engine::stash::StashEntry {
                    index: 0,
                    reference: "stash@{0}".into(),
                    commit: "abc123".into(),
                    message: "On main: work in progress".into(),
                    ..Default::default()
                }],
                error: None,
                notice: None,
                confirm_drop: None,
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
            assert!(text.iter().any(|line| line.contains("Stashes")));
            assert!(text.iter().any(|line| line.contains("stash@{0}")));
            assert!(text.iter().any(|line| line.contains("Restore…")));
            assert!(text.iter().any(|line| line.contains("Drop")));

            app.restore_stash = Some(RestoreStashDialog {
                reference: "stash@{0}".into(),
                restore_index: true,
                remove: true,
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
            assert!(text.iter().any(|line| line.contains("Restore stash")));
            assert!(
                text.iter()
                    .any(|line| line.contains("Restore staged state"))
            );
            assert!(
                text.iter()
                    .any(|line| line.contains("Remove stash after successful restore"))
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
    fn restore_stash_error_region_stays_bounded() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.restore_stash = Some(RestoreStashDialog {
            reference: "stash@{0}".into(),
            restore_index: false,
            remove: false,
            error: Some(
                (0..40)
                    .map(|line| format!("restore failure line {line}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        });
        let _ = crate::tests::run_ui(&ctx, frame_input(720.0, 480.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let output = crate::tests::run_ui(&ctx, frame_input(720.0, 480.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let error_clip = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text)
                if text.galley.job.text.contains("restore failure line 0") =>
            {
                Some(shape.clip_rect)
            }
            _ => None,
        });
        assert!(error_clip.is_some(), "the restore error is rendered");
        if let Some(error_clip) = error_clip {
            assert!(
                error_clip.height() <= 80.0,
                "error region grew to {:?}",
                error_clip
            );
        }
        for expected in [
            "Restore stash",
            "Restore staged state",
            "Remove stash after successful restore",
        ] {
            assert!(
                output.shapes.iter().any(|shape| matches!(
                    &shape.shape,
                    egui::epaint::Shape::Text(text)
                        if text.galley.job.text.contains(expected)
                )),
                "missing {expected} while an error is shown"
            );
        }
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
            assert!(text.iter().any(|line| line.contains("Force delete")));

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
            assert!(text.iter().any(|line| line.contains("Hard reset discards")));
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
                tab: ConflictTab::Ours,
                blocks: None,
                blocks_note: None,
                whole_file: false,
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
            assert!(text.iter().any(|line| line.contains("Resolve conflicts")));
            assert!(text.iter().any(|line| line.contains("Paths")));
            assert!(text.iter().any(|line| line.contains("Base")));
            assert!(text.iter().any(|line| line.contains("Ours")));
            assert!(text.iter().any(|line| line.contains("Theirs")));
            assert!(text.iter().any(|line| line.contains("Preview source")));
            assert!(text.iter().any(|line| line.contains("Working file")));
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
            assert!(text.iter().any(|line| line.contains("merge in progress")));
            assert!(text.iter().any(|line| line.contains("Continue")));
            assert!(text.iter().any(|line| line.contains("Abort")));
            assert!(!text.iter().any(|line| line == &"Skip"), "{text:?}");
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
            assert!(text.iter().any(|line| line.contains("rebase in progress")));
            assert!(text.iter().any(|line| line.contains("Continue")));
            assert!(text.iter().any(|line| line.contains("Skip")));
            assert!(text.iter().any(|line| line.contains("Abort")));
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
                "third change",
            ] {
                assert!(
                    text.iter().any(|line| line.contains(expected)),
                    "width {width}: missing {expected:?} in {text:?}"
                );
            }
            assert!(
                text.iter()
                    .any(|line| line.contains("feature · rebase 2/3")),
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
    fn panes_hold_still_when_an_operation_starts_grows_and_ends() {
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
        assert_eq!(
            top(&mut stopped),
            idle,
            "a rebase stop does not push History down"
        );
        assert_eq!(
            top(&mut committed),
            idle,
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
            (find("Changed files").or(find("PATH")), find("…HEAD onto"))
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
        assert!(text.iter().any(|line| line == "Back to rebase"), "{text:?}");
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
        app.rewrite_job = Some(RewriteJob {
            receiver,
            restore_row: None,
        });
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
                .filter(|line| line.contains("Rewriting commits"))
                .count(),
            1,
            "one operation area reports the rewrite: {text:?}"
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
                Ok(OperationOutcome::Rebase(gg_engine::rebase::Receipt {
                    reference: "main".into(),
                    commit: None,
                    previous: None,
                    backup: None,
                    applied: true,
                    in_progress: false,
                    conflicts: Vec::new(),
                    warnings: Vec::new(),
                }))
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
                .is_some_and(|notice| notice.contains("Rebased onto"))
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
                if text.galley.job.text.contains("Rebasing onto feature")
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
            app.mutation_notice
                .as_deref()
                .is_some_and(|notice| notice.contains("Rebased onto feature")),
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
            tab: ConflictTab::Ours,
            blocks: None,
            blocks_note: None,
            whole_file: false,
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
            "Conflict 1 of 1",
            "4 Theirs + ours",
            "Result · 1/1 blocks chosen · preview, not staged",
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

        app.stage_conflict_resolution(&ctx)?;
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
            snapshot: None,
            preview: gg_engine::conflict::Resolution::Working,
            tab: ConflictTab::Ours,
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
                egui::epaint::Shape::Text(text) if text.galley.job.text.starts_with("Result ·") => {
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
        assert!(app.conflict_view.is_none(), "Esc closes the workspace");
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
            snapshot: Some(gg_engine::conflict::Snapshot {
                base: Some("1".repeat(40)),
                ours: Some("2".repeat(40)),
                theirs: Some("3".repeat(40)),
                working: Some(b"ours\n".to_vec()),
                working_hash: None,
                working_note: None,
            }),
            preview: gg_engine::conflict::Resolution::Working,
            tab: ConflictTab::Ours,
        });
        assert!(app.stage_conflict_resolution(&ctx).is_ok());
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
            // The popup mirrors the Commit menu, then its own extras.
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
                "Reset current branch to here…",
                "Copy subject",
                "Signature details…",
                "Recovery…",
            ] {
                assert!(text.contains(&label), "missing {label} at {width}");
            }
            assert_eq!(
                text.iter().filter(|line| **line == "Actions").count(),
                usize::from(width >= 1040.0),
                "the inspector's Actions button opens the menu at {width}"
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
        assert!(text.contains(&"Navigation"));
        assert!(text.contains(&"Actions"));
        assert!(text.iter().any(|line| line.contains("Alt+A")));
        assert!(
            text.iter().all(|line| !line.contains("View, Stash, Help")),
            "the retired top-level Stash menu is still advertised: {text:?}"
        );
    }

    #[test]
    fn operation_outcome_shows_details_and_recovery() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.outcome = Some(Outcome {
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
        assert!(text.contains(&"Operation details"));
        assert!(text.contains(&"Backup: refs/gg/backup/rewrite/1"));
    }

    #[test]
    fn outcome_recovery_button_opens_backups() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.outcome = Some(Outcome {
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
                    .any(|line| line.contains("cherry-pick in progress"))
            );
            assert!(text.iter().any(|line| line.contains("Continue")));
            assert!(text.iter().any(|line| line.contains("Skip")));
            assert!(text.iter().any(|line| line.contains("Abort")));
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
                    .filter(|line| line.contains("merge in progress"))
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
            tab: ConflictTab::Base,
            blocks: None,
            blocks_note: None,
            whole_file: false,
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
        assert!(text.iter().any(|line| line.contains("Resolve conflicts")));
        assert_eq!(
            text.iter().filter(|line| line.contains("Continue")).count(),
            1,
            "only the operation area offers Continue: {text:?}"
        );
        assert_eq!(
            text.iter().filter(|line| line.contains("Skip")).count(),
            1,
            "only the operation area offers Skip: {text:?}"
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
        assert!(text.iter().any(|line| line.contains("rebase in progress")));
        assert!(text.iter().any(|line| line.contains("Continue")));
        assert!(text.iter().any(|line| line.contains("Skip")));
        assert!(text.iter().any(|line| line.contains("Abort")));
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
    fn finish_network_clone_opens_the_new_repository() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let mut app = Browser::new(".".into());
        app.outcome = Some(Outcome {
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
            severity: OutcomeSeverity::Warning,
            summary: "Switch to feature blocked".into(),
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
                |summary| summary.contains("Created feature but the switch was blocked")
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
                entry: "main@{0}".into(),
            },
        );
        assert!(app.recovery.is_some(), "recovery stays open");
        if let Some(view) = app.recovery.as_ref() {
            assert_eq!(view.reflog.reference(), "main");
            assert_eq!(view.reflog.selected_selector(), Some("main@{0}"));
            assert_eq!(view.source, return_location::RecoverySource::Reflog);
        }
        Ok(())
    }

    #[test]
    fn recovery_window_offers_one_source_selector() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.recovery = Some(RecoveryView::new(
                "/projects/a-very-long-repository-name".into(),
            ));
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
            assert!(text.iter().any(|line| line.contains("Source")));
            assert!(text.iter().any(|line| line.contains("Backups")));
            assert!(!text.iter().any(|line| line.contains("Reflog…")));
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
        app.recovery = Some(RecoveryView::new(path.to_path_buf()));
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
        for expected in ["Recovery", "Open commit", "Copy ref"] {
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
    fn signature_window_separates_validity_trust_and_configuration() {
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
        let config = gg_engine::signature::Config {
            format: "openpgp".into(),
            signing_key: Some("F4545BDB96888825AA7CC8C7DFC666CDAB5C0CC8".into()),
            gpg_program: None,
            commit_signing: true,
            tag_signing: false,
            allowed_signers: None,
            tool_available: true,
            note: None,
        };
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.signature_window = Some(SignatureWindow {
                reference: verification.reference.clone(),
                verification: Some(verification.clone()),
                config: Some(config.clone()),
                error: None,
                busy: None,
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
                "Signature details",
                "Good signature",
                "Test User <test@example.org>",
                "F4545BDB96888825AA7CC8C7DFC666CDAB5C0CC8",
                "ultimate",
                "Signing configuration",
                "Sign commits",
                "Enabled",
                "Sign tags",
                "Disabled",
                "Verification is read-only",
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
            app.lfs_window = Some(LfsWindow {
                request: crate::diff_view::LfsRequest::Change(gg_engine::review::FileChange {
                    path: "asset.bin".into(),
                    path_bytes: b"asset.bin".to_vec(),
                    kind: gg_engine::review::ChangeKind::Modified,
                    old: None,
                    new: None,
                }),
                details: Some(details.clone()),
                content: Some(crate::binary_preview::decode(b"fetched content")),
                preview_note: None,
                texture: None,
                error: None,
                notice: None,
                busy: None,
                confirm_fetch: false,
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
                "LFS pointer",
                "asset.bin",
                oid,
                "12.1 KiB",
                "git-lfs is not installed",
                "Content preview",
                "Hexadecimal preview",
                "Bounded preview",
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
        };
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.submodule_window = Some(SubmoduleWindow {
                request: crate::diff_view::SubmoduleRequest {
                    path: "vendor/lib".into(),
                    path_bytes: b"vendor/lib".to_vec(),
                    before: Some("b".repeat(40)),
                    after: Some("a".repeat(40)),
                },
                details: Some(details.clone()),
                error: None,
                notice: None,
                busy: None,
                confirm_update: false,
                refresh_parent: false,
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
                "Submodule details",
                "vendor/lib",
                "Different commit",
                "Index",
                "Checked out",
                "Open nested repository",
                "Update…",
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
                snapshot: Some(snapshot.clone()),
                selected: Some(1),
                error: None,
                notice: None,
                create: None,
                confirm_remove: None,
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
                if width >= 1040.0 {
                    assert!(!app.history_rect.intersects(app.details_rect));
                    assert!(app.details_rect.right() <= width);
                } else {
                    assert_eq!(app.details_rect, egui::Rect::NOTHING);
                }
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
    fn incoming_count_waits_for_wheel_settling_and_preserves_hovered_content_and_offset() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let pointer = app.row_rects[5].1.center();
        app.staged = Some(stream::Update {
            request: stream::Request {
                selected_id: app.state.selected.clone(),
                selected_row: app.state.selected_row,
                ..Default::default()
            },
            anchor_row: None,
            identity: History {
                path: "repo".into(),
                head: "main".into(),
                commits: vec![],
                truncated: false,
                shallow: false,
            },
            query: String::new(),
            count: 100_000,
            scanned: 100_000,
            complete: true,
            rows: vec![],
            graphs: vec![],
            ref_lanes: Default::default(),
            refs: Default::default(),
            selection: None,
            error: None,
        });
        let mut input = frame_input(
            1120.0,
            740.0,
            vec![
                egui::Event::PointerMoved(pointer),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -52.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        input.time = Some(1.0);
        let _ = crate::tests::run_ui(&ctx, input, |ctx| app.draw_ui(ctx));
        assert_eq!(app.state.count, 100);
        for i in 1..20 {
            let mut input = frame_input(1120.0, 740.0, vec![]);
            input.time = Some(1.0 + i as f64 * 0.1);
            let _ = crate::tests::run_ui(&ctx, input, |ctx| app.draw_ui(ctx));
        }
        assert_eq!(app.state.count, 100_000);
        let offset = app.scroll_offset;
        let mut input = frame_input(1120.0, 740.0, vec![egui::Event::PointerGone]);
        input.time = Some(4.0);
        let _ = crate::tests::run_ui(&ctx, input, |ctx| app.draw_ui(ctx));
        assert_eq!(app.state.count, 100_000);
        assert!((app.scroll_offset - offset).abs() < 0.1);
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
        assert_eq!(shown("SIGN"), [PrefSection::Commit]);
        assert_eq!(shown("side by"), [PrefSection::Diff]);
        assert!(shown("nothing like this").is_empty());
    }

    #[test]
    fn window_title_names_the_folder_and_a_stopped_rebase() {
        assert_eq!(
            window_title("/home/me/src/gg", Some("/home/me"), None),
            "gg — ~/src/gg"
        );
        assert_eq!(
            window_title("/home/meow/gg", Some("/home/me"), None),
            "gg — /home/meow/gg"
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
            "gg — /r — rebasing feature · step 2 of 4"
        );
        progress.stop = gg_engine::mutation::RebaseStop::Conflict;
        assert_eq!(
            window_title("/r", None, Some(&progress)),
            "gg — /r — rebase conflict · step 2 of 4"
        );
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
                    "gg",
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
                    "-",
                    "Backups…",
                    "Reflog…",
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
                "Discard all…",
                "-",
                "Commit",
                "Amend HEAD",
                "-",
                "Stash changes",
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
            ["Keyboard shortcuts", "Documentation", "About gg"]
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
        let _ = frame(
            &mut app,
            vec![key(egui::Key::Num2, egui::Modifiers::COMMAND)],
        );
        assert!(app.working.is_none(), "Ctrl+2 returns to History");
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
    fn window_toolbar_keeps_its_icons_in_place_across_views_and_widths()
    -> Result<(), Box<dyn std::error::Error>> {
        let order = [
            crate::phosphor::SIDEBAR_SIMPLE,
            crate::phosphor::DOWNLOAD_SIMPLE,
            crate::phosphor::ARROW_LINE_DOWN,
            crate::phosphor::ARROW_LINE_UP,
            crate::phosphor::GIT_BRANCH,
            crate::phosphor::TAG,
            crate::phosphor::GIT_MERGE,
            crate::phosphor::ARCHIVE,
            crate::phosphor::BOX_ARROW_UP,
            crate::phosphor::LIST_NUMBERS,
            crate::phosphor::CLOCK_COUNTER_CLOCKWISE,
            crate::phosphor::MAGNIFYING_GLASS,
        ];
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
    fn toolbar_rewrite_covers_the_branch_and_keeps_the_selection()
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
        // Select an older commit on main: the toolbar must not depend on it.
        let selected = ids[3].clone();
        app.state.selected = Some(selected.clone());
        app.state.reconcile_selection();
        app.local_summary = Some(LocalSummary::read(path.to_path_buf()));
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text)
                    if text.galley.job.text == crate::phosphor::LIST_NUMBERS =>
                {
                    Some(text.pos + text.galley.size() / 2.0)
                }
                _ => None,
            })
            .ok_or("missing rewrite toolbar button")?;
        let events = vec![
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
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, events), |ctx| {
            app.draw_ui(ctx)
        });
        let dialog = app
            .rewrite_dialog
            .as_ref()
            .ok_or("rewrite dialog did not open")?;
        assert_eq!(
            app.state.selected.as_deref(),
            Some(selected.as_str()),
            "the toolbar leaves the history selection alone"
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
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text)
                    if text.galley.job.text == crate::phosphor::BOX_ARROW_UP =>
                {
                    Some(text.pos + text.galley.size() / 2.0)
                }
                _ => None,
            })
            .ok_or("missing stash picker button")?;
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
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, click), |ctx| {
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
            "2",
            "Apply stash@{0}",
            "Pop",
            "Show changes",
            "Drop…",
            // Ref, message without the branch prefix, branch, age, file count.
            "stash@{0}",
            "abc123 work 0",
            "main",
            "2 hours ago",
            "1 file",
            "detached",
            "3 files",
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
        assert_eq!(ready.branch_rewrite_count, 2);
        assert_eq!(ready.branch_rewrite_reason, None);
    }

    #[test]
    fn pull_is_ready_only_when_the_current_branch_is_behind() {
        let mut app = layout_fixture();
        assert_eq!(
            app.frame_readiness().pull_reason,
            Some("The current branch has no upstream to pull from")
        );
        app.state.refs.head = Some(gg_engine::refs::Head {
            name: Some("refs/heads/feature".into()),
            detached: false,
            commit: Some("a".repeat(40)),
        });
        let tracking = |behind| gg_engine::refs::Tracking {
            branch: "feature".into(),
            full_name: "refs/heads/feature".into(),
            upstream: "origin/feature".into(),
            upstream_full_name: "refs/remotes/origin/feature".into(),
            remote: "origin".into(),
            remote_branch: "feature".into(),
            ahead: Some(0),
            behind: Some(behind),
        };
        app.state.refs.tracking = vec![tracking(0)];
        let ready = app.frame_readiness();
        assert_eq!(ready.behind, 0);
        assert_eq!(
            ready.pull_reason,
            Some("Up to date with the upstream; fetch to check again")
        );
        app.state.refs.tracking = vec![tracking(3)];
        let ready = app.frame_readiness();
        assert_eq!(ready.behind, 3);
        assert_eq!(ready.pull_reason, None);
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
