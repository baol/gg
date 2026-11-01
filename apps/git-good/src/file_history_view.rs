//! A bounded newest-to-oldest version list for the shared reader.
//!
//! History changes the reader's comparison source. It owns the worker-backed
//! timeline and the current version, but it never renders a second content
//! viewer: `prepare` hands the current edge to the review's `DiffView`. The
//! list replaces the changed-file list while it is open: 22 px rows of kind,
//! short ID, subject (a rename adds `← old path`) and date, the selected row
//! being the comparison's metadata. Pages load as the list scrolls, with the
//! next slot blank; a failed page is one red row with **Retry**; merge
//! candidates nest under the merge row.
use crate::{
    bounded_text,
    diff_view::DiffView,
    file_history_worker::{Update, Worker},
    line_map_worker, navigation,
};
use eframe::egui;
use gg_engine::{
    file::ReadingSide,
    file_history::{
        FileVersion, HistoryKind, LineMap, PRELOAD, Predecessor, Timeline, VersionCandidate,
    },
    review::{ChangeKind, FileChange},
};
use std::path::PathBuf;

#[derive(Clone, Copy)]
enum MapDirection {
    Older,
    Newer,
}

struct PendingMap {
    target: usize,
    direction: MapDirection,
}

pub struct FileHistory {
    generation: u64,
    repo: PathBuf,
    revision: String,
    file: String,
    path_bytes: Vec<u8>,
    worker: Option<Worker>,
    timeline: Option<Timeline>,
    index: usize,
    requested: usize,
    busy: bool,
    choosing: Option<usize>,
    error: Option<String>,
    close: bool,
    line: Option<(usize, usize)>,
    initial_index: Option<usize>,
    line_note: Option<String>,
    line_older_blocked: Option<String>,
    line_newer_blocked: Option<String>,
    mapping: bool,
    map_generation: u64,
    map_worker: Option<line_map_worker::Worker>,
    pending_map: Option<PendingMap>,
    /// The highlighted merge candidate (Up/Down, Enter picks it).
    choice: usize,
    /// Scroll the list so the selected row shows on the next frame.
    reveal: bool,
}

/// One row of the version list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    Version(usize),
    /// `CHOOSE EARLIER VERSION n` under a merge row.
    Choose(usize),
    Candidate(usize),
    /// The next page: blank while it loads.
    Next,
    Failed,
}

const ROW: f32 = 22.0;

impl FileHistory {
    pub fn new(
        repo: PathBuf,
        revision: String,
        file: String,
        path_bytes: Vec<u8>,
        line: Option<(usize, usize)>,
        start_index: usize,
        ctx: &egui::Context,
    ) -> Self {
        let mut view = Self {
            generation: 1,
            repo,
            revision,
            file,
            path_bytes,
            worker: None,
            timeline: None,
            index: 0,
            requested: PRELOAD,
            busy: true,
            choosing: None,
            error: None,
            close: false,
            line,
            initial_index: (line.is_some() && start_index > 0).then_some(start_index),
            line_note: None,
            line_older_blocked: None,
            line_newer_blocked: None,
            mapping: false,
            map_generation: 0,
            map_worker: None,
            pending_map: None,
            choice: 0,
            reveal: true,
        };
        view.start(ctx);
        view
    }

    fn start(&mut self, ctx: &egui::Context) {
        self.worker = None;
        self.timeline = None;
        self.index = 0;
        self.requested = PRELOAD;
        self.busy = true;
        self.choosing = None;
        self.error = None;
        self.mapping = false;
        self.pending_map = None;
        self.line_note = None;
        self.line_older_blocked = None;
        self.line_newer_blocked = None;
        match Worker::start(ctx.clone()) {
            Ok(worker) => {
                worker.open(
                    self.generation,
                    self.repo.clone(),
                    self.revision.clone(),
                    self.path_bytes.clone(),
                );
                self.worker = Some(worker);
            }
            Err(error) => {
                self.error = Some(error);
                self.busy = false;
            }
        }
    }

    /// Retry a failed page without discarding the versions already loaded.
    ///
    /// The worker keeps its session and pending boundary on failure, so asking
    /// for more versions resumes from the same edge instead of restarting.
    fn retry(&mut self, ctx: &egui::Context) {
        self.error = None;
        if self.timeline.is_some() && self.worker.is_some() {
            let target = self
                .timeline
                .as_ref()
                .map_or(PRELOAD, |timeline| timeline.versions.len() + PRELOAD);
            self.requested = self.requested.max(target);
            self.busy = true;
            if let Some(worker) = &self.worker {
                worker.need(self.generation, self.requested);
            }
            ctx.request_repaint();
        } else {
            self.generation += 1;
            self.start(ctx);
        }
    }

    /// Accept current worker output. History is polled while its strip is open.
    pub fn poll(&mut self) {
        self.poll_line_map();
        let Some(update) = self.worker.as_ref().and_then(Worker::poll) else {
            return;
        };
        self.accept(update);
    }

    fn poll_line_map(&mut self) {
        let Some(update) = self
            .map_worker
            .as_ref()
            .and_then(line_map_worker::Worker::poll)
        else {
            return;
        };
        if update.generation != self.map_generation {
            return;
        }
        self.mapping = false;
        let Some(pending) = self.pending_map.take() else {
            return;
        };
        match update.result {
            Ok(LineMap::Mapped { start, end }) => {
                self.index = pending.target;
                self.line = Some((start, end));
                self.line_note = None;
                self.line_older_blocked = None;
                self.line_newer_blocked = None;
            }
            Ok(boundary) => {
                let reason = boundary.reason().unwrap_or("Line could not be mapped");
                match pending.direction {
                    MapDirection::Older => self.line_older_blocked = Some(reason.into()),
                    MapDirection::Newer => self.line_newer_blocked = Some(reason.into()),
                }
                self.line_note = Some(reason.into());
            }
            Err(error) => {
                self.line_note = Some(error);
            }
        }
    }

