//! A review context keeps history input and viewport state untouched.
use crate::{
    bounded_text,
    review_worker::{Update, Worker},
};
use eframe::egui;
use gg_engine::{file::ReadingSide, review::*};
use std::{collections::BTreeMap, ops::Range};
#[cfg(test)]
const ROW: f32 = crate::density::COMPACT_ROW;
/// Fixed slot for the "N files" count in the header.
const FILE_COUNT_WIDTH: f32 = 72.0;

/// Run `add` in a scope that keeps its space but draws nothing when hidden.
fn visible_scope<R>(
    ui: &mut egui::Ui,
    visible: bool,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.scope(|ui| {
        if !visible {
            ui.set_invisible();
        }
        add(ui)
    })
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn review_source(reference: Option<&str>, id: Option<&str>, absent: &str) -> String {
    match (reference, id) {
        (Some(reference), Some(id)) => format!("{reference} ({})", short_id(id)),
        // The commit arrives with the comparison; until then the name alone.
        (Some(reference), None) => reference.to_string(),
        (None, Some(id)) => short_id(id),
        (None, None) => absent.to_string(),
    }
}

pub struct Review {
    request: ReviewRequest,
    subject: String,
    worker: Option<Worker>,
    staged: Option<Update>,
    pub comparison: Option<Comparison>,
    pub rows: BTreeMap<usize, FileChange>,
    pub count: usize,
    pub selected: Option<usize>,
    complete: bool,
    error: Option<String>,
    viewport: Range<usize>,
    offset: f32,
    page: usize,
    rect: egui::Rect,
    row_rects: Vec<(usize, egui::Rect)>,
    last_scroll: f64,
    diff: crate::diff_view::DiffView,
    show_diff: bool,
    restore_files: bool,
    replacing: bool,
    history: Option<crate::file_history_view::FileHistory>,
    open_history: bool,
    return_location: Option<crate::return_location::ReturnLocation>,
    saved_reader: Option<crate::reader::ReaderContext>,
    history_entry_row: Option<usize>,
    reveal_selection: bool,
    /// Drawn inside another surface: no Back button, identity, or status
    /// line, and the reader sits below the file list.
    embedded: bool,
    /// File to select and read once its row arrives.
    pending_path: Option<String>,
}
impl Review {
    pub fn new(request: ReviewRequest, subject: String, ctx: &egui::Context) -> Self {
        let mut view = Self {
            request,
            subject,
            worker: None,
            staged: None,
            comparison: None,
            rows: BTreeMap::new(),
            count: 0,
            selected: None,
            complete: false,
            error: None,
            viewport: 0..64,
            offset: 0.0,
            page: 1,
            rect: egui::Rect::NOTHING,
            row_rects: vec![],
            last_scroll: f64::NEG_INFINITY,
            diff: crate::diff_view::DiffView::default(),
            show_diff: false,
            restore_files: false,
            replacing: false,
            history: None,
            open_history: false,
            return_location: None,
            saved_reader: None,
            history_entry_row: None,
            reveal_selection: false,
            embedded: false,
            pending_path: None,
        };
        view.retry(ctx);
        view
    }

    /// Show the review inside another surface, such as the interactive rebase
    /// dialog: files above the reader, no Back button, identity, or status line.
    pub fn embedded(mut self) -> Self {
        self.embedded = true;
        self
    }

    /// Open on `path`: select it and show its reader once its row arrives.
    pub fn select_path(mut self, path: String) -> Self {
        self.pending_path = Some(path);
        self
    }

    /// One-line commit identity: short ID and subject.
    fn draw_identity(&self, ui: &mut egui::Ui) {
        let id = self
            .comparison
            .as_ref()
            .map(|c| c.commit.as_str())
            .unwrap_or(self.request.revision.as_str());
        ui.label(egui::RichText::new(short_id(id)).monospace().weak())
            .on_hover_text(id);
        ui.add(egui::Label::new(egui::RichText::new(&self.subject).strong()).truncate())
            .on_hover_text(&self.subject);
    }

    /// Record the exact caller state before a tool opens this review.
    pub fn set_return_location(&mut self, location: crate::return_location::ReturnLocation) {
        self.return_location = Some(location);
    }

    pub fn take_return_location(&mut self) -> Option<crate::return_location::ReturnLocation> {
        self.return_location.take()
    }

    /// Caller state to preserve when this review hands off to another commit.
    pub fn return_location(&self) -> Option<&crate::return_location::ReturnLocation> {
        self.return_location.as_ref()
    }

    /// Take a commit request raised by the attribution strip.
    pub fn take_open_commit(&mut self) -> Option<(String, String)> {
        self.diff.take_open_commit()
    }

    /// Take a submodule-details request raised by the reader.
    pub fn take_submodule_request(&mut self) -> Option<crate::diff_view::SubmoduleRequest> {
        self.diff.take_submodule_request()
    }

    /// Take an LFS-details request raised by the reader.
    pub fn take_lfs_request(&mut self) -> Option<crate::diff_view::LfsRequest> {
        self.diff.take_lfs_request()
    }

    /// Exact endpoint labels shared by the identity strip and the reader.
    fn reader_endpoints(&self) -> (String, String) {
        let after_id = self
            .comparison
            .as_ref()
            .map(|c| c.commit.as_str())
            .unwrap_or(self.request.revision.as_str());
        let parent = self.comparison.as_ref().and_then(|c| c.parent.as_deref());
        if let Some(against) = self.request.against.as_deref() {
            (
                review_source(Some(against), parent, ""),
                review_source(None, Some(after_id), ""),
            )
        } else {
            (
                review_source(None, parent, "empty tree"),
                review_source(None, Some(after_id), ""),
            )
        }
    }

    fn open_history_now(&mut self, ctx: &egui::Context) {
        self.open_history_now_with_lines(ctx, None, 0);
    }

    fn open_history_now_with_lines(
        &mut self,
        ctx: &egui::Context,
        lines: Option<(usize, usize)>,
        start_index: usize,
    ) {
        if self.history.is_some() {
            return;
        }
        let Some(change) = self.selected.and_then(|row| self.rows.get(&row)) else {
            return;
        };
        let entry_revision = self
            .comparison
            .as_ref()
            .map(|comparison| comparison.commit.clone());
        self.history_entry_row = self.selected;
        self.saved_reader = self.diff.save_context().map(|context| {
            if let Some(revision) = &entry_revision {
                context.with_revision(revision.clone())
            } else {
                context
            }
        });
        let revision = entry_revision.unwrap_or_else(|| self.request.revision.clone());
        self.history = Some(crate::file_history_view::FileHistory::new(
            self.request.path.clone(),
            revision,
            change.path.clone(),
            change.path_bytes.clone(),
            lines,
            start_index,
            ctx,
        ));
    }

    /// Close the history source and restore the captured entry context once.
    fn close_history(&mut self, ctx: &egui::Context) {
        self.history = None;
        let entry_row = self.history_entry_row.take();
        if let Some(context) = self.saved_reader.take() {
            let row = entry_row.or_else(|| {
                self.rows
                    .iter()
                    .find(|(_, change)| change.path_bytes == context.change.path_bytes)
                    .map(|(row, _)| *row)
            });
            if let Some(row) = row {
                self.selected = Some(row);
                self.reveal_selection = true;
            }
            let same_revision = context.revision.as_ref().is_none_or(|revision| {
                self.comparison
                    .as_ref()
                    .is_none_or(|comparison| &comparison.commit == revision)
            });
            if same_revision {
                self.diff.open_context(&context, ctx);
            }
        }
    }

    /// Leave history because another file was selected. The previous path's
    /// version cursor is never reused and the entry reader is not restored.
    fn abandon_history(&mut self) {
        self.history = None;
        self.saved_reader = None;
        self.history_entry_row = None;
    }

    /// Reset the version cursor when the file-list selection moves to another
    /// path. The original entry context stays saved for Close history.
    fn reset_history_for_selection(&mut self, ctx: &egui::Context) {
        let Some(change) = self.selected.and_then(|row| self.rows.get(&row)) else {
            return;
        };
        let revision = self
            .comparison
            .as_ref()
            .map(|comparison| comparison.commit.clone())
            .unwrap_or_else(|| self.request.revision.clone());
        self.history = Some(crate::file_history_view::FileHistory::new(
            self.request.path.clone(),
            revision,
            change.path.clone(),
            change.path_bytes.clone(),
            None,
            0,
            ctx,
        ));
    }

    /// Keep history active for the newly selected file and reset its cursor.
    fn reconcile_history_selection(&mut self, ctx: &egui::Context) -> bool {
        let Some(history) = &self.history else {
            return false;
        };
        let selected = self.selected.and_then(|row| self.rows.get(&row));
        if selected.is_some_and(|change| change.path_bytes == history.path_bytes()) {
            return false;
        }
        self.reset_history_for_selection(ctx);
        true
    }
    fn retry(&mut self, ctx: &egui::Context) {
        self.worker = None;
        self.staged = None;
        self.error = None;
        self.replacing = self.comparison.is_some();
        match Worker::start(self.request.clone(), ctx.clone()) {
            Ok(worker) => {
                worker.request(if self.replacing {
                    0..64
                } else {
                    self.viewport.clone()
                });
                self.worker = Some(worker);
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn switch_parent(&mut self, parent: usize, ctx: &egui::Context) {
        let Some(comparison) = &self.comparison else {
            return;
        };
        if parent == 0 || parent > comparison.parents.len() {
            return;
        }
        let path = comparison.path.clone();
        let commit = comparison.commit.clone();
        self.abandon_history();
        self.request.path = path;
        self.request.revision = commit;
        self.request.parent = Some(parent);
        self.retry(ctx);
    }
    fn adopt_replacement(&mut self) {
        let same_comparison = self
            .comparison
            .as_ref()
            .zip(self.staged.as_ref())
            .is_some_and(|(current, update)| {
                current.path == update.comparison.path
                    && current.commit == update.comparison.commit
                    && current.parent == update.comparison.parent
            });
        self.replacing = false;
        // Immutable comparisons have the same paths and order on retry. Keep
        // their displayed rows and anchors while the new spool catches up.
        if same_comparison {
            return;
        }
        self.rows.clear();
        self.selected = None;
        self.count = 0;
        self.offset = 0.0;
        self.viewport = 0..64;
        self.restore_files = true;
        self.diff = crate::diff_view::DiffView::default();
    }
    /// Draw the history strip above whichever reader path is active.
    fn draw_reader(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, reading: bool) {
        let mut message = None;
        let mut close = false;
        if let Some(history) = &mut self.history {
            history.draw_strip(ui, ctx);
            close = history.take_close();
            if !close {
                ui.separator();
                message = history.prepare(&mut self.diff, ctx);
            }
        }
        if close {
            self.close_history(ctx);
            return;
        }
        if let Some(message) = message {
            bounded_text::field(ui, message, egui::FontId::proportional(12.0), 22.0);
            return;
        }
        if reading || self.history.is_some() {
            self.diff.draw_reading(ui);
        } else {
            self.diff.draw(ui);
        }
        if let Some(lines) = self.diff.take_line_history() {
            let start_index = if self.diff.place().side == ReadingSide::Before {
                1
            } else {
                0
            };
            self.open_history_now_with_lines(ctx, Some(lines), start_index);
        }
    }
    /// Return true when Back is explicitly requested.
    fn back_label(&self) -> &'static str {
        if self.show_diff {
            "Back to files"
        } else if matches!(
            self.return_location,
            Some(crate::return_location::ReturnLocation::Recovery { .. })
        ) {
            "Back to recovery"
        } else if matches!(
            self.return_location,
            Some(crate::return_location::ReturnLocation::Rebase)
        ) {
            "Back to rebase"
        } else {
            "Back to history"
        }
    }

    pub(crate) fn draw_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        let narrow = !self.embedded && ctx.content_rect().width() < 1040.0;
        if self.open_history {
            self.open_history = false;
            self.open_history_now(ctx);
        }
        let mut history_closed = false;
        let export_open = self.diff.export_open();
        if let Some(history) = &mut self.history {
            history.poll();
            if !export_open && history.keys(ctx) {
                history_closed = true;
            }
        }
        if history_closed {
            self.close_history(ctx);
        }
        self.reconcile_history_selection(ctx);
        let (attribution_after, attribution_before) = if let Some(history) = &self.history {
            (history.current_revision(), history.predecessor_revision())
        } else if let Some(comparison) = &self.comparison {
            (Some(comparison.commit.clone()), comparison.parent.clone())
        } else {
            (None, None)
        };
        self.diff
            .set_attribution_revisions(attribution_after, attribution_before);
        if !export_open
            && crate::navigation::available(ctx)
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::B))
        {
            self.diff.toggle_attribution(ctx);
        }
        if self.history.is_none() {
            let (before, after) = self.reader_endpoints();
            self.diff.set_endpoints(before, after);
        }
        let history_available = self.selected.is_some() && (!narrow || self.show_diff);
        if self.history.is_none()
            && history_available
            && !export_open
            && crate::navigation::activation_available(ctx)
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::H))
        {
            self.open_history = true;
            ctx.request_repaint();
        }
        let (point, active, now) = ctx.input(|i| {
            (
                i.pointer.hover_pos(),
                i.pointer.any_down() || i.smooth_scroll_delta != egui::Vec2::ZERO,
                i.time,
            )
        });
        if active {
            self.last_scroll = now;
        }
        let quiet = !active && now - self.last_scroll > 0.2;
        if !quiet {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        let pinned = point.and_then(|p| {
            self.row_rects
                .iter()
                .find(|(_, r)| r.contains(p))
                .map(|(row, _)| *row)
        });
        let scrollbar =
            point.is_some_and(|p| self.rect.contains(p) && p.x >= self.rect.right() - 18.0);
        if let Some(result) = self.worker.as_ref().and_then(Worker::poll) {
            match result {
                Ok(update) => self.staged = Some(update),
                Err(error) => self.error = Some(error),
            }
        }
        let replacement_safe =
            quiet && point.is_none_or(|p| !self.rect.contains(p)) && !self.diff.hovered(point);
        if self.replacing && self.staged.is_some() && replacement_safe {
            self.adopt_replacement();
        }
        if !self.replacing
            && let Some(update) = &self.staged
        {
            for (row, change) in &update.rows {
                if pinned != Some(*row) || self.rows.contains_key(row) {
                    self.rows.entry(*row).or_insert_with(|| change.clone());
                }
            }
            if quiet && !scrollbar {
                self.count = if update.complete {
                    update.count
                } else {
                    self.count.max(update.count)
                };
                self.complete = update.complete;
                self.error = update.error.clone();
                self.subject = update.comparison.subject.clone();
                self.request.revision = update.comparison.commit.clone();
                self.request.path = update.comparison.path.clone();
                self.comparison = Some(update.comparison.clone());
                if self.selected.is_none() && self.count > 0 && pinned.is_none() {
                    self.selected = Some(0);
                }
            }
            if quiet && pinned.is_none() && !scrollbar {
                self.staged = None;
            }
        }
        if let Some(path) = &self.pending_path {
            if let Some(row) = self
                .rows
                .iter()
                .find(|(_, change)| &change.path == path)
                .map(|(row, _)| *row)
            {
                self.selected = Some(row);
                self.show_diff = true;
                self.reveal_selection = true;
                self.pending_path = None;
            } else if self.complete && self.staged.is_none() {
                self.pending_path = None;
            }
        }
        while self.rows.len() > 768 {
            let victim = self
                .rows
                .keys()
                .copied()
                .filter(|row| {
                    !self.viewport.contains(row)
                        && Some(*row) != self.selected
                        && Some(*row) != pinned
                })
                .max_by_key(|row| row.abs_diff(self.viewport.start));
            if let Some(row) = victim {
                self.rows.remove(&row);
            } else {
                break;
            }
        }
        let mut back = false;
        let mut reveal = self.reveal_selection;
        self.reveal_selection = false;
        if crate::navigation::available(ctx) && !self.diff.export_open() {
            if self.history.is_none() {
                back = ctx.input_mut(|i| {
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                        || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
                });
                if back && self.show_diff {
                    self.show_diff = false;
                    self.restore_files = true;
                    back = false;
                }
            }
            if (self.show_diff || self.history.is_some())
                && crate::navigation::activation_available(ctx)
            {
                let mut file_step = 0isize;
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowUp)) {
                    file_step = -1;
                } else if ctx
                    .input_mut(|i| i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowDown))
                {
                    file_step = 1;
                }
                if file_step != 0 && self.count > 0 {
                    self.selected = Some(
                        self.selected
                            .unwrap_or(0)
                            .saturating_add_signed(file_step)
                            .min(self.count - 1),
                    );
                    reveal = true;
                    crate::navigation::focus_content(ctx);
                }
            }
            if self.history.is_none()
                && self.selected.is_some()
                && crate::navigation::activation_available(ctx)
                && ctx.input_mut(|i| {
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                        || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)
                })
            {
                self.show_diff = true;
            }
            let page = self.page.saturating_sub(1).max(1) as isize;
            for (modifiers, key, step) in [
                (egui::Modifiers::NONE, egui::Key::ArrowUp, -1),
                (egui::Modifiers::NONE, egui::Key::K, -1),
                (egui::Modifiers::CTRL, egui::Key::P, -1),
                (egui::Modifiers::NONE, egui::Key::ArrowDown, 1),
                (egui::Modifiers::NONE, egui::Key::J, 1),
                (egui::Modifiers::CTRL, egui::Key::N, 1),
                (egui::Modifiers::NONE, egui::Key::PageUp, -page),
                (egui::Modifiers::ALT, egui::Key::V, -page),
                (egui::Modifiers::NONE, egui::Key::PageDown, page),
            ] {
                if !(self.show_diff)
                    && self.history.is_none()
                    && ctx.input_mut(|i| i.consume_key(modifiers, key))
                    && self.count > 0
                {
                    self.selected = Some(
                        self.selected
                            .unwrap_or(0)
                            .saturating_add_signed(step)
                            .min(self.count - 1),
                    );
                    crate::navigation::focus_content(ctx);
                    reveal = true;
                }
            }
        }
        if self.history.is_none()
            && (!narrow || self.show_diff)
            && (self.selected.is_none()
                || self.selected.is_some_and(|r| self.rows.contains_key(&r)))
        {
            self.diff.select(
                self.request.path.clone(),
                self.selected.and_then(|row| self.rows.get(&row)).cloned(),
                ctx,
            );
        }
        egui::Panel::top("review-header")
            .exact_size(34.0)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    if !self.embedded
                        && self.history.is_none()
                        && ui.button(self.back_label()).clicked()
                    {
                        if self.show_diff {
                            self.show_diff = false;
                            self.restore_files = true;
                        } else {
                            back = true;
                        }
                    }
                    if !self.embedded {
                        ui.separator();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Every control keeps its slot; ones that do not apply are
                        // hidden rather than removed, so nothing shifts when the
                        // comparison loads, fails, or the reading mode changes.
                        let parents = self
                            .comparison
                            .as_ref()
                            .map(|comparison| comparison.parents.clone())
                            .unwrap_or_default();
                        visible_scope(ui, parents.len() > 1, |ui| {
                            let mut parent = self.request.parent.unwrap_or(1);
                            let before = parent;
                            egui::ComboBox::from_id_salt("review-parent")
                                .width(150.0)
                                .selected_text(format!("Parent {parent} of {}", parents.len()))
                                .show_ui(ui, |ui| {
                                    for (index, id) in parents.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut parent,
                                            index + 1,
                                            format!(
                                                "Parent {} · {}",
                                                index + 1,
                                                &id[..id.len().min(8)]
                                            ),
                                        )
                                        .on_hover_text(id);
                                    }
                                });
                            if parent != before {
                                self.switch_parent(parent, ctx);
                            }
                        });
                        let label = format!(
                            "{}{} file{}",
                            self.count,
                            if self.complete { "" } else { "+" },
                            if self.count == 1 && self.complete {
                                ""
                            } else {
                                "s"
                            }
                        );
                        ui.allocate_ui_with_layout(
                            egui::vec2(FILE_COUNT_WIDTH, ui.spacing().interact_size.y),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.set_min_width(FILE_COUNT_WIDTH);
                                ui.weak(label);
                            },
                        );
                        if ui
                            .add_visible(self.error.is_some(), egui::Button::new("Retry"))
                            .clicked()
                        {
                            self.retry(ctx);
                        }
                        let history_shown = self.history.is_none() && (!narrow || self.show_diff);
                        if ui
                            .add_visible(
                                history_shown,
                                egui::Button::new("History").sense(if history_available {
                                    egui::Sense::click()
                                } else {
                                    egui::Sense::hover()
                                }),
                            )
                            .on_hover_text(if history_available {
                                "Follow the selected file's history (H)"
                            } else {
                                "Select a file to follow its history"
                            })
                            .clicked()
                            && history_available
                        {
                            self.open_history = true;
                            ctx.request_repaint();
                        }
                        let next_enabled = self.show_diff
                            && self.count > 0
                            && self.selected.map_or(0, |row| row + 1) < self.count;
                        if ui
                            .scope(|ui| {
                                if !self.show_diff {
                                    ui.set_invisible();
                                }
                                ui.add_enabled(next_enabled, egui::Button::new("Next file"))
                            })
                            .inner
                            .clicked()
                        {
                            self.selected = Some(
                                self.selected
                                    .unwrap_or(0)
                                    .saturating_add(1)
                                    .min(self.count.saturating_sub(1)),
                            );
                            reveal = true;
                        }
                        let prev_enabled =
                            self.show_diff && self.count > 0 && self.selected.unwrap_or(0) > 0;
                        if ui
                            .scope(|ui| {
                                if !self.show_diff {
                                    ui.set_invisible();
                                }
                                ui.add_enabled(prev_enabled, egui::Button::new("Prev file"))
                            })
                            .inner
                            .clicked()
                        {
                            self.selected = Some(self.selected.unwrap_or(0).saturating_sub(1));
                            reveal = true;
                        }
                        if self.history.is_none() && !self.embedded {
                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| self.draw_identity(ui),
                            );
                        }
                    });
                });
            });
        if !self.embedded {
            egui::Panel::bottom("review-status")
                .exact_size(26.0)
                .show(ui, |ui| {
                    let pending = if self.request.against.is_some() {
                        "Loading comparison · showing previous comparison".into()
                    } else {
                        format!(
                            "Loading parent {} · showing previous comparison",
                            self.request.parent.unwrap_or(1)
                        )
                    };
                    let failure = self.error.as_ref().map(|error| {
                        if self.replacing {
                            if self.request.against.is_some() {
                                format!("Comparison failed · showing previous comparison · {error}")
                            } else {
                                format!(
                                    "Parent {} failed · showing previous comparison · {error}",
                                    self.request.parent.unwrap_or(1)
                                )
                            }
                        } else {
                            error.clone()
                        }
                    });
                    let history_state = self
                        .history
                        .as_ref()
                        .map(|history| (history.pending(), history.error().map(str::to_owned)));
                    let text = if let Some((pending, error)) = &history_state {
                        if let Some(error) = error {
                            error.as_str()
                        } else if *pending {
                            "Loading earlier versions…"
                        } else {
                            crate::navigation::HISTORY_HINT
                        }
                    } else {
                        failure.as_deref().unwrap_or(if self.replacing {
                            &pending
                        } else if self.show_diff && self.diff.binary() {
                            "Scroll to inspect · Export either side · Esc: files"
                        } else if self.show_diff {
                            crate::navigation::READING_HINT
                        } else if self.complete {
                            "Up/Down / j k: select · PgUp/PgDn: page · Esc: history"
                        } else {
                            "Reading changed files…"
                        })
                    };
                    bounded_text::field(ui, text, egui::FontId::proportional(12.0), 18.0);
                });
        }
        if self.show_diff {
            egui::CentralPanel::default().show(ui, |ui| self.draw_reader(ui, ctx, true));
            bounded_text::show_disclosure(ctx);
            return back;
        }
        if self.embedded {
            let height = (ui.available_height() * 0.72).max(120.0);
            egui::Panel::bottom("review-file-diff")
                .exact_size(height)
                .show(ui, |ui| self.draw_reader(ui, ctx, false));
        } else if !narrow {
            egui::Panel::right("review-file-diff")
                .default_size(640.0)
                .size_range(400.0..=(ctx.content_rect().width() - 320.0).max(400.0))
                .show(ui, |ui| self.draw_reader(ui, ctx, false));
        }
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(26.0);
                ui.weak("PATH");
            });
            self.rect = ui.available_rect_before_wrap();
            if self.count == 0 {
                // An empty list stays empty until the first files arrive.
                if self.complete {
                    ui.weak("This comparison has no changed files.");
                }
                return;
            }
            let mut scroll = egui::ScrollArea::vertical().id_salt("review-files");
            if self.restore_files {
                scroll = scroll.vertical_scroll_offset(self.offset);
                self.restore_files = false;
            }
            if reveal && let Some(row) = self.selected {
                let height = ui.available_height();
                let top = row as f32 * crate::density::row();
                let offset = if top < self.offset {
                    top
                } else if top + crate::density::row() > self.offset + height {
                    top + crate::density::row() - height
                } else {
                    self.offset
                };
                scroll = scroll.vertical_scroll_offset(offset.clamp(
                    0.0,
                    (self.count as f32 * crate::density::row() - height).max(0.0),
                ));
            }
            ui.spacing_mut().item_spacing.y = 0.0;
            let result = scroll.show_rows(ui, crate::density::row(), self.count, |ui, rows| {
                self.viewport = rows.start.saturating_sub(24)..rows.end.saturating_add(24);
                self.row_rects.clear();
                for row in rows {
                    let change = self.rows.get(&row);
                    let response = ui.add_sized(
                        [ui.available_width(), crate::density::row()],
                        egui::Button::new("")
                            .fill(if self.selected == Some(row) {
                                crate::colors::selection(ui.visuals())
                            } else {
                                egui::Color32::TRANSPARENT
                            })
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(0.0)
                            .selected(self.selected == Some(row))
                            .sense(if change.is_some() {
                                egui::Sense::click()
                            } else {
                                egui::Sense::hover()
                            }),
                    );
                    self.row_rects.push((row, response.rect));
                    let rect = response.rect.shrink2(egui::vec2(6.0, 0.0));
                    let status = egui::Rect::from_min_max(
                        rect.min,
                        egui::pos2(rect.left() + 14.0, rect.bottom()),
                    );
                    let path = egui::Rect::from_min_max(
                        egui::pos2(status.right() + 8.0, rect.top()),
                        rect.max,
                    );
                    if let Some(change) = change {
                        let (label, color) = crate::colors::change(ui.visuals(), &change.kind);
                        let response = response.clone().on_hover_text(label);
                        bounded_text::paint(
                            ui,
                            status,
                            crate::colors::change_symbol(&change.kind),
                            egui::FontId::proportional(13.0),
                            color,
                        );
                        bounded_text::paint(
                            ui,
                            path,
                            &change.path,
                            egui::FontId::proportional(13.0),
                            ui.visuals().strong_text_color(),
                        );
                        if response.double_clicked() {
                            response.surrender_focus();
                            self.selected = Some(row);
                            self.show_diff = true;
                        } else if response.clicked() {
                            response.surrender_focus();
                            self.selected = Some(row);
                        }
                    }
                    // A row not read yet keeps its slot, empty.
                }
            });
            self.offset = result.state.clamped_offset().y;
            self.page = (result.inner_rect.height() / crate::density::row())
                .floor()
                .max(1.0) as usize;
        });
        self.reconcile_history_selection(ctx);
        if self.history.is_none()
            && (!narrow || self.show_diff)
            && (self.selected.is_none()
                || self.selected.is_some_and(|r| self.rows.contains_key(&r)))
        {
            self.diff.select(
                self.request.path.clone(),
                self.selected.and_then(|row| self.rows.get(&row)).cloned(),
                ctx,
            );
        }
        if let Some(worker) = &self.worker {
            worker.request(if self.replacing {
                0..64
            } else {
                self.viewport.clone()
            });
        }
        bounded_text::show_disclosure(ctx);
        back
    }
}

