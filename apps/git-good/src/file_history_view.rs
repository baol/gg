//! A bounded newest-to-oldest content-version strip for the shared reader.
//!
//! History changes the reader's comparison source. It owns the worker-backed
//! timeline and the current version, but it never renders a second content
//! viewer: `prepare` hands the current edge to the review's `DiffView`.
use crate::{
    bounded_text,
    diff_view::DiffView,
    file_history_worker::{Update, Worker},
    navigation,
};
use eframe::egui;
use gg_engine::{
    file_history::{FileVersion, HistoryKind, PRELOAD, Predecessor, Timeline, VersionCandidate},
    review::{ChangeKind, FileChange},
};
use std::path::PathBuf;

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
}

impl FileHistory {
    pub fn new(
        repo: PathBuf,
        revision: String,
        file: String,
        path_bytes: Vec<u8>,
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
        let Some(update) = self.worker.as_ref().and_then(Worker::poll) else {
            return;
        };
        self.accept(update);
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
        if let Some(index) = self.choosing
            && self
                .timeline
                .as_ref()
                .is_some_and(|timeline| timeline.choices(index).is_none())
        {
            self.choosing = None;
        }
    }

    /// True when the strip's close control was clicked.
    pub fn take_close(&mut self) -> bool {
        std::mem::take(&mut self.close)
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

    fn current(&self) -> Option<&FileVersion> {
        self.timeline.as_ref()?.current(self.index)
    }

    fn last(&self) -> Option<&FileVersion> {
        self.timeline.as_ref()?.versions.last()
    }

    fn can_older(&self) -> bool {
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

    fn can_newer(&self) -> bool {
        self.index > 0
    }

    fn older_reason(&self) -> Option<String> {
        self.timeline
            .as_ref()?
            .before_reason(self.index, self.pending())
    }

    fn newer_reason(&self) -> Option<&'static str> {
        self.timeline.as_ref()?.after_reason(self.index)
    }

    fn older(&mut self, ctx: &egui::Context) {
        let Some(timeline) = &self.timeline else {
            return;
        };
        if self.index + 1 < timeline.versions.len() {
            self.index += 1;
            self.ensure_loaded(ctx);
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
            return Some(if self.error.is_some() {
                "File history unavailable"
            } else {
                "Loading file history…"
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
        let older =
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::ArrowLeft));
        if older {
            self.older(ctx);
        }
        let newer =
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::ArrowRight));
        if newer {
            self.newer();
        }
        ctx.input_mut(|input| {
            input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                || input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
        })
    }

    /// Draw the compact version strip directly above the shared reader.
    pub fn draw_strip(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal_wrapped(|ui| {
            ui.strong("File history");
            ui.separator();
            if ui
                .add_enabled(self.can_older(), egui::Button::new("Older version"))
                .on_disabled_hover_text(
                    self.older_reason()
                        .unwrap_or_else(|| "The earlier version is not available".into()),
                )
                .clicked()
            {
                self.older(ctx);
            }
            if ui
                .add_enabled(self.can_newer(), egui::Button::new("Newer version"))
                .on_disabled_hover_text("Newest version")
                .clicked()
            {
                self.newer();
            }
            if let Some(timeline) = &self.timeline {
                ui.weak(timeline.position(self.index));
            } else if self.error.is_some() {
                ui.weak("History unavailable");
            } else {
                ui.weak("Loading versions…");
            }
        });
        if let Some(version) = self.current().cloned() {
            let kind = version.kind.map(HistoryKind::label).unwrap_or("Version");
            let line = format!("{kind} · {} · {}", version.short_commit(), version.subject);
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 132.0).max(80.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(width, 18.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        bounded_text::field(ui, &line, egui::FontId::monospace(11.0), 18.0);
                    },
                );
                if ui.button("Close file history").clicked() {
                    self.close = true;
                }
            });
            let mut detail = format!("{} · {}", version.author, date(version.time));
            if let Some(old) = &version.old_path
                && old != &version.path
            {
                detail.push_str(&format!(" · from {old}"));
            }
            let reason = if !self.can_older() {
                self.older_reason()
            } else if !self.can_newer() {
                self.newer_reason().map(str::to_owned)
            } else {
                None
            };
            let note = self
                .timeline
                .as_ref()
                .and_then(|timeline| timeline.note.clone());
            for part in [reason, note].into_iter().flatten() {
                detail.push_str(" · ");
                detail.push_str(&part);
            }
            bounded_text::field(ui, &detail, egui::FontId::monospace(11.0), 18.0);
        }
        if self.current().is_none() {
            ui.horizontal(|ui| {
                if ui.button("Close file history").clicked() {
                    self.close = true;
                }
            });
        }
        if let Some(error) = self.error.clone() {
            let mut retry = false;
            ui.horizontal(|ui| {
                let retry_width = 64.0;
                let width = (ui.available_width() - retry_width - 8.0).max(80.0);
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(width, 18.0), egui::Sense::hover());
                bounded_text::paint(
                    ui,
                    rect,
                    &format!("History page failed · {error}"),
                    egui::FontId::proportional(12.0),
                    ui.visuals().error_fg_color,
                );
                if ui.button("Retry").clicked() {
                    retry = true;
                }
            });
            if retry {
                self.retry(ctx);
            }
        }
        if self
            .timeline
            .as_ref()
            .is_some_and(|timeline| timeline.choices(self.index).is_some())
        {
            self.chooser(ui, ctx);
        }
    }

    fn chooser(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let Some(choices) = self
            .timeline
            .as_ref()
            .and_then(|timeline| timeline.choices(self.index))
        else {
            return;
        };
        let choices = choices.to_vec();
        ui.horizontal(|ui| {
            ui.strong("Choose the earlier version");
            ui.weak(format!("{} predecessors reach this version", choices.len()));
        });
        let disabled = self.choosing.is_some() || self.pending();
        egui::ScrollArea::vertical()
            .id_salt("file-history-choices")
            .max_height(132.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (index, choice) in choices.iter().enumerate() {
                    let label = choice_label(choice);
                    if ui
                        .add_enabled(
                            !disabled,
                            egui::Button::new(label)
                                .min_size(egui::vec2(ui.available_width(), 24.0)),
                        )
                        .on_hover_text("Follow this predecessor")
                        .clicked()
                    {
                        self.choose(index, ctx);
                    }
                }
            });
    }
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
    fn the_rendered_strip_names_the_version_and_old_path() {
        let ctx = egui::Context::default();
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
        history.index = 0;
        let output = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(1120.0, 740.0, vec![]),
            |ui| {
                history.draw_strip(ui, &ctx);
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
        assert!(text.iter().any(|line| line.contains("Version 1 of 2")));
        assert!(text.iter().any(|line| line.contains("Renamed")));
        assert!(text.iter().any(|line| line.contains("from old.rs")));
        assert!(text.iter().any(|line| line.contains("Newest version")));
        assert!(text.contains(&"Older version"));
        assert!(text.contains(&"Newer version"));
        assert!(!text.iter().any(|line| line.contains("< Before")));
        assert!(!text.iter().any(|line| line.contains("After >")));
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
                history.draw_strip(ui, &ctx);
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
        assert!(
            text.iter()
                .any(|line| line.contains("Choose the earlier version"))
        );
        assert!(
            text.iter()
                .any(|line| line.contains("Older branch") && !line.contains("from old.rs"))
        );
        assert!(
            text.iter()
                .any(|line| line.contains("Renamed elsewhere") && line.contains("from old.rs"))
        );
    }
}