    fn accept(&mut self, update: Update) {
        if update.generation != self.generation {
            return;
        }
        self.busy = update.busy;
        if let Some(timeline) = update.timeline {
            self.timeline = Some(timeline);
        }
        self.error = update.error;
        if self.error.is_some() {
            self.choosing = None;
            if let Some(timeline) = &self.timeline {
                // Let a retry request the next page again.
                self.requested = timeline.versions.len();
            }
        }
        if let Some(timeline) = &self.timeline {
            if timeline.versions.is_empty() {
                self.index = 0;
            } else {
                self.index = self.index.min(timeline.versions.len() - 1);
            }
        }
        if let Some(initial) = self.initial_index.take()
            && let Some(timeline) = &self.timeline
            && !timeline.versions.is_empty()
        {
            self.index = initial.min(timeline.versions.len() - 1);
        }
        if let Some(index) = self.choosing
            && self
                .timeline
                .as_ref()
                .is_some_and(|timeline| timeline.choices(index).is_none())
        {
            self.choosing = None;
        }
    }

    /// True when the list's × was clicked.
    pub fn close_requested(&self) -> bool {
        self.close
    }

    pub fn pending(&self) -> bool {
        self.busy
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Exact bytes of the followed path. Review compares this with the file
    /// list selection to decide whether another file was chosen.
    pub fn path_bytes(&self) -> &[u8] {
        &self.path_bytes
    }

    /// Commit that owns the currently displayed content version.
    pub fn current_revision(&self) -> Option<String> {
        self.current().map(|version| version.commit.clone())
    }

    /// Commit that owns the adjacent older content version, when loaded.
    pub fn predecessor_revision(&self) -> Option<String> {
        self.timeline
            .as_ref()?
            .versions
            .get(self.index + 1)
            .map(|version| version.commit.clone())
    }

    fn current(&self) -> Option<&FileVersion> {
        self.timeline.as_ref()?.current(self.index)
    }

    fn last(&self) -> Option<&FileVersion> {
        self.timeline.as_ref()?.versions.last()
    }

    fn can_older(&self) -> bool {
        if self.mapping || self.line_older_blocked.is_some() {
            return false;
        }
        let Some(timeline) = &self.timeline else {
            return false;
        };
        if self.index + 1 < timeline.versions.len() {
            return true;
        }
        matches!(
            self.last().map(|version| &version.previous),
            Some(Predecessor::More)
        ) && !self.pending()
    }

    #[cfg(test)]
    fn can_newer(&self) -> bool {
        !self.mapping && self.line_newer_blocked.is_none() && self.index > 0
    }

    fn older_reason(&self) -> Option<String> {
        if let Some(reason) = &self.line_older_blocked {
            return Some(reason.clone());
        }
        if self.mapping {
            return Some("Mapping line…".into());
        }
        self.timeline
            .as_ref()?
            .before_reason(self.index, self.pending())
    }

    #[cfg(test)]
    fn newer_reason(&self) -> Option<&'static str> {
        self.timeline.as_ref()?.after_reason(self.index)
    }

    fn older(&mut self, ctx: &egui::Context) {
        let Some(timeline) = &self.timeline else {
            return;
        };
        if self.index + 1 < timeline.versions.len() {
            if self.line.is_some() && self.line_older_blocked.is_none() {
                self.start_line_map(ctx, MapDirection::Older);
            } else {
                self.index += 1;
                self.ensure_loaded(ctx);
            }
        } else if matches!(
            self.last().map(|version| &version.previous),
            Some(Predecessor::More)
        ) && !self.pending()
        {
            self.request_more(ctx);
        }
    }

    fn newer(&mut self) {
        self.index = self.index.saturating_sub(1);
    }

    fn newer_with_map(&mut self, ctx: &egui::Context) {
        if self.line.is_some() && self.line_newer_blocked.is_none() && !self.mapping {
            self.start_line_map(ctx, MapDirection::Newer);
        } else {
            self.newer();
        }
    }

    fn start_line_map(&mut self, ctx: &egui::Context, direction: MapDirection) {
        let Some((start, end)) = self.line else {
            return;
        };
        let Some(version) = self.current().cloned() else {
            return;
        };
        let Some(timeline) = &self.timeline else {
            return;
        };
        let (target, other) = match direction {
            MapDirection::Older => {
                let Some(older) = timeline.versions.get(self.index + 1).cloned() else {
                    return;
                };
                (self.index + 1, older)
            }
            MapDirection::Newer => {
                if self.index == 0 {
                    return;
                }
                let Some(newer) = timeline.versions.get(self.index - 1).cloned() else {
                    return;
                };
                (self.index - 1, newer)
            }
        };
        if other.side.is_none() {
            let reason = "File absent in that version";
            match direction {
                MapDirection::Older => self.line_older_blocked = Some(reason.into()),
                MapDirection::Newer => self.line_newer_blocked = Some(reason.into()),
            }
            self.line_note = Some(reason.into());
            return;
        }
        let change = FileChange {
            path: version.path.clone(),
            path_bytes: version.path_bytes.clone(),
            kind: version
                .kind
                .map(HistoryKind::change_kind)
                .unwrap_or(ChangeKind::Modified),
            old: match direction {
                MapDirection::Older => other.side.clone(),
                MapDirection::Newer => version.side.clone(),
            },
            new: match direction {
                MapDirection::Older => version.side.clone(),
                MapDirection::Newer => other.side.clone(),
            },
        };
        if change.old.is_none() || change.new.is_none() {
            return;
        }
        if self.map_worker.is_none() {
            match line_map_worker::Worker::start(ctx.clone()) {
                Ok(worker) => self.map_worker = Some(worker),
                Err(error) => {
                    self.line_note = Some(error);
                    return;
                }
            }
        }
        self.map_generation += 1;
        self.mapping = true;
        self.pending_map = Some(PendingMap { target, direction });
        self.line_note = None;
        match direction {
            MapDirection::Older => self.line_older_blocked = None,
            MapDirection::Newer => self.line_newer_blocked = None,
        }
        if let Some(worker) = &self.map_worker {
            worker.request(
                self.map_generation,
                self.repo.clone(),
                change,
                match direction {
                    MapDirection::Older => ReadingSide::After,
                    MapDirection::Newer => ReadingSide::Before,
                },
                start,
                end,
            );
        }
        ctx.request_repaint();
    }

