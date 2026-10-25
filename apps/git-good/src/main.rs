mod avatars;
mod backups_view;
mod binary_preview;
mod bounded_text;
mod branch_cli;
mod branding;
mod colors;
mod commit_cli;
mod conflict_cli;
mod details_view;
mod diff_view;
mod diff_worker;
mod export;
mod file_history_view;
mod file_history_worker;
mod graph;
mod history_view;
#[cfg(test)]
mod interaction_tests;
mod merge_cli;
mod mutation_cli;
mod navigation;
mod network;
mod phosphor;
mod reader;
mod rebase_cli;
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
mod stage_view;
mod stash_cli;
mod state;
mod status_cli;
mod status_view;
mod status_worker;
mod stream;
mod sync_cli;
mod tag_cli;
mod thumbnails;
use eframe::egui;
#[cfg(test)]
use gg_engine::History;
use gg_engine::HistoryRequest;
use std::{ops::Range, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let first = args.next();
    if first.as_deref() == Some(std::ffi::OsStr::new("--version")) {
        println!("git-good {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("--help")) {
        println!(
            "gg · good git\n\ngit-good [PATH]\ngit-good --version        Print version\ngit-good history [PATH]   Print history as JSON\ngit-good refs [PATH]      Print branches, remotes, tags, and HEAD as JSON\ngit-good status [PATH] [--file PATH --area AREA]   Print working status or diff as JSON\ngit-good stage --file PATH [--hunk HEADER]... [--repo PATH]   Stage one file or selected hunks\ngit-good unstage --file PATH [--repo PATH]   Unstage one file\ngit-good commit [--amend] -m MESSAGE [--repo PATH]   Commit the index or amend HEAD after Git hooks\ngit-good reflog [REF] [--repo PATH]   Print reflog entries as JSON\ngit-good switch [--create] BRANCH [--repo PATH]   Switch to or create a local branch\ngit-good fetch [REMOTE] [--prune] [--repo PATH]   Fetch and update remote-tracking refs\ngit-good push [REMOTE] [BRANCH] [--set-upstream] [--repo PATH]   Push a local branch without force\ngit-good clone URL [PATH]   Clone a repository and open it\ngit-good stash <list|push|apply|pop|drop> [OPTIONS] [--repo PATH]   List or mutate stashes\ngit-good cherry-pick COMMIT [--repo PATH]   Apply a commit\ngit-good revert COMMIT [--repo PATH]   Invert a commit\ngit-good merge REF [--no-ff] [--repo PATH]   Merge a branch, remote, or tag\ngit-good rebase REF [--autostash] [--repo PATH]   Rebase the current branch onto a branch, remote, or tag\ngit-good rebase --continue|--skip|--abort [--repo PATH]   Recover an in-progress rebase\ngit-good rewrite --from COMMIT --pick COMMIT... --squash COMMIT... [--autostash] [--repo PATH]   Reorder or squash a linear range\ngit-good rewrite --continue|--skip|--abort [--repo PATH]   Recover an in-progress rewrite\ngit-good conflicts [--repo PATH]   List unmerged paths\ngit-good conflict --file PATH [--use ours|theirs|--stage] [--repo PATH]   Read or resolve a conflict\ngit-good reset --soft|--mixed|--hard COMMIT [--repo PATH]   Reset the current branch\ngit-good branch --delete BRANCH [--force] [--repo PATH]   Delete a local branch\ngit-good branch --create NAME [--at REF] [--repo PATH]   Create a local branch without switching\ngit-good tag --delete TAG [--repo PATH]   Delete a local tag\ngit-good show <commit> [--repo PATH] [--parent N] [--file PATH] [--export before|after --output PATH]   Review or export\n\nBrowse commits reachable from HEAD and all local, remote, and tag refs."
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
    let path = if headless { args.next() } else { first }
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
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
            cc.egui_ctx.set_visuals(
                if cfg!(debug_assertions)
                    && std::env::var("GG_THEME").is_ok_and(|theme| theme == "light")
                {
                    egui::Visuals::light()
                } else {
                    egui::Visuals::dark()
                },
            );
            phosphor::install(&cc.egui_ctx);
            let mut app = Browser::new(path);
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
    destination: String,
}

struct StashDialog {
    message: String,
    include_untracked: bool,
    keep_index: bool,
    error: Option<String>,
}

struct StashWindow {
    entries: Vec<gg_engine::stash::StashEntry>,
    error: Option<String>,
    notice: Option<String>,
    confirm_drop: Option<String>,
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

struct RewriteRow {
    id: String,
    subject: String,
    action: gg_engine::rewrite::Action,
}

struct RewriteDialog {
    from: String,
    rows: Vec<RewriteRow>,
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

#[derive(Clone)]
struct CreateBranchDialog {
    name: String,
    commit: String,
    selector: String,
    error: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConflictSide {
    Base,
    Ours,
    Theirs,
}

enum ConflictUpdate {
    Paths(Vec<String>),
    Content(gg_engine::conflict::Content),
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
    tab: ConflictSide,
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
            "Ours is the base being rebased onto. Theirs is the commit being replayed."
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

fn conflict_side_label(
    operation: Option<gg_engine::mutation::Operation>,
    side: ConflictSide,
) -> &'static str {
    match (operation, side) {
        (_, ConflictSide::Base) => "Base",
        (Some(gg_engine::mutation::Operation::Rebase), ConflictSide::Ours) => {
            "Ours · base being rebased onto (stage 2)"
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
    Apply(String),
    Pop(String),
    Drop(String),
}

#[derive(Clone, Copy)]
enum MutationAction {
    Continue,
    Skip,
    Abort,
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
    Action(CommitAction),
    CopyHash,
    CopySubject,
    OpenBackups,
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
}

struct Outcome {
    severity: OutcomeSeverity,
    summary: String,
    details: Option<String>,
    recovery: Option<RecoveryAction>,
}

fn recovery_label(action: RecoveryAction) -> &'static str {
    match action {
        RecoveryAction::Backups => "Backups…",
        RecoveryAction::ResolveConflicts => "Resolve conflicts…",
        RecoveryAction::RetryFetch => "Retry fetch",
        RecoveryAction::RetryPush => "Retry push",
        RecoveryAction::ManageStashes => "Manage stashes…",
    }
}

fn commit_action_menu(
    ui: &mut egui::Ui,
    id: &str,
    can_mutate: bool,
    mutate_reason: Option<&str>,
) -> Option<CommitCommand> {
    let short: String = id.chars().take(7).collect();
    let mut command = None;
    ui.label(egui::RichText::new("Inspect").weak());
    if ui.button(format!("Review {short}")).clicked() {
        command = Some(CommitCommand::Action(CommitAction::Review));
    }
    if ui
        .button(format!("Compare {short} with current HEAD"))
        .clicked()
    {
        command = Some(CommitCommand::Action(CommitAction::Compare));
    }
    if ui.button("Copy hash").clicked() {
        command = Some(CommitCommand::CopyHash);
    }
    if ui.button("Copy subject").clicked() {
        command = Some(CommitCommand::CopySubject);
    }
    ui.separator();
    ui.label(egui::RichText::new("Integrate").weak());
    if ui
        .add_enabled(
            can_mutate,
            egui::Button::new(format!("Cherry-pick {short}")),
        )
        .on_hover_text(mutate_reason.unwrap_or("Apply this commit to the current branch"))
        .clicked()
    {
        command = Some(CommitCommand::Action(CommitAction::CherryPick));
    }
    if ui
        .add_enabled(can_mutate, egui::Button::new(format!("Revert {short}")))
        .on_hover_text(mutate_reason.unwrap_or("Invert this commit on the current branch"))
        .clicked()
    {
        command = Some(CommitCommand::Action(CommitAction::Revert));
    }
    ui.separator();
    ui.label(egui::RichText::new("Rewrite and destructive").weak());
    ui.menu_button(format!("Reset {short} here"), |ui| {
        for (label, mode) in [
            ("Soft", gg_engine::reset::Mode::Soft),
            ("Mixed", gg_engine::reset::Mode::Mixed),
            ("Hard", gg_engine::reset::Mode::Hard),
        ] {
            if ui
                .add_enabled(can_mutate, egui::Button::new(label))
                .on_hover_text(mutate_reason.unwrap_or("Reset the current branch here"))
                .clicked()
            {
                command = Some(CommitCommand::Action(CommitAction::Reset(mode)));
            }
        }
    });
    if ui
        .add_enabled(can_mutate, egui::Button::new("Reorder and squash to HEAD…"))
        .on_hover_text(mutate_reason.unwrap_or("Rewrite commits from this commit through HEAD"))
        .clicked()
    {
        command = Some(CommitCommand::Action(CommitAction::Rewrite));
    }
    ui.separator();
    ui.label(egui::RichText::new("Recovery").weak());
    if ui.button("Backups…").clicked() {
        command = Some(CommitCommand::OpenBackups);
    }
    command
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
    branch_error: Option<String>,
    branch_warning: Option<String>,
    new_branch: Option<String>,
    network: Option<network::Worker>,
    network_progress: Option<gg_engine::sync::Progress>,
    network_error: Option<String>,
    network_notice: Option<String>,
    clone_dialog: Option<CloneDialog>,
    track_dialog: Option<String>,
    stash_dialog: Option<StashDialog>,
    stash_window: Option<StashWindow>,
    stash_notice: Option<String>,
    mutation: Option<gg_engine::mutation::OperationState>,
    mutation_notice: Option<String>,
    mutation_error: Option<String>,
    outcome: Option<Outcome>,
    outcome_details_open: bool,
    #[cfg(debug_assertions)]
    screenshot_requested: bool,
    operation_worker: Option<std::sync::mpsc::Receiver<Result<OperationOutcome, String>>>,
    operation_restore_row: Option<usize>,
    merge_dialog: Option<MergeDialog>,
    rebase_dialog: Option<RebaseDialog>,
    rewrite_dialog: Option<RewriteDialog>,
    rewrite_job: Option<RewriteJob>,
    rewrite_refresh: Option<u64>,
    commit_actions: bool,
    help_shortcuts: bool,
    conflict_view: Option<ConflictView>,
    reset_confirm: Option<(String, gg_engine::reset::Mode)>,
    branch_delete: Option<BranchDeleteDialog>,
    tag_delete: Option<TagDeleteDialog>,
    create_branch: Option<CreateBranchDialog>,
    reflog: Option<reflog_view::ReflogView>,
    backups: Option<backups_view::BackupsView>,
    refs_visible: bool,
    refs_query: String,
    state: state::State,
    pending: Option<stream::Worker>,
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
            branch_error: None,
            branch_warning: None,
            new_branch: None,
            network: None,
            network_progress: None,
            network_error: None,
            network_notice: None,
            clone_dialog: None,
            track_dialog: None,
            stash_dialog: None,
            stash_window: None,
            stash_notice: None,
            mutation: None,
            mutation_notice: None,
            mutation_error: None,
            outcome: None,
            outcome_details_open: false,
            #[cfg(debug_assertions)]
            screenshot_requested: false,
            operation_worker: None,
            operation_restore_row: None,
            merge_dialog: None,
            rebase_dialog: None,
            rewrite_dialog: None,
            rewrite_job: None,
            rewrite_refresh: None,
            commit_actions: false,
            help_shortcuts: false,
            conflict_view: None,
            reset_confirm: None,
            branch_delete: None,
            tag_delete: None,
            create_branch: None,
            reflog: None,
            backups: None,
            refs_visible: true,
            refs_query: String::new(),
            state: state::State::default(),
            pending: None,
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
        match network::Worker::push(ctx, path, None, None, set_upstream) {
            Ok(worker) => {
                self.network = Some(worker);
                self.network_progress = None;
                self.network_error = None;
                self.network_notice = None;
            }
            Err(error) => self.network_error = Some(error),
        }
    }

    fn start_clone(&mut self, ctx: &egui::Context, url: String, destination: Option<PathBuf>) {
        if self.branch.is_some() || self.network.is_some() {
            return;
        }
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
        self.network = None;
        self.network_progress = None;
        match result {
            Ok(network::Outcome::Fetch(receipt)) => {
                self.network_error = None;
                self.network_notice = Some(format!("Fetched {}", receipt.remote));
                self.load(ctx);
            }
            Ok(network::Outcome::Push(receipt)) => {
                self.network_error = None;
                self.network_notice = Some(match &receipt.upstream {
                    Some(upstream) => format!(
                        "Pushed {} to {} · upstream {upstream}",
                        receipt.branch, receipt.remote
                    ),
                    None => format!("Pushed {} to {}", receipt.branch, receipt.remote),
                });
                self.load(ctx);
            }
            Ok(network::Outcome::Clone(receipt)) => {
                self.network_error = None;
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
                self.reflog = None;
                self.backups = None;
                self.load(ctx);
            }
            Err(error) => {
                let recovery = match operation {
                    Some(gg_engine::sync::Operation::Push) => Some(RecoveryAction::RetryPush),
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

    fn restore_stash(
        &mut self,
        ctx: &egui::Context,
        reference: &str,
        remove: bool,
    ) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before restoring a stash.")?;
        let receipt = if remove {
            gg_git::stash::pop(&path, Some(reference), false)?
        } else {
            gg_git::stash::apply(&path, Some(reference), false)?
        };
        let action = if remove { "Popped" } else { "Applied" };
        let summary = if receipt.conflicts.is_empty() {
            format!("{action} {reference}")
        } else {
            format!(
                "{action} {reference} · {} conflict(s)",
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
        if let Some(view) = &mut self.reflog {
            view.load();
        }
    }

    fn refresh_backups(&mut self) {
        if let Some(view) = &mut self.backups {
            view.load();
        }
    }

    fn refresh_working(&mut self, ctx: &egui::Context) {
        if let Some(working) = &mut self.working {
            working.refresh(ctx);
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
        if let Some(draft) = self.working_drafts.get(&working.repository_key()).cloned() {
            working.restore_draft(draft, ctx);
        }
        self.working = Some(working);
        ctx.request_repaint();
    }

    fn close_to_history(&mut self, ctx: &egui::Context) {
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

    fn operation_busy(&self) -> bool {
        self.operation_worker.is_some()
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
                    self.backups = Some(backups_view::BackupsView::new(history.path.clone()));
                }
            }
            RecoveryAction::ResolveConflicts => self.open_conflict_view(ctx),
            RecoveryAction::RetryFetch => self.start_fetch(ctx, None, false),
            RecoveryAction::RetryPush => self.start_push(ctx),
            RecoveryAction::ManageStashes => {
                self.stash_window = Some(StashWindow {
                    entries: Vec::new(),
                    error: None,
                    notice: None,
                    confirm_drop: None,
                });
                self.refresh_stashes();
            }
        }
    }

    fn mutation_ready(&self) -> bool {
        self.mutation.is_none()
            && !self.operation_busy()
            && self.branch.is_none()
            && self.network.is_none()
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
            CommitCommand::Action(action) => self.run_commit_action(ctx, id.to_string(), action),
            CommitCommand::CopyHash => ctx.copy_text(id.to_string()),
            CommitCommand::CopySubject => ctx.copy_text(subject.to_string()),
            CommitCommand::OpenBackups => {
                if let Some(history) = &self.state.history {
                    self.backups = Some(backups_view::BackupsView::new(history.path.clone()));
                }
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
        let shows_details = location.shows_details();
        let location = location.location().clone();
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
        self.operation_restore_row = self.state.selected_row;
        self.mutation_error = None;
        self.mutation_notice = Some(label);
        Ok(())
    }

    fn finish_operation(&mut self, ctx: &egui::Context, outcome: OperationOutcome) {
        let restore_row = self.operation_restore_row.take();
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
        self.start_operation(ctx, label, move || {
            gg_git::rebase::rebase(&path, &reference, autostash).map(OperationOutcome::Rebase)
        })
    }

    fn open_rewrite_dialog(&mut self, from: &str) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before rewriting history.")?;
        if self.mutation.is_some() {
            return Err("Recover the operation in progress before rewriting history.".into());
        }
        let commits = gg_git::rewrite::commits(&path, from)?;
        let first = commits
            .first()
            .ok_or("The rewrite range is empty.")?
            .id
            .clone();
        self.rewrite_dialog = Some(RewriteDialog {
            from: first,
            rows: commits
                .into_iter()
                .map(|commit| RewriteRow {
                    id: commit.id,
                    subject: commit.subject,
                    action: gg_engine::rewrite::Action::Pick,
                })
                .collect(),
            autostash: false,
            error: None,
        });
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
        let path = self
            .stash_path()
            .ok_or("Open a repository before rewriting history.")?;
        let restore_row = self.state.selected_row;
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
        Ok(())
    }

    fn finish_rewrite(
        &mut self,
        ctx: &egui::Context,
        receipt: gg_engine::rewrite::Receipt,
        restore_row: Option<usize>,
    ) -> u64 {
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
            tab: ConflictSide::Ours,
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
                    view.error = None;
                    load_content = !view.paths.is_empty();
                }
                Ok(ConflictUpdate::Content(content)) => {
                    view.content = Some(content);
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
            view.loading = false;
            return;
        };
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let repaint = ctx.clone();
        if std::thread::Builder::new()
            .name("gg-conflict-content".into())
            .spawn(move || {
                let result = gg_git::conflict::read(&path, &file).map(ConflictUpdate::Content);
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

    fn resolve_conflict(
        &mut self,
        ctx: &egui::Context,
        side: Option<gg_engine::conflict::Side>,
    ) -> Result<(), String> {
        let path = self
            .stash_path()
            .ok_or("Open a repository before resolving conflicts.")?;
        let file = self
            .conflict_view
            .as_ref()
            .and_then(|view| view.paths.get(view.selected))
            .cloned()
            .ok_or("Select a conflicted path first.")?;
        let label = match side {
            Some(_) => format!("Resolving {file}…"),
            None => format!("Staging {file}…"),
        };
        self.start_operation(ctx, label, move || {
            match side {
                Some(side) => gg_git::conflict::use_side(&path, &file, side)?,
                None => gg_git::conflict::stage(&path, &file)?,
            }
            Ok(OperationOutcome::Conflict { file })
        })
    }

    fn load(&mut self, ctx: &egui::Context) {
        self.branch_error = None;
        self.network_error = None;
        self.refresh_mutation();
        let path = PathBuf::from(&self.path);
        self.state.begin(path.clone());
        let request = stream::Request {
            revision: self.state.filter_revision,
            query: self.state.query.clone(),
            selected_id: self.state.selected.clone(),
            selected_row: self.restore_row.take(),
            anchor_id: self
                .state
                .rows
                .get(&((self.scroll_offset / history_view::ROW_HEIGHT) as usize))
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

    fn open_backup_review(&mut self, ctx: &egui::Context, commit: String, subject: String) {
        let Some(history) = &self.state.history else {
            return;
        };
        self.review = Some(review_view::Review::new(
            gg_engine::review::ReviewRequest {
                path: history.path.clone(),
                revision: commit,
                parent: None,
                against: None,
            },
            subject,
            ctx,
        ));
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
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        colors::apply(ctx);
        if let Some(result) = self.branch.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Some(Err("Branch worker stopped. Refresh refs.".into()))
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
        }) {
            self.branch = None;
            self.branch_label = None;
            match result {
                Ok(receipt) => {
                    self.branch_warning = if receipt.warnings.is_empty() {
                        None
                    } else {
                        Some(receipt.warnings.join(" · "))
                    };
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
                    self.branch_error = Some(error);
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
            match result {
                Ok(receipt) => {
                    let generation = self.finish_rewrite(ctx, receipt, restore_row);
                    self.rewrite_refresh = Some(generation);
                }
                Err(error) => {
                    self.refresh_mutation();
                    if let Some(dialog) = &mut self.rewrite_dialog {
                        dialog.error = Some(error);
                    }
                }
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
            match result {
                Ok(outcome) => self.finish_operation(ctx, outcome),
                Err(error) => {
                    self.operation_restore_row = None;
                    self.mutation_notice = None;
                    self.set_outcome(OutcomeSeverity::Error, error.clone(), None, None);
                    self.mutation_error = Some(error);
                    self.refresh_mutation();
                }
            }
        }
        self.poll_conflict_view(ctx);
        let narrow = ctx.content_rect().width() < 1040.0;
        let wide_refs = !narrow && self.refs_visible && ctx.content_rect().width() >= 1120.0;
        let filter_id = egui::Id::unique("history-filter");
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
            // A rewrite dialog covers the list, so the refresh can replace rows immediately.
            let safe = self.rewrite_refresh.is_some()
                || (!hovering_history && !interacting && now - self.last_scroll > 0.2);
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
                            row as f32 * history_view::ROW_HEIGHT
                                + self.scroll_offset % history_view::ROW_HEIGHT
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
            let focus_filter =
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F));
            if focus_filter {
                self.narrow_details = false;
                ctx.memory_mut(|m| m.request_focus(filter_id));
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::R)) {
                self.load(ctx);
            }
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
        egui::Panel::top("toolbar").exact_size(42.0).show(ui, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                branding::toolbar(ui, &mut self.logo);
                ui.menu_button("Repository", |ui| {
                    if ui
                        .add_enabled(
                            self.network.is_none()
                                && self.branch.is_none()
                                && !self.operation_busy(),
                            egui::Button::new("Open…"),
                        )
                        .clicked()
                    {
                        self.edit_path = true;
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            self.pending.is_none(),
                            egui::Button::new("Refresh").shortcut_text("Ctrl/Cmd+R"),
                        )
                        .clicked()
                    {
                        self.load(ctx);
                        ui.close();
                    }
                    if ui
                        .add_enabled(self.state.history.is_some(), egui::Button::new("Reflog…"))
                        .on_hover_text("Inspect HEAD or branch reflog entries and recover a tip")
                        .clicked()
                    {
                        if let Some(history) = &self.state.history {
                            self.reflog = Some(reflog_view::ReflogView::new(
                                history.path.clone(),
                                "HEAD".into(),
                            ));
                        }
                        ui.close();
                    }
                    if ui
                        .add_enabled(self.state.history.is_some(), egui::Button::new("Backups…"))
                        .on_hover_text("Recover a tip saved before a history rewrite")
                        .clicked()
                    {
                        if let Some(history) = &self.state.history {
                            self.backups =
                                Some(backups_view::BackupsView::new(history.path.clone()));
                        }
                        ui.close();
                    }
                    ui.separator();
                    let network_idle =
                        self.network.is_none() && self.branch.is_none() && !self.operation_busy();
                    let has_repo = self.state.history.is_some();
                    if ui
                        .add_enabled(network_idle && has_repo, egui::Button::new("Fetch"))
                        .on_hover_text("Fetch the default remote without changing local branches")
                        .clicked()
                    {
                        self.start_fetch(ctx, None, false);
                        ui.close();
                    }
                    if ui
                        .add_enabled(network_idle && has_repo, egui::Button::new("Push"))
                        .on_hover_text(
                            "Push the current branch without force; set upstream when absent",
                        )
                        .clicked()
                    {
                        self.start_push(ctx);
                        ui.close();
                    }
                    if ui
                        .add_enabled(network_idle, egui::Button::new("Clone…"))
                        .on_hover_text("Clone a repository and open it after success")
                        .clicked()
                    {
                        self.clone_dialog = Some(CloneDialog {
                            url: String::new(),
                            destination: String::new(),
                        });
                        ui.close();
                    }
                    if let Some(worker) = &self.network {
                        ui.separator();
                        if ui
                            .add_enabled(
                                !worker.is_cancelled(),
                                egui::Button::new("Cancel network operation"),
                            )
                            .clicked()
                        {
                            worker.cancel();
                            ui.close();
                        }
                    }
                });
                let refs = self.state.refs.clone();
                let branch_pending = self.branch.is_some()
                    || self.network.is_some()
                    || self.mutation.is_some()
                    || self.operation_busy();
                let mut jump = None;
                let mut switch = None;
                let mut new_branch = false;
                let mut track_branch = false;
                let mut merge = false;
                let mut rebase = false;
                ui.menu_button("Refs", |ui| {
                    let head_name = refs.head.as_ref().and_then(|head| head.name.as_deref());
                    if ui
                        .add_enabled(
                            !branch_pending && refs.head.is_some() && self.network.is_none(),
                            egui::Button::new("New branch…"),
                        )
                        .clicked()
                    {
                        new_branch = true;
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            !branch_pending
                                && self.network.is_none()
                                && refs
                                    .items
                                    .iter()
                                    .any(|item| item.kind == gg_engine::refs::RefKind::Remote),
                            egui::Button::new("Track remote branch…"),
                        )
                        .clicked()
                    {
                        track_branch = true;
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            !branch_pending
                                && self.mutation.is_none()
                                && self.network.is_none()
                                && !self.operation_busy(),
                            egui::Button::new("Merge branch…"),
                        )
                        .on_hover_text("Merge a local branch, remote-tracking ref, or tag")
                        .clicked()
                    {
                        merge = true;
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            !branch_pending
                                && self.mutation.is_none()
                                && self.network.is_none()
                                && !self.operation_busy(),
                            egui::Button::new("Rebase branch…"),
                        )
                        .on_hover_text("Rebase the current branch onto a branch, remote, or tag")
                        .clicked()
                    {
                        rebase = true;
                        ui.close();
                    }
                    let branches = refs
                        .items
                        .iter()
                        .filter(|item| item.kind == gg_engine::refs::RefKind::Branch)
                        .collect::<Vec<_>>();
                    if !branches.is_empty() {
                        ui.menu_button("Branches", |ui| {
                            for item in &branches {
                                let current = Some(item.full_name.as_str()) == head_name;
                                let label = if current {
                                    format!("{}  ·  HEAD", item.name)
                                } else {
                                    item.name.clone()
                                };
                                if ui.button(label).clicked() {
                                    jump = Some(item.commit.clone());
                                    ui.close();
                                }
                            }
                        });
                        ui.menu_button("Switch to branch", |ui| {
                            for item in &branches {
                                let current = Some(item.full_name.as_str()) == head_name;
                                if ui
                                    .add_enabled(
                                        !current && !branch_pending,
                                        egui::Button::new(format!("Switch to {}", item.name)),
                                    )
                                    .on_hover_text(if current {
                                        "Already on this branch"
                                    } else {
                                        "Switch the worktree after a clean-worktree check"
                                    })
                                    .clicked()
                                {
                                    switch = Some(item.name.clone());
                                    ui.close();
                                }
                            }
                        });
                    }
                    for (title, kind) in [
                        ("Remotes", gg_engine::refs::RefKind::Remote),
                        ("Tags", gg_engine::refs::RefKind::Tag),
                    ] {
                        let filtered = refs
                            .items
                            .iter()
                            .filter(|item| item.kind == kind)
                            .collect::<Vec<_>>();
                        if filtered.is_empty() {
                            continue;
                        }
                        ui.menu_button(title, |ui| {
                            for item in filtered {
                                if ui.button(item.name.clone()).clicked() {
                                    jump = Some(item.commit.clone());
                                    ui.close();
                                }
                            }
                        });
                    }
                    if refs.items.is_empty() {
                        ui.weak("No branches or tags");
                    }
                });
                if let Some(name) = switch {
                    self.start_branch(ctx, name, false);
                }
                if new_branch {
                    self.new_branch = Some(String::new());
                }
                if track_branch {
                    self.track_dialog = Some(self.default_track_branch());
                }
                if merge {
                    self.merge_dialog = Some(MergeDialog {
                        reference: String::new(),
                        no_ff: false,
                        error: None,
                    });
                }
                if rebase {
                    self.rebase_dialog = Some(RebaseDialog {
                        reference: String::new(),
                        autostash: false,
                        error: None,
                    });
                }
                if let Some(id) = jump {
                    self.jump_to_ref(id);
                }
                ui.menu_button("View", |ui| {
                    let shown = if narrow {
                        &mut self.narrow_details
                    } else {
                        &mut self.details_visible
                    };
                    if ui.checkbox(shown, "Commit details · Space").clicked() {
                        ui.close();
                    }
                    ui.separator();
                    let sidebar_available = ctx.content_rect().width() >= 1120.0;
                    ui.add_enabled(
                        sidebar_available,
                        egui::Checkbox::new(&mut self.refs_visible, "Refs sidebar"),
                    )
                    .on_hover_text(if sidebar_available {
                        "Show branches, remotes, and tags on the left"
                    } else {
                        "Resize the window to at least 1120 px to show the sidebar"
                    });
                });
                let stash_busy =
                    self.branch.is_some() || self.network.is_some() || self.operation_busy();
                let has_repo = self.state.history.is_some();
                ui.menu_button("Stash", |ui| {
                    if ui
                        .add_enabled(has_repo && !stash_busy, egui::Button::new("Stash changes…"))
                        .on_hover_text("Record working changes in a new stash entry")
                        .clicked()
                    {
                        self.stash_dialog = Some(StashDialog {
                            message: String::new(),
                            include_untracked: false,
                            keep_index: false,
                            error: None,
                        });
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            has_repo && !stash_busy,
                            egui::Button::new("Manage stashes…"),
                        )
                        .on_hover_text("List, apply, pop, or drop stash entries")
                        .clicked()
                    {
                        self.stash_window = Some(StashWindow {
                            entries: Vec::new(),
                            error: None,
                            notice: None,
                            confirm_drop: None,
                        });
                        self.refresh_stashes();
                        ui.close();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Keyboard shortcuts…").clicked() {
                        self.help_shortcuts = true;
                        ui.close();
                    }
                });
                ui.separator();
                let trailing = if self.network.is_some() { 40.0 } else { 0.0 };
                let identity_width = (ui.available_width() - trailing).max(80.0);
                let (identity, path) = if let Some(history) = &self.state.history {
                    let tracking = self
                        .state
                        .refs
                        .current_tracking()
                        .map(|tracking| format!("  ·  {}", refs_view::tracking_label(tracking)))
                        .unwrap_or_default();
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
                if let Some(worker) = &self.network {
                    let cancelled = worker.is_cancelled();
                    let button = egui::Button::new(phosphor::icon(phosphor::X).size(13.0))
                        .min_size(egui::vec2(26.0, 26.0));
                    if ui
                        .add_enabled(!cancelled, button)
                        .on_hover_text("Cancel network operation")
                        .clicked()
                    {
                        worker.cancel();
                    }
                }
            });
        });
        egui::Panel::top("navigation")
            .exact_size(32.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let has_repo = self.state.history.is_some();
                    let local_selected = self.working.is_some();
                    if ui
                        .add_enabled(
                            has_repo,
                            egui::Button::selectable(local_selected, "Local Changes")
                                .min_size(egui::vec2(112.0, 24.0)),
                        )
                        .on_hover_text("Stage, unstage, commit, and amend in one view")
                        .clicked()
                    {
                        go_local = true;
                    }
                    if ui
                        .add_enabled(
                            has_repo,
                            egui::Button::selectable(!local_selected, "History")
                                .min_size(egui::vec2(84.0, 24.0)),
                        )
                        .on_hover_text("Browse commit history")
                        .clicked()
                    {
                        go_history = true;
                    }
                });
            });
        let mut mutation_action = None;
        let mut resolve_conflicts = false;
        let mut open_backups = false;
        let operation_pending = self.operation_busy();
        if let Some(state) = &self.mutation {
            let conflicts_remain = !state.conflicts.is_empty();
            egui::Panel::top("mutation")
                .exact_size(30.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} in progress", state.operation.as_str()));
                        if !state.conflicts.is_empty() {
                            ui.label(format!("· {} conflict(s)", state.conflicts.len()));
                            if ui
                                .add_enabled(
                                    !operation_pending,
                                    egui::Button::new("Resolve conflicts…"),
                                )
                                .clicked()
                            {
                                resolve_conflicts = true;
                            }
                        }
                        if ui
                            .add_enabled(
                                !operation_pending && !conflicts_remain,
                                egui::Button::new("Continue"),
                            )
                            .on_hover_text(if conflicts_remain {
                                "Resolve every unmerged path first."
                            } else {
                                "Continue the in-progress operation."
                            })
                            .clicked()
                        {
                            mutation_action = Some(MutationAction::Continue);
                        }
                        if state.operation != gg_engine::mutation::Operation::Merge
                            && ui
                                .add_enabled(!operation_pending, egui::Button::new("Skip"))
                                .clicked()
                        {
                            mutation_action = Some(MutationAction::Skip);
                        }
                        if ui
                            .add_enabled(!operation_pending, egui::Button::new("Abort"))
                            .clicked()
                        {
                            mutation_action = Some(MutationAction::Abort);
                        }
                        if ui.button("Backups…").clicked() {
                            open_backups = true;
                        }
                    });
                });
        }
        if resolve_conflicts {
            self.open_conflict_view(ctx);
        }
        if open_backups && let Some(history) = &self.state.history {
            self.backups = Some(backups_view::BackupsView::new(history.path.clone()));
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
                            .desired_width((ui.available_width() - 80.0).max(100.0))
                            .hint_text("Repository path"),
                    );
                    let enter =
                        response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui
                        .add_enabled(
                            self.pending.is_none()
                                && self.network.is_none()
                                && self.branch.is_none(),
                            egui::Button::new("Open"),
                        )
                        .clicked()
                        || enter
                    {
                        self.reflog = None;
                        self.backups = None;
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
            egui::Panel::left("refs-sidebar")
                .exact_size(190.0)
                .resizable(false)
                .frame(egui::Frame::side_top_panel(ui.style().as_ref()).inner_margin(8.0))
                .show(ui, |ui| {
                    ui.add_enabled_ui(has_repo, |ui| {
                        action = refs_view::show(ui, &refs, &mut self.refs_query, busy);
                    });
                });
            match action {
                Some(refs_view::Action::Switch(name)) => self.start_branch(ctx, name, false),
                Some(refs_view::Action::Jump(id)) => self.jump_to_ref(id),
                Some(refs_view::Action::NewBranch) => self.new_branch = Some(String::new()),
                Some(refs_view::Action::Track(name)) => {
                    self.track_dialog = Some(name);
                }
                Some(refs_view::Action::Merge(name)) => {
                    self.merge_dialog = Some(MergeDialog {
                        reference: name,
                        no_ff: false,
                        error: None,
                    });
                }
                Some(refs_view::Action::Rebase(name)) => {
                    self.rebase_dialog = Some(RebaseDialog {
                        reference: name,
                        autostash: false,
                        error: None,
                    });
                }
                Some(refs_view::Action::DeleteBranch(name)) => {
                    self.branch_delete = Some(BranchDeleteDialog { name, error: None });
                }
                Some(refs_view::Action::DeleteTag(name)) => {
                    self.tag_delete = Some(TagDeleteDialog { name, error: None });
                }
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
        let new_branch_pending = self.branch.is_some() || self.operation_busy();
        if let Some(name) = &mut self.new_branch {
            let mut submit = None;
            let mut open =
                !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            let branch_pending = new_branch_pending;
            egui::Window::new("New branch")
                .id(egui::Id::unique("new-branch"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(360.0)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label("Create a branch from the current HEAD. The worktree must be clean.");
                    let response = ui.add(
                        egui::TextEdit::singleline(name)
                            .hint_text("Branch name")
                            .desired_width(f32::INFINITY),
                    );
                    let enter =
                        response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !branch_pending && !name.trim().is_empty(),
                                egui::Button::new("Create branch"),
                            )
                            .clicked()
                            || enter
                        {
                            submit = Some(name.trim().to_string());
                        }
                        if ui.button("Cancel").clicked() {
                            submit = Some(String::new());
                        }
                    });
                });
            if let Some(name) = submit {
                if !name.is_empty() {
                    self.start_branch(ctx, name, true);
                }
                self.new_branch = None;
            } else if !open {
                self.new_branch = None;
            }
        }
        if let Some(dialog) = &mut self.clone_dialog {
            let mut submit = None;
            let mut cancelled = false;
            let mut open =
                !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            egui::Window::new("Clone repository")
                .id(egui::Id::unique("clone-repository"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(420.0)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.label(
                        "Clone with Git credentials and progress. The worktree opens after success.",
                    );
                    let url = ui.add(
                        egui::TextEdit::singleline(&mut dialog.url)
                            .hint_text("Repository URL")
                            .desired_width(f32::INFINITY),
                    );
                    let destination = ui.add(
                        egui::TextEdit::singleline(&mut dialog.destination)
                            .hint_text("Destination (optional; defaults from the URL)")
                            .desired_width(f32::INFINITY),
                    );
                    let enter = (url.lost_focus() || destination.lost_focus())
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !dialog.url.trim().is_empty()
                                    && self.network.is_none()
                                    && self.branch.is_none(),
                                egui::Button::new("Clone"),
                            )
                            .clicked()
                            || (enter && self.network.is_none() && self.branch.is_none())
                        {
                            submit = Some((
                                dialog.url.trim().to_string(),
                                dialog.destination.trim().to_string(),
                            ));
                        }
                        if ui.button("Cancel").clicked() {
                            cancelled = true;
                        }
                    });
                });
            if cancelled {
                open = false;
            }
            if let Some((url, destination)) = submit {
                self.clone_dialog = None;
                self.start_clone(
                    ctx,
                    url,
                    (!destination.is_empty()).then(|| PathBuf::from(destination)),
                );
            } else if !open {
                self.clone_dialog = None;
            }
        }
        let track_busy = self.branch.is_some() || self.network.is_some() || self.operation_busy();
        if let Some(remote_branch) = &mut self.track_dialog {
            let mut submit = false;
            let mut cancelled = false;
            let mut open =
                !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            let busy = track_busy;
            egui::Window::new("Track remote branch")
                .id(egui::Id::unique("track-remote-branch"))
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
        let reflog_references = self.reflog_references();
        let mut reflog_action = None;
        let mut reflog_open = true;
        if let Some(view) = &mut self.reflog {
            reflog_action = view.draw(ctx, &reflog_references, &mut reflog_open);
        }
        if !reflog_open {
            self.reflog = None;
        }
        if let Some(action) = reflog_action {
            match action {
                reflog_view::Action::Reset { commit, mode } => {
                    self.reset_confirm = Some((commit, mode));
                }
                reflog_view::Action::CreateBranch { commit, selector } => {
                    self.create_branch = Some(CreateBranchDialog {
                        name: String::new(),
                        commit,
                        selector,
                        error: None,
                    });
                }
            }
        }

        let mut backups_open = true;
        let mut backups_action = None;
        if let Some(view) = &mut self.backups {
            backups_action = view.draw(ctx, &mut backups_open);
        }
        if !backups_open {
            self.backups = None;
        }
        if let Some(action) = backups_action {
            match action {
                backups_view::Action::Open { commit, subject } => {
                    self.open_backup_review(ctx, commit, subject);
                }
                backups_view::Action::Reset { commit, mode } => {
                    self.reset_confirm = Some((commit, mode));
                }
                backups_view::Action::CreateBranch { commit, selector } => {
                    self.create_branch = Some(CreateBranchDialog {
                        name: String::new(),
                        commit,
                        selector,
                        error: None,
                    });
                }
            }
        }

        let mut create_branch_submit = None;
        let mut create_branch_cancel = false;
        if let Some(dialog) = &mut self.create_branch {
            egui::Window::new("Create branch here")
                .id(egui::Id::unique("create-branch-here"))
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
                            create_branch_submit = Some(dialog.name.trim().to_string());
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
        if let Some(name) = create_branch_submit
            && let Some(dialog) = self.create_branch.clone()
        {
            match self.stash_path() {
                Some(path) => match gg_git::branch::create_at(&path, &name, &dialog.commit) {
                    Ok(receipt) => {
                        self.create_branch = None;
                        self.load(ctx);
                        if let Some(view) = &mut self.reflog {
                            view.notice(format!(
                                "Created {} at {}",
                                receipt.branch,
                                receipt.commit.chars().take(8).collect::<String>()
                            ));
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
        if let Some((commit, mode)) = self.reset_confirm.clone() {
            egui::Window::new("Reset here")
                .id(egui::Id::unique("reset-here"))
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
        if reset_cancel {
            self.reset_confirm = None;
        }
        if let Some((commit, mode)) = reset_submit {
            self.reset_confirm = None;
            if let Err(error) = self.start_reset(ctx, &commit, mode) {
                self.mutation_error = Some(error);
            }
        }

        let conflict_mutation = self.mutation.clone();
        let conflict_operation_busy = self.operation_busy();
        let mut conflict_select = None;
        let mut conflict_action = None;
        let mut conflict_recovery = None;
        let mut conflict_open = true;
        if let Some(view) = &mut self.conflict_view {
            let operation = conflict_mutation.as_ref().map(|state| state.operation);
            let unresolved = conflict_mutation
                .as_ref()
                .map(|state| state.conflicts.len())
                .unwrap_or(view.paths.len());
            let conflicts_remain = unresolved > 0;
            egui::Window::new("Resolve conflicts")
                .id(egui::Id::unique("resolve-conflicts"))
                .open(&mut conflict_open)
                .collapsible(false)
                .resizable(true)
                .default_width(760.0)
                .default_pos(ctx.content_rect().center())
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
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
                        if view.loading {
                            ui.spinner();
                        }
                    });
                    ui.label(conflict_side_hint(operation));
                    if let Some(error) = &view.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    if view.paths.is_empty() && !view.loading {
                        ui.label("No unmerged paths.");
                    }
                    ui.separator();
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.label("Paths");
                            egui::ScrollArea::vertical()
                                .id_salt("conflict-paths")
                                .max_height(360.0)
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
                                if ui
                                    .button(format!(
                                        "Use ours · {}",
                                        if operation == Some(gg_engine::mutation::Operation::Rebase)
                                        {
                                            "base"
                                        } else {
                                            "current branch"
                                        }
                                    ))
                                    .clicked()
                                {
                                    conflict_action = Some(Some(gg_engine::conflict::Side::Ours));
                                }
                                if ui
                                    .button(format!(
                                        "Use theirs · {}",
                                        if operation == Some(gg_engine::mutation::Operation::Rebase)
                                        {
                                            "replayed commit"
                                        } else {
                                            "incoming"
                                        }
                                    ))
                                    .clicked()
                                {
                                    conflict_action = Some(Some(gg_engine::conflict::Side::Theirs));
                                }
                                if ui.button("Stage resolved").clicked() {
                                    conflict_action = Some(None);
                                }
                            });
                            ui.weak(
                                "Choosing a side replaces the whole file and stages it. Stage resolved stages external or manually edited content.",
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
                                            .max_height(360.0)
                                            .show(column, |ui| {
                                                ui.monospace(text.as_deref().unwrap_or("(absent)"));
                                            });
                                    }
                                });
                            } else {
                                ui.horizontal(|ui| {
                                    ui.selectable_value(&mut view.tab, ConflictSide::Base, "Base");
                                    ui.selectable_value(&mut view.tab, ConflictSide::Ours, "Ours");
                                    ui.selectable_value(
                                        &mut view.tab,
                                        ConflictSide::Theirs,
                                        "Theirs",
                                    );
                                });
                                let (title, text) = match view.tab {
                                    ConflictSide::Base => {
                                        (conflict_side_label(operation, ConflictSide::Base), &content.base)
                                    }
                                    ConflictSide::Ours => {
                                        (conflict_side_label(operation, ConflictSide::Ours), &content.ours)
                                    }
                                    ConflictSide::Theirs => (
                                        conflict_side_label(operation, ConflictSide::Theirs),
                                        &content.theirs,
                                    ),
                                };
                                ui.strong(title);
                                egui::ScrollArea::both()
                                    .id_salt("conflict-side")
                                    .max_height(320.0)
                                    .show(ui, |ui| {
                                        ui.monospace(text.as_deref().unwrap_or("(absent)"));
                                    });
                            }
                        });
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !conflicts_remain && !conflict_operation_busy,
                                egui::Button::new("Continue"),
                            )
                            .on_hover_text(if conflicts_remain {
                                "Resolve every unmerged path first."
                            } else {
                                "Continue the in-progress operation."
                            })
                            .clicked()
                        {
                            conflict_recovery = Some(MutationAction::Continue);
                        }
                        if operation != Some(gg_engine::mutation::Operation::Merge)
                            && ui
                                .add_enabled(
                                    !conflict_operation_busy,
                                    egui::Button::new("Skip"),
                                )
                                .on_hover_text("Skip the current commit and continue.")
                                .clicked()
                        {
                            conflict_recovery = Some(MutationAction::Skip);
                        }
                        if ui
                            .add_enabled(!conflict_operation_busy, egui::Button::new("Abort"))
                            .on_hover_text("Abort the operation and restore the previous tip.")
                            .clicked()
                        {
                            conflict_recovery = Some(MutationAction::Abort);
                        }
                    });
                });
        }
        if let Some(index) = conflict_select
            && let Some(view) = &mut self.conflict_view
        {
            view.selected = index;
        }
        if conflict_select.is_some() {
            self.load_conflict_content(ctx);
        }
        if let Some(side) = conflict_action
            && let Err(error) = self.resolve_conflict(ctx, side)
            && let Some(view) = &mut self.conflict_view
        {
            view.error = Some(error);
        }
        if let Some(action) = conflict_recovery
            && let Some(operation) = conflict_mutation.as_ref().map(|state| state.operation)
            && let Err(error) = self.recover_mutation(ctx, operation, action)
        {
            self.mutation_error = Some(error);
        }
        if conflict_recovery.is_none() && self.conflict_view.is_some() && !conflict_open {
            self.conflict_view = None;
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
                            "Ctrl/Cmd+R: refresh · Ctrl/Cmd+F: filter · Ctrl/Cmd+C: copy ID",
                        ),
                        (
                            "Commit actions",
                            "Alt+A: actions for the selected commit · menus: Repository, Refs, View, Stash, Help",
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
                .id(egui::Id::unique("merge-branch"))
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
                .id(egui::Id::unique("rebase-branch"))
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

        if let Some(generation) = self.rewrite_refresh {
            if self.state.loaded_generation >= generation {
                self.rewrite_refresh = None;
                self.rewrite_dialog = None;
            } else if let Some(error) = self.state.error.clone() {
                self.rewrite_refresh = None;
                self.rewrite_dialog = None;
                self.mutation_error = Some(format!("History refresh failed: {error}"));
            }
        }
        let rewrite_busy = self.rewrite_job.is_some();
        let rewrite_refreshing = self.rewrite_refresh.is_some();
        let mut rewrite_submit = None;
        let mut rewrite_cancel = false;
        if let Some(dialog) = &mut self.rewrite_dialog {
            let mut move_up = None;
            let mut move_down = None;
            let modal = egui::Modal::new(egui::Id::unique("reorder-squash")).show(ctx, |ui| {
                let width = (ctx.content_rect().width() - 64.0).clamp(360.0, 620.0);
                ui.set_width(width);
                ui.heading("Reorder and squash");
                ui.add_space(4.0);
                if rewrite_busy || rewrite_refreshing {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(if rewrite_busy {
                            "Rewriting commits…"
                        } else {
                            "Refreshing history…"
                        });
                    });
                    ui.weak("This dialog closes when the refreshed history is ready.");
                    ui.add_space(8.0);
                }
                ui.add_enabled_ui(!(rewrite_busy || rewrite_refreshing), |ui| {
                    ui.label(
                        "Reorder commits from the selected commit through HEAD. Squash combines a commit into the row above. Merge commits are not supported.",
                    );
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for index in 0..dialog.rows.len() {
                                let id = dialog.rows[index].id.clone();
                                let subject = dialog.rows[index].subject.clone();
                                let mut action = dialog.rows[index].action;
                                ui.horizontal(|ui| {
                                    ui.add_sized(
                                        [22.0, 24.0],
                                        egui::Label::new(
                                            egui::RichText::new(format!("{}", index + 1))
                                                .monospace()
                                                .weak(),
                                        ),
                                    );
                                    let up = egui::Button::new(
                                        phosphor::icon(phosphor::ARROW_UP).size(12.0),
                                    )
                                    .min_size(egui::vec2(24.0, 24.0));
                                    if ui
                                        .add_enabled(index > 0, up)
                                        .on_hover_text("Move up")
                                        .clicked()
                                    {
                                        move_up = Some(index);
                                    }
                                    let down = egui::Button::new(
                                        phosphor::icon(phosphor::ARROW_DOWN).size(12.0),
                                    )
                                    .min_size(egui::vec2(24.0, 24.0));
                                    if ui
                                        .add_enabled(index + 1 < dialog.rows.len(), down)
                                        .on_hover_text("Move down")
                                        .clicked()
                                    {
                                        move_down = Some(index);
                                    }
                                    let mut squash =
                                        action == gg_engine::rewrite::Action::Squash;
                                    if ui
                                        .add_enabled(
                                            index > 0,
                                            egui::Checkbox::new(&mut squash, "Squash"),
                                        )
                                        .on_hover_text("Combine this commit into the row above")
                                        .changed()
                                    {
                                        action = if squash {
                                            gg_engine::rewrite::Action::Squash
                                        } else {
                                            gg_engine::rewrite::Action::Pick
                                        };
                                    }
                                    let short = id.chars().take(7).collect::<String>();
                                    ui.label(egui::RichText::new(short).monospace().weak());
                                    let remaining = ui.available_width().max(80.0);
                                    ui.add_sized(
                                        [remaining, 24.0],
                                        egui::Label::new(subject.clone()).truncate(),
                                    )
                                    .on_hover_text(subject);
                                });
                                dialog.rows[index].action = action;
                            }
                        });
                    if let Some(index) = move_up {
                        dialog.rows.swap(index, index - 1);
                    }
                    if let Some(index) = move_down {
                        dialog.rows.swap(index, index + 1);
                    }
                    if let Some(first) = dialog.rows.first_mut() {
                        first.action = gg_engine::rewrite::Action::Pick;
                    }
                    ui.add_space(4.0);
                    ui.checkbox(
                        &mut dialog.autostash,
                        "Automatically stash working changes",
                    );
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        "Reordering rewrites commits. A private backup ref is created before Git starts.",
                    );
                    if let Some(error) = &dialog.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Rewrite commits").clicked() {
                            rewrite_submit = Some((
                                dialog.from.clone(),
                                dialog
                                    .rows
                                    .iter()
                                    .map(|row| gg_engine::rewrite::Step {
                                        action: row.action,
                                        commit: row.id.clone(),
                                    })
                                    .collect::<Vec<_>>(),
                                dialog.autostash,
                            ));
                        }
                        if ui.button("Cancel").clicked() {
                            rewrite_cancel = true;
                        }
                    });
                });
            });
            if !(rewrite_busy || rewrite_refreshing) && modal.should_close() {
                rewrite_cancel = true;
            }
        }
        if let Some((from, steps, autostash)) = rewrite_submit
            && let Err(error) = self.begin_rewrite(ctx, from, steps, autostash)
            && let Some(dialog) = &mut self.rewrite_dialog
        {
            dialog.error = Some(error);
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
                                    if ui.button("Pop").clicked() {
                                        stash_action =
                                            Some(StashAction::Pop(entry.reference.clone()));
                                    }
                                    if ui.button("Apply").clicked() {
                                        stash_action =
                                            Some(StashAction::Apply(entry.reference.clone()));
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
        if stash_refresh {
            self.refresh_stashes();
        }
        if let Some(action) = stash_action {
            let result = match action {
                StashAction::Apply(reference) => self.restore_stash(ctx, &reference, false),
                StashAction::Pop(reference) => self.restore_stash(ctx, &reference, true),
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

        if self.working.is_some() {
            let close = self
                .working
                .as_mut()
                .is_some_and(|working| working.draw_ui(ui));
            let committed = self
                .working
                .as_mut()
                .and_then(|working| working.take_commit());
            if let Some(id) = committed {
                self.pending_commit = Some(id);
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
            }
            return;
        }

        egui::Panel::bottom("status").exact_size(25.0).show(ui, |ui| {
            let (message, color) = if let Some(error) = &self.network_error {
                (format!("Network · {error}"), ui.visuals().error_fg_color)
            } else if let Some(error) = &self.state.error {
                (format!("{}{}", if self.state.history.is_some() { "Previous results · " } else { "" }, error), ui.visuals().error_fg_color)
            } else if let Some(error) = &self.branch_error {
                (format!("Branch · {error}"), ui.visuals().error_fg_color)
            } else if let Some(error) = &self.mutation_error {
                (format!("History · {error}"), ui.visuals().error_fg_color)
            } else if let Some(outcome) = &self.outcome {
                let color = match outcome.severity {
                    OutcomeSeverity::Info => ui.visuals().text_color(),
                    OutcomeSeverity::Warning => ui.visuals().warn_fg_color,
                    OutcomeSeverity::Error => ui.visuals().error_fg_color,
                };
                (format!("History · {}", outcome.summary), color)
            } else if let Some(worker) = &self.network {
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
            } else if let Some(notice) = &self.network_notice {
                (format!("Network · {notice}"), ui.visuals().text_color())
            } else if let Some(warning) = &self.branch_warning {
                (format!("Branch · {warning}"), ui.visuals().text_color())
            } else if let Some(label) = &self.branch_label {
                (label.clone(), ui.visuals().text_color())
            } else if let Some(notice) = &self.stash_notice {
                (format!("Stash · {notice}"), ui.visuals().text_color())
            } else if let Some(notice) = &self.mutation_notice {
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
                (format!("{tracking}Up/Down / j k: select   ·   PgUp/PgDn: page   ·   Enter: review   ·   Space: details   ·   Esc: history   ·   Ctrl/Cmd+F: filter   ·   Ctrl/Cmd+C: copy ID"), ui.visuals().weak_text_color())
            };
            let outcome_active = self.network_error.is_none()
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
            ui.horizontal(|ui| {
                let reserved = if recovery.is_some() {
                    150.0
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
            });
            if details_clicked {
                self.outcome_details_open = true;
            }
            if let Some(action) = recovery_clicked {
                self.run_recovery_action(ctx, action);
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
        if !narrow || !self.narrow_details {
            egui::Panel::top("filter").exact_size(40.0).show(ui, |ui| {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.strong("History");
                    let count = format!(
                        "{}{}",
                        self.state.count,
                        if self.state.finished { "" } else { "+" }
                    );
                    ui.allocate_ui_with_layout(
                        egui::vec2(90.0, 20.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.set_min_width(90.0);
                            bounded_text::field(ui, &count, egui::FontId::proportional(12.0), 20.0);
                        },
                    )
                    .response
                    .on_hover_text(
                        "Discovered matches; + means counting or searching is in progress.",
                    );
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.state.query)
                            .id(filter_id)
                            .hint_text("Filter by message, author, or ID")
                            .desired_width(ui.available_width()),
                    );
                    if response.changed() {
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
                    if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        response.surrender_focus();
                    }
                });
            });
        }
        self.details_rect = egui::Rect::NOTHING;
        if !narrow && self.details_visible {
            let details = egui::Panel::right("details")
                .default_size(300.0)
                .size_range(
                    300.0
                        ..=(ctx.content_rect().width()
                            - if wide_refs { 190.0 } else { 0.0 }
                            - history_view::MIN_WIDTH)
                            .max(300.0),
                )
                .frame(egui::Frame::side_top_panel(ui.style().as_ref()).inner_margin(12.0))
                .show(ui, |ui| {
                    ui.weak("DETAILS");
                    ui.add_space(8.0);
                    let commit = self.state.displayed_details();
                    let loading = self.state.details.is_none();
                    let ready = self.mutation_ready();
                    let reason = self.mutation_reason();
                    let command =
                        details_view::show(ui, commit, loading, &mut self.avatars, |ui| {
                            commit.and_then(|commit| {
                                commit_action_menu(ui, &commit.id, ready, reason)
                            })
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
                let ready = self.mutation_ready();
                let reason = self.mutation_reason();
                let command = details_view::show(ui, commit, loading, &mut self.avatars, |ui| {
                    commit.and_then(|commit| commit_action_menu(ui, &commit.id, ready, reason))
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
                egui::ScrollArea::horizontal()
                    .id_salt("history-columns")
                    .show(ui, |ui| {
                        ui.set_min_width(history_view::MIN_WIDTH);
                        history_view::header(ui);
                        let mut scroll = egui::ScrollArea::vertical().id_salt("history-rows");
                        if self.scroll_to_selection
                            && let Some(index) = self.state.selected_row
                        {
                            let height = ui.available_height();
                            let top = index as f32 * history_view::ROW_HEIGHT;
                            let bottom = top + history_view::ROW_HEIGHT;
                            let offset = self.scroll_offset;
                            let reveal = if top < offset {
                                top
                            } else if bottom > offset + height {
                                bottom - height
                            } else {
                                offset
                            };
                            let maximum = (self.state.count as f32 * history_view::ROW_HEIGHT
                                - height)
                                .max(0.0);
                            scroll = scroll.vertical_scroll_offset(reveal.clamp(0.0, maximum));
                        }
                        if let Some(offset) = self.restore_offset.take() {
                            scroll = scroll.vertical_scroll_offset(offset);
                        }
                        let previous_spacing = ui.spacing().item_spacing.y;
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let scroll_output = scroll.show_rows(
                            ui,
                            history_view::ROW_HEIGHT,
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
                                        response.context_menu(|ui| {
                                            let id = commit.id.clone();
                                            if let Some(command) = commit_action_menu(
                                                ui,
                                                &id,
                                                self.mutation_ready(),
                                                self.mutation_reason(),
                                            ) {
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
                                            egui::vec2(
                                                ui.available_width(),
                                                history_view::ROW_HEIGHT,
                                            ),
                                            egui::Sense::hover(),
                                        );
                                        self.row_rects.push((row, rect));
                                        ui.painter().text(
                                            rect.left_center() + egui::vec2(30.0, 0.0),
                                            egui::Align2::LEFT_CENTER,
                                            "Loading...",
                                            egui::FontId::proportional(12.0),
                                            ui.visuals().weak_text_color(),
                                        );
                                    }
                                }
                            },
                        );
                        self.scroll_offset = scroll_output.state.clamped_offset().y;
                        self.page_rows = (scroll_output.inner_rect.height()
                            / history_view::ROW_HEIGHT)
                            .floor()
                            .max(1.0) as usize;
                        ui.spacing_mut().item_spacing.y = previous_spacing;
                    });
            } else if self.pending.is_some() {
                ui.label("Reading repository history…");
            } else {
                ui.label("Choose Open… to enter a repository path.");
            }
        });
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
                text.iter().any(|line| line.contains("Branches  0")),
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
    fn clone_and_tracking_dialogs_render_at_supported_widths() {
        for width in [720.0, 1120.0] {
            for clone in [true, false] {
                let ctx = egui::Context::default();
                let mut app = layout_fixture();
                if clone {
                    app.clone_dialog = Some(CloneDialog {
                        url: "https://example.org/gg.git".into(),
                        destination: String::new(),
                    });
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
            assert!(text.iter().any(|line| line.contains("Apply")));
            assert!(text.iter().any(|line| line.contains("Pop")));
            assert!(text.iter().any(|line| line.contains("Drop")));
        }
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
                tab: ConflictSide::Ours,
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
            assert!(text.iter().any(|line| line.contains("Use ours")));
            assert!(text.iter().any(|line| line.contains("Use theirs")));
            assert!(text.iter().any(|line| line.contains("Stage resolved")));
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

    #[test]
    fn rewrite_dialog_renders_at_supported_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.rewrite_dialog = Some(RewriteDialog {
                from: "a".repeat(40),
                rows: vec![
                    RewriteRow {
                        id: "a".repeat(40),
                        subject: "first change".into(),
                        action: gg_engine::rewrite::Action::Pick,
                    },
                    RewriteRow {
                        id: "b".repeat(40),
                        subject: "second change".into(),
                        action: gg_engine::rewrite::Action::Squash,
                    },
                ],
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
            assert!(text.iter().any(|line| line.contains("Reorder and squash")));
            assert!(text.iter().any(|line| line.contains("Squash")));
            assert!(text.iter().any(|line| line.contains("Rewrite commits")));
            assert!(text.iter().any(|line| line.contains("private backup ref")));
            assert!(text.iter().any(|line| line.contains("first change")));
            assert!(text.iter().any(|line| line.contains("second change")));
        }
    }

    #[test]
    fn rewrite_dialog_stays_open_while_rewriting() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.rewrite_dialog = Some(RewriteDialog {
            from: "a".repeat(40),
            rows: vec![RewriteRow {
                id: "a".repeat(40),
                subject: "first change".into(),
                action: gg_engine::rewrite::Action::Pick,
            }],
            autostash: false,
            error: None,
        });
        let (_sender, receiver) = std::sync::mpsc::channel();
        app.rewrite_job = Some(RewriteJob {
            receiver,
            restore_row: None,
        });
        let _ = crate::tests::run_ui(
            &ctx,
            frame_input(
                1120.0,
                740.0,
                vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
            ),
            |ctx| app.draw_ui(ctx),
        );
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.rewrite_dialog.is_some());
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.iter().any(|line| line.contains("Rewriting commits")));
    }

    #[test]
    fn rewrite_dialog_closes_after_history_refresh_is_loaded() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.rewrite_dialog = Some(RewriteDialog {
            from: "a".repeat(40),
            rows: vec![RewriteRow {
                id: "a".repeat(40),
                subject: "first change".into(),
                action: gg_engine::rewrite::Action::Pick,
            }],
            autostash: false,
            error: None,
        });
        app.rewrite_refresh = Some(app.state.loaded_generation + 1);
        let output = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.rewrite_dialog.is_some());
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.iter().any(|line| line.contains("Refreshing history")));

        app.state.loaded_generation = app.rewrite_refresh.unwrap_or_default();
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.rewrite_dialog.is_none());
        assert!(app.rewrite_refresh.is_none());
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
            tab: ConflictSide::Ours,
        });
        let _ = crate::tests::run_ui(&ctx, frame_input(1120.0, 740.0, vec![]), |ctx| {
            app.draw_ui(ctx)
        });
        assert!(app.conflict_view.as_ref().is_some_and(|view| view.loading));
        assert!(
            tx.send(Ok(ConflictUpdate::Content(gg_engine::conflict::Content {
                path: "a".into(),
                base: Some("base\n".into()),
                ours: Some("ours\n".into()),
                theirs: Some("theirs\n".into()),
                binary: false,
                truncated: false,
            })))
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
    fn conflict_resolution_runs_on_the_operation_worker() {
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
            tab: ConflictSide::Ours,
        });
        assert!(
            app.resolve_conflict(&ctx, Some(gg_engine::conflict::Side::Ours))
                .is_ok()
        );
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
            for label in [
                "Inspect",
                "Integrate",
                "Rewrite and destructive",
                "Recovery",
                "Backups…",
            ] {
                assert!(text.contains(&label), "missing {label} at {width}");
            }
            assert!(
                text.iter().any(|line| line.starts_with("Cherry-pick ")),
                "missing target-aware cherry-pick at {width}"
            );
            assert_eq!(
                text.iter()
                    .filter(|line| **line == "Commit actions")
                    .count(),
                1,
                "the scoped menu replaces the separate window at {width}"
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
            egui::epaint::Shape::Text(text) if text.galley.job.text == "Integrate"
        )));
    }

    #[test]
    fn commit_action_menu_returns_the_clicked_command() {
        let ctx = egui::Context::default();
        let id = "a".repeat(40);
        let command = std::cell::RefCell::new(None);
        let first = crate::tests::run_ui(&ctx, frame_input(360.0, 600.0, vec![]), |ui| {
            command
                .borrow_mut()
                .replace(commit_action_menu(ui, &id, true, None));
        });
        let compare = first.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text)
                if text.galley.job.text == "Compare aaaaaaa with current HEAD" =>
            {
                Some(text.pos)
            }
            _ => None,
        });
        let Some(compare) = compare else {
            return;
        };
        let press = egui::Event::PointerButton {
            pos: compare + egui::vec2(5.0, 0.0),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        };
        let release = egui::Event::PointerButton {
            pos: compare + egui::vec2(5.0, 0.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = crate::tests::run_ui(
            &ctx,
            frame_input(
                360.0,
                600.0,
                vec![egui::Event::PointerMoved(compare), press],
            ),
            |ui| {
                command
                    .borrow_mut()
                    .replace(commit_action_menu(ui, &id, true, None));
            },
        );
        let _ = crate::tests::run_ui(&ctx, frame_input(360.0, 600.0, vec![release]), |ui| {
            command
                .borrow_mut()
                .replace(commit_action_menu(ui, &id, true, None));
        });
        assert!(matches!(
            command.borrow().as_ref(),
            Some(Some(CommitCommand::Action(CommitAction::Compare)))
        ));
    }

    #[test]
    fn help_shortcuts_window_renders() {
        let ctx = egui::Context::default();
        let mut app = layout_fixture();
        app.help_shortcuts = true;
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
        assert!(text.contains(&"Commit actions"));
        assert!(text.iter().any(|line| line.contains("Alt+A")));
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
            egui::epaint::Shape::Text(text) if text.galley.job.text == "Backups…" => {
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
        assert!(app.backups.is_some());
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
            assert!(text.iter().any(|line| line.contains("Backups…")));
        }
    }

    #[test]
    fn rebase_banner_surfaces_backups() {
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
        assert!(text.iter().any(|line| line.contains("Backups…")));
    }

    #[test]
    fn network_cancel_button_stays_inside_the_toolbar() {
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
                        if text.galley.job.text == phosphor::X
                            && text.pos.x + text.galley.size().x <= width
                )),
                "cancel icon missing or clipped at width {width}"
            );
        }
    }

    #[test]
    fn finish_network_clone_opens_the_new_repository() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let mut app = Browser::new(".".into());
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
        Ok(())
    }

    #[test]
    fn branch_creation_window_renders_at_supported_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut app = layout_fixture();
            app.new_branch = Some("feature-x".into());
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
            assert!(text.iter().any(|line| line.contains("Create branch")));
        }
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
    fn filter_focus_keeps_arrow_keys_in_the_editor() {
        let ctx = egui::Context::default();
        let mut app = Browser::new(".".into());
        let _ = crate::tests::run_ui(&ctx, egui::RawInput::default(), |ctx| app.draw_ui(ctx));
        let _ = crate::tests::run_ui(
            &ctx,
            egui::RawInput {
                events: vec![key(egui::Key::F, egui::Modifiers::COMMAND)],
                ..Default::default()
            },
            |ctx| app.draw_ui(ctx),
        );
        assert!(ctx.memory(|m| m.has_focus(egui::Id::unique("history-filter"))));
        let _ = crate::tests::run_ui(
            &ctx,
            egui::RawInput {
                events: vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
                ..Default::default()
            },
            |ctx| app.draw_ui(ctx),
        );
        assert!(!ctx.memory(|m| m.has_focus(egui::Id::unique("history-filter"))));
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
                assert!(app.history_rect.top() <= 130.0);
                assert!(app.history_rect.bottom() <= height - 20.0);
                if width >= 1040.0 {
                    assert!(!app.history_rect.intersects(app.details_rect));
                    assert!(app.details_rect.right() <= width);
                } else {
                    assert_eq!(app.details_rect, egui::Rect::NOTHING);
                }
                for label in ["Repository", "View", "Local Changes"] {
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
                        if text.galley.job.text == label && text.pos.y < 80.0 =>
                    {
                        Some(text.pos)
                    }
                    _ => None,
                })
            };
            let local = nav("Local Changes").ok_or("missing Local Changes")?;
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
                    egui::epaint::Shape::Text(text) if text.galley.job.text == "Refs"
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
            for label in ["Repository", "Local Changes", "Commit", "Amend"] {
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
                            if text.galley.job.text == "Local Changes" =>
                        {
                            Some(text.pos.x)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    positions.len(),
                    2,
                    "one strip entry plus the workspace heading at {width}: {positions:?}"
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
                    if text.galley.job.text == "Refs" && text.pos.y < 42.0 =>
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
                if text.iter().any(|line| line == "Switch to branch") {
                    break;
                }
            }
            assert!(
                text.iter().any(|line| line == "Switch to branch"),
                "missing explicit switch route at {width}: {text:?}"
            );
            assert!(
                text.iter().any(|line| line == "Branches"),
                "missing read-only branch browsing at {width}"
            );
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
