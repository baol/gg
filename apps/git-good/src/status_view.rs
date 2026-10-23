//! Compact, paged working status with stable interaction boundaries.
use crate::{
    bounded_text,
    status_worker::{Update, Worker},
};
use eframe::egui;
use gg_engine::status::{Area, Kind, StatusEntry};
use std::{collections::BTreeMap, ops::Range, path::PathBuf};
const HEIGHT: f32 = 24.0;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mutation {
    Stage,
    Unstage,
}
fn commit_reason(
    busy: bool,
    scanning: bool,
    message: &str,
    amend: bool,
    staged: usize,
) -> Option<&'static str> {
    if busy {
        Some("Wait for the current operation")
    } else if scanning {
        Some("Waiting for the status scan")
    } else if message.trim().is_empty() {
        Some("Enter a commit message")
    } else if !amend && staged == 0 {
        Some("Stage at least one file")
    } else {
        None
    }
}

/// Session-level commit text and selection retained while the workspace is closed.
#[derive(Clone, Default)]
pub(crate) struct Draft {
    pub normal: String,
    pub amend: String,
    pub amend_mode: bool,
    pub amend_base: Option<String>,
    pub selected: Option<(Vec<u8>, Area)>,
}
pub struct StatusView {
    path: PathBuf,
    identity: Option<gg_engine::status::StatusIdentity>,
    stage: crate::stage_view::StageView,
    reading: bool,
    mutation: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    mutation_kind: Option<Mutation>,
    commit: Option<std::sync::mpsc::Receiver<Result<gg_git::commit::Receipt, String>>>,
    commit_done: Option<String>,
    last_commit: Option<String>,
    commit_warning: Option<String>,
    message: String,
    normal_draft: String,
    amend_draft: String,
    amend: bool,
    amend_base: Option<String>,
    amend_subject: Option<String>,
    message_expanded: bool,
    pending_selection: Option<(Vec<u8>, Area)>,
    last_amended: bool,
    worker: Option<Worker>,
    pending: Option<Worker>,
    staged: Option<Update>,
    rows: BTreeMap<usize, StatusEntry>,
    count: usize,
    staged_count: usize,
    complete: bool,
    error: Option<String>,
    action_error: Option<String>,
    replacing: bool,
    viewport: Range<usize>,
    offset: f32,
    selected: Option<usize>,
    reveal: bool,
    content: egui::Rect,
    last_scroll: f64,
    restore: bool,
}
impl StatusView {
    pub fn new(path: PathBuf, ctx: &egui::Context) -> Self {
        let mut view = Self {
            path,
            identity: None,
            stage: crate::stage_view::StageView::default(),
            reading: false,
            mutation: None,
            mutation_kind: None,
            commit: None,
            commit_done: None,
            last_commit: None,
            commit_warning: None,
            message: String::new(),
            normal_draft: String::new(),
            amend_draft: String::new(),
            amend: false,
            amend_base: None,
            amend_subject: None,
            message_expanded: false,
            pending_selection: None,
            last_amended: false,
            worker: None,
            pending: None,
            staged: None,
            rows: BTreeMap::new(),
            count: 0,
            staged_count: 0,
            complete: false,
            error: None,
            action_error: None,
            replacing: false,
            viewport: 0..64,
            offset: 0.0,
            selected: None,
            reveal: false,
            content: egui::Rect::NOTHING,
            last_scroll: f64::NEG_INFINITY,
            restore: false,
        };
        view.start(ctx, false);
        view
    }
    fn start(&mut self, ctx: &egui::Context, replacing: bool) {
        self.staged = None;
        self.error = None;
        self.action_error = None;
        self.replacing = replacing;
        match Worker::start(self.path.clone(), ctx.clone(), self.amend_base.clone()) {
            Ok(worker) => {
                if replacing {
                    self.pending = Some(worker);
                } else {
                    self.worker = Some(worker);
                }
            }
            Err(e) => {
                self.error = Some(e);
                if replacing {
                    self.pending = None;
                } else {
                    self.worker = None;
                }
            }
        }
    }
    fn restart(&mut self, ctx: &egui::Context) {
        self.worker = None;
        self.pending = None;
        self.rows.clear();
        self.count = 0;
        self.complete = false;
        self.selected = None;
        self.offset = 0.0;
        self.viewport = 0..64;
        self.restore = false;
        self.start(ctx, false);
    }
    fn accept(&mut self, safe: bool) {
        if !safe {
            return;
        }
        if self.replacing
            && self
                .staged
                .as_ref()
                .is_some_and(|u| !u.complete && u.error.is_none())
        {
            return;
        }
        let Some(update) = self.staged.take() else {
            return;
        };
        if self.replacing && update.error.is_some() {
            self.error = update.error;
            return;
        }
        if self.replacing {
            self.worker = self.pending.take();
            self.rows.clear();
            self.offset = 0.0;
            self.viewport = 0..64;
            self.selected = None;
            self.restore = true;
            self.replacing = false;
        }
        self.path = update.identity.path.clone();
        self.identity = Some(update.identity);
        self.count = update.count;
        self.staged_count = update.staged;
        self.complete = update.complete;
        // Paging a retained scan must preserve an action failure until explicit retry.
        self.error = update.error.or_else(|| self.action_error.clone());
        self.rows.extend(update.rows);
        let keep = self.viewport.start.saturating_sub(256)..self.viewport.end.saturating_add(256);
        self.rows.retain(|row, _| keep.contains(row));
        while self.rows.len() > 768 {
            self.rows.pop_last();
        }
        if let Some((bytes, area)) = self.pending_selection.clone()
            && let Some((row, _)) = self
                .rows
                .iter()
                .find(|(_, entry)| entry.path_bytes == bytes && entry.area == area)
        {
            self.selected = Some(*row);
            self.reveal = true;
            self.pending_selection = None;
        }
        if self.selected.is_none() && self.count > 0 {
            self.selected = Some(0);
        }
    }
    fn fill_unhovered(&mut self, pinned: Option<usize>) {
        if self.replacing {
            return;
        }
        if let Some(update) = &self.staged {
            for (row, entry) in &update.rows {
                if *row < self.count && Some(*row) != pinned {
                    self.rows.insert(*row, entry.clone());
                }
            }
        }
        let keep = self.viewport.start.saturating_sub(256)..self.viewport.end.saturating_add(256);
        self.rows
            .retain(|row, _| keep.contains(row) || Some(*row) == pinned);
        while self.rows.len() > 768 {
            let victim = self.rows.keys().copied().find(|row| Some(*row) != pinned);
            if let Some(row) = victim {
                self.rows.remove(&row);
            } else {
                break;
            }
        }
    }
    fn open_diff(&mut self, ctx: &egui::Context) {
        if let Some(entry) = self.selected.and_then(|row| self.rows.get(&row)).cloned()
            && let Some(repository) = self.identity.clone()
        {
            let companion = self
                .rows
                .values()
                .find(|candidate| {
                    candidate.path_bytes == entry.path_bytes && candidate.area != entry.area
                })
                .cloned()
                .or_else(|| {
                    self.staged.as_ref().and_then(|update| {
                        update
                            .rows
                            .iter()
                            .map(|(_, entry)| entry)
                            .find(|candidate| {
                                candidate.path_bytes == entry.path_bytes
                                    && candidate.area != entry.area
                            })
                            .cloned()
                    })
                });
            self.stage.select_working(repository, entry, companion, ctx);
            self.reading = true;
        }
    }
    fn busy(&self) -> bool {
        self.mutation.is_some() || self.commit.is_some()
    }

    pub fn take_commit(&mut self) -> Option<String> {
        self.commit_done.take()
    }

    /// Canonical worktree identity used to keep drafts from crossing repositories.
    pub(crate) fn repository_key(&self) -> PathBuf {
        std::fs::canonicalize(&self.path).unwrap_or_else(|_| self.path.clone())
    }

