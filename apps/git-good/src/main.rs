mod avatars;
mod binary_preview;
mod bounded_text;
mod branch_cli;
mod branding;
mod colors;
mod commit_cli;
mod details_view;
mod diff_view;
mod diff_worker;
mod export;
mod graph;
mod history_view;
#[cfg(test)]
mod interaction_tests;
mod navigation;
mod network;
mod phosphor;
mod records;
mod refs_cli;
mod refs_view;
mod review_cli;
mod review_view;
mod review_worker;
mod stage_view;
mod state;
mod status_cli;
mod status_view;
mod status_worker;
mod stream;
mod sync_cli;
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
            "gg · good git\n\ngit-good [PATH]\ngit-good --version        Print version\ngit-good history [PATH]   Print history as JSON\ngit-good refs [PATH]      Print branches, remotes, tags, and HEAD as JSON\ngit-good status [PATH] [--file PATH --area AREA]   Print working status or diff as JSON\ngit-good stage --file PATH [--hunk HEADER]... [--repo PATH]   Stage one file or selected hunks\ngit-good unstage --file PATH [--repo PATH]   Unstage one file\ngit-good commit -m MESSAGE [--repo PATH]   Commit the index after Git hooks\ngit-good switch [--create] BRANCH [--repo PATH]   Switch to or create a local branch\ngit-good show <commit> [--repo PATH] [--parent N] [--file PATH] [--export before|after --output PATH]   Review or export\n\nBrowse commits reachable from HEAD and all local, remote, and tag refs."
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
    if first.as_deref() == Some(std::ffi::OsStr::new("refs")) {
        return refs_cli::run(args);
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("switch")) {
        return branch_cli::run(args);
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

struct Browser {
    logo: Option<egui::TextureHandle>,
    path: String,
    review: Option<review_view::Review>,
    working: Option<status_view::StatusView>,
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

    fn finish_network(&mut self, ctx: &egui::Context, result: Result<network::Outcome, String>) {
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
                self.path = receipt.path.to_string_lossy().into_owned();
                self.state.history = None;
                self.state.rows.clear();
                self.state.graphs.clear();
                self.state.details = None;
                self.state.selected = None;
                self.state.selected_row = None;
                self.state.refs = Default::default();
                self.refs_query.clear();
                self.load(ctx);
            }
            Err(error) => {
                self.network_error = Some(error);
            }
        }
    }

    fn load(&mut self, ctx: &egui::Context) {
        self.branch_error = None;
        self.network_error = None;
        let path = PathBuf::from(&self.path);
        self.state.begin(path.clone());
        let request = stream::Request {
            revision: self.state.filter_revision,
            query: self.state.query.clone(),
            selected_id: self.state.selected.clone(),
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

    fn open_review(&mut self, ctx: &egui::Context) {
        if let (Some(history), Some(id)) = (&self.state.history, &self.state.selected) {
            let subject = self
                .state
                .displayed_details()
                .map(|c| c.subject().to_owned())
                .unwrap_or_default();
            self.review = Some(review_view::Review::new(
                gg_engine::review::ReviewRequest {
                    path: history.path.clone(),
                    revision: id.clone(),
                    parent: None,
                },
                subject,
                ctx,
            ));
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
            self.finish_network(ctx, result);
        }
        if let Some(working) = &mut self.working {
            let close = working.draw_ui(ui);
            if let Some(id) = working.take_commit() {
                self.pending_commit = Some(id);
            }
            if close {
                self.working = None;
                if let Some(id) = self.pending_commit.take() {
                    self.state.selected = Some(id);
                    self.scroll_offset = 0.0;
                    self.restore_offset = None;
                    self.scroll_to_selection = true;
                    self.load(ctx);
                } else {
                    self.restore_offset = Some(self.scroll_offset);
                }
                ctx.request_repaint();
            }
            return;
        }
        if let Some(review) = &mut self.review {
            if review.draw_ui(ui) {
                self.review = None;
                self.restore_offset = Some(self.scroll_offset);
                ctx.request_repaint();
            }
            return;
        }
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
            && self.staged.as_ref().is_some_and(|u| {
                u.request.anchor_id.is_none()
                    || u.anchor_row.is_some()
                    || u.complete
                    || u.error.is_some()
            })
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
        let filter_id = egui::Id::unique("history-filter");
        let focus_filter = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F));
        if focus_filter {
            self.narrow_details = false;
            ctx.memory_mut(|m| m.request_focus(filter_id));
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::R)) {
            self.load(ctx);
        }
        let narrow = ctx.content_rect().width() < 1040.0;
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
                self.open_review(ctx);
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
        egui::Panel::top("toolbar").exact_size(42.0).show(ui, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                branding::toolbar(ui, &mut self.logo);
                ui.menu_button("Repository", |ui| {
                    if ui
                        .add_enabled(
                            self.network.is_none() && self.branch.is_none(),
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
                    ui.separator();
                    let network_idle = self.network.is_none() && self.branch.is_none();
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
                let branch_pending = self.branch.is_some();
                let mut jump = None;
                let mut switch = None;
                let mut new_branch = false;
                let mut track_branch = false;
                let mut section = |ui: &mut egui::Ui, title: &str, kind| {
                    let filtered = refs
                        .items
                        .iter()
                        .filter(|item| item.kind == kind)
                        .collect::<Vec<_>>();
                    if filtered.is_empty() {
                        return;
                    }
                    ui.menu_button(title, |ui| {
                        for item in filtered {
                            if ui
                                .add_enabled(!branch_pending, egui::Button::new(item.name.clone()))
                                .clicked()
                            {
                                jump = Some(item.commit.clone());
                                ui.close();
                            }
                        }
                    });
                };
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
                    let branches = refs
                        .items
                        .iter()
                        .filter(|item| item.kind == gg_engine::refs::RefKind::Branch)
                        .collect::<Vec<_>>();
                    if !branches.is_empty() {
                        ui.menu_button("Branches", |ui| {
                            for item in branches {
                                let current = Some(item.full_name.as_str()) == head_name;
                                let label = if current {
                                    format!("{}  ·  HEAD", item.name)
                                } else {
                                    item.name.clone()
                                };
                                if ui
                                    .add_enabled(
                                        !current && !branch_pending,
                                        egui::Button::new(label),
                                    )
                                    .on_hover_text(if current {
                                        "Already on this branch"
                                    } else {
                                        "Switch to this branch (requires a clean worktree)"
                                    })
                                    .clicked()
                                {
                                    switch = Some(item.name.clone());
                                    ui.close();
                                }
                            }
                        });
                    }
                    section(ui, "Remotes", gg_engine::refs::RefKind::Remote);
                    section(ui, "Tags", gg_engine::refs::RefKind::Tag);
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
                ui.separator();
                let trailing = if self.network.is_some() { 120.0 } else { 84.0 };
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
                if ui
                    .add_enabled(
                        self.state.history.is_some(),
                        egui::Button::new("Status").min_size(egui::vec2(66.0, 26.0)),
                    )
                    .clicked()
                    && let Some(history) = &self.state.history
                {
                    self.working = Some(status_view::StatusView::new(history.path.clone(), ctx));
                }
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
                        self.load(ctx);
                        self.edit_path = false;
                    }
                });
            });
        }
        let wide_refs = !narrow && self.refs_visible && ctx.content_rect().width() >= 1120.0;
        if wide_refs {
            let refs = self.state.refs.clone();
            let busy = self.branch.is_some() || self.network.is_some();
            let mut action = None;
            egui::Panel::left("refs-sidebar")
                .exact_size(190.0)
                .resizable(false)
                .frame(egui::Frame::side_top_panel(ui.style().as_ref()).inner_margin(8.0))
                .show(ui, |ui| {
                    action = refs_view::show(ui, &refs, &mut self.refs_query, busy);
                });
            match action {
                Some(refs_view::Action::Switch(name)) => self.start_branch(ctx, name, false),
                Some(refs_view::Action::Jump(id)) => self.jump_to_ref(id),
                Some(refs_view::Action::NewBranch) => self.new_branch = Some(String::new()),
                Some(refs_view::Action::Track(name)) => {
                    self.track_dialog = Some(name);
                }
                None => {}
            }
        }
        if let Some(name) = &mut self.new_branch {
            let mut submit = None;
            let mut open =
                !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            let branch_pending = self.branch.is_some();
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
        if let Some(remote_branch) = &mut self.track_dialog {
            let mut submit = false;
            let mut cancelled = false;
            let mut open =
                !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            let busy = self.branch.is_some() || self.network.is_some();
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
        egui::Panel::bottom("status").exact_size(25.0).show(ui, |ui| {
            let (message, color) = if let Some(error) = &self.network_error {
                (format!("Network · {error}"), ui.visuals().error_fg_color)
            } else if let Some(error) = &self.state.error {
                (format!("{}{}", if self.state.history.is_some() { "Previous results · " } else { "" }, error), ui.visuals().error_fg_color)
            } else if let Some(error) = &self.branch_error {
                (format!("Branch · {error}"), ui.visuals().error_fg_color)
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
            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
            bounded_text::paint(ui, rect, &message, egui::FontId::proportional(11.0), color);
        });
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
                    details_view::show(ui, commit, self.state.details.is_none(), &mut self.avatars);
                });
            self.details_rect = details.response.rect;
        }
        egui::CentralPanel::default().show(ui, |ui| {
            if narrow && self.narrow_details {
                self.history_rect = egui::Rect::NOTHING;
                self.details_rect = ui.available_rect_before_wrap();
                if ui.button("Back to history").clicked() {
                    self.narrow_details = false;
                }
                ui.add_space(8.0);
                let commit = self.state.displayed_details();
                details_view::show(ui, commit, self.state.details.is_none(), &mut self.avatars);
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
                                        if let Some(action) = ref_action {
                                            response.surrender_focus();
                                            self.state.select_row(row);
                                            if self.worker.is_none() {
                                                self.state.details =
                                                    self.state.rows.get(&row).cloned();
                                            }
                                            match action {
                                                history_view::RefAction::Select => {}
                                                history_view::RefAction::Switch {
                                                    name,
                                                    full_name,
                                                } => {
                                                    let current = self
                                                        .state
                                                        .refs
                                                        .head
                                                        .as_ref()
                                                        .and_then(|head| head.name.as_deref())
                                                        == Some(full_name.as_str());
                                                    if !current && self.branch.is_none() {
                                                        self.start_branch(ctx, name, false);
                                                    }
                                                }
                                                history_view::RefAction::Jump(id) => {
                                                    self.jump_to_ref(id)
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
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            if self.pending.is_none() {
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
                assert!(app.history_rect.top() <= 100.0);
                assert!(app.history_rect.bottom() <= height - 20.0);
                if width >= 1040.0 {
                    assert!(!app.history_rect.intersects(app.details_rect));
                    assert!(app.details_rect.right() <= width);
                } else {
                    assert_eq!(app.details_rect, egui::Rect::NOTHING);
                }
                for label in ["Repository", "View", "Status"] {
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
