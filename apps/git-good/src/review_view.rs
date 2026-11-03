//! A review context keeps history input and viewport state untouched.
use crate::{
    bounded_text,
    review_worker::{StatsUpdate, Update, Worker},
};
use eframe::egui;
use gg_engine::{file::ReadingSide, review::*};
use std::{collections::BTreeMap, ops::Range};
#[cfg(test)]
const ROW: f32 = crate::density::COMPACT_ROW;
/// Width of the file list beside the reader in wide layouts.
const FILE_LIST_WIDTH: f32 = 300.0;

/// The status line of the narrow reading view.
const NARROW_READING_HINT: &str =
    "Esc: history · F: files · Alt+Up/Down: next file · [ ]: changes · B: blame · H: file history";
/// Font of the per-file line counts.
fn stat_font() -> egui::FontId {
    egui::FontId::monospace(10.5)
}

/// Compact line count: exact below 10,000, then thousands (`12k`).
fn stat_number(value: u64) -> String {
    if value < 10_000 {
        value.to_string()
    } else {
        format!("{}k", value / 1000)
    }
}

/// `+18 −7` for a row; zero sides are left out, as in the Review mockup.
fn stat_label(stat: &LineStat) -> String {
    match *stat {
        LineStat::Lines { added, deleted } => {
            let mut parts = Vec::new();
            if added > 0 {
                parts.push(format!("+{}", stat_number(added.into())));
            }
            if deleted > 0 {
                parts.push(format!("−{}", stat_number(deleted.into())));
            }
            parts.join(" ")
        }
        LineStat::Binary => "bin".into(),
        LineStat::Skipped => String::new(),
    }
}

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
    /// Line counts of cached rows, filled in behind the listing.
    stats: BTreeMap<usize, LineStat>,
    /// Running line totals; incomplete until every file is counted.
    stat_total: StatsUpdate,
    pub selected: Option<usize>,
    complete: bool,
    error: Option<String>,
    viewport: Range<usize>,
    offset: f32,
    page: usize,
    rect: egui::Rect,
    row_rects: Vec<(usize, egui::Rect)>,
    /// Keeps a click off a file row that changed under a still pointer.
    guard: crate::target_guard::Guard<Option<Vec<u8>>>,
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
    /// The commit's parents, kept while comparing against HEAD (that
    /// comparison does not list them) so Compare with can return to one.
    known_parents: Vec<String>,
    /// The commit message, when the caller knows it: the narrow file list
    /// shows its body under MESSAGE.
    message: Option<String>,
}
impl Review {
    pub fn new(request: ReviewRequest, subject: String, ctx: &egui::Context) -> Self {
        let mut view = Self {
            known_parents: Vec::new(),
            message: None,
            request,
            subject,
            worker: None,
            staged: None,
            comparison: None,
            rows: BTreeMap::new(),
            count: 0,
            stats: BTreeMap::new(),
            stat_total: StatsUpdate::default(),
            selected: None,
            complete: false,
            error: None,
            viewport: 0..64,
            offset: 0.0,
            page: 1,
            rect: egui::Rect::NOTHING,
            row_rects: vec![],
            guard: Default::default(),
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

    /// The file Review opens on once its row arrives.
    #[cfg(test)]
    pub fn pending_path(&self) -> Option<&str> {
        self.pending_path.as_deref()
    }

    /// The commit's full message, shown under MESSAGE in the narrow file list.
    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }

    /// MESSAGE under the narrow file list: the body (or the subject when
    /// there is none) and the byline, so nothing of the wide identity row is
    /// lost (narrow proposal).
    fn draw_message(&self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("MESSAGE").small().weak());
        let body = self
            .message
            .as_deref()
            .and_then(|message| message.split_once('\n'))
            .map_or("", |(_, body)| body.trim());
        ui.add(egui::Label::new(if body.is_empty() { &self.subject } else { body }).wrap());
        if let Some(comparison) = self
            .comparison
            .as_ref()
            .filter(|comparison| !comparison.author.is_empty())
        {
            ui.add_space(6.0);
            ui.weak(format!("{} · {}", comparison.author, comparison.date));
        }
    }

    /// Open on `path`: select it and show its reader once its row arrives.
    pub fn select_path(mut self, path: String) -> Self {
        self.pending_path = Some(path);
        self
    }