    fn request_more(&mut self, ctx: &egui::Context) {
        let Some(timeline) = &self.timeline else {
            return;
        };
        if timeline.complete {
            return;
        }
        let target = timeline.versions.len() + PRELOAD;
        if target <= self.requested {
            return;
        }
        self.requested = target;
        self.busy = true;
        if let Some(worker) = &self.worker {
            worker.need(self.generation, target);
        }
        ctx.request_repaint();
    }

    /// Keep a short lookahead so an Older step is usually already loaded.
    fn ensure_loaded(&mut self, ctx: &egui::Context) {
        let Some(timeline) = &self.timeline else {
            return;
        };
        if timeline.complete || self.pending() || self.index + 4 < timeline.versions.len() {
            return;
        }
        if matches!(
            self.last().map(|version| &version.previous),
            Some(Predecessor::More)
        ) {
            self.request_more(ctx);
        }
    }

    fn choose(&mut self, choice: usize, ctx: &egui::Context) {
        if self.choosing.is_some() || self.pending() {
            return;
        }
        self.choosing = Some(self.index);
        self.busy = true;
        if let Some(worker) = &self.worker {
            worker.choose(self.generation, choice);
        }
        ctx.request_repaint();
    }

    /// Build the edge into the current version. The flag is false when the
    /// earlier side is not resolved, so only the current content is shown.
    fn edge(&self) -> Option<(FileChange, bool)> {
        let timeline = self.timeline.as_ref()?;
        let version = timeline.current(self.index)?;
        let change = |old: Option<gg_engine::review::FileSide>,
                      new: Option<gg_engine::review::FileSide>,
                      kind| FileChange {
            path_bytes: version.path_bytes.clone(),
            path: version.path.clone(),
            kind,
            old,
            new,
        };
        match &version.previous {
            Predecessor::Linked => {
                let older = timeline.current(self.index + 1)?;
                Some((
                    change(
                        older.side.clone(),
                        version.side.clone(),
                        version.kind.unwrap_or(HistoryKind::Modified).change_kind(),
                    ),
                    version.kind.is_some(),
                ))
            }
            Predecessor::Oldest => {
                version.side.as_ref()?;
                Some((
                    change(
                        None,
                        version.side.clone(),
                        version.kind.unwrap_or(HistoryKind::Added).change_kind(),
                    ),
                    true,
                ))
            }
            _ => {
                version.side.as_ref()?;
                Some((
                    change(
                        version.side.clone(),
                        version.side.clone(),
                        ChangeKind::Modified,
                    ),
                    false,
                ))
            }
        }
    }

    fn edge_labels(&self) -> Option<(String, String)> {
        let timeline = self.timeline.as_ref()?;
        let version = timeline.current(self.index)?;
        let after = if version.path == self.file {
            version.short_commit()
        } else {
            format!("{} · {}", version.short_commit(), version.path)
        };
        let before = match &version.previous {
            Predecessor::Linked => timeline
                .current(self.index + 1)
                .map(|older| {
                    if older.path == version.path {
                        older.short_commit()
                    } else {
                        format!("{} · {}", older.short_commit(), older.path)
                    }
                })
                .unwrap_or_else(|| "Earlier version".into()),
            Predecessor::Oldest => "Absent".into(),
            _ => "Earlier version not loaded".into(),
        };
        Some((before, after))
    }