#[cfg(test)]
impl Review {
    pub fn fixture(ctx: &egui::Context) -> Self {
        let mut view = Self::new(
            ReviewRequest {
                path: "/missing-review-fixture".into(),
                revision: "commit".into(),
                parent: None,
                against: None,
            },
            "Review fixture".into(),
            ctx,
        );
        view.worker = None;
        view.staged = Some(Update {
            comparison: Comparison {
                path: "repo".into(),
                commit: "commit".into(),
                parent: None,
                parents: vec![],
                subject: "Review fixture".into(),
            },
            count: 1000,
            complete: true,
            error: None,
            rows: (0..64)
                .map(|row| {
                    let path = format!("src/{row:04}-{}.rs", "long-path-".repeat(20));
                    (
                        row,
                        FileChange {
                            path_bytes: path.as_bytes().to_vec(),
                            path,
                            kind: ChangeKind::Added,
                            old: None,
                            new: Some(FileSide {
                                id: "a".repeat(40),
                                mode: 0o100644,
                            }),
                        },
                    )
                })
                .collect(),
        });
        view
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn key_event(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn frame(
        view: &mut Review,
        ctx: &egui::Context,
        width: f32,
        height: f32,
        events: Vec<egui::Event>,
    ) -> (bool, Vec<String>) {
        let mut back = false;
        let output = crate::tests::run_ui(
            ctx,
            crate::tests::frame_input(width, height, events),
            |ui| {
                back = view.draw_ui(ui);
            },
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect();
        (back, text)
    }

    fn history_timeline(path_bytes: &[u8]) -> gg_engine::file_history::Timeline {
        use gg_engine::{
            file_history::{FileVersion, HistoryKind, Predecessor, Timeline},
            review::FileSide,
        };
        let display = String::from_utf8_lossy(path_bytes).into_owned();
        let side = |id: &str| FileSide {
            id: id.into(),
            mode: 0o100644,
        };
        let version = |id: &str, kind, old_path: Option<&str>, previous| FileVersion {
            path_bytes: path_bytes.to_vec(),
            path: display.clone(),
            side: Some(side(id)),
            commit: id.repeat(40),
            subject: "History version".into(),
            author: "Ada".into(),
            time: 1_767_268_800,
            kind: Some(kind),
            old_path_bytes: old_path.map(|path| path.as_bytes().to_vec()),
            old_path: old_path.map(str::to_owned),
            previous,
        };
        Timeline::new(
            vec![
                version(
                    "e",
                    HistoryKind::Renamed,
                    Some("old.rs"),
                    Predecessor::Linked,
                ),
                version("d", HistoryKind::Added, None, Predecessor::Oldest),
            ],
            true,
            None,
        )
    }

    fn install_history(view: &mut Review) -> Result<crate::reader::ReaderContext, String> {
        let selected = view.selected.ok_or("selected file")?;
        let change = view.rows.get(&selected).ok_or("selected change")?.clone();
        let saved = view.diff.save_context().ok_or("reader context")?;
        view.saved_reader = Some(saved.clone());
        view.history_entry_row = view.selected;
        view.history = Some(crate::file_history_view::FileHistory::fixture(
            change.path_bytes.clone(),
            history_timeline(&change.path_bytes),
            0,
        ));
        Ok(saved)
    }

    #[test]
    fn recovery_caller_names_its_back_target() {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        view.set_return_location(crate::return_location::ReturnLocation::Recovery {
            source: crate::return_location::RecoverySource::Reflog,
            reference: Some("HEAD".into()),
            entry: "HEAD@{2}".into(),
        });
        let output = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(1120.0, 740.0, vec![]),
            |ctx| {
                view.draw_ui(ctx);
            },
        );
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.job.text == "Back to recovery"
        )));
        assert!(!output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.job.text == "Back to history"
        )));
    }

    #[test]
    fn select_path_opens_the_reader_on_that_file() {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        let wanted = format!("src/0003-{}.rs", "long-path-".repeat(20));
        view.pending_path = Some(wanted.clone());
        for _ in 0..2 {
            let _ = crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(1120.0, 740.0, vec![]),
                |ctx| {
                    view.draw_ui(ctx);
                },
            );
        }
        assert_eq!(view.selected, Some(3));
        assert!(view.show_diff);
        assert_eq!(view.pending_path, None);
    }

    #[test]
    fn header_controls_keep_their_place_across_states() {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        let place = |view: &mut Review| {
            let mut output = None;
            for _ in 0..2 {
                output = Some(crate::tests::run_ui(
                    &ctx,
                    crate::tests::frame_input(1120.0, 740.0, vec![]),
                    |ctx| {
                        view.draw_ui(ctx);
                    },
                ));
            }
            let Some(output) = output else { unreachable!() };
            let find = |wanted: &str| {
                output.shapes.iter().find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) if text.galley.job.text == wanted => {
                        Some(text.pos)
                    }
                    _ => None,
                })
            };
            (find("History"), find("1000 files"))
        };
        let files = place(&mut view);
        assert!(files.0.is_some() && files.1.is_some(), "{files:?}");
        view.error = Some("transient failure".into());
        assert_eq!(place(&mut view), files, "Retry appears in its own slot");
        view.error = None;
        view.show_diff = true;
        assert_eq!(
            place(&mut view),
            files,
            "Prev/Next file appear in reserved slots"
        );
    }

    #[test]
    fn unread_rows_stay_empty_and_loaded_rows_stay_put() {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        let frame = |view: &mut Review| {
            crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(1120.0, 740.0, vec![]),
                |ui| {
                    view.draw_ui(ui);
                },
            )
        };
        let _ = frame(&mut view);
        // Rows 1–4 are still being read while row 0 is shown.
        for row in 1..5 {
            view.rows.remove(&row);
        }
        let output = frame(&mut view);
        let texts = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            !texts.iter().any(|text| {
                let text = text.to_lowercase();
                text.contains("loading") || text.contains("resolving")
            }),
            "no placeholder text in the pane: {texts:?}"
        );
        assert!(
            texts.iter().any(|text| text.starts_with("src/0000-")),
            "the loaded row stays visible: {texts:?}"
        );
        assert!(
            !texts.iter().any(|text| text.starts_with("src/0001-")),
            "an unread row stays empty"
        );
    }

    #[test]
    fn commit_identity_shares_one_line_with_back_and_file_count() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut view = Review::fixture(&ctx);
            let mut output = None;
            for _ in 0..2 {
                output = Some(crate::tests::run_ui(
                    &ctx,
                    crate::tests::frame_input(width, 740.0, vec![]),
                    |ctx| {
                        view.draw_ui(ctx);
                    },
                ));
            }
            let Some(output) = output else { unreachable!() };
            let row = |wanted: &str| {
                output.shapes.iter().find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) if text.galley.job.text == wanted => {
                        Some(text.visual_bounding_rect().center().y)
                    }
                    _ => None,
                })
            };
            let back = row("Back to history");
            let subject = row("Review fixture");
            let files = row("1000 files");
            assert!(back.is_some() && subject.is_some() && files.is_some());
            assert!(
                back.zip(subject).is_some_and(|(a, b)| (a - b).abs() < 2.0)
                    && back.zip(files).is_some_and(|(a, b)| (a - b).abs() < 2.0),
                "width {width}: back={back:?} subject={subject:?} files={files:?}"
            );
        }
    }

    #[test]
    fn against_comparison_names_before_and_after_sources() {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        view.staged = None;
        view.request.against = Some("HEAD".into());
        view.request.revision = "a".repeat(40);
        view.comparison = Some(Comparison {
            path: "repo".into(),
            commit: "a".repeat(40),
            parent: Some("b".repeat(40)),
            parents: Vec::new(),
            subject: "Compare".into(),
        });
        view.rows.insert(
            0,
            FileChange {
                path: "src/app.rs".into(),
                path_bytes: b"src/app.rs".to_vec(),
                kind: ChangeKind::Modified,
                old: Some(FileSide {
                    id: "b".repeat(40),
                    mode: 0o100644,
                }),
                new: Some(FileSide {
                    id: "a".repeat(40),
                    mode: 0o100644,
                }),
            },
        );
        view.selected = Some(0);
        let output = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(1120.0, 740.0, vec![]),
            |ctx| {
                view.draw_ui(ctx);
            },
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.iter().any(|line| {
            line.contains("Before") && line.contains("HEAD") && line.contains(&"b".repeat(8))
        }));
        assert!(
            text.iter()
                .any(|line| line.contains("After") && line.contains(&"a".repeat(8)))
        );
    }

    #[test]
    fn retry_same_comparison_keeps_selection_anchor_and_diff() -> Result<(), String> {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut view = Review::fixture(&ctx);
            let frame = |view: &mut Review| {
                let _ = crate::tests::run_ui(
                    &ctx,
                    crate::tests::frame_input(width, 740.0, vec![]),
                    |ctx| {
                        view.draw_ui(ctx);
                    },
                );
            };
            frame(&mut view);
            view.selected = Some(20);
            view.offset = 300.0;
            view.restore_files = true;
            frame(&mut view);
            let anchor = view.offset;
            let selected = view.rows[&20].clone();
            let generation = view.diff.generation;
            let comparison = view.comparison.clone().ok_or("missing comparison")?;
            view.error = Some("Temporary read failure".into());
            view.retry(&ctx);
            view.worker = None;
            for (count, complete) in [(1, false), (1000, true)] {
                view.staged = Some(Update {
                    comparison: comparison.clone(),
                    count,
                    complete,
                    error: None,
                    rows: vec![(0, view.rows[&0].clone())],
                });
                frame(&mut view);
                assert_eq!(view.selected, Some(20));
                assert_eq!(view.rows[&20].path_bytes, selected.path_bytes);
                assert_eq!(view.offset, anchor);
                assert_eq!(view.count, 1000);
                assert_eq!(view.diff.generation, generation);
            }
            assert!(view.complete);
            assert!(view.error.is_none());
        }
        Ok(())
    }
    #[test]
    fn parent_replacement_retains_identity_until_pointer_leaves_and_resets_selection()
    -> Result<(), String> {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut view = Review::fixture(&ctx);
            let frame = |view: &mut Review, events, time| {
                let mut input = crate::tests::frame_input(width, 740.0, events);
                input.time = Some(time);
                let _ = crate::tests::run_ui(&ctx, input, |ctx| {
                    view.draw_ui(ctx);
                });
            };
            frame(&mut view, vec![], 1.0);
            let comparison = view.comparison.as_mut().ok_or("comparison missing")?;
            comparison.parents = vec!["first".into(), "second".into()];
            comparison.parent = Some("first".into());
            let mut next = comparison.clone();
            next.parent = Some("second".into());
            view.selected = Some(2);
            let point = view.row_rects[2].1.center();
            view.switch_parent(2, &ctx);
            view.worker = None;
            assert_eq!(view.request.revision, "commit");
            assert_eq!(view.request.parent, Some(2));
            assert_eq!(view.selected, Some(2));
            let mut file = view.rows[&0].clone();
            file.path = "second-parent.rs".into();
            file.path_bytes = file.path.as_bytes().to_vec();
            view.staged = Some(Update {
                comparison: next,
                count: 1,
                complete: true,
                error: None,
                rows: vec![(0, file)],
            });
            frame(&mut view, vec![egui::Event::PointerMoved(point)], 1.1);
            assert_eq!(
                view.comparison.as_ref().and_then(|c| c.parent.as_deref()),
                Some("first")
            );
            assert_eq!(view.selected, Some(2));
            assert!(view.replacing);
            frame(&mut view, vec![egui::Event::PointerGone], 1.4);
            assert_eq!(
                view.comparison.as_ref().and_then(|c| c.parent.as_deref()),
                Some("second")
            );
            assert_eq!(view.rows.len(), 1);
            assert_eq!(view.rows[&0].path, "second-parent.rs");
            assert_eq!(view.selected, Some(0));
            assert_eq!(view.offset, 0.0);
            assert!(!view.replacing);
            // A failed request retains the displayed comparison; retry has a fresh mailbox.
            view.switch_parent(1, &ctx);
            view.worker = None;
            view.error = Some("Missing parent object".into());
            frame(&mut view, vec![], 1.5);
            assert_eq!(view.rows[&0].path, "second-parent.rs");
            assert_eq!(
                view.comparison.as_ref().and_then(|c| c.parent.as_deref()),
                Some("second")
            );
            view.retry(&ctx);
            assert!(view.error.is_none());
            assert!(view.staged.is_none());
            assert!(view.replacing);
            view.worker = None;
            let mut empty = view.comparison.clone().ok_or("comparison missing")?;
            empty.parent = Some("first".into());
            view.staged = Some(Update {
                comparison: empty,
                count: 0,
                complete: true,
                error: None,
                rows: vec![],
            });
            frame(&mut view, vec![], 1.8);
            assert!(view.rows.is_empty());
            assert!(view.selected.is_none());
            assert!(view.diff.change.is_none());
        }
        Ok(())
    }
    #[test]
    fn reading_mode_exposes_file_and_change_navigation_at_both_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut view = Review::fixture(&ctx);
            view.worker = None;
            view.selected = Some(0);
            view.show_diff = true;
            let frame = |view: &mut Review, events| {
                let _ = crate::tests::run_ui(
                    &ctx,
                    crate::tests::frame_input(width, 480.0, events),
                    |ctx| {
                        view.draw_ui(ctx);
                    },
                );
            };
            let output = crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(width, 480.0, vec![]),
                |ctx| {
                    view.draw_ui(ctx);
                },
            );
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            for label in [
                "Prev file",
                "Next file",
                "Diff",
                "Full file",
                "No changes",
                crate::phosphor::CARET_LEFT,
                crate::phosphor::CARET_RIGHT,
            ] {
                assert!(
                    text.iter().any(|line| line.contains(label)),
                    "missing {label} at {width}: {text:?}"
                );
            }
            crate::navigation::focus_content(&ctx);
            frame(
                &mut view,
                vec![egui::Event::Key {
                    key: egui::Key::ArrowDown,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::ALT,
                }],
            );
            assert!(view.show_diff);
            assert_eq!(view.selected, Some(1));
            crate::navigation::focus_content(&ctx);
            frame(
                &mut view,
                vec![egui::Event::Key {
                    key: egui::Key::ArrowUp,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::ALT,
                }],
            );
            assert!(view.show_diff);
            assert_eq!(view.selected, Some(0));
        }
    }

    #[test]
    fn enter_and_left_open_and_return_to_files_at_both_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut view = Review::fixture(&ctx);
            let frame = |view: &mut Review, key: Option<egui::Key>| {
                let events = key
                    .into_iter()
                    .map(|key| egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    })
                    .collect();
                let _ = crate::tests::run_ui(
                    &ctx,
                    crate::tests::frame_input(width, 480.0, events),
                    |ctx| {
                        view.draw_ui(ctx);
                    },
                );
            };
            frame(&mut view, None::<egui::Key>);
            let selected = view.selected;
            frame(&mut view, Some(egui::Key::Enter));
            assert!(view.show_diff);
            frame(&mut view, Some(egui::Key::ArrowLeft));
            assert!(!view.show_diff);
            assert_eq!(view.selected, selected);
        }
    }
    #[test]
    fn narrow_diff_back_preserves_file_selection_and_offset() {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        let frame = |view: &mut Review, events| {
            let _ = crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(720.0, 480.0, events),
                |ctx| {
                    view.draw_ui(ctx);
                },
            );
        };
        frame(&mut view, vec![]);
        view.selected = Some(20);
        view.offset = 300.0;
        view.show_diff = true;
        frame(&mut view, vec![]);
        let event = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut view, vec![event(egui::Key::ArrowDown)]);
        assert_eq!(view.selected, Some(20));
        assert_eq!(view.offset, 300.0);
        frame(&mut view, vec![event(egui::Key::Escape)]);
        assert!(!view.show_diff);
        assert_eq!(view.selected, Some(20));
        assert_eq!(view.offset, 300.0);
    }
    #[test]
    fn compact_rows_and_placeholder_identity_survive_arrival() -> Result<(), String> {
        for (width, height) in [(720.0, 480.0), (1120.0, 740.0)] {
            let ctx = egui::Context::default();
            let mut view = Review::fixture(&ctx);
            let frame = |view: &mut Review, events, time| {
                let mut input = crate::tests::frame_input(width, height, events);
                input.time = Some(time);
                crate::tests::run_ui(&ctx, input, |ctx| {
                    view.draw_ui(ctx);
                })
            };
            frame(&mut view, vec![], 1.0);
            frame(&mut view, vec![], 1.1);
            for (_, rect) in &view.row_rects {
                assert_eq!(rect.height(), ROW);
                assert!(rect.left() >= 0.0);
                assert!(rect.right() <= width);
            }
            let point = view
                .row_rects
                .iter()
                .find(|(row, _)| *row == 2)
                .ok_or("row missing")?
                .1
                .center();
            let change = view.rows.remove(&2).ok_or("change missing")?;
            frame(&mut view, vec![egui::Event::PointerMoved(point)], 1.2);
            view.staged = Some(Update {
                comparison: Comparison {
                    path: "repo".into(),
                    commit: "commit".into(),
                    parent: None,
                    parents: vec![],
                    subject: "fixture".into(),
                },
                count: 1001,
                complete: false,
                error: None,
                rows: vec![(2, change)],
            });
            frame(&mut view, vec![], 1.3);
            assert!(!view.rows.contains_key(&2));
            frame(&mut view, vec![egui::Event::PointerGone], 1.4);
            assert!(view.rows.contains_key(&2));
        }
        Ok(())
    }

    #[test]
    fn history_opens_in_the_reader_and_escape_restores_the_entry_context_once()
    -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        let _ = frame(&mut view, &ctx, 1120.0, 740.0, vec![]);
        view.show_diff = true;
        view.selected = Some(20);
        view.offset = 300.0;
        let location = crate::return_location::ReturnLocation::History(
            crate::return_location::HistoryLocation {
                selected: Some("caller".into()),
                query: "fix".into(),
                offset: 12.0,
            },
        );
        view.set_return_location(location.clone());
        let _ = frame(&mut view, &ctx, 1120.0, 740.0, vec![]);
        let saved = install_history(&mut view)?;
        let selected = view.selected;
        let file_offset = view.offset;
        let (_, text) = frame(&mut view, &ctx, 1120.0, 740.0, vec![]);
        assert!(view.history.is_some());
        for label in [
            "File history",
            "Older version",
            "Newer version",
            "Version 1 of 2",
        ] {
            assert!(
                text.iter().any(|line| line.contains(label)),
                "missing {label}: {text:?}"
            );
        }
        let (back, _) = frame(
            &mut view,
            &ctx,
            1120.0,
            740.0,
            vec![key_event(egui::Key::ArrowLeft, egui::Modifiers::ALT)],
        );
        assert!(!back);
        assert_eq!(
            view.history.as_ref().map(|history| history.index()),
            Some(1)
        );
        assert_eq!(
            view.return_location.as_ref(),
            Some(&location),
            "a version step adds no Back/return-location entry"
        );
        let history_context = view.diff.save_context().ok_or("history context")?;
        assert_ne!(
            history_context.change.new, saved.change.new,
            "the version step did not change the reader source"
        );
        let (back, _) = frame(
            &mut view,
            &ctx,
            1120.0,
            740.0,
            vec![key_event(egui::Key::Escape, egui::Modifiers::NONE)],
        );
        assert!(!back, "closing history must not leave the review");
        assert!(view.history.is_none());
        assert!(view.saved_reader.is_none());
        assert!(view.show_diff);
        assert_eq!(view.selected, selected);
        assert_eq!(view.offset, file_offset);
        assert_eq!(
            view.return_location.as_ref(),
            Some(&location),
            "closing history restores the reader without adding Back"
        );
        let restored = view.diff.save_context().ok_or("restored context")?;
        assert_eq!(restored.change.path_bytes, saved.change.path_bytes);
        assert_eq!(restored.change.old, saved.change.old);
        assert_eq!(restored.change.new, saved.change.new);
        assert_eq!(restored.before, saved.before);
        assert_eq!(restored.after, saved.after);
        assert_eq!(restored.place, saved.place);
        // A later frame must not apply the saved context again.
        let _ = frame(&mut view, &ctx, 1120.0, 740.0, vec![]);
        assert_eq!(
            view.diff.save_context().ok_or("context")?.place,
            saved.place
        );
        Ok(())
    }

    #[test]
    fn history_opens_from_the_wide_reader_and_never_stacks() {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        view.selected = Some(4);
        let _ = frame(&mut view, &ctx, 1120.0, 740.0, vec![]);
        assert!(!view.show_diff);
        let file_offset = view.offset;
        let (_, text) = frame(
            &mut view,
            &ctx,
            1120.0,
            740.0,
            vec![key_event(egui::Key::H, egui::Modifiers::NONE)],
        );
        assert!(view.open_history, "H requests history on the next frame");
        let (_, text2) = frame(&mut view, &ctx, 1120.0, 740.0, vec![]);
        assert!(view.history.is_some());
        assert!(!view.show_diff, "the wide file list keeps its place");
        assert_eq!(view.selected, Some(4));
        assert_eq!(view.offset, file_offset);
        assert!(
            text.iter()
                .chain(text2.iter())
                .any(|line| line == "History"),
            "the visible History button remains reachable"
        );
        let saved = view.saved_reader.is_some();
        assert_eq!(
            view.saved_reader
                .as_ref()
                .and_then(|context| context.revision.as_deref()),
            Some("commit"),
            "opening history captures the entry revision"
        );
        let (_, _) = frame(
            &mut view,
            &ctx,
            1120.0,
            740.0,
            vec![key_event(egui::Key::H, egui::Modifiers::NONE)],
        );
        assert!(!view.open_history);
        assert!(view.history.is_some());
        assert_eq!(view.saved_reader.is_some(), saved);
    }

    #[test]
    fn narrow_history_uses_the_reading_layout_and_escape_closes_it_first()
    -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        view.show_diff = true;
        view.selected = Some(6);
        let _ = frame(&mut view, &ctx, 720.0, 480.0, vec![]);
        let saved = install_history(&mut view)?;
        let (_, text) = frame(&mut view, &ctx, 720.0, 480.0, vec![]);
        assert!(view.history.is_some());
        assert!(text.iter().any(|line| line.contains("Older version")));
        let (back, _) = frame(
            &mut view,
            &ctx,
            720.0,
            480.0,
            vec![key_event(egui::Key::Escape, egui::Modifiers::NONE)],
        );
        assert!(!back);
        assert!(view.history.is_none());
        assert!(view.show_diff, "history returns to the reading layout");
        assert_eq!(
            view.diff.save_context().ok_or("restored context")?.place,
            saved.place
        );
        let (back, _) = frame(
            &mut view,
            &ctx,
            720.0,
            480.0,
            vec![key_event(egui::Key::Escape, egui::Modifiers::NONE)],
        );
        assert!(!back);
        assert!(!view.show_diff, "the next Escape returns to the file list");
        let (back, _) = frame(
            &mut view,
            &ctx,
            720.0,
            480.0,
            vec![key_event(egui::Key::Escape, egui::Modifiers::NONE)],
        );
        assert!(back, "the last Escape leaves the review");
        Ok(())
    }

    #[test]
    fn selecting_another_file_resets_history_and_keeps_the_return_context()
    -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        view.selected = Some(0);
        let _ = frame(&mut view, &ctx, 1120.0, 740.0, vec![]);
        let saved = install_history(&mut view)?;
        view.history.as_mut().ok_or("history")?.set_index(1);
        view.selected = Some(1);
        let new_path = view.rows[&1].path_bytes.clone();
        let _ = frame(&mut view, &ctx, 1120.0, 740.0, vec![]);
        let history = view.history.as_ref().ok_or("history stays active")?;
        assert_eq!(history.path_bytes(), new_path.as_slice());
        assert_eq!(
            history.index(),
            0,
            "the new file starts at its newest version"
        );
        assert!(
            view.saved_reader.is_some(),
            "the original entry context stays saved"
        );
        assert_ne!(saved.change.path_bytes, new_path);
        // The entry row can fall outside the bounded row cache while history is
        // open. Closing must still restore its selection and place by identity.
        view.rows.remove(&0);

        let _ = frame(
            &mut view,
            &ctx,
            1120.0,
            740.0,
            vec![key_event(egui::Key::Escape, egui::Modifiers::NONE)],
        );
        assert!(view.history.is_none(), "Escape closes history");
        let restored = view.diff.save_context().ok_or("restored comparison")?;
        assert_eq!(restored.change.path_bytes, saved.change.path_bytes);
        Ok(())
    }

    #[test]
    fn an_ambiguous_history_edge_keeps_full_content_and_disables_diff()
    -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        view.show_diff = true;
        view.selected = Some(2);
        let _ = frame(&mut view, &ctx, 1120.0, 740.0, vec![]);
        let change = view.rows.get(&2).ok_or("change")?.clone();
        let saved = view.diff.save_context().ok_or("reader context")?;
        view.saved_reader = Some(saved);
        let candidate = gg_engine::file_history::VersionCandidate {
            commit: "c".repeat(40),
            path_bytes: change.path_bytes.clone(),
            path: change.path.clone(),
            side: change.new.clone(),
            subject: "Older".into(),
            author: "Ada".into(),
            time: 1,
            kind: gg_engine::file_history::HistoryKind::Modified,
            old_path_bytes: None,
            old_path: None,
        };
        let version = gg_engine::file_history::FileVersion {
            path_bytes: change.path_bytes.clone(),
            path: change.path.clone(),
            side: change.new.clone(),
            commit: "d".repeat(40),
            subject: "Current".into(),
            author: "Ada".into(),
            time: 2,
            kind: None,
            old_path_bytes: None,
            old_path: None,
            previous: gg_engine::file_history::Predecessor::Candidates {
                choices: vec![candidate],
            },
        };
        view.history = Some(crate::file_history_view::FileHistory::fixture(
            change.path_bytes.clone(),
            gg_engine::file_history::Timeline::new(vec![version], true, None),
            0,
        ));
        let (_, text) = frame(&mut view, &ctx, 1120.0, 740.0, vec![]);
        assert!(view.history.is_some());
        assert!(
            text.iter()
                .any(|line| line.contains("Choose the earlier version")),
            "the chooser stays inside the version strip: {text:?}"
        );
        let capabilities = view.diff.capabilities();
        assert!(!capabilities.diff, "Diff waits for the earlier edge");
        assert_eq!(
            view.diff.place().mode,
            crate::diff_worker::Mode::Full,
            "the current Full file stays readable"
        );
        Ok(())
    }
}

