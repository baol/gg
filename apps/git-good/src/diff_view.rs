//! Virtualized source rows with pinned identity, fixed gutters, and horizontal scrolling.
use crate::{
    bounded_text,
    diff_worker::{Update, Worker},
};
use eframe::egui;
use gg_engine::{
    diff::{DiffKind, DiffRow, FileDiff, RowKind},
    review::FileChange,
};
use std::{collections::BTreeMap, ops::Range, path::PathBuf};
const HEIGHT: f32 = 20.0;
pub struct DiffView {
    worker: Option<Worker>,
    export: crate::export::Export,
    preview: Option<std::sync::Arc<crate::binary_preview::Pair>>,
    binary_view: crate::binary_preview::View,
    pub change: Option<FileChange>,
    working: Option<gg_engine::status::WorkingRequest>,
    displayed: Option<FileChange>,
    displayed_path: PathBuf,
    displayed_working: Option<gg_engine::status::WorkingRequest>,
    displayed_generation: u64,
    path: PathBuf,
    pub generation: u64,
    pub rows: BTreeMap<usize, DiffRow>,
    pub count: usize,
    selected_hunk: Option<(usize, String)>,
    columns: usize,
    info: Option<FileDiff>,
    error: Option<String>,
    range: Range<usize>,
    sent: Option<(u64, Range<usize>)>,
    rect: egui::Rect,
    bounds: egui::Rect,
    last_scroll: f64,
    staged: Option<Update>,
    offset: f32,
    reset: bool,
}
impl Default for DiffView {
    fn default() -> Self {
        Self {
            worker: None,
            export: crate::export::Export::default(),
            preview: None,
            binary_view: crate::binary_preview::View::default(),
            change: None,
            working: None,
            displayed: None,
            displayed_path: PathBuf::new(),
            displayed_working: None,
            displayed_generation: 0,
            path: PathBuf::new(),
            generation: 0,
            rows: BTreeMap::new(),
            count: 0,
            selected_hunk: None,
            columns: 0,
            info: None,
            error: None,
            range: 0..64,
            sent: None,
            rect: egui::Rect::NOTHING,
            bounds: egui::Rect::NOTHING,
            last_scroll: f64::NEG_INFINITY,
            staged: None,
            offset: 0.0,
            reset: false,
        }
    }
}
impl DiffView {
    pub fn export_open(&self) -> bool {
        self.export.is_open()
    }
    pub fn binary(&self) -> bool {
        self.preview.is_some()
    }
    pub fn hovered(&self, point: Option<egui::Pos2>) -> bool {
        point.is_some_and(|p| self.bounds.contains(p))
    }
    pub fn selected_hunk(&self) -> Option<&str> {
        self.selected_hunk
            .as_ref()
            .map(|(_, header)| header.as_str())
    }
    /// A selected hunk that the engine can stage: an unstaged text modification.
    pub fn stageable_hunk(&self) -> Option<(&gg_engine::status::WorkingRequest, &str)> {
        Some((self.stageable_working()?, self.selected_hunk()?))
    }
    fn stageable_working(&self) -> Option<&gg_engine::status::WorkingRequest> {
        let request = self.displayed_working.as_ref()?;
        if request.entry.area != gg_engine::status::Area::Unstaged
            || request.entry.kind != gg_engine::status::Kind::Modified
        {
            return None;
        }
        let info = self.info.as_ref()?;
        if info.kind != DiffKind::Text || !info.complete {
            return None;
        }
        Some(request)
    }
    pub fn select_working(
        &mut self,
        request: gg_engine::status::WorkingRequest,
        ctx: &egui::Context,
    ) {
        if self.working.as_ref() == Some(&request) {
            return;
        }
        self.path = request.repository.path.clone();
        self.change = Some(FileChange {
            path: request.entry.path.clone(),
            path_bytes: request.entry.path_bytes.clone(),
            kind: gg_engine::review::ChangeKind::Modified,
            old: None,
            new: None,
        });
        self.working = Some(request);
        self.generation += 1;
        self.staged = None;
        self.sent = None;
        self.error = None;
        self.selected_hunk = None;
        self.send(ctx);
        ctx.request_repaint();
    }
    pub fn select(&mut self, path: PathBuf, change: Option<FileChange>, ctx: &egui::Context) {
        let same = self.path == path
            && self
                .change
                .as_ref()
                .map(|c| (&c.path_bytes, &c.old, &c.new))
                == change.as_ref().map(|c| (&c.path_bytes, &c.old, &c.new));
        if same {
            return;
        }
        if change.is_none() {
            *self = Self::default();
            return;
        }
        self.path = path;
        self.change = change;
        self.working = None;
        self.generation += 1;
        self.error = None;
        self.staged = None;
        self.sent = None;
        self.selected_hunk = None;
        self.send(ctx);
        ctx.request_repaint();
    }
    fn send(&mut self, ctx: &egui::Context) {
        let Some(change) = &self.change else {
            return;
        };
        if self.worker.is_none() {
            match Worker::start(ctx.clone()) {
                Ok(w) => self.worker = Some(w),
                Err(e) => {
                    self.error = Some(e);
                    return;
                }
            }
        }
        let range = if self.displayed_generation == self.generation {
            self.range.clone()
        } else {
            0..64
        };
        if self.sent.as_ref() != Some(&(self.generation, range.clone()))
            && let Some(worker) = &self.worker
        {
            if let Some(working) = &self.working {
                worker.request_working(self.generation, working.clone(), range.clone());
            } else {
                worker.request(
                    self.generation,
                    self.path.clone(),
                    change.clone(),
                    range.clone(),
                );
            }
            self.sent = Some((self.generation, range));
        }
    }
    pub fn accept(&mut self, update: Update, pinned: Option<usize>) {
        if update.generation != self.generation {
            return;
        }
        match update.result {
            Ok(mut info) => {
                if self.displayed_generation != update.generation {
                    self.rows.clear();
                    self.offset = 0.0;
                    self.reset = true;
                    self.range = 0..64;
                    self.binary_view = crate::binary_preview::View::default();
                    self.displayed = self.change.clone();
                    self.displayed_path = self.path.clone();
                    self.displayed_working = self.working.clone();
                    self.displayed_generation = update.generation;
                    self.selected_hunk = None;
                }
                self.preview = update.preview;
                self.count = update.count;
                self.columns = update.columns;
                for (index, row) in info.rows.drain(..).enumerate() {
                    let index = update.page_start + index;
                    if pinned != Some(index) || self.rows.contains_key(&index) {
                        self.rows.insert(index, row);
                    }
                }
                self.info = Some(info);
                self.error = None;
                while self.rows.len() > 768 {
                    let victim = self
                        .rows
                        .keys()
                        .copied()
                        .filter(|i| !self.range.contains(i) && Some(*i) != pinned)
                        .max_by_key(|i| i.abs_diff(self.range.start));
                    if let Some(i) = victim {
                        self.rows.remove(&i);
                    } else {
                        break;
                    }
                }
            }
            Err(e) => self.error = Some(e),
        }
    }
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        self.draw_inner(ui, false);
    }
    pub fn draw_reading(&mut self, ui: &mut egui::Ui) {
        self.draw_inner(ui, true);
    }
    fn draw_inner(&mut self, ui: &mut egui::Ui, keyboard: bool) {
        self.bounds = ui.max_rect();
        let ctx = ui.ctx().clone();
        self.export.draw(&ctx);
        let (point, active, now) = ctx.input(|i| {
            (
                i.pointer.hover_pos(),
                i.pointer.any_down()
                    || i.raw_scroll_delta != egui::Vec2::ZERO
                    || i.smooth_scroll_delta != egui::Vec2::ZERO,
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
        let hovering = self.hovered(point);
        let pinned = point
            .filter(|p| self.rect.contains(*p))
            .map(|p| ((p.y - self.rect.top() + self.offset) / HEIGHT).floor() as usize);
        if let Some(update) = self.worker.as_ref().and_then(Worker::poll)
            && update.generation == self.generation
        {
            self.staged = Some(update);
        }
        // Initial rows wait until the reading area's hit targets are clear.
        if ((quiet && !hovering)
            || (self.info.is_some() && self.displayed_generation == self.generation))
            && let Some(update) = self.staged.clone()
        {
            self.accept(update, pinned);
            if pinned.is_none() {
                self.staged = None;
            }
        }
        let Some(change) = self.displayed.as_ref().or(self.change.as_ref()) else {
            ui.weak("Select a file to read its diff");
            return;
        };
        bounded_text::field(ui, &change.path, egui::FontId::proportional(14.0), 24.0);
        let displayed_working = if self.displayed.is_some() {
            &self.displayed_working
        } else {
            &self.working
        };
        if let Some(working) = displayed_working {
            let label = match working.entry.area {
                gg_engine::status::Area::Staged => "Staged: HEAD to index",
                gg_engine::status::Area::Unstaged => "Unstaged: index to working snapshot",
                gg_engine::status::Area::Untracked => "Untracked: empty to working snapshot",
                gg_engine::status::Area::Conflict => "Unmerged path",
            };
            bounded_text::field(ui, label, egui::FontId::proportional(12.0), 20.0);
        } else {
            let side = |s: &Option<gg_engine::review::FileSide>| {
                s.as_ref()
                    .map(|s| format!("{:06o} {}", s.mode, s.id))
                    .unwrap_or_else(|| "absent".into())
            };
            bounded_text::field(
                ui,
                &format!("{} to {}", side(&change.old), side(&change.new)),
                egui::FontId::monospace(10.0),
                20.0,
            );
            ui.horizontal(|ui| {
                for (label, side) in [
                    ("Export before", &change.old),
                    ("Export after", &change.new),
                ] {
                    if ui
                        .add_enabled(
                            self.displayed.is_some()
                                && side.as_ref().is_some_and(|s| s.mode != 0o160000)
                                && !self.export.busy(),
                            egui::Button::new(label),
                        )
                        .clicked()
                        && let Some(side) = side
                    {
                        self.export.choose(
                            self.displayed_path.clone(),
                            side.clone(),
                            format!("{label} · {}", change.path),
                        );
                    }
                }
            });
        }
        if let Some(error) = &self.error {
            bounded_text::field(ui, error, egui::FontId::proportional(12.0), 22.0);
            if ui.button("Retry diff").clicked() {
                self.generation += 1;
                self.sent = None;
                self.error = None;
                self.send(&ctx);
            }
        }
        let pending = self.displayed_generation != self.generation;
        let status = if pending && self.error.is_some() {
            "Showing previous diff".into()
        } else if pending {
            format!(
                "Loading {} · previous diff retained",
                self.change.as_ref().map_or("file", |c| c.path.as_str())
            )
        } else {
            String::new()
        };
        if let Some(info) = &self.info {
            bounded_text::field(
                ui,
                &format!(
                    "Before: {} bytes · After: {} bytes{}",
                    info.old_size,
                    info.new_size,
                    if info.complete {
                        ""
                    } else {
                        " · text comparison incomplete"
                    }
                ),
                egui::FontId::proportional(11.0),
                18.0,
            );
            bounded_text::field(
                ui,
                if pending { &status } else { &info.note },
                egui::FontId::proportional(12.0),
                22.0,
            );
        } else {
            bounded_text::field(
                ui,
                if self.error.is_some() {
                    "Diff unavailable"
                } else {
                    "Loading selected-file diff…"
                },
                egui::FontId::proportional(11.0),
                18.0,
            );
            bounded_text::field(ui, "", egui::FontId::proportional(12.0), 22.0);
        }
        ui.separator();
        self.rect = ui.available_rect_before_wrap();
        if let Some(preview) = &self.preview {
            self.binary_view.draw(ui, preview, keyboard);
            return;
        }
        if self.count == 0 {
            self.send(&ctx);
            return;
        }
        let mut scroll = egui::ScrollArea::both()
            .id_salt(("file-diff", self.displayed_generation))
            .auto_shrink([false, false]);
        if self.reset {
            scroll = scroll.scroll_offset(egui::Vec2::ZERO);
            self.reset = false;
        }
        if keyboard && crate::navigation::available(&ctx) {
            let page = (self.rect.height() / HEIGHT).floor().max(2.0) as isize - 1;
            let step = crate::navigation::step(&ctx, page);
            if step != 0 {
                self.offset = (self.offset + step as f32 * HEIGHT).clamp(
                    0.0,
                    (self.count as f32 * HEIGHT - self.rect.height()).max(0.0),
                );
                scroll = scroll.vertical_scroll_offset(self.offset);
            }
        }
        ui.spacing_mut().item_spacing.y = 0.0;
        let working = self.stageable_working().is_some();
        let result = scroll.show_rows(ui, HEIGHT, self.count, |ui, rows| {
            self.range = rows.start.saturating_sub(24)..rows.end.saturating_add(24);
            // A conservative monospace width keeps every source column reachable.
            let width = (120.0 + self.columns as f32 * 16.0).max(ui.available_width());
            for index in rows {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(width, HEIGHT), egui::Sense::click());
                let Some(row) = self.rows.get(&index) else {
                    ui.painter().text(
                        rect.left_center(),
                        egui::Align2::LEFT_CENTER,
                        "Loading…",
                        egui::FontId::monospace(12.0),
                        ui.visuals().weak_text_color(),
                    );
                    continue;
                };
                let (sign, color, fill) = crate::colors::diff(ui.visuals(), row.kind);
                ui.painter().rect_filled(rect, 0.0, fill);
                let selected = working
                    && row.kind == RowKind::Hunk
                    && self
                        .selected_hunk
                        .as_ref()
                        .is_some_and(|(selected, _)| *selected == index);
                if selected {
                    ui.painter()
                        .rect_filled(rect, 0.0, crate::colors::selection(ui.visuals()));
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(rect.min, egui::vec2(3.0, HEIGHT)),
                        0.0,
                        ui.visuals().selection.stroke.color,
                    );
                }
                let font = egui::FontId::monospace(12.0);
                for (x, text) in [
                    (0.0, row.old_line.map(|n| n.to_string()).unwrap_or_default()),
                    (
                        48.0,
                        row.new_line.map(|n| n.to_string()).unwrap_or_default(),
                    ),
                    (96.0, sign.into()),
                ] {
                    ui.painter()
                        .with_clip_rect(
                            egui::Rect::from_min_size(
                                rect.min + egui::vec2(x, 0.0),
                                egui::vec2(if x == 96.0 { 16.0 } else { 46.0 }, HEIGHT),
                            )
                            .intersect(ui.clip_rect()),
                        )
                        .text(
                            rect.min + egui::vec2(x + 4.0, HEIGHT / 2.0),
                            egui::Align2::LEFT_CENTER,
                            text,
                            font.clone(),
                            ui.visuals().weak_text_color(),
                        );
                }
                let origin = rect.min + egui::vec2(116.0, HEIGHT / 2.0);
                let clip = egui::Rect::from_min_max(rect.min + egui::vec2(116.0, 0.0), rect.max)
                    .intersect(ui.clip_rect());
                // Shape only visible source characters, including at deep horizontal offsets.
                let (start, end, advance) = ui.fonts_mut(|fonts| {
                    visible_text(
                        &row.text,
                        clip.left() - origin.x,
                        clip.right() - origin.x,
                        |ch| fonts.glyph_width(&font, ch),
                    )
                });
                ui.painter().with_clip_rect(clip).text(
                    origin + egui::vec2(advance, 0.0),
                    egui::Align2::LEFT_CENTER,
                    &row.text[start..end],
                    font,
                    color,
                );
                if response
                    .on_hover_text(if working && row.kind == RowKind::Hunk {
                        "Click to select this hunk for staging"
                    } else {
                        "Click to copy this displayed line"
                    })
                    .clicked()
                {
                    if working && row.kind == RowKind::Hunk {
                        self.selected_hunk = Some((index, row.text.clone()));
                    } else {
                        ctx.copy_text(row.text.clone());
                    }
                }
            }
        });
        self.offset = result.state.offset.y;
        self.send(&ctx);
    }
}