    /// Hand the current version edge to the shared reader.
    ///
    /// Returns a visible message when there is no comparison to render, such
    /// as a loading first page or an absent root version.
    pub fn prepare(&mut self, diff: &mut DiffView, ctx: &egui::Context) -> Option<&'static str> {
        let edge = self.edge();
        if let Some((change, known)) = edge {
            diff.set_side_labels("Before", "After");
            if let Some((before, after)) = self.edge_labels() {
                diff.set_endpoints(before, after);
            }
            diff.select_edge(self.repo.clone(), Some(change), known, ctx);
            return None;
        }
        if self.timeline.is_none() {
            // The reader stays empty until the first page arrives.
            return Some(if self.error.is_some() {
                "File history unavailable"
            } else {
                ""
            });
        }
        Some(match self.current() {
            None => "No content versions are available.",
            Some(version) if version.side.is_none() => "The file is absent in this version.",
            Some(_) => "This version has no file content.",
        })
    }

    /// Return true when Escape/Back closes the history source.
    pub fn keys(&mut self, ctx: &egui::Context) -> bool {
        if !navigation::available(ctx) {
            return false;
        }
        // Match Alt+Left/Right before the plain Back key: egui ignores extra
        // Alt modifiers when matching `Modifiers::NONE`.
        // Up/Down and Enter pick among merge candidates while they show.
        if let Some(count) = self
            .timeline
            .as_ref()
            .and_then(|timeline| timeline.choices(self.index))
            .map(<[VersionCandidate]>::len)
            .filter(|count| *count > 0)
        {
            ctx.input_mut(|input| {
                if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                    self.choice = (self.choice + 1).min(count - 1);
                }
                if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                    self.choice = self.choice.saturating_sub(1);
                }
            });
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                self.choose(self.choice.min(count - 1), ctx);
            }
        }
        let older =
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::ArrowLeft));
        if older {
            self.older(ctx);
            self.reveal = true;
        }
        let newer =
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::ArrowRight));
        if newer {
            self.newer_with_map(ctx);
            self.reveal = true;
        }
        ctx.input_mut(|input| {
            input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                || input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
        })
    }

    /// `FILE HISTORY 12+`, or `LINE HISTORY 41–46` for a line range.
    fn title(&self) -> String {
        match self.line {
            Some((start, end)) if start == end => format!("LINE HISTORY {start}"),
            Some((start, end)) => format!("LINE HISTORY {start}–{end}"),
            None => {
                let (count, more) = self.timeline.as_ref().map_or((0, "+"), |timeline| {
                    (
                        timeline.versions.len(),
                        if timeline.complete { "" } else { "+" },
                    )
                });
                format!("FILE HISTORY {count}{more}")
            }
        }
    }

    /// Why the selection cannot step further, and Git's notes: the header's
    /// tooltip, since the selected row already is the metadata.
    fn notes(&self) -> String {
        let mut notes = Vec::new();
        if let Some(note) = &self.line_note {
            notes.push(note.clone());
        } else if !self.can_older() {
            notes.extend(self.older_reason());
        }
        if let Some(note) = self
            .timeline
            .as_ref()
            .and_then(|timeline| timeline.note.clone())
        {
            notes.push(note);
        }
        notes.join(" · ")
    }

    fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        let Some(timeline) = &self.timeline else {
            return rows;
        };
        for index in 0..timeline.versions.len() {
            rows.push(Row::Version(index));
            if index == self.index
                && let Some(choices) = timeline.choices(index)
            {
                rows.push(Row::Choose(choices.len()));
                rows.extend((0..choices.len()).map(Row::Candidate));
            }
        }
        if self.error.is_some() {
            rows.push(Row::Failed);
        } else if !timeline.complete
            && matches!(
                timeline.versions.last().map(|version| &version.previous),
                Some(Predecessor::More) | None
            )
        {
            rows.push(Row::Next);
        }
        rows
    }

    /// Select version `index`: adjacent steps keep the line range mapped.
    fn select(&mut self, index: usize, ctx: &egui::Context) {
        if index == self.index + 1 {
            self.older(ctx);
        } else if index + 1 == self.index {
            self.newer_with_map(ctx);
        } else if !self.mapping {
            self.index = index;
            self.line_older_blocked = None;
            self.line_newer_blocked = None;
            self.choice = 0;
            self.ensure_loaded(ctx);
        }
    }

    /// Draw the version list in the file-list pane: the header with ×,
    /// then the rows.
    pub fn draw_list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let (header, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), crate::density::row()),
            egui::Sense::hover(),
        );
        let close =
            egui::Rect::from_min_max(egui::pos2(header.right() - 24.0, header.top()), header.max);
        let title = header.shrink2(egui::vec2(6.0, 0.0));
        let title = egui::Rect::from_min_max(title.min, egui::pos2(close.left(), title.bottom()));
        bounded_text::paint(
            ui,
            title,
            &self.title(),
            egui::FontId::proportional(11.0),
            ui.visuals().weak_text_color(),
        );
        let notes = self.notes();
        if !notes.is_empty() {
            ui.interact(
                title,
                ui.scope_id().with("history-notes"),
                egui::Sense::hover(),
            )
            .on_hover_text(notes);
        }
        if ui
            .put(close.shrink(2.0), egui::Button::new("×").small())
            .on_hover_text("Close history (Esc)")
            .clicked()
        {
            self.close = true;
        }
        let rows = self.rows();
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt("file-history-list")
            .auto_shrink([false, false]);
        if std::mem::take(&mut self.reveal)
            && let Some(at) = rows.iter().position(|row| *row == Row::Version(self.index))
        {
            let height = ui.available_height();
            let top = at as f32 * ROW;
            scroll = scroll.vertical_scroll_offset((top - height / 2.0).max(0.0));
        }
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut picked = None;
        let mut chosen = None;
        let mut retry = false;
        let mut more = false;
        scroll.show_rows(ui, ROW, rows.len(), |ui, range| {
            if range.end >= rows.len() && rows.last() == Some(&Row::Next) {
                more = true;
            }
            for row in &rows[range] {
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), ROW),
                    egui::Sense::click(),
                );
                match *row {
                    Row::Version(index) => {
                        if self.version_row(ui, rect, &response, index) {
                            picked = Some(index);
                        }
                    }
                    Row::Choose(count) => {
                        bounded_text::paint(
                            ui,
                            rect.shrink2(egui::vec2(22.0, 0.0)),
                            &format!("CHOOSE EARLIER VERSION {count}"),
                            egui::FontId::proportional(10.0),
                            ui.visuals().weak_text_color(),
                        );
                    }
                    Row::Candidate(index) => {
                        if self.candidate_row(ui, rect, &response, index) {
                            chosen = Some(index);
                        }
                    }
                    // The slot stays blank while the next page loads.
                    Row::Next => {}
                    Row::Failed => {
                        let retry_rect = egui::Rect::from_min_max(
                            egui::pos2(rect.right() - 56.0, rect.top()),
                            rect.max,
                        );
                        let text = egui::Rect::from_min_max(
                            rect.min + egui::vec2(6.0, 0.0),
                            egui::pos2(retry_rect.left() - 4.0, rect.bottom()),
                        );
                        bounded_text::paint_named(
                            ui,
                            text,
                            &format!(
                                "History page failed · {}",
                                self.error.as_deref().unwrap_or_default()
                            ),
                            egui::FontId::proportional(12.0),
                            ui.visuals().error_fg_color,
                            "Error",
                        );
                        retry |= ui
                            .put(retry_rect.shrink(1.0), egui::Button::new("Retry").small())
                            .clicked();
                    }
                }
            }
        });
        if let Some(index) = picked {
            self.select(index, ctx);
        }
        if let Some(index) = chosen {
            self.choice = index;
            self.choose(index, ctx);
        }
        if retry {
            self.retry(ctx);
        } else if more && !self.pending() && self.error.is_none() {
            self.request_more(ctx);
        }
    }

    /// `~ 9a4f9a95 Wire action chips ← src/review.rs 10-20`; true on click.
    fn version_row(
        &self,
        ui: &egui::Ui,
        rect: egui::Rect,
        response: &egui::Response,
        index: usize,
    ) -> bool {
        let Some(version) = self.timeline.as_ref().and_then(|t| t.current(index)) else {
            return false;
        };
        let selected = index == self.index;
        if selected {
            ui.painter()
                .rect_filled(rect, 0.0, crate::colors::selection(ui.visuals()));
        } else if response.hovered() {
            ui.painter()
                .rect_filled(rect, 0.0, ui.visuals().widgets.hovered.weak_bg_fill);
        }
        let kind = version.kind.unwrap_or(HistoryKind::Modified).change_kind();
        let (label, color) = crate::colors::change(ui.visuals(), &kind);
        let renamed = version
            .old_path
            .as_ref()
            .filter(|old| **old != version.path)
            .map(|old| format!("← {old}"));
        paint_row(
            ui,
            rect,
            0.0,
            (crate::colors::change_symbol(&kind), color),
            &version.short_commit(),
            &version.subject,
            renamed.as_deref(),
            &short_date(version.time),
            selected,
        );
        let mut tip = format!(
            "{label} · {} · {}\n{} · {}",
            version.short_commit(),
            version.subject,
            version.author,
            date(version.time)
        );
        if let Some(old) = &renamed {
            tip.push_str(&format!(" · renamed {old}"));
        }
        response.clone().on_hover_text(tip).clicked()
    }

    fn candidate_row(
        &self,
        ui: &egui::Ui,
        rect: egui::Rect,
        response: &egui::Response,
        index: usize,
    ) -> bool {
        let Some(choice) = self
            .timeline
            .as_ref()
            .and_then(|timeline| timeline.choices(self.index))
            .and_then(|choices| choices.get(index))
        else {
            return false;
        };
        let disabled = self.choosing.is_some() || self.pending();
        let cursor = index == self.choice;
        if cursor {
            ui.painter()
                .rect_filled(rect, 0.0, ui.visuals().widgets.hovered.weak_bg_fill);
        }
        let kind = choice.kind.change_kind();
        let (_, color) = crate::colors::change(ui.visuals(), &kind);
        let renamed = choice
            .old_path
            .as_ref()
            .filter(|old| **old != choice.path)
            .map(|old| format!("← {old}"));
        let who = format!(
            "{} · {}",
            choice.author.split_whitespace().next().unwrap_or_default(),
            short_date(choice.time)
        );
        paint_row(
            ui,
            rect,
            16.0,
            (crate::colors::change_symbol(&kind), color),
            &choice.short_commit(),
            &choice.subject,
            renamed.as_deref(),
            &who,
            false,
        );
        !disabled
            && response
                .clone()
                .on_hover_text(format!(
                    "{}\nFollow this predecessor (Enter)",
                    choice_label(choice)
                ))
                .clicked()
    }
}

