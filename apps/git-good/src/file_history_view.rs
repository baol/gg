//! A bounded, newest-to-oldest content timeline for the selected file.
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
    diff: DiffView,
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
            diff: DiffView::default(),
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
        self.diff = DiffView::default();
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

    fn retry(&mut self, ctx: &egui::Context) {
        self.generation += 1;
        self.start(ctx);
    }

    fn poll(&mut self) {
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

    fn pending(&self) -> bool {
        self.busy
    }

    fn current(&self) -> Option<&FileVersion> {
        self.timeline.as_ref()?.current(self.index)
    }

    fn last(&self) -> Option<&FileVersion> {
        self.timeline.as_ref()?.versions.last()
    }

    fn can_before(&self) -> bool {
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

    fn can_after(&self) -> bool {
        self.index > 0
    }

    fn before_reason(&self) -> Option<String> {
        self.timeline
            .as_ref()?
            .before_reason(self.index, self.pending())
    }

    fn after_reason(&self) -> Option<&'static str> {
        self.timeline.as_ref()?.after_reason(self.index)
    }

    fn before(&mut self, ctx: &egui::Context) {
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

    fn after(&mut self) {
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

    /// Keep a short lookahead so a Before step is usually already loaded.
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
        let after = format!("{} · {}", version.short_commit(), version.path);
        let before = match &version.previous {
            Predecessor::Linked => timeline
                .current(self.index + 1)
                .map(|older| format!("{} · {}", older.short_commit(), older.path))
                .unwrap_or_else(|| "Earlier version".into()),
            Predecessor::Oldest => "Absent".into(),
            _ => "Earlier version not loaded".into(),
        };
        Some((before, after))
    }

    fn keys(&mut self, ctx: &egui::Context) -> bool {
        if !navigation::available(ctx) {
            return false;
        }
        let back =
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if back {
            return true;
        }
        let before =
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::ArrowLeft));
        if before {
            self.before(ctx);
        }
        let after =
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::ArrowRight));
        if after {
            self.after();
        }
        false
    }

    fn header(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> bool {
        let mut back = false;
        ui.horizontal(|ui| {
            if ui.button("Back to file").clicked() {
                back = true;
            }
            ui.separator();
            ui.strong("File history");
            ui.weak(&self.file);
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(self.can_before(), egui::Button::new("< Before"))
                .on_disabled_hover_text(
                    self.before_reason()
                        .unwrap_or_else(|| "The earlier version is not available".into()),
                )
                .clicked()
            {
                self.before(ctx);
            }
            if ui
                .add_enabled(self.can_after(), egui::Button::new("After >"))
                .clicked()
            {
                self.after();
            }
            ui.separator();
            if let Some(timeline) = &self.timeline {
                ui.weak(timeline.position(self.index));
            } else {
                ui.weak("Loading versions…");
            }
            if let Some(reason) = self.before_reason() {
                ui.weak(reason);
            } else if let Some(reason) = self.after_reason() {
                ui.weak(reason);
            }
        });
        if let Some(version) = self.current() {
            let kind = version.kind.map(HistoryKind::label).unwrap_or("Version");
            bounded_text::field(
                ui,
                &format!("{kind} · {} · {}", version.short_commit(), version.subject),
                egui::FontId::monospace(11.0),
                18.0,
            );
            let mut detail = format!("{} · {}", version.author, date(version.time));
            if let Some(old) = &version.old_path
                && old != &version.path
            {
                detail.push_str(&format!(" · from {old}"));
            }
            bounded_text::field(ui, &detail, egui::FontId::monospace(11.0), 18.0);
        }
        if self.error.is_some() && self.timeline.is_some() && ui.button("Retry").clicked() {
            self.retry(ctx);
        }
        back
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

    fn content(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if self
            .timeline
            .as_ref()
            .is_some_and(|timeline| timeline.choices(self.index).is_some())
        {
            ui.weak("Choose one of the earlier versions above to compare it with this version.");
            return;
        }
        match self.edge() {
            Some((change, known)) => {
                self.diff.set_side_labels("Old", "New");
                if let Some((before, after)) = self.edge_labels() {
                    self.diff.set_endpoints(before, after);
                }
                self.diff
                    .select_edge(self.repo.clone(), Some(change), known, ctx);
                self.diff.draw_reading(ui);
            }
            None => {
                let text = if self.current().is_some() {
                    "This version has no file content."
                } else {
                    "The file is absent in this version."
                };
                ui.weak(text);
            }
        }
    }

    /// Return true when Back to file is requested.
    pub fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        self.poll();
        if self.keys(ctx) {
            return true;
        }
        let mut back = false;
        egui::Panel::top("file-history-header")
            .frame(egui::Frame::side_top_panel(ui.style().as_ref()).inner_margin(8.0))
            .show(ui, |ui| {
                back = self.header(ui, ctx);
            });
        if self
            .timeline
            .as_ref()
            .is_some_and(|timeline| timeline.choices(self.index).is_some())
        {
            egui::Panel::top("file-history-chooser")
                .frame(egui::Frame::side_top_panel(ui.style().as_ref()).inner_margin(8.0))
                .show(ui, |ui| self.chooser(ui, ctx));
        }
        egui::Panel::bottom("file-history-status")
            .exact_size(26.0)
            .show(ui, |ui| {
                let text = if let Some(error) = &self.error {
                    error.as_str()
                } else if self.pending() {
                    "Loading earlier versions…"
                } else {
                    navigation::HISTORY_HINT
                };
                bounded_text::field(ui, text, egui::FontId::proportional(12.0), 18.0);
            });
        egui::CentralPanel::default().show(ui, |ui| {
            if self.timeline.is_none() {
                ui.weak(if self.error.is_some() {
                    "File history unavailable"
                } else {
                    "Loading file history…"
                });
                if self.error.is_some() && ui.button("Retry").clicked() {
                    self.retry(ctx);
                }
                return;
            }
            self.content(ui, ctx);
        });
        bounded_text::show_disclosure(ctx);
        back
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
        FileHistory {
            generation: 1,
            repo: "/repo".into(),
            revision: "HEAD".into(),
            file: "src/lib.rs".into(),
            path_bytes: b"src/lib.rs".to_vec(),
            worker: None,
            timeline: Some(timeline),
            index: 0,
            requested: PRELOAD,
            busy: false,
            choosing: None,
            error: None,
            diff: DiffView::default(),
        }
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
        assert!(history.can_before());
        assert!(!history.can_after());
        assert_eq!(history.after_reason(), Some("Newest version"));
        assert_eq!(history.before_reason(), None);

        history.index = 1;
        assert!(!history.can_before());
        assert!(history.can_after());
        assert_eq!(history.before_reason().as_deref(), Some("Oldest version"));
        assert_eq!(history.after_reason(), None);
        history.after();
        assert_eq!(history.index, 0);
        history.after();
        assert_eq!(history.index, 0);
    }

    #[test]
    fn a_loading_predecessor_disables_before_and_names_the_reason() {
        let mut history = view(Timeline::new(
            vec![version(Predecessor::More, Some(HistoryKind::Modified))],
            false,
            None,
        ));
        history.busy = true;
        assert!(!history.can_before());
        assert_eq!(
            history.before_reason().as_deref(),
            Some("Loading earlier versions…")
        );
        history.busy = false;
        assert!(history.can_before());
        assert_eq!(history.before_reason(), None);
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
        assert!(!history.can_before());
        assert_eq!(
            history.before_reason().as_deref(),
            Some("Choose one of 1 predecessors")
        );
    }

    #[test]
    fn the_edge_uses_old_and_new_sides_and_an_unknown_edge_has_no_diff()
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
    fn dates_are_utc_and_handle_the_epoch() {
        assert_eq!(date(0), "1970-01-01 00:00 UTC");
        assert_eq!(date(1_767_268_800), "2026-01-01 12:00 UTC");
        assert_eq!(date(1_767_268_800 + 86_400), "2026-01-02 12:00 UTC");
    }

    #[test]
    fn the_rendered_header_names_the_version_and_old_path() {
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
                history.draw(ui);
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
    }
}