    /// Capture the active draft before the workspace is dropped.
    pub(crate) fn capture_draft(&self) -> Draft {
        let (normal, amend) = if self.amend {
            (self.normal_draft.clone(), self.message.clone())
        } else {
            (self.message.clone(), self.amend_draft.clone())
        };
        Draft {
            normal,
            amend,
            amend_mode: self.amend,
            amend_base: self.amend_base.clone(),
            selected: self
                .selected
                .and_then(|row| self.rows.get(&row))
                .map(|entry| (entry.path_bytes.clone(), entry.area)),
        }
    }

    /// Restore a draft after reopening Local Changes for the same worktree.
    pub(crate) fn restore_draft(&mut self, draft: Draft, ctx: &egui::Context) {
        self.normal_draft = draft.normal;
        self.amend_draft = draft.amend;
        self.pending_selection = draft.selected;
        self.message = self.normal_draft.clone();
        let Some(base) = draft.amend_base else {
            return;
        };
        if !draft.amend_mode
            || gg_git::status::amend_base_tree(&self.path).ok().as_deref() != Some(base.as_str())
        {
            return;
        }
        self.amend = true;
        self.amend_base = Some(base);
        let head = gg_git::commit::head_message(&self.path).ok();
        self.amend_subject = head
            .as_deref()
            .and_then(|message| message.lines().next())
            .map(str::to_string);
        self.message = if self.amend_draft.is_empty() {
            head.unwrap_or_default()
        } else {
            self.amend_draft.clone()
        };
        self.amend_draft = self.message.clone();
        self.restart(ctx);
    }

    #[cfg(test)]
    pub(crate) fn draft(&self) -> &str {
        &self.message
    }

    #[cfg(test)]
    pub(crate) fn set_draft(&mut self, message: String) {
        self.message = message;
    }

    /// Refresh the status scan without discarding the amend mode or message draft.
    pub fn refresh(&mut self, ctx: &egui::Context) {
        if self.amend {
            match gg_git::status::amend_base_tree(&self.path) {
                Ok(base) => self.amend_base = Some(base),
                Err(error) => {
                    self.amend = false;
                    self.amend_base = None;
                    self.action_error = Some(error);
                }
            }
        }
        self.reading = false;
        self.stage = crate::stage_view::StageView::default();
        self.start(ctx, true);
    }

    /// Enable amend mode, load the current HEAD message, and compare the index with HEAD's parent.
    pub fn start_amend(&mut self, ctx: &egui::Context) -> Result<(), String> {
        if self.busy() || self.replacing {
            return Err("Wait for the current operation to finish.".into());
        }
        let base = gg_git::status::amend_base_tree(&self.path)?;
        let message = gg_git::commit::head_message(&self.path)?;
        if !self.amend {
            self.normal_draft = self.message.clone();
        }
        self.amend_subject = Some(
            message
                .lines()
                .next()
                .unwrap_or("(empty message)")
                .to_string(),
        );
        self.message = if self.amend_draft.trim().is_empty() {
            message
        } else {
            self.amend_draft.clone()
        };
        self.amend_draft = self.message.clone();
        self.amend = true;
        self.amend_base = Some(base);
        self.error = None;
        self.action_error = None;
        self.reading = false;
        self.restart(ctx);
        Ok(())
    }

    fn start_commit(&mut self, ctx: &egui::Context) {
        if self.busy() || self.replacing || self.message.trim().is_empty() {
            return;
        }
        let Some(repository) = self.identity.clone() else {
            self.error = Some("Status has not finished loading".into());
            return;
        };
        let message = self.message.clone();
        let amend = self.amend;
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let repaint = ctx.clone();
        match std::thread::Builder::new()
            .name("gg-commit".into())
            .spawn(move || {
                let result = if amend {
                    gg_git::commit::amend(&repository, &message)
                } else {
                    gg_git::commit::create(&repository, &message)
                };
                let _ = tx.send(result);
                repaint.request_repaint();
            }) {
            Ok(_) => {
                self.commit = Some(rx);
                self.error = None;
                self.action_error = None;
            }
            Err(e) => self.error = Some(format!("Start commit: {e}")),
        }
    }