/// Paint one list row: kind, short ID, subject, the old path, and the
/// right-aligned date (or author and date for a candidate).
#[allow(clippy::too_many_arguments)]
fn paint_row(
    ui: &egui::Ui,
    rect: egui::Rect,
    indent: f32,
    (symbol, color): (&str, egui::Color32),
    hash: &str,
    subject: &str,
    extra: Option<&str>,
    right: &str,
    selected: bool,
) {
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    let left = rect.left() + 6.0 + indent;
    let y = rect.center().y;
    painter.text(
        egui::pos2(left, y),
        egui::Align2::LEFT_CENTER,
        symbol,
        egui::FontId::proportional(13.0),
        color,
    );
    let weak = ui.visuals().weak_text_color();
    let strong = if selected {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().text_color()
    };
    painter.text(
        egui::pos2(left + 16.0, y),
        egui::Align2::LEFT_CENTER,
        hash,
        egui::FontId::monospace(10.5),
        weak,
    );
    let right_width = if right.len() > 6 { 84.0 } else { 40.0 };
    painter.text(
        egui::pos2(rect.right() - 6.0, y),
        egui::Align2::RIGHT_CENTER,
        right,
        egui::FontId::proportional(11.0),
        weak,
    );
    let text = egui::Rect::from_min_max(
        egui::pos2(left + 76.0, rect.top()),
        egui::pos2(rect.right() - right_width - 8.0, rect.bottom()),
    );
    let mut job = egui::text::LayoutJob::default();
    job.append(
        subject,
        0.0,
        egui::TextFormat::simple(egui::FontId::proportional(12.5), strong),
    );
    if let Some(extra) = extra {
        job.append(
            extra,
            8.0,
            egui::TextFormat::simple(egui::FontId::proportional(11.5), weak),
        );
    }
    job.wrap.max_width = text.width().max(0.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    ui.painter().with_clip_rect(text).galley(
        egui::pos2(text.left(), y - galley.size().y / 2.0),
        galley,
        strong,
    );
}

/// `10-27`: the list's compact date.
fn short_date(seconds: i64) -> String {
    date(seconds).get(5..10).unwrap_or_default().to_string()
}

#[cfg(test)]
impl FileHistory {
    /// A deterministic strip fixture with no worker session.
    pub fn fixture(path_bytes: Vec<u8>, timeline: Timeline, index: usize) -> Self {
        let file = String::from_utf8_lossy(&path_bytes).into_owned();
        Self {
            generation: 1,
            repo: "/repo".into(),
            revision: "HEAD".into(),
            file,
            path_bytes,
            worker: None,
            timeline: Some(timeline),
            index,
            requested: PRELOAD,
            busy: false,
            choosing: None,
            error: None,
            close: false,
            line: None,
            initial_index: None,
            line_note: None,
            line_older_blocked: None,
            line_newer_blocked: None,
            mapping: false,
            map_generation: 0,
            map_worker: None,
            pending_map: None,
            choice: 0,
            reveal: false,
        }
    }

    pub fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

fn choice_label(choice: &VersionCandidate) -> String {
    let mut label = format!(
        "{} · {} · {} · {} · {}",
        choice.kind.label(),
        choice.short_commit(),
        choice.subject,
        choice.author,
        date(choice.time)
    );
    if let Some(old) = &choice.old_path
        && old != &choice.path
    {
        label.push_str(&format!(" · from {old}"));
    }
    if label.chars().count() > 150 {
        label = label.chars().take(147).collect::<String>() + "…";
    }
    label
}

/// Format Unix seconds as a compact UTC timestamp without a calendar crate.
fn date(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let second = seconds.rem_euclid(86_400);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    if month <= 2 {
        year += 1;
    }
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        second / 3_600,
        (second % 3_600) / 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gg_engine::{file_history::VersionCandidate, review::FileSide};

    fn side(id: &str) -> FileSide {
        FileSide {
            id: id.into(),
            mode: 0o100644,
        }
    }

    fn version(previous: Predecessor, kind: Option<HistoryKind>) -> FileVersion {
        FileVersion {
            path_bytes: b"src/lib.rs".to_vec(),
            path: "src/lib.rs".into(),
            side: Some(side("a")),
            commit: "a".repeat(40),
            subject: "Change".into(),
            author: "Ada".into(),
            time: 1_767_268_800,
            kind,
            old_path_bytes: None,
            old_path: None,
            previous,
        }
    }

    fn view(timeline: Timeline) -> FileHistory {
        FileHistory::fixture(b"src/lib.rs".to_vec(), timeline, 0)
    }

    #[test]
    fn initial_line_history_index_selects_the_before_version() {
        let timeline = Timeline::new(
            vec![
                version(Predecessor::Linked, Some(HistoryKind::Modified)),
                version(Predecessor::Oldest, Some(HistoryKind::Added)),
            ],
            true,
            None,
        );
        let mut history = view(timeline.clone());
        history.line = Some((2, 2));
        history.initial_index = Some(1);
        history.accept(Update {
            generation: 1,
            timeline: Some(timeline),
            busy: false,
            error: None,
        });
        assert_eq!(history.index, 1);
        assert!(history.initial_index.is_none());
    }

    #[test]
    fn line_history_names_the_anchor_and_boundary() {
        let timeline = Timeline::new(
            vec![
                version(Predecessor::Linked, Some(HistoryKind::Modified)),
                version(Predecessor::Oldest, Some(HistoryKind::Added)),
            ],
            true,
            None,
        );
        let mut history = view(timeline);
        history.line = Some((2, 3));
        history.line_note = Some("Line introduced in this version".into());
        let ctx = egui::Context::default();
        let output = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(1120.0, 740.0, vec![]),
            |ui| history.draw_list(ui, &ctx),
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.contains(&"LINE HISTORY 2–3"), "{text:?}");
        assert!(history.notes().contains("Line introduced in this version"));
    }

    #[test]
    fn boundaries_name_the_reason_and_never_wrap() {
        let timeline = Timeline::new(
            vec![
                version(Predecessor::Linked, Some(HistoryKind::Modified)),
                version(Predecessor::Oldest, Some(HistoryKind::Added)),
            ],
            true,
            None,
        );
        let mut history = view(timeline);
        assert!(history.can_older());
        assert!(!history.can_newer());
        assert_eq!(history.newer_reason(), Some("Newest version"));
        assert_eq!(history.older_reason(), None);

        history.index = 1;
        assert!(!history.can_older());
        assert!(history.can_newer());
        assert_eq!(history.older_reason().as_deref(), Some("Oldest version"));
        assert_eq!(history.newer_reason(), None);
        history.newer();
        assert_eq!(history.index, 0);
        history.newer();
        assert_eq!(history.index, 0);
    }

    #[test]
    fn a_loading_predecessor_disables_older_and_names_the_reason() {
        let mut history = view(Timeline::new(
            vec![version(Predecessor::More, Some(HistoryKind::Modified))],
            false,
            None,
        ));
        history.busy = true;
        assert!(!history.can_older());
        assert_eq!(
            history.older_reason().as_deref(),
            Some("Loading earlier versions…")
        );
        history.busy = false;
        assert!(history.can_older());
        assert_eq!(history.older_reason(), None);
    }

    #[test]
    fn an_ambiguous_predecessor_requires_a_choice() {
        let choices = vec![VersionCandidate {
            commit: "b".repeat(40),
            path_bytes: b"src/lib.rs".to_vec(),
            path: "src/lib.rs".into(),
            side: Some(side("b")),
            subject: "Older".into(),
            author: "Ada".into(),
            time: 1,
            kind: HistoryKind::Modified,
            old_path_bytes: None,
            old_path: None,
        }];
        let history = view(Timeline::new(
            vec![version(Predecessor::Candidates { choices }, None)],
            true,
            None,
        ));
        assert!(!history.can_older());
        assert_eq!(
            history.older_reason().as_deref(),
            Some("Choose one of 1 predecessors")
        );
    }

    #[test]
    fn the_edge_uses_before_and_after_sides_and_an_unknown_edge_has_no_diff()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut timeline = Timeline::new(
            vec![
                version(Predecessor::Linked, Some(HistoryKind::Renamed)),
                version(Predecessor::Oldest, Some(HistoryKind::Added)),
            ],
            true,
            None,
        );
        timeline.versions[0].old_path = Some("old.rs".into());
        let mut history = view(timeline);
        let (change, known) = history.edge().ok_or("the edge is missing")?;
        assert!(known);
        assert_eq!(change.kind, ChangeKind::Modified);
        assert_eq!(change.old.as_ref().map(|side| side.id.as_str()), Some("a"));
        assert_eq!(change.new.as_ref().map(|side| side.id.as_str()), Some("a"));

        history.index = 1;
        let (change, known) = history.edge().ok_or("the edge is missing")?;
        assert!(known);
        assert_eq!(change.kind, ChangeKind::Added);
        assert!(change.old.is_none());

        history.timeline = Some(Timeline::new(
            vec![version(Predecessor::More, Some(HistoryKind::Modified))],
            false,
            None,
        ));
        history.index = 0;
        let (change, known) = history.edge().ok_or("the edge is missing")?;
        assert!(!known);
        assert_eq!(change.old, change.new);
        Ok(())
    }

    #[test]
    fn an_unresolved_edge_prepares_full_current_content_without_diff()
    -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let mut history = view(Timeline::new(
            vec![version(Predecessor::More, Some(HistoryKind::Modified))],
            false,
            None,
        ));
        let mut diff = DiffView::default();
        assert!(history.prepare(&mut diff, &ctx).is_none());
        let capabilities = diff.capabilities();
        assert!(!capabilities.diff);
        assert!(capabilities.full_after);
        assert_eq!(diff.place().mode, crate::diff_worker::Mode::Full);
        Ok(())
    }

    #[test]
    fn dates_are_utc_and_handle_the_epoch() {
        assert_eq!(date(0), "1970-01-01 00:00 UTC");
        assert_eq!(date(1_767_268_800), "2026-01-01 12:00 UTC");
        assert_eq!(date(1_767_268_800 + 86_400), "2026-01-02 12:00 UTC");
    }

    #[test]
    fn the_version_list_names_each_version_and_its_old_path() {
        let ctx = egui::Context::default();
        let mut timeline = Timeline::new(
            vec![
                version(Predecessor::Linked, Some(HistoryKind::Renamed)),
                version(Predecessor::More, Some(HistoryKind::Added)),
            ],
            false,
            None,
        );
        timeline.versions[0].old_path = Some("old.rs".into());
        let mut history = view(timeline);
        history.index = 0;
        history.busy = true;
        let output = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(320.0, 740.0, vec![]),
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| history.draw_list(ui, &ctx));
            },
        );
        let texts = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let text = texts
            .iter()
            .map(|text| text.galley.job.text.as_str())
            .collect::<Vec<_>>();
        assert!(text.contains(&"FILE HISTORY 2+"), "{text:?}");
        assert!(text.contains(&"×"), "the list header closes history");
        assert!(
            text.iter().any(|line| line.contains("← old.rs")),
            "{text:?}"
        );
        assert!(
            !text
                .iter()
                .any(|line| line.to_lowercase().contains("loading")),
            "the next page's slot stays blank: {text:?}"
        );
        assert!(!text.iter().any(|line| line.contains("Version 1 of")));
        assert!(!text.contains(&"Older version"));
        // 22 px rows: the two hashes sit one row apart.
        let hashes = texts
            .iter()
            .filter(|text| text.galley.job.text == "a".repeat(8))
            .map(|text| text.pos.y)
            .collect::<Vec<_>>();
        assert_eq!(hashes.len(), 2, "{text:?}");
        assert!((hashes[1] - hashes[0] - ROW).abs() < 0.5, "{hashes:?}");
        // × in the list header closes history.
        let close = texts
            .iter()
            .find(|text| text.galley.job.text == "×")
            .map(|text| text.visual_bounding_rect().center())
            .unwrap_or_default();
        for pressed in [true, false] {
            let _ = crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(
                    320.0,
                    740.0,
                    vec![
                        egui::Event::PointerMoved(close),
                        egui::Event::PointerButton {
                            pos: close,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                ),
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| history.draw_list(ui, &ctx));
                },
            );
        }
        assert!(history.close_requested());
    }

    #[test]
    fn a_failed_page_is_one_red_row_with_retry() {
        let ctx = egui::Context::default();
        let timeline = Timeline::new(
            vec![version(Predecessor::More, Some(HistoryKind::Modified))],
            false,
            None,
        );
        let mut history = view(timeline);
        history.error = Some("object 0f51a473 not found".into());
        assert_eq!(
            history.rows(),
            [Row::Version(0), Row::Failed],
            "the failure replaces the next page's slot"
        );
        let output = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(320.0, 400.0, vec![]),
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| history.draw_list(ui, &ctx));
            },
        );
        let failed = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text)
                if text.galley.job.text.starts_with("History page failed") =>
            {
                Some(text.fallback_color)
            }
            _ => None,
        });
        assert_eq!(
            failed,
            Some(ctx.style_of(ctx.theme()).visuals.error_fg_color)
        );
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text) if text.galley.job.text == "Retry"
        )));
    }

    #[test]
    fn a_failed_page_retry_keeps_the_loaded_timeline() -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let timeline = Timeline::new(
            vec![
                version(Predecessor::Linked, Some(HistoryKind::Modified)),
                version(Predecessor::More, Some(HistoryKind::Added)),
            ],
            false,
            None,
        );
        let mut history = view(timeline);
        history.worker = Some(Worker::start(ctx.clone())?);
        history.error = Some("History page failed".into());
        let before = history.timeline.as_ref().map(|t| t.versions.len());
        history.retry(&ctx);
        assert_eq!(history.timeline.as_ref().map(|t| t.versions.len()), before);
        assert!(history.error.is_none());
        assert_eq!(history.index, 0);
        Ok(())
    }

    #[test]
    fn rapid_version_steps_stop_at_both_boundaries() {
        let ctx = egui::Context::default();
        let timeline = Timeline::new(
            vec![
                version(Predecessor::Linked, Some(HistoryKind::Modified)),
                version(Predecessor::Linked, Some(HistoryKind::Modified)),
                version(Predecessor::Oldest, Some(HistoryKind::Added)),
            ],
            true,
            None,
        );
        let mut history = view(timeline);
        history.older(&ctx);
        history.older(&ctx);
        assert_eq!(history.index, 2);
        history.older(&ctx);
        assert_eq!(history.index, 2);
        history.newer();
        history.newer();
        assert_eq!(history.index, 0);
        history.newer();
        assert_eq!(history.index, 0);
    }

    #[test]
    fn a_deleted_version_compares_the_absent_after_side() -> Result<(), Box<dyn std::error::Error>>
    {
        let ctx = egui::Context::default();
        let mut deleted = version(Predecessor::Linked, Some(HistoryKind::Deleted));
        deleted.side = None;
        let timeline = Timeline::new(
            vec![
                deleted,
                version(Predecessor::Oldest, Some(HistoryKind::Added)),
            ],
            true,
            None,
        );
        let mut history = view(timeline);
        let mut diff = DiffView::default();
        assert!(history.prepare(&mut diff, &ctx).is_none());
        let capabilities = diff.capabilities();
        assert!(capabilities.diff);
        assert!(capabilities.full_before);
        assert!(!capabilities.full_after);
        let change = diff.change.as_ref().ok_or("deleted edge")?;
        assert!(change.old.is_some());
        assert!(change.new.is_none());
        Ok(())
    }

    #[test]
    fn an_absent_root_version_has_no_content_to_render() {
        let ctx = egui::Context::default();
        let mut absent = version(Predecessor::Oldest, Some(HistoryKind::Deleted));
        absent.side = None;
        let mut history = view(Timeline::new(vec![absent], true, None));
        let mut diff = DiffView::default();
        assert_eq!(
            history.prepare(&mut diff, &ctx),
            Some("The file is absent in this version.")
        );
    }

    #[test]
    fn the_strip_offers_every_predecessor_choice() {
        let ctx = egui::Context::default();
        let choices = vec![
            VersionCandidate {
                commit: "b".repeat(40),
                path_bytes: b"src/lib.rs".to_vec(),
                path: "src/lib.rs".into(),
                side: Some(side("b")),
                subject: "Older branch".into(),
                author: "Ada".into(),
                time: 1,
                kind: HistoryKind::Modified,
                old_path_bytes: None,
                old_path: None,
            },
            VersionCandidate {
                commit: "c".repeat(40),
                path_bytes: b"src/new.rs".to_vec(),
                path: "src/new.rs".into(),
                side: Some(side("c")),
                subject: "Renamed elsewhere".into(),
                author: "Bob".into(),
                time: 2,
                kind: HistoryKind::Renamed,
                old_path_bytes: Some(b"old.rs".to_vec()),
                old_path: Some("old.rs".into()),
            },
        ];
        let mut history = view(Timeline::new(
            vec![version(Predecessor::Candidates { choices }, None)],
            true,
            None,
        ));
        let output = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(720.0, 480.0, vec![]),
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| history.draw_list(ui, &ctx));
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
        assert!(text.contains(&"CHOOSE EARLIER VERSION 2"), "{text:?}");
        assert_eq!(
            history.rows(),
            [
                Row::Version(0),
                Row::Choose(2),
                Row::Candidate(0),
                Row::Candidate(1)
            ],
            "candidates nest under the merge row as list rows"
        );
        assert!(text.iter().any(|line| line.contains("Older branch")));
        assert!(
            text.iter()
                .any(|line| line.contains("Renamed elsewhere") && line.contains("← old.rs")),
            "{text:?}"
        );
        // Down then Enter picks the second candidate.
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(720.0, 480.0, vec![key(egui::Key::ArrowDown)]),
            |ui| {
                history.keys(ui.ctx());
            },
        );
        assert_eq!(history.choice, 1);
    }
}