fn visible_text(
    text: &str,
    left: f32,
    right: f32,
    mut width: impl FnMut(char) -> f32,
) -> (usize, usize, f32) {
    let mut x = 0.0;
    let mut start = text.len();
    let mut end = text.len();
    let mut advance = 0.0;
    for (index, ch) in text.char_indices() {
        if x > right {
            end = index;
            break;
        }
        let next = x + width(ch);
        if start == text.len() && next >= left {
            start = index;
            advance = x;
        }
        x = next;
    }
    (start.min(end), end, advance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gg_engine::diff::{DiffKind, RowKind};
    #[test]
    fn selection_keeps_displayed_diff_until_replacement_arrives() {
        let ctx = egui::Context::default();
        let mut view = DiffView::default();
        let file = |name: &str| FileChange {
            path: name.into(),
            path_bytes: name.as_bytes().into(),
            kind: gg_engine::review::ChangeKind::Added,
            old: None,
            new: None,
        };
        view.select("/repo".into(), Some(file("a")), &ctx);
        let mut doc = FileDiff::summary(DiffKind::Text, 0, 1, "");
        doc.rows.push(DiffRow {
            kind: RowKind::Added,
            old_line: None,
            new_line: Some(1),
            text: "first".into(),
            bytes: b"first".to_vec(),
        });
        let update = Update {
            generation: view.generation,
            preview: None,
            count: 1,
            columns: 5,
            page_start: 0,
            result: Ok(doc),
        };
        view.accept(update.clone(), None);
        view.select("/repo".into(), Some(file("b")), &ctx);
        assert_eq!(view.count, 1);
        assert_eq!(view.rows[&0].text, "first");
        assert_eq!(view.displayed.as_ref().map(|f| f.path.as_str()), Some("a"));
        view.accept(update.clone(), None); // A late old page cannot adopt the requested identity.
        assert_eq!(view.displayed.as_ref().map(|f| f.path.as_str()), Some("a"));
        let mut failure = update.clone();
        failure.generation = view.generation;
        failure.result = Err("Unavailable object".into());
        view.accept(failure, None);
        assert_eq!(view.rows[&0].text, "first");
        assert_eq!(view.displayed.as_ref().map(|f| f.path.as_str()), Some("a"));
        let mut replacement = update;
        replacement.generation = view.generation;
        if let Ok(doc) = &mut replacement.result {
            doc.rows[0].text = "second".into();
        }
        view.accept(replacement, None);
        assert_eq!(view.rows[&0].text, "second");
    }
    #[test]
    fn reading_keys_scroll_without_changing_file_and_replacement_waits_for_hover() {
        let ctx = egui::Context::default();
        let mut view = DiffView::default();
        let file = FileChange {
            path: "a".into(),
            path_bytes: b"a".to_vec(),
            kind: gg_engine::review::ChangeKind::Added,
            old: None,
            new: None,
        };
        view.change = Some(file.clone());
        view.generation = 1;
        let mut doc = FileDiff::summary(DiffKind::Text, 0, 100, "");
        doc.rows = (0..100)
            .map(|i| DiffRow {
                kind: RowKind::Added,
                old_line: None,
                new_line: Some(i + 1),
                text: format!("row {i}"),
                bytes: vec![],
            })
            .collect();
        let update = Update {
            generation: 1,
            preview: None,
            count: 100,
            columns: 8,
            page_start: 0,
            result: Ok(doc),
        };
        view.accept(update.clone(), None);
        view.change = None; // Isolate rendered input from asynchronous repository reads.

        let frame = |view: &mut DiffView, events| {
            let _ = ctx.run(crate::tests::frame_input(720.0, 480.0, events), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| view.draw_reading(ui));
            });
        };
        frame(&mut view, vec![]);
        for key in [egui::Key::ArrowDown, egui::Key::J, egui::Key::PageDown] {
            let previous = view.offset;
            frame(
                &mut view,
                vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert!(
                view.offset > previous,
                "{key:?}: {previous} -> {}, count {}, rect {:?}",
                view.offset,
                view.count,
                view.rect
            );
            assert_eq!(view.displayed.as_ref().map(|f| &f.path), Some(&file.path));
        }
        let pointer = view.rect.center();
        view.generation = 2;
        let mut next = update;
        next.generation = 2;
        view.staged = Some(next);
        frame(&mut view, vec![egui::Event::PointerMoved(pointer)]);
        assert_eq!(view.displayed_generation, 1);
        frame(&mut view, vec![egui::Event::PointerGone]);
        assert_eq!(view.displayed_generation, 2);
    }
    #[test]
    fn byte_sizes_identify_each_side_in_the_rendered_header() {
        let ctx = egui::Context::default();
        let mut view = DiffView {
            change: Some(FileChange {
                path_bytes: b"image.png".to_vec(),
                path: "image.png".into(),
                kind: gg_engine::review::ChangeKind::Modified,
                old: None,
                new: None,
            }),
            info: Some(FileDiff::summary(DiffKind::Binary, 256, 268, "")),
            ..Default::default()
        };
        let output = ctx.run(crate::tests::frame_input(720.0, 480.0, vec![]), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| view.draw(ui));
        });
        assert!(output.shapes.iter().any(|shape| {
            matches!(&shape.shape, egui::Shape::Text(text)
                if text.galley.job.text == "Before: 256 bytes · After: 268 bytes")
        }));
    }
    #[test]
    fn horizontal_window_shapes_only_visible_unicode() {
        let text = "界".repeat(100_000);
        let (start, end, advance) = visible_text(&text, 600_000.0, 600_120.0, |_| 12.0);
        assert!(text.is_char_boundary(start) && text.is_char_boundary(end));
        assert!(text[start..end].chars().count() <= 12);
        assert!((599_988.0..=600_000.0).contains(&advance));
    }
    #[test]
    fn stale_pages_and_hovered_placeholders_preserve_displayed_rows() {
        let mut view = DiffView {
            generation: 2,
            ..Default::default()
        };
        let mut doc = FileDiff::summary(DiffKind::Text, 0, 1, "");
        doc.rows.push(DiffRow {
            kind: RowKind::Added,
            old_line: None,
            new_line: Some(1),
            text: "new".into(),
            bytes: b"new".to_vec(),
        });
        let mut update = Update {
            generation: 1,
            preview: None,
            count: 1,
            columns: 3,
            page_start: 0,
            result: Ok(doc),
        };
        view.accept(update.clone(), None);
        assert_eq!(view.count, 0);
        update.generation = 2;
        view.accept(update.clone(), Some(0));
        assert_eq!(view.count, 1);
        assert!(view.rows.is_empty());
        view.accept(update, None);
        assert_eq!(view.rows[&0].text, "new");
    }
}