    fn unstage(&mut self, ctx: &egui::Context) {
        if self.busy() || self.replacing {
            return;
        }
        let Some(entry) = self.selected.and_then(|r| self.rows.get(&r)).cloned() else {
            return;
        };
        let Some(repository) = self.identity.clone() else {
            return;
        };
        if entry.area != Area::Staged {
            return;
        }
        self.mutate(ctx, Mutation::Unstage, "unstage", move || {
            gg_git::unstage::file(&gg_engine::status::WorkingRequest { repository, entry })
        });
    }
    fn stage(&mut self, ctx: &egui::Context) {
        if self.busy() || self.replacing {
            return;
        }
        let Some(entry) = self.selected.and_then(|r| self.rows.get(&r)).cloned() else {
            return;
        };
        let Some(repository) = self.identity.clone() else {
            return;
        };
        if !matches!(entry.area, Area::Unstaged | Area::Untracked) {
            return;
        }
        self.mutate(ctx, Mutation::Stage, "stage", move || {
            gg_git::stage::file(&gg_engine::status::WorkingRequest { repository, entry })
        });
    }
    fn mutate(
        &mut self,
        ctx: &egui::Context,
        kind: Mutation,
        name: &'static str,
        apply: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let ctx = ctx.clone();
        match std::thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                let result = apply();
                let _ = tx.send(result);
                ctx.request_repaint();
            }) {
            Ok(_) => {
                self.mutation = Some(rx);
                self.mutation_kind = Some(kind);
                self.error = None;
            }
            Err(e) => self.error = Some(format!("Start {name}: {e}")),
        }
    }
    fn draw_diff(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        let pending = self.stage.pending();
        let mut back = !pending
            && crate::navigation::available(ctx)
            && ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                    || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
            });
        let mut reload = false;
        egui::Panel::top("working-diff-header")
            .exact_size(42.0)
            .show(ui, |ui| {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    back |= ui
                        .add_enabled(
                            !pending,
                            egui::Button::new("Back to files").min_size(egui::vec2(100.0, 26.0)),
                        )
                        .clicked();
                    ui.strong("Review & stage");
                    ui.allocate_ui_with_layout(
                        egui::vec2((ui.available_width() - 110.0).max(100.0), 26.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            bounded_text::field(
                                ui,
                                self.stage.path(),
                                egui::FontId::proportional(12.0),
                                26.0,
                            );
                        },
                    );
                    reload |= ui
                        .add_enabled(
                            !pending,
                            egui::Button::new("Reload").min_size(egui::vec2(94.0, 26.0)),
                        )
                        .clicked();
                });
            });
        if let Some(error) = &self.error {
            egui::Panel::top("working-diff-error")
                .exact_size(23.0)
                .show(ui, |ui| {
                    bounded_text::field(ui, error, egui::FontId::proportional(12.0), 22.0);
                });
        }
        egui::CentralPanel::default().show(ui, |ui| self.stage.draw(ui, ctx));
        bounded_text::show_disclosure(ctx);
        if reload {
            self.stage.reload(ctx);
        }
        if back {
            if self.stage.take_changed() {
                self.start(ctx, true);
            }
            self.reading = false;
            self.restore = true;
        }
    }
    pub(crate) fn draw_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        if let Some(result) = self.mutation.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Some(Err("Mutation worker stopped. Refresh status.".into()))
            }
            Err(_) => None,
        }) {
            self.mutation = None;
            match result {
                Ok(()) => {
                    self.start(ctx, true);
                }
                Err(e) => {
                    self.action_error = Some(e.clone());
                    self.error = Some(e);
                }
            }
        }
        if let Some(result) = self.commit.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Some(Err("Commit worker stopped. Refresh status.".into()))
            }
            Err(_) => None,
        }) {
            self.commit = None;
            match result {
                Ok(receipt) => {
                    self.last_commit = Some(receipt.id.get(..7).unwrap_or(&receipt.id).to_string());
                    self.last_amended = receipt.amended;
                    if self.amend {
                        self.amend_draft.clear();
                    } else {
                        self.normal_draft.clear();
                    }
                    self.amend = false;
                    self.amend_base = None;
                    self.amend_subject = None;
                    self.commit_done = Some(receipt.id.clone());
                    self.commit_warning = if receipt.warnings.is_empty() {
                        None
                    } else {
                        Some(receipt.warnings.join(" · "))
                    };
                    self.message.clear();
                    self.start(ctx, true);
                }
                Err(e) => {
                    self.start(ctx, true);
                    self.action_error = Some(e.clone());
                    self.error = Some(e);
                }
            }
        }
        if self.reading {
            self.draw_diff(ui);
            return false;
        }
        let (now, scrolling, pointer_down, hovered) = ctx.input(|i| {
            (
                i.time,
                i.smooth_scroll_delta != egui::Vec2::ZERO,
                i.pointer.any_down(),
                i.pointer
                    .hover_pos()
                    .is_some_and(|p| self.content.contains(p)),
            )
        });
        if scrolling {
            self.last_scroll = now;
            ctx.request_repaint_after(std::time::Duration::from_millis(210));
        }
        let safe = !hovered && !pointer_down && now - self.last_scroll > 0.2;
        let pinned = ctx
            .input(|i| i.pointer.hover_pos())
            .filter(|p| self.content.contains(*p))
            .map(|p| {
                ((p.y - self.content.top() + self.offset) / HEIGHT)
                    .floor()
                    .max(0.0) as usize
            });
        if self.replacing
            && let Some(Ok(update)) = self.worker.as_ref().and_then(Worker::poll)
        {
            self.rows.extend(
                update
                    .rows
                    .into_iter()
                    .filter(|(row, _)| *row < self.count && Some(*row) != pinned && !pointer_down),
            );
        }
        let source = if self.replacing {
            &self.pending
        } else {
            &self.worker
        };
        if let Some(result) = source.as_ref().and_then(Worker::poll) {
            match result {
                Ok(update) => self.staged = Some(update),
                Err(error) => self.error = Some(error),
            }
        }
        self.accept(safe);
        let keep = self.viewport.start.saturating_sub(256)..self.viewport.end.saturating_add(256);
        self.rows
            .retain(|row, _| keep.contains(row) || Some(*row) == pinned);
        if !safe && !pointer_down {
            let pinned = ctx
                .input(|i| i.pointer.hover_pos())
                .filter(|p| self.content.contains(*p))
                .map(|p| {
                    ((p.y - self.content.top() + self.offset) / HEIGHT)
                        .floor()
                        .max(0.0) as usize
                });
            self.fill_unhovered(pinned);
        }
        let mut back = false;
        let mut refresh = false;
        let mut open = false;
        let mut unstage = false;
        let mut stage = false;
        let mut commit_now = false;
        if !self.busy() && crate::navigation::available(ctx) {
            open |= crate::navigation::activation_available(ctx)
                && ctx.input_mut(|i| {
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                        || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)
                });
            back |= ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                    || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
            });
            refresh |= ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::R));
            let page = (self.content.height() / HEIGHT).floor().max(2.0) as isize - 1;
            for (mods, key, distance) in [
                (egui::Modifiers::NONE, egui::Key::ArrowDown, 1),
                (egui::Modifiers::NONE, egui::Key::ArrowUp, -1),
                (egui::Modifiers::NONE, egui::Key::J, 1),
                (egui::Modifiers::NONE, egui::Key::K, -1),
                (egui::Modifiers::CTRL, egui::Key::N, 1),
                (egui::Modifiers::CTRL, egui::Key::P, -1),
                (egui::Modifiers::NONE, egui::Key::PageDown, page),
                (egui::Modifiers::NONE, egui::Key::PageUp, -page),
                (egui::Modifiers::ALT, egui::Key::V, -page),
            ] {
                if ctx.input_mut(|i| i.consume_key(mods, key)) && self.count > 0 {
                    self.selected = Some(
                        self.selected
                            .unwrap_or(0)
                            .saturating_add_signed(distance)
                            .min(self.count - 1),
                    );
                    crate::navigation::focus_content(ctx);
                    self.reveal = true;
                }
            }
        }
        egui::Panel::top("working-header")
            .exact_size(42.0)
            .show(ui, |ui| {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.strong("Local Changes");
                    ui.allocate_ui_with_layout(
                        egui::vec2((ui.available_width() - 82.0).max(100.0), 26.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            bounded_text::field(
                                ui,
                                &self.path.display().to_string(),
                                egui::FontId::proportional(13.0),
                                26.0,
                            );
                        },
                    );
                    refresh |= ui
                        .add_enabled(
                            !self.busy(),
                            egui::Button::new(if self.error.is_some() {
                                "Retry"
                            } else {
                                "Refresh"
                            })
                            .min_size(egui::vec2(74.0, 26.0)),
                        )
                        .clicked();
                });
            });
        egui::Panel::bottom("working-status")
            .exact_size(25.0)
            .show(ui, |ui| {
                let text = if let Some(error) = &self.error {
                    format!(
                        "{}{}",
                        if self.replacing {
                            "Previous results · "
                        } else {
                            "Incomplete · "
                        },
                        error
                    )
                } else if let Some(warning) = &self.commit_warning {
                    format!(
                        "Committed {} · {warning}",
                        self.last_commit.as_deref().unwrap_or("")
                    )
                } else if let Some(id) = &self.last_commit {
                    format!(
                        "{} {id} · {}",
                        if self.last_amended {
                            "Amended"
                        } else {
                            "Committed"
                        },
                        if self.complete && self.count == 0 {
                            "Clean working tree".into()
                        } else {
                            format!(
                                "{} {}",
                                self.count,
                                if self.count == 1 { "entry" } else { "entries" }
                            )
                        }
                    )
                } else if self.replacing {
                    format!(
                        "Refreshing · showing previous results · {} {}",
                        self.count,
                        if self.count == 1 { "entry" } else { "entries" }
                    )
                } else if self.complete && self.count == 0 {
                    "Clean working tree".into()
                } else {
                    format!(
                        "{} {}{}",
                        self.count,
                        if self.count == 1 { "entry" } else { "entries" },
                        if self.complete { "" } else { " · scanning…" }
                    )
                };
                bounded_text::field(ui, &text, egui::FontId::proportional(12.0), 22.0);
            });
        let commit_height = if self.message_expanded { 150.0 } else { 82.0 };
        let message_height = if self.message_expanded { 118.0 } else { 54.0 };
        egui::Panel::bottom("working-commit")
            .exact_size(commit_height)
            .show(ui, |ui| {
                ui.add_space(4.0);
                let busy = self.busy() || self.replacing;
                let scanning = self.replacing || !self.complete || self.identity.is_none();
                let reason = commit_reason(
                    busy,
                    scanning,
                    &self.message,
                    self.amend,
                    self.staged_count,
                );
                let can_commit = reason.is_none();
                ui.horizontal(|ui| {
                    let button_width = if self.amend { 104.0 } else { 88.0 };
                    let width = (ui.available_width() - button_width - 26.0).max(160.0);
                    ui.add_sized(
                        [width, message_height],
                        egui::TextEdit::multiline(&mut self.message)
                            .hint_text("Commit subject\n\nOptional body")
                            .desired_rows(2),
                    );
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let mut amend = self.amend;
                            if ui
                                .add_enabled(!busy, egui::Checkbox::new(&mut amend, "Amend"))
                                .on_hover_text(
                                    "Show the commit's contents as staged. Unstage removes changes from the amended commit; working files stay unchanged.",
                                )
                                .changed()
                            {
                                if amend {
                                    if let Err(error) = self.start_amend(ctx) {
                                        self.action_error = Some(error);
                                    }
                                } else {
                                    if self.amend {
                                        self.amend_draft = self.message.clone();
                                    }
                                    self.amend = false;
                                    self.amend_base = None;
                                    self.amend_subject = None;
                                    self.message = self.normal_draft.clone();
                                    self.restart(ctx);
                                }
                            }
                            if ui
                                .small_button(if self.message_expanded {
                                    "Collapse"
                                } else {
                                    "Expand"
                                })
                                .on_hover_text("Expand the commit message editor")
                                .clicked()
                            {
                                self.message_expanded = !self.message_expanded;
                            }
                        });
                        let label = if self.commit.is_some() {
                            "Committing…"
                        } else if self.amend {
                            "Amend commit"
                        } else {
                            "Commit"
                        };
                        let response = ui.add_enabled(
                            can_commit,
                            egui::Button::new(label).min_size(egui::vec2(button_width, 26.0)),
                        );
                        let response = if let Some(reason) = reason {
                            response.on_hover_text(reason)
                        } else {
                            response
                        };
                        commit_now |= response.clicked();
                    });
                });
            });
        if self.amend {
            egui::Panel::bottom("working-amend")
                .exact_size(23.0)
                .show(ui, |ui| {
                    let head = self
                        .identity
                        .as_ref()
                        .and_then(|identity| identity.head.as_deref())
                        .and_then(|id| id.get(..7))
                        .unwrap_or("HEAD");
                    let subject = self.amend_subject.as_deref().unwrap_or("(empty message)");
                    bounded_text::field(
                        ui,
                        &format!(
                            "Amending {head} · {subject} · Staged changes are the commit contents · Unstage removes them from the amended commit"
                        ),
                        egui::FontId::proportional(12.0),
                        22.0,
                    );
                });
        }
        let selected_area = self
            .selected
            .and_then(|r| self.rows.get(&r))
            .map(|entry| entry.area);
        let pending = self.mutation.is_some();
        let stage_label = if pending && self.mutation_kind == Some(Mutation::Stage) {
            "Staging…"
        } else {
            "Stage file"
        };
        let unstage_label = if pending && self.mutation_kind == Some(Mutation::Unstage) {
            "Unstaging…"
        } else {
            "Unstage file"
        };
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                stage |= ui
                    .add_enabled(
                        !self.busy()
                            && !self.replacing
                            && self.error.is_none()
                            && matches!(selected_area, Some(Area::Unstaged | Area::Untracked)),
                        egui::Button::new(stage_label).min_size(egui::vec2(82.0, 26.0)),
                    )
                    .clicked();
                unstage |= ui
                    .add_enabled(
                        !self.busy()
                            && !self.replacing
                            && self.error.is_none()
                            && selected_area == Some(Area::Staged),
                        egui::Button::new(unstage_label).min_size(egui::vec2(100.0, 26.0)),
                    )
                    .clicked();
                open |= ui
                    .add_enabled(
                        !self.busy()
                            && self
                                .selected
                                .is_some_and(|row| self.rows.contains_key(&row)),
                        egui::Button::new("View diff").min_size(egui::vec2(82.0, 26.0)),
                    )
                    .clicked();
            });
            let header = ui
                .allocate_exact_size(
                    egui::vec2(ui.available_width(), HEIGHT),
                    egui::Sense::hover(),
                )
                .0;
            for (x, width, label) in [
                (0.0, 92.0, "Area"),
                (100.0, 110.0, "Change"),
                (220.0, header.width() - 220.0, "Path"),
            ] {
                let rect = egui::Rect::from_min_size(
                    header.min + egui::vec2(x, 0.0),
                    egui::vec2(width, HEIGHT),
                );
                bounded_text::paint(
                    ui,
                    rect,
                    label,
                    egui::FontId::proportional(12.0),
                    ui.visuals().weak_text_color(),
                );
            }
            ui.separator();
            self.content = ui.available_rect_before_wrap();
            if self.count == 0 {
                ui.weak(if self.complete {
                    "Everything is up to date."
                } else if self.error.is_some() {
                    "Status is unavailable. Retry to scan again."
                } else {
                    "Reading working changes…"
                });
                return;
            }
            let mut scroll = egui::ScrollArea::vertical()
                .id_salt("working-rows")
                .auto_shrink([false; 2]);
            if self.restore {
                scroll = scroll.vertical_scroll_offset(self.offset);
                self.restore = false;
            }
            if self.reveal
                && let Some(row) = self.selected
            {
                let top = row as f32 * HEIGHT;
                let bottom = top + HEIGHT;
                let offset = if top < self.offset {
                    top
                } else if bottom > self.offset + self.content.height() {
                    bottom - self.content.height()
                } else {
                    self.offset
                };
                scroll = scroll.vertical_scroll_offset(offset.max(0.0));
                self.reveal = false;
            }
            ui.spacing_mut().item_spacing.y = 0.0;
            let out = scroll.show_rows(ui, HEIGHT, self.count, |ui, visible| {
                self.viewport = visible.start.saturating_sub(16)..visible.end.saturating_add(16);
                for row in visible {
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), HEIGHT),
                        egui::Sense::click(),
                    );
                    if self.selected == Some(row) {
                        ui.painter()
                            .rect_filled(rect, 0.0, crate::colors::selection(ui.visuals()));
                    }
                    if let Some(entry) = self.rows.get(&row) {
                        if response.clicked() && !self.busy() {
                            response.surrender_focus();
                            self.selected = Some(row);
                        }
                        if response.double_clicked() && !self.busy() {
                            self.selected = Some(row);
                            open = true;
                        }
                        let color = match entry.kind {
                            Kind::Added => {
                                crate::colors::change(
                                    ui.visuals(),
                                    &gg_engine::review::ChangeKind::Added,
                                )
                                .1
                            }
                            Kind::Deleted => {
                                crate::colors::change(
                                    ui.visuals(),
                                    &gg_engine::review::ChangeKind::Deleted,
                                )
                                .1
                            }
                            Kind::Conflicted => ui.visuals().error_fg_color,
                            _ => ui.visuals().text_color(),
                        };
                        for (x, width, text, color) in [
                            (
                                0.0,
                                92.0,
                                entry.area.label(),
                                if entry.area == Area::Conflict {
                                    ui.visuals().error_fg_color
                                } else {
                                    ui.visuals().text_color()
                                },
                            ),
                            (100.0, 110.0, entry.kind.label(), color),
                            (
                                220.0,
                                rect.width() - 220.0,
                                entry.path.as_str(),
                                ui.visuals().text_color(),
                            ),
                        ] {
                            bounded_text::paint(
                                ui,
                                egui::Rect::from_min_size(
                                    rect.min + egui::vec2(x, 0.0),
                                    egui::vec2(width.max(0.0), HEIGHT),
                                ),
                                text,
                                egui::FontId::proportional(13.0),
                                color,
                            );
                        }
                    } else {
                        ui.painter().text(
                            rect.left_center(),
                            egui::Align2::LEFT_CENTER,
                            "Loading…",
                            egui::FontId::proportional(13.0),
                            ui.visuals().weak_text_color(),
                        );
                    }
                }
            });
            self.offset = out.state.clamped_offset().y;
        });
        if let Some(worker) = &self.worker {
            worker.request(self.viewport.clone());
        }
        bounded_text::show_disclosure(ctx);
        if self.busy() {
            return false;
        }
        if commit_now {
            self.start_commit(ctx);
            return false;
        }
        if stage {
            self.stage(ctx);
            return false;
        }
        if unstage {
            self.unstage(ctx);
            return false;
        }
        if refresh {
            self.start(ctx, true);
        }
        if open {
            self.open_diff(ctx);
        }
        back
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(row: usize) -> StatusEntry {
        StatusEntry::new(
            Area::Unstaged,
            Kind::Modified,
            format!("{row:06}-long-path-界.rs").as_bytes(),
        )
    }
    fn view(count: usize) -> StatusView {
        StatusView {
            path: "/repository".into(),
            identity: None,
            stage: crate::stage_view::StageView::default(),
            reading: false,
            mutation: None,
            mutation_kind: None,
            commit: None,
            commit_done: None,
            last_commit: None,
            commit_warning: None,
            message: String::new(),
            normal_draft: String::new(),
            amend_draft: String::new(),
            amend: false,
            amend_base: None,
            amend_subject: None,
            message_expanded: false,
            pending_selection: None,
            last_amended: false,
            worker: None,
            pending: None,
            staged: None,
            rows: (0..count.min(256)).map(|i| (i, entry(i))).collect(),
            count,
            staged_count: 0,
            complete: true,
            error: None,
            action_error: None,
            replacing: false,
            viewport: 0..64,
            offset: 0.0,
            selected: (count > 0).then_some(0),
            reveal: false,
            content: egui::Rect::NOTHING,
            last_scroll: f64::NEG_INFINITY,
            restore: false,
        }
    }
    fn update(count: usize, complete: bool) -> Update {
        Update {
            identity: gg_engine::status::StatusIdentity {
                path: "/repository".into(),
                head: None,
                index: "absent".into(),
                base_tree: None,
            },
            rows: (0..count.min(256)).map(|i| (i, entry(i))).collect(),
            count,
            staged: 0,
            complete,
            error: None,
        }
    }
    fn frame(ctx: &egui::Context, view: &mut StatusView, width: f32, events: Vec<egui::Event>) {
        let _ = crate::tests::run_ui(
            ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 480.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                view.draw_ui(ctx);
            },
        );
    }
    #[test]
    fn hovered_rows_and_refresh_results_wait_for_safe_adoption() {
        let ctx = egui::Context::default();
        let mut view = view(10);
        frame(&ctx, &mut view, 720.0, vec![]);
        view.staged = Some(update(1000, true));
        let point = view.content.center();
        frame(
            &ctx,
            &mut view,
            720.0,
            vec![egui::Event::PointerMoved(point)],
        );
        assert_eq!(view.count, 10);
        frame(&ctx, &mut view, 720.0, vec![egui::Event::PointerGone]);
        assert_eq!(view.count, 1000);
        view.replacing = true;
        view.selected = Some(8);
        view.offset = 60.0;
        view.staged = Some(update(2, false));
        view.accept(true);
        assert_eq!(
            (view.count, view.selected, view.offset),
            (1000, Some(8), 60.0)
        );
        let mut failure = update(2, false);
        failure.error = Some("read failed".into());
        view.staged = Some(failure);
        view.accept(true);
        assert_eq!(view.count, 1000);
        assert!(view.error.is_some());
        view.staged = Some(update(0, true));
        view.accept(false);
        assert_eq!(view.count, 1000);
        view.accept(true);
        assert_eq!((view.count, view.selected, view.offset), (0, None, 0.0));
        assert!(view.complete);
        assert!(view.error.is_none());
    }
    #[test]
    fn refresh_rereads_working_bytes_when_head_and_index_are_unchanged()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(["init", "-q"])
                .status()?
                .success()
        );
        std::fs::write(dir.path().join("a"), b"before\n")?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        let wait = |view: &mut StatusView,
                    reading: bool,
                    text: &str|
         -> Result<(), Box<dyn std::error::Error>> {
            let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                frame(&ctx, view, 1120.0, vec![]);
                if (!reading && view.complete && !view.replacing)
                    || (reading && view.stage.ready() && view.stage.has_text(text))
                {
                    return Ok(());
                }
                if std::time::Instant::now() > until {
                    return Err("working view timed out".into());
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        };
        wait(&mut view, false, "")?;
        let identity = view.identity.clone();
        view.open_diff(&ctx);
        wait(&mut view, true, "before")?;
        view.reading = false;
        std::fs::write(dir.path().join("a"), b"after\n")?;
        view.start(&ctx, true);
        wait(&mut view, false, "")?;
        assert_eq!(view.identity, identity);
        view.open_diff(&ctx);
        wait(&mut view, true, "after")?;
        assert!(!view.stage.has_text("before"));
        Ok(())
    }
    #[test]
    fn working_diff_back_retains_file_selection_and_scroll() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut view = view(1000);
            view.identity = Some(update(1000, true).identity);
            view.selected = Some(20);
            view.offset = 200.0;
            view.restore = true;
            frame(&ctx, &mut view, width, vec![]);
            let offset = view.offset;
            let key = |key| egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            frame(&ctx, &mut view, width, vec![key(egui::Key::Enter)]);
            assert!(view.reading);
            frame(&ctx, &mut view, width, vec![key(egui::Key::ArrowDown)]);
            assert_eq!(view.selected, Some(20));
            frame(&ctx, &mut view, width, vec![key(egui::Key::Escape)]);
            assert!(!view.reading);
            frame(&ctx, &mut view, width, vec![]);
            assert_eq!(view.selected, Some(20));
            assert!((view.offset - offset).abs() < 1.0);
        }
    }
    #[test]
    fn loading_rows_fill_around_the_hovered_slot_without_changing_extent() {
        let mut view = view(1000);
        view.rows.clear();
        view.viewport = 500..532;
        let mut incoming = update(2000, false);
        incoming.rows = (500..532).map(|row| (row, entry(row))).collect();
        view.staged = Some(incoming);
        view.fill_unhovered(Some(510));
        assert_eq!(view.count, 1000);
        assert!(!view.rows.contains_key(&510));
        assert!(view.rows.contains_key(&509) && view.rows.contains_key(&511));
        view.accept(true);
        assert_eq!(view.count, 2000);
        assert!(view.rows.contains_key(&510));
        assert!(view.rows.len() <= 768);
    }
    #[test]
    fn keyboard_pages_keep_last_page_stable_at_both_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut view = view(1000);
            frame(&ctx, &mut view, width, vec![]);
            assert!(view.content.top() >= 42.0 && view.content.bottom() <= 455.0);
            view.selected = Some(998);
            view.reveal = true;
            frame(&ctx, &mut view, width, vec![]);
            let event = egui::Event::Key {
                key: egui::Key::ArrowDown,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            frame(&ctx, &mut view, width, vec![event]);
            assert_eq!(view.selected, Some(999));
            let offset = view.offset;
            for _ in 0..3 {
                frame(&ctx, &mut view, width, vec![]);
                assert!((view.offset - offset).abs() < 1.0);
            }
            let event = egui::Event::Key {
                key: egui::Key::PageUp,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            frame(&ctx, &mut view, width, vec![event]);
            assert!(view.selected.is_some_and(|row| row < 999));
        }
    }
    #[test]
    fn unstage_worker_refreshes_and_retains_rows_on_failure()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let git = |args: &[&str]| -> Result<(), Box<dyn std::error::Error>> {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .output()?;
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            Ok(())
        };
        git(&["init"])?;
        std::fs::write(dir.path().join("a"), "staged")?;
        git(&["add", "a"])?;
        std::fs::write(dir.path().join("a"), "working")?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        let wait = |view: &mut StatusView| {
            let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                frame(&ctx, view, 720.0, vec![]);
                if view.mutation.is_none() && !view.replacing && view.complete {
                    break;
                }
                assert!(std::time::Instant::now() < until, "status did not settle");
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        };
        wait(&mut view);
        assert_eq!(view.rows[&0].area, Area::Staged);
        let identity = view.identity.clone();
        std::fs::write(dir.path().join(".git/index.lock"), "owner")?;
        view.unstage(&ctx);
        view.unstage(&ctx); // A pending action remains single-flight.
        wait(&mut view);
        assert!(view.error.as_ref().is_some_and(|e| e.contains("Unstage")));
        assert_eq!(view.identity, identity);
        assert_eq!(view.rows[&0].area, Area::Staged);
        std::fs::remove_file(dir.path().join(".git/index.lock"))?;
        view.start(&ctx, true);
        wait(&mut view);
        view.unstage(&ctx);
        wait(&mut view);
        assert!(view.error.is_none());
        assert!(view.rows.values().all(|e| e.area == Area::Untracked));
        assert_eq!(std::fs::read(dir.path().join("a"))?, b"working");
        Ok(())
    }
    #[test]
    fn stage_worker_refreshes_and_retains_rows_on_failure() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(["init", "-q"])
                .status()?
                .success()
        );
        std::fs::write(dir.path().join("a"), "working")?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        let wait = |view: &mut StatusView| {
            let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                frame(&ctx, view, 720.0, vec![]);
                if view.mutation.is_none() && !view.replacing && view.complete {
                    break;
                }
                assert!(std::time::Instant::now() < until, "status did not settle");
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        };
        wait(&mut view);
        assert_eq!(view.rows[&0].area, Area::Untracked);
        let identity = view.identity.clone();
        std::fs::write(dir.path().join(".git/index.lock"), "owner")?;
        view.stage(&ctx);
        view.stage(&ctx); // A pending action remains single-flight.
        wait(&mut view);
        assert!(view.error.as_ref().is_some_and(|e| e.contains("Stage")));
        assert_eq!(view.identity, identity);
        assert_eq!(view.rows[&0].area, Area::Untracked);
        std::fs::remove_file(dir.path().join(".git/index.lock"))?;
        view.start(&ctx, true);
        wait(&mut view);
        view.stage(&ctx);
        wait(&mut view);
        assert!(view.error.is_none());
        assert_eq!(view.rows[&0].area, Area::Staged);
        assert_eq!(std::fs::read(dir.path().join("a"))?, b"working");
        Ok(())
    }
    #[test]
    fn action_row_offers_stage_unstage_and_view_diff() {
        let ctx = egui::Context::default();
        let mut view = view(1);
        view.identity = Some(update(1, true).identity);
        let output = crate::tests::run_ui(
            &ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(720.0, 480.0),
                )),
                ..Default::default()
            },
            |ctx| {
                view.draw_ui(ctx);
            },
        );
        for label in ["Stage file", "Unstage file", "View diff"] {
            assert!(
                output.shapes.iter().any(|shape| matches!(
                    &shape.shape,
                    egui::epaint::Shape::Text(text) if text.galley.job.text == label
                )),
                "missing {label}"
            );
        }
    }
    fn run(
        ctx: &egui::Context,
        view: &mut StatusView,
        width: f32,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        crate::tests::run_ui(
            ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 480.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                view.draw_ui(ctx);
            },
        )
    }
    fn text_pos(output: &egui::FullOutput, needle: &str) -> Option<egui::Pos2> {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text)
                if text.galley.job.text == needle
                    || text.galley.job.text.starts_with(needle) && needle.starts_with("@@") =>
            {
                Some(text.pos)
            }
            _ => None,
        })
    }
    fn click(ctx: &egui::Context, view: &mut StatusView, width: f32, pos: egui::Pos2) {
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
        run(
            ctx,
            view,
            width,
            vec![egui::Event::PointerMoved(pos), press],
        );
        run(ctx, view, width, vec![release]);
    }
    fn git(path: &std::path::Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.org")
            .env("GIT_COMMITTER_EMAIL", "test@example.org")
            .output()?;
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        Ok(())
    }
    fn file(path: &std::path::Path, args: &[&str]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()?;
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        Ok(out.stdout)
    }
    fn pump(
        ctx: &egui::Context,
        view: &mut StatusView,
        width: f32,
        mut done: impl FnMut(&StatusView) -> bool,
    ) {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            frame(ctx, view, width, vec![]);
            if done(view) {
                return;
            }
            assert!(std::time::Instant::now() < until, "working view timed out");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    fn settle(ctx: &egui::Context, view: &mut StatusView, width: f32) {
        pump(ctx, view, width, |view| {
            view.complete && !view.replacing && view.mutation.is_none()
        });
    }
    fn ready(ctx: &egui::Context, view: &mut StatusView, width: f32) {
        pump(ctx, view, width, |view| {
            view.stage.ready() && view.stage.pane_ready(0) && view.stage.pane_ready(1)
        });
    }
    fn generated(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        git(path, &["init", "-q"])?;
        git(path, &["config", "core.autocrlf", "false"])?;
        std::fs::write(
            path.join("a"),
            b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n",
        )?;
        git(path, &["add", "a"])?;
        git(path, &["commit", "-qm", "base"])?;
        std::fs::write(
            path.join("a"),
            b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n",
        )?;
        Ok(())
    }
    fn select_area(view: &mut StatusView, area: Area) {
        view.selected = view
            .rows
            .iter()
            .find(|(_, entry)| entry.area == area)
            .map(|(row, _)| *row);
    }
    fn select_path(view: &mut StatusView, path: &str, area: Area) {
        view.selected = view
            .rows
            .iter()
            .find(|(_, entry)| entry.path == path && entry.area == area)
            .map(|(row, _)| *row);
    }

    #[test]
    fn combined_view_shows_both_panes_and_adopts_under_hover()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        git(dir.path(), &["add", "a"])?;
        std::fs::write(
            dir.path().join("a"),
            b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE!\n",
        )?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_area(&mut view, Area::Staged);
        assert!(view.selected.is_some());
        view.open_diff(&ctx);
        // A pointer resting over the split view must not delay the first result.
        let hovered = |ctx: &egui::Context, view: &mut StatusView| {
            let _ = run(
                ctx,
                view,
                1120.0,
                vec![egui::Event::PointerMoved(egui::pos2(560.0, 250.0))],
            );
        };
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            hovered(&ctx, &mut view);
            if view.stage.ready() && view.stage.pane_ready(0) && view.stage.pane_ready(1) {
                break;
            }
            assert!(std::time::Instant::now() < until, "hovered diff timed out");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        for width in [1120.0, 720.0] {
            let output = run(&ctx, &mut view, width, vec![]);
            for label in ["Staged", "Unstaged", "Unstage hunk", "Stage hunk"] {
                assert!(
                    text_pos(&output, label).is_some(),
                    "missing {label} at {width}"
                );
            }
        }
        assert!(view.reading);
        Ok(())
    }

    #[test]
    fn inline_hunk_button_moves_the_hunk_and_keeps_the_view()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let working = std::fs::read(dir.path().join("a"))?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_area(&mut view, Area::Unstaged);
        view.open_diff(&ctx);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let button = text_pos(&output, "Stage hunk").ok_or("missing stage hunk")?;
        click(&ctx, &mut view, 1120.0, button + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.pending() && view.stage.ready() && view.stage.pane_has_text(0, "ONE")
        });
        assert!(view.reading, "staging must not leave the combined view");
        assert_eq!(
            file(dir.path(), &["show", ":a"])?,
            b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n"
        );
        assert!(view.stage.pane_has_text(1, "TWELVE"));
        assert!(!view.stage.pane_has_text(1, "ONE"));
        assert_eq!(std::fs::read(dir.path().join("a"))?, working);

        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&ctx, &mut view, 1120.0, vec![key(egui::Key::Escape)]);
        assert!(!view.reading);
        settle(&ctx, &mut view, 1120.0);
        assert_eq!(view.rows.len(), 2);
        assert!(view.rows.values().any(|entry| entry.area == Area::Staged));
        assert!(view.rows.values().any(|entry| entry.area == Area::Unstaged));
        Ok(())
    }

    #[test]
    fn inline_selected_line_stages_only_that_line_and_stays()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let working = std::fs::read(dir.path().join("a"))?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_area(&mut view, Area::Unstaged);
        view.open_diff(&ctx);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let line = text_pos(&output, "TWELVE").ok_or("missing added line")?;
        click(&ctx, &mut view, 1120.0, line + egui::vec2(5.0, 0.0));
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let button = text_pos(&output, "Stage 1").ok_or("missing selected-line action")?;
        click(&ctx, &mut view, 1120.0, button + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.pending() && view.stage.ready() && view.stage.pane_has_text(0, "TWELVE")
        });
        assert!(view.reading);
        assert_eq!(
            file(dir.path(), &["show", ":a"])?,
            b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\nTWELVE\n"
        );
        assert!(view.stage.pane_has_text(1, "twelve"));
        assert_eq!(std::fs::read(dir.path().join("a"))?, working);
        Ok(())
    }

    #[test]
    fn inline_unstage_restores_head_and_keeps_the_view() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        git(dir.path(), &["add", "a"])?;
        std::fs::write(
            dir.path().join("a"),
            b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE!\n",
        )?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_area(&mut view, Area::Staged);
        view.open_diff(&ctx);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let button = text_pos(&output, "Unstage hunk").ok_or("missing unstage hunk")?;
        click(&ctx, &mut view, 1120.0, button + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.pending() && view.stage.ready()
        });
        assert!(view.reading);
        assert_eq!(
            file(dir.path(), &["show", ":a"])?,
            b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n"
        );
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let button = text_pos(&output, "Unstage file").ok_or("missing unstage all")?;
        click(&ctx, &mut view, 1120.0, button + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.pending() && view.stage.ready() && !view.stage.pane_has_text(0, "TWELVE")
        });
        assert!(view.reading);
        assert_eq!(
            file(dir.path(), &["show", ":a"])?,
            b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n"
        );
        Ok(())
    }

    #[test]
    fn commit_panel_is_visible_at_both_widths() -> Result<(), Box<dyn std::error::Error>> {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut view = view(2);
            view.message = "A commit".into();
            let output = run(&ctx, &mut view, width, vec![]);
            assert!(
                text_pos(&output, "Commit").is_some(),
                "missing commit at {width}"
            );
        }
        Ok(())
    }

    #[test]
    fn commit_readiness_explains_disabled_states() {
        assert_eq!(
            commit_reason(false, false, "subject", false, 0),
            Some("Stage at least one file")
        );
        assert_eq!(
            commit_reason(false, false, "   ", false, 1),
            Some("Enter a commit message")
        );
        assert_eq!(
            commit_reason(false, true, "subject", true, 1),
            Some("Waiting for the status scan")
        );
        assert_eq!(
            commit_reason(true, false, "subject", false, 1),
            Some("Wait for the current operation")
        );
        assert_eq!(commit_reason(false, false, "subject", true, 0), None);
        assert_eq!(commit_reason(false, false, "subject", false, 1), None);
    }

    #[test]
    fn commit_editor_expands_and_collapses() {
        let ctx = egui::Context::default();
        let mut view = view(2);
        view.identity = Some(gg_engine::status::StatusIdentity {
            path: "/repository".into(),
            head: Some("a".repeat(40)),
            index: "index".into(),
            base_tree: None,
        });
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let expand = text_pos(&output, "Expand");
        assert!(expand.is_some(), "missing expand button");
        click(
            &ctx,
            &mut view,
            1120.0,
            expand.unwrap_or_default() + egui::vec2(5.0, 0.0),
        );
        assert!(view.message_expanded);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let collapse = text_pos(&output, "Collapse");
        assert!(collapse.is_some(), "missing collapse button");
        click(
            &ctx,
            &mut view,
            1120.0,
            collapse.unwrap_or_default() + egui::vec2(5.0, 0.0),
        );
        assert!(!view.message_expanded);
    }

    #[test]
    fn amend_panel_loads_head_and_rewrites_the_commit() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        git(dir.path(), &["config", "user.name", "Test"])?;
        git(dir.path(), &["config", "user.email", "test@example.org"])?;
        git(dir.path(), &["add", "a"])?;
        let previous = file(dir.path(), &["rev-parse", "HEAD"])?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        view.start_amend(&ctx)?;
        settle(&ctx, &mut view, 1120.0);
        assert_eq!(view.message, "base");
        assert!(view.amend);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.job.text.contains("Amending")
                    && text.galley.job.text.contains("base")
        )));
        let amend = text_pos(&output, "Amend commit").ok_or("missing amend button")?;
        click(&ctx, &mut view, 1120.0, amend + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing && view.commit_done.is_some()
        });
        assert!(view.take_commit().is_some());
        assert!(!view.amend);
        assert_eq!(
            file(dir.path(), &["show", "-s", "--format=%s", "HEAD"])?,
            b"base\n"
        );
        assert_eq!(
            file(dir.path(), &["show", "HEAD:a"])?,
            b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n"
        );
        assert_eq!(file(dir.path(), &["rev-list", "--count", "HEAD"])?, b"1\n");
        assert_ne!(file(dir.path(), &["rev-parse", "HEAD"])?, previous);
        assert!(
            std::fs::read_dir(dir.path().join(".git/refs/gg/backup/amend"))?
                .next()
                .is_some()
        );
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        assert!(
            output.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::epaint::Shape::Text(text)
                    if text.galley.job.text.starts_with("Amended ")
            )),
            "missing amended status line"
        );
        Ok(())
    }

    #[test]
    fn normal_and_amend_drafts_stay_separate() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        view.message = "normal draft".into();
        view.start_amend(&ctx)?;
        assert!(view.amend);
        assert_eq!(view.normal_draft, "normal draft");
        assert_eq!(view.message, "base");
        view.message = "amend draft".into();

        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let checkbox = text_pos(&output, "Amend").ok_or("missing amend checkbox")?;
        click(&ctx, &mut view, 1120.0, checkbox + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing && !view.amend
        });
        assert_eq!(view.message, "normal draft");
        assert_eq!(view.amend_draft, "amend draft");

        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let checkbox = text_pos(&output, "Amend").ok_or("missing amend checkbox")?;
        click(&ctx, &mut view, 1120.0, checkbox + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing && view.amend
        });
        assert_eq!(view.message, "amend draft");
        Ok(())
    }

    #[test]
    fn draft_restores_the_selected_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let ctx = egui::Context::default();
        let mut first = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut first, 1120.0);
        select_path(&mut first, "a", Area::Unstaged);
        let draft = first.capture_draft();
        let mut second = StatusView::new(dir.path().into(), &ctx);
        second.restore_draft(draft, &ctx);
        settle(&ctx, &mut second, 1120.0);
        assert_eq!(
            second
                .selected
                .and_then(|row| second.rows.get(&row))
                .map(|entry| entry.path.as_str()),
            Some("a")
        );
        Ok(())
    }

    #[test]
    fn amend_view_shows_commit_contents_and_restages_through_unstage_and_stage()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let p = dir.path();
        git(p, &["init", "-q"])?;
        git(p, &["config", "user.name", "Test"])?;
        git(p, &["config", "user.email", "test@example.org"])?;
        std::fs::write(p.join("a"), b"base\n")?;
        git(p, &["add", "a"])?;
        git(p, &["commit", "-qm", "base"])?;
        std::fs::write(p.join("a"), b"commit\n")?;
        git(p, &["commit", "-qam", "second"])?;
        std::fs::write(p.join("a"), b"working\n")?;

        let ctx = egui::Context::default();
        let mut view = StatusView::new(p.into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        view.start_amend(&ctx)?;
        settle(&ctx, &mut view, 1120.0);
        assert_eq!(view.message, "second");
        assert!(
            view.rows
                .values()
                .any(|entry| entry.path == "a" && entry.area == Area::Staged),
            "the amended commit's contents are missing"
        );
        assert!(
            view.rows
                .values()
                .any(|entry| entry.path == "a" && entry.area == Area::Unstaged)
        );

        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let checkbox = text_pos(&output, "Amend").ok_or("missing amend checkbox")?;
        click(&ctx, &mut view, 1120.0, checkbox + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing
        });
        assert!(!view.amend);
        assert!(view.amend_base.is_none());
        assert!(
            !view
                .rows
                .values()
                .any(|entry| entry.path == "a" && entry.area == Area::Staged)
        );
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let checkbox = text_pos(&output, "Amend").ok_or("missing amend checkbox")?;
        click(&ctx, &mut view, 1120.0, checkbox + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing
        });
        assert!(view.amend);
        assert!(view.amend_base.is_some());
        assert!(
            view.rows
                .values()
                .any(|entry| entry.path == "a" && entry.area == Area::Staged)
        );

        select_path(&mut view, "a", Area::Staged);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let unstage = text_pos(&output, "Unstage file").ok_or("missing unstage file")?;
        click(&ctx, &mut view, 1120.0, unstage + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing
        });
        assert_eq!(file(p, &["show", ":a"])?, b"base\n");
        assert!(
            !view
                .rows
                .values()
                .any(|entry| entry.path == "a" && entry.area == Area::Staged)
        );
        assert!(
            view.rows
                .values()
                .any(|entry| entry.path == "a" && entry.area == Area::Unstaged)
        );

        select_path(&mut view, "a", Area::Unstaged);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let stage = text_pos(&output, "Stage file").ok_or("missing stage file")?;
        click(&ctx, &mut view, 1120.0, stage + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing
        });
        assert_eq!(file(p, &["show", ":a"])?, b"working\n");

        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let amend = text_pos(&output, "Amend commit").ok_or("missing amend button")?;
        click(&ctx, &mut view, 1120.0, amend + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing && view.commit_done.is_some()
        });
        assert!(view.take_commit().is_some());
        assert!(!view.amend);
        assert!(view.amend_base.is_none());
        assert_eq!(file(p, &["show", "HEAD:a"])?, b"working\n");
        assert_eq!(file(p, &["rev-list", "--count", "HEAD"])?, b"2\n");
        Ok(())
    }

    #[test]
    fn commit_panel_commits_the_index_and_reports_hook_failures()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        git(dir.path(), &["config", "user.name", "Test"])?;
        git(dir.path(), &["config", "user.email", "test@example.org"])?;
        git(dir.path(), &["add", "a"])?;
        let index = std::fs::read(dir.path().join(".git/index"))?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        view.message = "UI commit".into();
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let commit = text_pos(&output, "Commit").ok_or("missing commit button")?;
        click(&ctx, &mut view, 1120.0, commit + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing && view.commit_done.is_some()
        });
        assert!(view.take_commit().is_some());
        assert_eq!(
            file(dir.path(), &["show", "-s", "--format=%s", "HEAD"])?,
            b"UI commit\n"
        );
        assert!(scan_is_clean(dir.path())?);
        assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);

        std::fs::write(dir.path().join("a"), b"next\n")?;
        git(dir.path(), &["add", "a"])?;
        view.start(&ctx, true);
        settle(&ctx, &mut view, 1120.0);
        let blocked = std::fs::read(dir.path().join(".git/index"))?;
        let road = dir.path().join(".git/hooks/pre-commit");
        std::fs::create_dir_all(road.parent().ok_or("missing hooks dir")?)?;
        std::fs::write(&road, "#!/bin/sh\necho blocked >&2\nexit 4\n")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&road, std::fs::Permissions::from_mode(0o755))?;
        }
        view.message = "should fail".into();
        view.start_commit(&ctx);
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing && view.error.is_some()
        });
        assert!(
            view.error
                .as_ref()
                .is_some_and(|e| e.contains("pre-commit"))
        );
        assert!(view.error.as_ref().is_some_and(|e| e.contains("preserved")));
        assert_eq!(view.message, "should fail");
        assert_eq!(std::fs::read(dir.path().join(".git/index"))?, blocked);
        assert_eq!(file(dir.path(), &["rev-list", "--count", "HEAD"])?, b"2\n");
        Ok(())
    }

    fn scan_is_clean(path: &std::path::Path) -> Result<bool, Box<dyn std::error::Error>> {
        let session = gg_git::status::Session::open(path)?;
        let mut clean = true;
        session.scan(&std::sync::atomic::AtomicBool::new(false), |_| {
            clean = false;
            Ok(())
        })?;
        Ok(clean)
    }

    #[test]
    fn selected_line_drop_reverts_the_whole_change_block() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let index = std::fs::read(dir.path().join(".git/index"))?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_area(&mut view, Area::Unstaged);
        view.open_diff(&ctx);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let line = text_pos(&output, "TWELVE").ok_or("missing selected line")?;
        click(&ctx, &mut view, 1120.0, line + egui::vec2(5.0, 0.0));
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let drop = text_pos(&output, "Discard selected").ok_or("missing selected drop")?;
        click(&ctx, &mut view, 1120.0, drop + egui::vec2(5.0, 0.0));
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let discard = text_pos(&output, "Discard").ok_or("missing discard")?;
        click(&ctx, &mut view, 1120.0, discard + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.pending() && view.stage.ready()
        });
        assert!(view.reading);
        assert_eq!(
            std::fs::read(dir.path().join("a"))?,
            b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n"
        );
        assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);
        Ok(())
    }

    #[test]
    fn drop_requires_confirmation_and_leaves_the_index_alone()
    -> Result<(), Box<dyn std::error::Error>> {
        for width in [720.0, 1120.0] {
            let dir = tempfile::tempdir()?;
            generated(dir.path())?;
            let index = std::fs::read(dir.path().join(".git/index"))?;
            let working = std::fs::read(dir.path().join("a"))?;
            let ctx = egui::Context::default();
            let mut view = StatusView::new(dir.path().into(), &ctx);
            settle(&ctx, &mut view, width);
            select_area(&mut view, Area::Unstaged);
            view.open_diff(&ctx);
            ready(&ctx, &mut view, width);

            let output = run(&ctx, &mut view, width, vec![]);
            let drop = text_pos(&output, "Discard hunk").ok_or("missing drop hunk")?;
            click(&ctx, &mut view, width, drop + egui::vec2(5.0, 0.0));
            let output = run(&ctx, &mut view, width, vec![]);
            let cancel = text_pos(&output, "Cancel").ok_or("missing drop cancel")?;
            click(&ctx, &mut view, width, cancel + egui::vec2(5.0, 0.0));
            assert_eq!(std::fs::read(dir.path().join("a"))?, working);
            assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);

            let output = run(&ctx, &mut view, width, vec![]);
            let drop = text_pos(&output, "Discard hunk").ok_or("missing drop hunk")?;
            click(&ctx, &mut view, width, drop + egui::vec2(5.0, 0.0));
            let output = run(&ctx, &mut view, width, vec![]);
            let discard = text_pos(&output, "Discard").ok_or("missing discard")?;
            click(&ctx, &mut view, width, discard + egui::vec2(5.0, 0.0));
            pump(&ctx, &mut view, width, |view| {
                !view.stage.pending()
                    && view.stage.ready()
                    && view.stage.pane_has_text(1, "TWELVE")
                    && !view.stage.pane_has_text(1, "ONE")
            });
            assert!(view.reading);
            assert_eq!(
                std::fs::read(dir.path().join("a"))?,
                b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n"
            );
            assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);

            let output = run(&ctx, &mut view, width, vec![]);
            let drop = text_pos(&output, "Discard file").ok_or("missing drop all")?;
            click(&ctx, &mut view, width, drop + egui::vec2(5.0, 0.0));
            let output = run(&ctx, &mut view, width, vec![]);
            let discard = text_pos(&output, "Discard").ok_or("missing discard all")?;
            click(&ctx, &mut view, width, discard + egui::vec2(5.0, 0.0));
            pump(&ctx, &mut view, width, |view| {
                !view.stage.pending() && view.stage.ready()
            });
            assert!(view.reading);
            assert_eq!(
                std::fs::read(dir.path().join("a"))?,
                b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n"
            );
            assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);
            frame(
                &ctx,
                &mut view,
                width,
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert!(!view.reading);
            settle(&ctx, &mut view, width);
            assert!(view.rows.is_empty());
        }
        Ok(())
    }

    #[test]
    fn inline_action_failure_keeps_the_view_and_index() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_area(&mut view, Area::Unstaged);
        view.open_diff(&ctx);
        ready(&ctx, &mut view, 1120.0);
        let index = std::fs::read(dir.path().join(".git/index"))?;
        std::fs::write(dir.path().join(".git/index.lock"), "owner")?;
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let button = text_pos(&output, "Stage hunk").ok_or("missing stage hunk")?;
        click(&ctx, &mut view, 1120.0, button + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| !view.stage.pending());
        assert!(view.reading);
        assert!(
            view.stage
                .error()
                .is_some_and(|error| error.contains("Stage"))
        );
        assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);
        std::fs::remove_file(dir.path().join(".git/index.lock"))?;
        Ok(())
    }
}