    /// One-line commit identity: short ID and subject.
    /// Subject, hash pill, and author · date, as in the Review mockup.
    /// Narrow windows leave the byline to the file list's MESSAGE section.
    fn draw_identity(&self, ui: &mut egui::Ui, narrow: bool) {
        let id = self
            .comparison
            .as_ref()
            .map(|c| c.commit.as_str())
            .unwrap_or(self.request.revision.as_str());
        let byline = self
            .comparison
            .as_ref()
            .filter(|comparison| !comparison.author.is_empty() && !narrow)
            .map(|comparison| format!("{} · {}", comparison.author, comparison.date));
        // The subject truncates; the hash pill and the byline keep their width.
        let byline_width = byline.as_ref().map_or(0.0, |text| {
            ui.painter()
                .layout_no_wrap(
                    text.clone(),
                    egui::TextStyle::Body.resolve(ui.style()),
                    egui::Color32::PLACEHOLDER,
                )
                .size()
                .x
        });
        let reserved = byline_width + 90.0 + 3.0 * ui.spacing().item_spacing.x;
        ui.scope(|ui| {
            ui.set_max_width((ui.available_width() - reserved).max(80.0));
            ui.add(
                egui::Label::new(egui::RichText::new(&self.subject).strong().size(13.5)).truncate(),
            )
            .on_hover_text(&self.subject);
        });
        ui.label(
            egui::RichText::new(format!(" {} ", &id[..id.len().min(8)]))
                .monospace()
                .size(11.0)
                .background_color(ui.visuals().faint_bg_color),
        )
        .on_hover_text(id);
        if let Some(byline) = byline {
            ui.weak(byline);
        }
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
    /// The footer receipt of a finished ⋯ › Export.
    pub fn take_export_receipt(&mut self) -> Option<String> {
        self.diff.take_export_receipt()
    }

    /// Take the path chosen with ⋯ › Open in editor or Reveal in folder.
    pub fn take_editor_request(&mut self) -> Option<crate::editor::Request> {
        self.diff.take_editor_request()
    }

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
        if parent == 0 || parent > self.known_parents.len().max(comparison.parents.len()) {
            return;
        }
        let path = comparison.path.clone();
        let commit = comparison.commit.clone();
        self.abandon_history();
        self.request.path = path;
        self.request.revision = commit;
        self.request.parent = Some(parent);
        self.request.against = None;
        self.retry(ctx);
    }

    /// Compare the commit against HEAD instead of a parent.
    fn compare_with_head(&mut self, ctx: &egui::Context) {
        let Some(comparison) = &self.comparison else {
            return;
        };
        let path = comparison.path.clone();
        let commit = comparison.commit.clone();
        self.abandon_history();
        self.request.path = path;
        self.request.revision = commit;
        self.request.parent = None;
        self.request.against = Some("HEAD".into());
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
        self.stats.clear();
        self.stat_total = StatsUpdate::default();
        self.selected = None;
        self.count = 0;
        self.offset = 0.0;
        self.viewport = 0..64;
        self.restore_files = true;
    }
    /// Adopt line counts. A retried, immutable comparison keeps its finished
    /// total while the new counter catches up.
    fn apply_stats(&mut self, update: StatsUpdate) {
        let StatsUpdate {
            rows,
            added,
            deleted,
            complete,
        } = update;
        self.stats.extend(rows);
        if complete || !self.stat_total.complete {
            self.stat_total = StatsUpdate {
                rows: Vec::new(),
                added,
                deleted,
                complete,
            };
        }
    }