#[cfg(test)]
mod performance {
    use super::*;
    use gg_engine::review::{ReviewReader, ReviewRequest, ReviewStep};
    use std::time::{Duration, Instant};

    #[test]
    #[ignore = "set GG_STRESS_REPOSITORY to scripts/review-stress-fixture.py output; use release mode"]
    fn packed_diff_stress_and_rapid_selection() -> Result<(), Box<dyn std::error::Error>> {
        let path: PathBuf = std::env::var_os("GG_STRESS_REPOSITORY")
            .ok_or("GG_STRESS_REPOSITORY is required")?
            .into();
        let mut reader = gg_git::review::ReviewSession::open(&ReviewRequest {
            path: path.clone(),
            revision: "HEAD".into(),
            parent: None,
        })?;
        let mut files = Vec::new();
        loop {
            match reader.step()? {
                ReviewStep::Change(file) => files.push(file),
                ReviewStep::Complete => break,
                ReviewStep::Pending => {}
            }
        }
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "changed.txt");
        assert_eq!(files[1].path, "image.png");
        let ctx = egui::Context::default();
        let mut view = DiffView::default();
        let mut frames = Vec::new();
        let frame = |view: &mut DiffView| {
            let start = Instant::now();
            let _ = ctx.run(crate::tests::frame_input(1120.0, 740.0, vec![]), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| view.draw(ui));
            });
            assert!(view.rows.len() <= 768);
            start.elapsed()
        };
        let mut durations = Vec::new();
        for file in &files {
            let start = Instant::now();
            view.select(path.clone(), Some(file.clone()), &ctx);
            while view.info.is_none() {
                frames.push(frame(&mut view));
                assert!(view.error.is_none(), "{:?}", view.error);
                assert!(start.elapsed() < Duration::from_secs(10));
                std::thread::sleep(Duration::from_millis(1));
            }
            durations.push(start.elapsed());
            if file.path == "changed.txt" {
                assert!(start.elapsed() < Duration::from_millis(500));
                let info = view.info.as_ref().ok_or("missing text result")?;
                assert!(info.complete || info.note.contains("budget"));
            } else {
                let preview = view.preview.as_ref().ok_or("missing preview")?;
                for side in [&preview.before, &preview.after] {
                    assert_eq!(
                        side.image.as_ref().ok_or("missing image")?.size,
                        [1024, 512]
                    );
                    assert!(side.note.contains("4096 × 2048"));
                }
            }
        }
        // Supersede real image reads/decodes and then let the final text request settle.
        for _ in 0..8 {
            view.select(path.clone(), Some(files[0].clone()), &ctx);
            frame(&mut view);
            view.select(path.clone(), Some(files[1].clone()), &ctx);
            frame(&mut view);
            std::thread::sleep(Duration::from_millis(5));
        }
        view.select(path.clone(), Some(files[0].clone()), &ctx);
        let final_generation = view.generation;
        let start = Instant::now();
        while view.info.is_none() {
            frames.push(frame(&mut view));
            assert!(view.error.is_none(), "{:?}", view.error);
            assert!(start.elapsed() < Duration::from_secs(10));
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(view.generation, final_generation);
        assert_eq!(
            view.change.as_ref().ok_or("missing selection")?.path,
            "changed.txt"
        );
        assert!(
            view.preview
                .as_ref()
                .is_none_or(|pair| pair.before.image.is_none() && pair.after.image.is_none())
        );
        assert_eq!(
            view.info.as_ref().ok_or("missing result")?.new_size,
            1024 * 1024
        );
        frames.sort();
        eprintln!(
            "fully changed text={:?}; 8MP image pair={:?}; working frame p95={:?}; final selection={:?}",
            durations[0],
            durations[1],
            frames[frames.len() * 95 / 100],
            start.elapsed()
        );
        assert!(frames[frames.len() * 95 / 100] < Duration::from_millis(16));
        Ok(())
    }
}