#[cfg(test)]
mod performance {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    #[ignore = "set GG_REVIEW_REPOSITORY to scripts/review-fixture.py output; run in release mode"]
    fn packed_review_latency_and_navigation() -> Result<(), Box<dyn std::error::Error>> {
        let path =
            std::env::var_os("GG_REVIEW_REPOSITORY").ok_or("GG_REVIEW_REPOSITORY is required")?;
        let ctx = egui::Context::default();
        let start = Instant::now();
        let mut view = Review::new(
            ReviewRequest {
                path: path.into(),
                revision: "HEAD".into(),
                parent: None,
                against: None,
            },
            String::new(),
            &ctx,
        );
        let mut first = None;
        let mut diff = None;
        let mut working_input = None;
        let mut frames = Vec::new();
        let frame = |view: &mut Review, events| {
            let began = Instant::now();
            let mut input = crate::tests::frame_input(1120.0, 740.0, events);
            input.time = Some(start.elapsed().as_secs_f64());
            let _ = crate::tests::run_ui(&ctx, input, |ctx| {
                view.draw_ui(ctx);
            });
            assert!(view.rows.len() <= 768);
            assert!(view.diff.rows.len() <= 768);
            began.elapsed()
        };
        while !view.complete || diff.is_none() {
            frames.push(frame(&mut view, vec![]));
            if !view.rows.is_empty() {
                first.get_or_insert(start.elapsed());
            }
            if !view.diff.rows.is_empty() {
                diff.get_or_insert(start.elapsed());
            }
            if diff.is_some() && !view.complete && view.count > 1 && working_input.is_none() {
                let selected = view.selected;
                let elapsed = frame(
                    &mut view,
                    vec![egui::Event::Key {
                        key: egui::Key::ArrowDown,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
                assert_eq!(view.selected, selected.map(|row| row + 1));
                assert!(elapsed < Duration::from_millis(50));
                working_input = Some(elapsed);
            }
            assert!(view.error.is_none(), "{:?}", view.error);
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "review timed out"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(view.count, 10000);
        let complete = start.elapsed();
        let mut navigation = Vec::new();
        for row in [9998, 0] {
            // Position near the boundary, then use the production keyboard/reveal path.
            view.selected = Some(row);
            let began = Instant::now();
            let input_frame = frame(
                &mut view,
                vec![egui::Event::Key {
                    key: egui::Key::ArrowDown,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert_eq!(view.selected, Some(row + 1));
            assert!(input_frame < Duration::from_millis(50));
            while !view.rows.contains_key(&(row + 1)) {
                frame(&mut view, vec![]);
                assert!(began.elapsed() < Duration::from_secs(5));
                std::thread::sleep(Duration::from_millis(1));
            }
            navigation.push(began.elapsed());
        }
        let mut cached = (0..200)
            .map(|_| frame(&mut view, vec![]))
            .collect::<Vec<_>>();
        cached.sort();
        frames.sort();
        eprintln!(
            "first summaries={first:?}; first diff={diff:?}; complete={complete:?}; working input={working_input:?}; working frame p95={:?}; cached frame p95={:?}; navigation={navigation:?}; retained files={}; retained diff rows={}",
            frames[frames.len() * 95 / 100],
            cached[190],
            view.rows.len(),
            view.diff.rows.len()
        );
        assert!(first.ok_or("missing summaries")? < Duration::from_millis(500));
        assert!(diff.ok_or("missing diff")? < Duration::from_millis(500));
        assert!(cached[190] < Duration::from_millis(16));
        Ok(())
    }
}