    /// Paint the list header: `FILES n` at the left and the line totals,
    /// with `+` while incomplete, at the right.
    fn draw_list_header(&self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), crate::density::row()),
            egui::Sense::hover(),
        );
        let rect = rect.shrink2(egui::vec2(6.0, 0.0));
        let font = egui::FontId::proportional(11.0);
        let weak = ui.visuals().weak_text_color();
        let more = if self.complete { "" } else { "+" };
        bounded_text::paint(
            ui,
            rect,
            &format!("FILES {}{more}", self.count),
            font.clone(),
            weak,
        );
        let total = &self.stat_total;
        if total.added == 0 && total.deleted == 0 {
            return;
        }
        let more = if total.complete { "" } else { "+" };
        let mut job = egui::text::LayoutJob::default();
        let format = |color| egui::TextFormat::simple(font.clone(), color);
        let added = crate::colors::diff(ui.visuals(), gg_engine::diff::RowKind::Added).1;
        let deleted = crate::colors::diff(ui.visuals(), gg_engine::diff::RowKind::Deleted).1;
        job.append(&format!("+{}{more}", total.added), 0.0, format(added));
        job.append(&format!("−{}{more}", total.deleted), 6.0, format(deleted));
        let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
        let at = egui::pos2(
            rect.right() - galley.size().x,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter()
            .with_clip_rect(rect)
            .galley(at, galley, ui.visuals().text_color());
    }

    /// The changed-file list: header, then virtualized rows.
    fn draw_files(&mut self, ui: &mut egui::Ui, reveal: bool) {
        self.draw_list_header(ui);
        self.rect = ui.available_rect_before_wrap();
        if self.count == 0 {
            // An empty list stays empty; the reader says why (proposal).
            self.guard.leave();
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
        // The count slot fits the widest compact count, so rows never move
        // as counts arrive.
        let stat_width = ui
            .fonts_mut(|fonts| {
                fonts.layout_no_wrap(
                    "+9999 −9999".into(),
                    stat_font(),
                    egui::Color32::PLACEHOLDER,
                )
            })
            .size()
            .x;
        let (now, moved, pressed, released) = ui.input(|i| {
            (
                i.time,
                // The guard compares pointer positions itself.
                i.smooth_scroll_delta != egui::Vec2::ZERO || i.pointer.is_decidedly_dragging(),
                i.pointer.primary_pressed(),
                i.pointer.primary_released(),
            )
        });
        let mut hovered = false;
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
                // A row that changed under a still pointer fades in and
                // refuses clicks briefly.
                let point = ui
                    .ctx()
                    .pointer_hover_pos()
                    .filter(|_| ui.rect_contains_pointer(response.rect));
                let target = if let Some(point) = point {
                    hovered = true;
                    let key = change.map(|change| change.path_bytes.clone());
                    self.guard.hover(now, &key, point, moved, pressed)
                } else {
                    crate::target_guard::Target::STILL
                };
                let opacity = ui.opacity();
                ui.multiply_opacity(target.alpha);
                let rect = response.rect.shrink2(egui::vec2(6.0, 0.0));
                let status = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.left() + 14.0, rect.bottom()),
                );
                let stat = egui::Rect::from_min_max(
                    egui::pos2(rect.right() - stat_width, rect.top()),
                    rect.max,
                );
                let path = egui::Rect::from_min_max(
                    egui::pos2(status.right() + 8.0, rect.top()),
                    egui::pos2(stat.left() - 6.0, rect.bottom()),
                );
                if let Some(change) = change {
                    if let Some(label) = self.stats.get(&row).map(stat_label) {
                        let galley = ui.fonts_mut(|fonts| {
                            fonts.layout_no_wrap(label, stat_font(), egui::Color32::PLACEHOLDER)
                        });
                        let at = egui::pos2(
                            stat.right() - galley.size().x,
                            stat.center().y - galley.size().y / 2.0,
                        );
                        ui.painter().with_clip_rect(stat).galley(
                            at,
                            galley,
                            ui.visuals().weak_text_color(),
                        );
                    }
                    let (label, color) = crate::colors::change(ui.visuals(), &change.kind);
                    // Long paths lose their start so the file name stays
                    // visible; hovering the row names the full path.
                    let elided = bounded_text::paint_path(
                        ui,
                        path,
                        &change.path,
                        egui::FontId::proportional(13.0),
                        ui.visuals().strong_text_color(),
                    );
                    let response = response.clone().on_hover_text(if elided {
                        format!("{} · {label}", change.path)
                    } else {
                        label.to_string()
                    });
                    bounded_text::paint(
                        ui,
                        status,
                        crate::colors::change_symbol(&change.kind),
                        egui::FontId::proportional(13.0),
                        color,
                    );
                    if !target.clickable {
                        // The row changed under the pointer: drop the click.
                    } else if response.double_clicked() {
                        response.surrender_focus();
                        self.selected = Some(row);
                        self.show_diff = true;
                    } else if response.clicked() {
                        response.surrender_focus();
                        self.selected = Some(row);
                    }
                }
                // A row not read yet keeps its slot, empty.
                ui.set_opacity(opacity);
            }
        });
        if !hovered {
            self.guard.leave();
        }
        if released {
            self.guard.release();
        }
        if self.guard.animating(now) {
            ui.ctx().request_repaint();
        }
        self.offset = result.state.clamped_offset().y;
        self.page = (result.inner_rect.height() / crate::density::row())
            .floor()
            .max(1.0) as usize;
    }

    /// The version list in the file-list pane while history is open.
    fn draw_history_list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(history) = &mut self.history {
            history.draw_list(ui, ctx);
        }
    }

    /// Draw whichever reader path is active, fed by history when it is open.
    fn draw_reader(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, reading: bool) {
        let mut message = None;
        if self
            .history
            .as_ref()
            .is_some_and(|history| history.close_requested())
        {
            self.close_history(ctx);
        }
        if let Some(history) = &mut self.history {
            message = history.prepare(&mut self.diff, ctx);
        }
        if let Some(message) = message {
            bounded_text::field_named(ui, message, egui::FontId::proportional(12.0), 22.0, "Error");
            return;
        }
        // A finished comparison with no files: say why, and offer the other
        // parent of a merge (empty-states proposal).
        if self.count == 0 && self.complete && self.history.is_none() {
            let parents = self.known_parents.len();
            let parent = self.request.parent.unwrap_or(1);
            let mut other = None;
            ui.vertical_centered(|ui| {
                ui.add_space((ui.available_height() * 0.3).max(24.0));
                if self.request.against.is_some() {
                    ui.label(egui::RichText::new("No differences from HEAD").size(16.0));
                } else if parents > 1 {
                    ui.label(
                        egui::RichText::new(format!("No changes against parent {parent}"))
                            .size(16.0),
                    );
                    let next = if parent == 1 { 2 } else { 1 };
                    ui.add_space(8.0);
                    if ui.button(format!("Compare with parent {next}")).clicked() {
                        other = Some(next);
                    }
                } else {
                    ui.label(egui::RichText::new("This commit changes no files").size(16.0));
                }
            });
            if let Some(next) = other {
                self.switch_parent(next, ctx);
            }
            return;
        }
        // History sits on the reader's mode row (mockup), pressed while the
        // version list is open.
        self.diff.set_history_button(
            (!self.embedded).then_some(self.selected.is_some() || self.history.is_some()),
            self.history.is_some(),
        );
        if reading || self.history.is_some() {
            self.diff.draw_reading(ui);
        } else {
            self.diff.draw(ui);
        }
        if self.diff.take_history_request() {
            if self.history.is_some() {
                self.close_history(ctx);
            } else {
                self.open_history = true;
            }
            ctx.request_repaint();
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
    /// The back pill's label, as in the mockup: `‹ History`, followed by a
    /// dimmed `Esc`.
    fn back_label(&self) -> &'static str {
        if self.show_diff {
            "‹ Files"
        } else {
            self.caller_label()
        }
    }

    /// The caller the identity row returns to: `‹ History`, `‹ Rebase`, or
    /// `‹ Recovery`.
    fn caller_label(&self) -> &'static str {
        if matches!(
            self.return_location,
            Some(crate::return_location::ReturnLocation::Recovery { .. })
        ) {
            "‹ Recovery"
        } else if matches!(
            self.return_location,
            Some(crate::return_location::ReturnLocation::Rebase)
        ) {
            "‹ Rebase"
        } else {
            "‹ History"
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
        // The scrollbar keeps its extent while its thumb is dragged.
        let bar_drag = ctx.input(|i| {
            i.pointer.primary_down()
                && i.pointer
                    .press_origin()
                    .is_some_and(|p| self.rect.contains(p) && p.x >= self.rect.right() - 18.0)
        });
        if let Some(result) = self.worker.as_ref().and_then(Worker::poll) {
            match result {
                Ok(update) => self.staged = Some(update),
                Err(error) => self.error = Some(error),
            }
        }
        // A replacement comparison is adopted as soon as it arrives; the
        // reader keeps the old diff, labelled, until the new one arrives.
        if self.replacing && self.staged.is_some() {
            self.adopt_replacement();
        }
        // Counts wait while a replacement is pending; they name its rows.
        if !self.replacing
            && let Some(update) = self.worker.as_ref().and_then(Worker::poll_stats)
        {
            self.apply_stats(update);
        }
        if !self.replacing
            && !bar_drag
            && let Some(update) = self.staged.take()
        {
            for (row, change) in update.rows {
                self.rows.entry(row).or_insert(change);
            }
            self.count = if update.complete {
                update.count
            } else {
                self.count.max(update.count)
            };
            self.complete = update.complete;
            self.error = update.error;
            self.subject = update.comparison.subject.clone();
            self.request.revision = update.comparison.commit.clone();
            self.request.path = update.comparison.path.clone();
            if !update.comparison.parents.is_empty() {
                self.known_parents = update.comparison.parents.clone();
            }
            self.comparison = Some(update.comparison);
            if self.selected.is_none() && self.count > 0 {
                self.selected = Some(0);
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
                .filter(|row| !self.viewport.contains(row) && Some(*row) != self.selected)
                .max_by_key(|row| row.abs_diff(self.viewport.start));
            if let Some(row) = victim {
                self.rows.remove(&row);
            } else {
                break;
            }
        }
        // Counts of rows far from the viewport are re-sent when they return.
        if self.stats.len() > 768 {
            let keep = self.viewport.start.saturating_sub(256)..self.viewport.end + 256;
            self.stats.retain(|row, _| keep.contains(row));
        }
        let mut back = false;
        let mut reveal = self.reveal_selection;
        self.reveal_selection = false;
        if crate::navigation::available(ctx) && !self.diff.export_open() {
            if self.history.is_none() {
                let files_key = self.show_diff && crate::navigation::activation_available(ctx);
                let (escape, left, files) = ctx.input_mut(|i| {
                    (
                        i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                        i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft),
                        files_key && i.consume_key(egui::Modifiers::NONE, egui::Key::F),
                    )
                });
                back = escape || left;
                // Narrow windows: Esc goes back to the caller from the
                // reading view too; F (or Left) returns to the file list.
                if (back && self.show_diff && !(narrow && escape)) || files {
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
            for (modifiers, key, step) in crate::navigation::keys(page) {
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
                    // Narrow: the identity row always goes back to the
                    // caller; `‹ Files` is on the reader's header row.
                    let label = if narrow {
                        self.caller_label()
                    } else {
                        self.back_label()
                    };
                    if !self.embedded
                        && self.history.is_none()
                        && ui
                            .button((label, egui::RichText::new("Esc").small().weak()))
                            .clicked()
                    {
                        if self.show_diff && !narrow {
                            self.show_diff = false;
                            self.restore_files = true;
                        } else {
                            back = true;
                        }
                    }
                    if !self.embedded && !narrow {
                        ui.separator();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Every control keeps its slot; ones that do not apply are
                        // hidden rather than removed, so nothing shifts when the
                        // comparison loads, fails, or the reading mode changes.
                        // Compare with: each parent, or HEAD, as in the
                        // Review mockup. Hidden (slot kept) until loaded.
                        let parents = if self.known_parents.is_empty() {
                            self.comparison
                                .as_ref()
                                .map(|comparison| comparison.parents.clone())
                                .unwrap_or_default()
                        } else {
                            self.known_parents.clone()
                        };
                        let against_head = self.request.against.as_deref() == Some("HEAD");
                        visible_scope(ui, self.comparison.is_some() && !self.embedded, |ui| {
                            let short = |id: &str| id[..id.len().min(8)].to_string();
                            let current = if against_head {
                                "HEAD".to_string()
                            } else {
                                let parent = self.request.parent.unwrap_or(1);
                                parents.get(parent - 1).map_or_else(
                                    || "parent".to_string(),
                                    |id| {
                                        if parents.len() > 1 {
                                            format!("parent {parent} {}", short(id))
                                        } else {
                                            format!("parent {}", short(id))
                                        }
                                    },
                                )
                            };
                            let mut choice = None;
                            egui::ComboBox::from_id_salt("review-parent")
                                .width(if narrow { 128.0 } else { 150.0 })
                                .selected_text(current)
                                .show_ui(ui, |ui| {
                                    for (index, id) in parents.iter().enumerate() {
                                        let label = if parents.len() > 1 {
                                            format!("parent {} {}", index + 1, short(id))
                                        } else {
                                            format!("parent {}", short(id))
                                        };
                                        let selected = !against_head
                                            && self.request.parent.unwrap_or(1) == index + 1;
                                        if crate::menus::choice(ui, selected, label)
                                            .on_hover_text(id)
                                            .clicked()
                                        {
                                            choice = Some(Some(index + 1));
                                        }
                                    }
                                    if crate::menus::choice(ui, against_head, "HEAD").clicked() {
                                        choice = Some(None);
                                    }
                                });
                            // Narrow: the identity row's `vs` picker holds
                            // the endpoints the wide reader header shows.
                            ui.weak(if narrow { "vs" } else { "Compare with" });
                            match choice {
                                Some(Some(parent))
                                    if against_head
                                        || self.request.parent.unwrap_or(1) != parent =>
                                {
                                    self.request.against = None;
                                    self.switch_parent(parent, ctx);
                                }
                                Some(None) if !against_head => self.compare_with_head(ctx),
                                _ => {}
                            }
                        });
                        if ui
                            .add_visible(self.error.is_some(), egui::Button::new("Retry"))
                            .clicked()
                        {
                            self.retry(ctx);
                        }
                        // File history is on the reader's row (wide) or in
                        // its ⋯ (narrow), and H opens it. Narrow windows
                        // change files with Alt+Up/Down, so the identity
                        // row stays one line (narrow proposal).
                        let next_enabled = self.show_diff
                            && self.count > 0
                            && self.selected.map_or(0, |row| row + 1) < self.count;
                        if !narrow
                            && ui
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
                        if !narrow
                            && ui
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
                                |ui| self.draw_identity(ui, narrow),
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
                    // Paging and failures live in the version list itself.
                    let text = if history_state.is_some() {
                        crate::navigation::HISTORY_HINT
                    } else {
                        failure.as_deref().unwrap_or(if self.replacing {
                            &pending
                        } else if self.show_diff && self.diff.binary() {
                            "Scroll to inspect · Export either side · Esc: files"
                        } else if self.show_diff && narrow {
                            NARROW_READING_HINT
                        } else if self.show_diff {
                            crate::navigation::READING_HINT
                        } else if narrow && self.complete {
                            "Up/Down: choose · Enter: read file · Esc: history · Alt+Left/Right: versions"
                        } else if self.complete {
                            "Esc: history · Up/Down: files · [ ]: changes · Alt+Left/Right: versions · H: file history · B: blame"
                        } else {
                            "Reading changed files…"
                        })
                    };
                    bounded_text::field(ui, text, egui::FontId::proportional(12.0), 18.0);
                });
        }
        if self.history.is_some() && !self.embedded {
            // The version list takes the file-list pane: at the left when
            // wide, above the reader when narrow.
            if narrow {
                egui::Panel::top("review-history")
                    .exact_size(crate::density::row() * 7.0)
                    .show(ui, |ui| self.draw_history_list(ui, ctx));
            } else {
                egui::Panel::left("review-files")
                    .exact_size(FILE_LIST_WIDTH)
                    .resizable(false)
                    .show(ui, |ui| self.draw_history_list(ui, ctx));
            }
            egui::CentralPanel::default().show(ui, |ui| self.draw_reader(ui, ctx, true));
            bounded_text::show_disclosure(ctx);
            return back;
        }
        if self.show_diff {
            // Narrow: `‹ Files 1/3` starts the reader's one header row.
            let position = (narrow && self.history.is_none())
                .then(|| format!("{}/{}", self.selected.map_or(0, |row| row + 1), self.count));
            self.diff.set_narrow_header(position, narrow);
            egui::CentralPanel::default().show(ui, |ui| self.draw_reader(ui, ctx, true));
            if self.diff.take_files_request() {
                self.show_diff = false;
                self.restore_files = true;
            }
            bounded_text::show_disclosure(ctx);
            return back;
        }
        self.diff.set_narrow_header(None, false);
        if self.embedded {
            let height = (ui.available_height() * 0.72).max(120.0);
            egui::Panel::bottom("review-file-diff")
                .exact_size(height)
                .show(ui, |ui| self.draw_reader(ui, ctx, false));
            egui::CentralPanel::default().show(ui, |ui| self.draw_files(ui, reveal));
        } else if narrow {
            // The file list over the commit message (narrow proposal); the
            // list takes what its rows need, at most 60% of the height.
            egui::CentralPanel::default().show(ui, |ui| {
                let rows = (self.count as f32 + 1.0) * crate::density::row() + 4.0;
                let height = rows
                    .min(ui.available_height() * 0.6)
                    .max(4.0 * crate::density::row());
                let width = ui.available_width();
                ui.allocate_ui(egui::vec2(width, height), |ui| {
                    ui.set_height(height);
                    self.draw_files(ui, reveal);
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("review-message")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.draw_message(ui));
            });
        } else {
            // A fixed file list at the left, the reader in the remaining
            // width, as in the Review mockup.
            egui::Panel::left("review-files")
                .exact_size(FILE_LIST_WIDTH)
                .resizable(false)
                .show(ui, |ui| self.draw_files(ui, reveal));
            egui::CentralPanel::default().show(ui, |ui| self.draw_reader(ui, ctx, false));
        }
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
                author: String::new(),
                date: String::new(),
            },
            count: 1000,
            complete: true,
            error: None,
            rows: (0..64)
                .map(|row| {
                    let path = format!("src/{}/{row:04}.rs", "long-path-".repeat(20));
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
                if text.galley.job.text == "‹ Recovery"
        )));
        assert!(!output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.job.text == "‹ History"
        )));
    }

    #[test]
    fn select_path_opens_the_reader_on_that_file() {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        let wanted = format!("src/{}/0003.rs", "long-path-".repeat(20));
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
            // History moved to the reader's row; the header keeps Compare with.
            (find("Compare with"), find("Compare with"))
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
            texts.iter().any(|text| text.ends_with("/0000.rs")),
            "the loaded row stays visible: {texts:?}"
        );
        assert!(
            !texts.iter().any(|text| text.ends_with("/0001.rs")),
            "an unread row stays empty"
        );
    }

    #[test]
    fn commit_identity_shares_one_line_with_back() {
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
            let back = row("‹ History");
            let subject = row("Review fixture");
            assert!(
                back.zip(subject).is_some_and(|(a, b)| (a - b).abs() < 2.0),
                "width {width}: back={back:?} subject={subject:?}"
            );
        }
    }

    /// Text shapes with their painted rectangles.
    fn texts(output: &egui::FullOutput) -> Vec<(String, egui::Rect)> {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some((
                    text.galley.job.text.clone(),
                    egui::Rect::from_min_size(text.pos, text.galley.size()),
                )),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn file_list_header_and_line_counts_keep_their_slots_at_both_widths() -> Result<(), String> {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut view = Review::fixture(&ctx);
            let draw = |view: &mut Review| {
                let mut output = None;
                for _ in 0..2 {
                    output = Some(crate::tests::run_ui(
                        &ctx,
                        crate::tests::frame_input(width, 740.0, vec![]),
                        |ui| {
                            view.draw_ui(ui);
                        },
                    ));
                }
                output.map(|output| texts(&output)).ok_or("no frame")
            };
            let before = draw(&mut view)?;
            // Only the list's rows; the reader names the selected path too.
            let rows = view.rect;
            let path = |texts: &[(String, egui::Rect)], row: &str| {
                texts
                    .iter()
                    .find(|(text, rect)| text.ends_with(row) && rows.contains(rect.center()))
                    .cloned()
                    .ok_or(format!("width {width}: path {row} missing"))
            };
            // Long paths lose their start, so the file name stays visible.
            let (first, first_rect) = path(&before, "/0000.rs")?;
            assert!(first.starts_with('…'), "{first}");
            let header = before
                .iter()
                .find(|(text, _)| text == "FILES 1000")
                .map(|(_, rect)| *rect)
                .ok_or("files header")?;
            assert!(header.bottom() <= first_rect.top() + 1.0);
            assert!(
                !before.iter().any(|(text, _)| text.ends_with(" files")
                    && text.starts_with(|c: char| c.is_ascii_digit())),
                "the list header replaces the separate file count"
            );
            // Counts arrive in the background: the listing does not move.
            view.apply_stats(StatsUpdate {
                rows: vec![
                    (
                        0,
                        LineStat::Lines {
                            added: 18,
                            deleted: 7,
                        },
                    ),
                    (
                        1,
                        LineStat::Lines {
                            added: 12_345,
                            deleted: 0,
                        },
                    ),
                    (2, LineStat::Binary),
                    (3, LineStat::Skipped),
                ],
                added: 12_363,
                deleted: 7,
                complete: false,
            });
            let after = draw(&mut view)?;
            for row in ["/0000.rs", "/0001.rs", "/0002.rs", "/0003.rs"] {
                assert_eq!(path(&after, row)?, path(&before, row)?, "{row} moved");
            }
            let find = |wanted: &str| {
                after
                    .iter()
                    .find(|(text, _)| text == wanted)
                    .map(|(_, rect)| *rect)
                    .ok_or(format!("width {width}: {wanted} missing"))
            };
            let (stat, large, binary) = (find("+18 −7")?, find("+12k")?, find("bin")?);
            assert!((stat.right() - large.right()).abs() < 0.5);
            assert!((stat.right() - binary.right()).abs() < 0.5);
            assert!(first_rect.right() < stat.left());
            let total = find("+12363+−7+")?;
            assert!((total.center().y - header.center().y).abs() < 1.0);
            let list = view.rect;
            assert!(total.right() <= list.right() && stat.right() <= list.right());
            if width > 1040.0 {
                assert!(list.right() <= FILE_LIST_WIDTH + 1.0, "{list:?}");
            } else {
                assert!(list.width() > width - 40.0, "{list:?}");
            }
        }
        Ok(())
    }

    #[test]
    fn line_stat_labels_follow_the_mockup() {
        let lines = |added, deleted| LineStat::Lines { added, deleted };
        assert_eq!(stat_label(&lines(18, 7)), "+18 −7");
        assert_eq!(stat_label(&lines(15, 0)), "+15");
        assert_eq!(stat_label(&lines(0, 11)), "−11");
        assert_eq!(stat_label(&lines(0, 0)), "");
        assert_eq!(stat_label(&lines(100_000, 9_999)), "+100k −9999");
        assert_eq!(stat_label(&LineStat::Binary), "bin");
        assert_eq!(stat_label(&LineStat::Skipped), "");
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
            author: String::new(),
            date: String::new(),
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
    fn compare_with_switches_between_head_and_a_parent() -> Result<(), String> {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        let _ = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(1120.0, 740.0, vec![]),
            |ui| {
                view.draw_ui(ui);
            },
        );
        let comparison = view.comparison.as_mut().ok_or("comparison missing")?;
        comparison.parents = vec!["first".into()];
        view.known_parents = vec!["first".into()];
        view.compare_with_head(&ctx);
        assert_eq!(view.request.against.as_deref(), Some("HEAD"));
        assert_eq!(view.request.parent, None);
        view.switch_parent(1, &ctx);
        assert_eq!(view.request.against, None, "a parent replaces HEAD");
        assert_eq!(view.request.parent, Some(1));
        Ok(())
    }

    #[test]
    fn parent_replacement_keeps_labelled_content_until_it_arrives_and_resets_selection()
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
            // Until the replacement arrives the old comparison stays, labelled.
            frame(&mut view, vec![egui::Event::PointerMoved(point)], 1.05);
            assert_eq!(
                view.comparison.as_ref().and_then(|c| c.parent.as_deref()),
                Some("first")
            );
            assert_eq!(view.rows.len(), 64);
            assert_eq!(view.selected, Some(2));
            assert!(view.replacing);
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
            // It is adopted as soon as it arrives, even under the pointer.
            frame(&mut view, vec![], 1.1);
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
            // Narrow windows start the header row with `‹ Files 1/n` and
            // change files with Alt+Up/Down; wide ones show Prev/Next file.
            let files: &[&str] = if width < 1040.0 {
                &["‹ Files", "1/"]
            } else {
                &["Prev file", "Next file"]
            };
            for label in files.iter().copied().chain([
                "Diff",
                "Full file",
                "No changes",
                crate::phosphor::CARET_LEFT,
                crate::phosphor::CARET_RIGHT,
            ]) {
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
    fn narrow_files_key_returns_to_the_list_and_escape_to_the_caller() {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        let frame = |view: &mut Review, events| {
            let mut back = false;
            let _ = crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(720.0, 480.0, events),
                |ctx| {
                    back = view.draw_ui(ctx);
                },
            );
            back
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
        assert!(!frame(&mut view, vec![event(egui::Key::F)]));
        assert!(!view.show_diff, "F returns to the file list");
        assert_eq!(view.selected, Some(20));
        assert_eq!(view.offset, 300.0);
        view.show_diff = true;
        frame(&mut view, vec![]);
        assert!(
            frame(&mut view, vec![event(egui::Key::Escape)]),
            "Esc in the narrow reading view goes back to the caller"
        );
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
                    author: String::new(),
                    date: String::new(),
                },
                count: 1001,
                complete: false,
                error: None,
                rows: vec![(2, change)],
            });
            // The row fills in place at once and fades in; a click already
            // on its way lands within the grace and is dropped.
            frame(&mut view, vec![], 1.3);
            assert!(view.rows.contains_key(&2));
            let selected = view.selected;
            let output = frame(&mut view, vec![button(point, true)], 1.4);
            let opacity = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text)
                        if text.galley.job.text.ends_with("/0002.rs") =>
                    {
                        Some(text.fallback_color.a())
                    }
                    _ => None,
                })
                .ok_or("arrived row painted")?;
            assert!(opacity < 200, "the arrived row fades in: {opacity}");
            frame(&mut view, vec![button(point, false)], 1.45);
            assert_eq!(view.selected, selected);
            // After the grace a fresh click on the same row acts.
            frame(&mut view, vec![], 1.7);
            frame(&mut view, vec![button(point, true)], 1.75);
            frame(&mut view, vec![button(point, false)], 1.8);
            assert_eq!(view.selected, Some(2));
        }
        Ok(())
    }

    fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn moving_onto_a_row_or_scrolling_keeps_it_clickable() -> Result<(), String> {
        let ctx = egui::Context::default();
        let mut view = Review::fixture(&ctx);
        let frame = |view: &mut Review, events, time| {
            let mut input = crate::tests::frame_input(1120.0, 740.0, events);
            input.time = Some(time);
            let _ = crate::tests::run_ui(&ctx, input, |ctx| {
                view.draw_ui(ctx);
            });
        };
        frame(&mut view, vec![], 1.0);
        let rect = |view: &Review, row| {
            view.row_rects
                .iter()
                .find(|(at, _)| *at == row)
                .map(|(_, rect)| rect.center())
                .ok_or("row missing")
        };
        let first = rect(&view, 3)?;
        frame(&mut view, vec![egui::Event::PointerMoved(first)], 1.1);
        // The user's own motion onto another row starts no grace.
        let second = rect(&view, 4)?;
        frame(&mut view, vec![egui::Event::PointerMoved(second)], 1.2);
        frame(&mut view, vec![button(second, true)], 1.21);
        frame(&mut view, vec![button(second, false)], 1.22);
        assert_eq!(view.selected, Some(4));
        // Scrolling the list under a still pointer is the user's motion too.
        frame(
            &mut view,
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, -3.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
                source: egui::MouseWheelSource::Unknown,
            }],
            1.3,
        );
        let mut time = 1.3;
        while time < 3.0 {
            time += 0.016;
            frame(&mut view, vec![], time);
        }
        assert!(view.offset > ROW, "the list scrolled: {}", view.offset);
        let row = view
            .row_rects
            .iter()
            .find(|(_, rect)| rect.contains(second))
            .map(|(row, _)| *row)
            .ok_or("row under the pointer")?;
        assert_ne!(row, 4);
        frame(&mut view, vec![button(second, true)], time + 0.01);
        frame(&mut view, vec![button(second, false)], time + 0.02);
        assert_eq!(view.selected, Some(row));
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
        // The version list takes the file-list pane; History stays on the
        // reader row.
        for label in ["FILE HISTORY 2", "×", "History"] {
            assert!(
                text.contains(&label.to_string()),
                "missing {label}: {text:?}"
            );
        }
        assert!(
            !text.iter().any(|line| line.starts_with("FILES ")),
            "the file list is replaced: {text:?}"
        );
        assert!(!text.iter().any(|line| line.contains("Older version")));
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
        assert!(
            text.iter().any(|line| line.starts_with("FILE HISTORY")),
            "{text:?}"
        );
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
        assert!(
            back,
            "the next Escape leaves the narrow review (F returns to the files)"
        );
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
                .any(|line| line.contains("CHOOSE EARLIER VERSION 1")),
            "candidates nest under the merge row: {text:?}"
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
