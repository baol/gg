//! Virtualized source rows with pinned identity, fixed gutters, and horizontal scrolling.
use crate::{
    bounded_text,
    diff_worker::{Update, Worker},
};
use eframe::egui;
use gg_engine::{
    diff::{DiffRow, FileDiff, RowKind},
    review::FileChange,
};
use std::{collections::BTreeMap, ops::Range, path::PathBuf};
const HEIGHT: f32 = 20.0;
pub struct DiffView {
    worker: Option<Worker>,
    pub change: Option<FileChange>,
    path: PathBuf,
    pub generation: u64,
    pub rows: BTreeMap<usize, DiffRow>,
    pub count: usize,
    columns: usize,
    info: Option<FileDiff>,
    error: Option<String>,
    range: Range<usize>,
    sent: Option<(u64, Range<usize>)>,
    rect: egui::Rect,
    last_scroll: f64,
    staged: Option<Update>,
    offset: f32,
    reset: bool,
}
impl Default for DiffView {
    fn default() -> Self {
        Self {
            worker: None,
            change: None,
            path: PathBuf::new(),
            generation: 0,
            rows: BTreeMap::new(),
            count: 0,
            columns: 0,
            info: None,
            error: None,
            range: 0..64,
            sent: None,
            rect: egui::Rect::NOTHING,
            last_scroll: f64::NEG_INFINITY,
            staged: None,
            offset: 0.0,
            reset: false,
        }
    }
}
impl DiffView {
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
        self.path = path;
        self.change = change;
        self.generation += 1;
        self.rows.clear();
        self.count = 0;
        self.columns = 0;
        self.info = None;
        self.error = None;
        self.staged = None;
        self.range = 0..64;
        self.sent = None;
        self.offset = 0.0;
        self.reset = true;
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
        if self.sent.as_ref() != Some(&(self.generation, self.range.clone()))
            && let Some(worker) = &self.worker
        {
            worker.request(
                self.generation,
                self.path.clone(),
                change.clone(),
                self.range.clone(),
            );
            self.sent = Some((self.generation, self.range.clone()));
        }
    }
    pub fn accept(&mut self, update: Update, pinned: Option<usize>) {
        if update.generation != self.generation {
            return;
        }
        match update.result {
            Ok(mut info) => {
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
        let ctx = ui.ctx().clone();
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
        let hovering = point.is_some_and(|p| self.rect.contains(p));
        let pinned = point
            .filter(|p| self.rect.contains(*p))
            .map(|p| ((p.y - self.rect.top() + self.offset) / HEIGHT).floor() as usize);
        if let Some(update) = self.worker.as_ref().and_then(Worker::poll)
            && update.generation == self.generation
        {
            self.staged = Some(update);
        }
        // Initial rows wait until the reading area's hit targets are clear.
        if (quiet && !hovering) || self.info.is_some() {
            if let Some(update) = self.staged.clone() {
                self.accept(update, pinned);
                if pinned.is_none() {
                    self.staged = None;
                }
            }
        }
        let Some(change) = &self.change else {
            ui.weak("Select a file to read its diff");
            return;
        };
        bounded_text::field(ui, &change.path, egui::FontId::proportional(14.0), 24.0);
        let side = |s: &Option<gg_engine::review::FileSide>| {
            s.as_ref()
                .map(|s| format!("{:06o} {}", s.mode, s.id))
                .unwrap_or_else(|| "absent".into())
        };
        bounded_text::field(
            ui,
            &format!("{} → {}", side(&change.old), side(&change.new)),
            egui::FontId::monospace(10.0),
            20.0,
        );
        if let Some(error) = &self.error {
            bounded_text::field(ui, error, egui::FontId::proportional(12.0), 22.0);
            if ui.button("Retry diff").clicked() {
                self.generation += 1;
                self.sent = None;
                self.error = None;
                self.send(&ctx);
            }
            return;
        }
        if let Some(info) = &self.info {
            bounded_text::field(
                ui,
                &format!(
                    "{} → {} bytes{}",
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
            if !info.note.is_empty() {
                bounded_text::field(ui, &info.note, egui::FontId::proportional(12.0), 22.0);
            }
        } else {
            ui.weak("Loading selected-file diff…");
        }
        ui.separator();
        self.rect = ui.available_rect_before_wrap();
        if self.count == 0 {
            self.send(&ctx);
            return;
        }
        let mut scroll = egui::ScrollArea::both()
            .id_salt(("file-diff", self.generation))
            .auto_shrink([false, false]);
        if self.reset {
            scroll = scroll.scroll_offset(egui::Vec2::ZERO);
            self.reset = false;
        }
        ui.spacing_mut().item_spacing.y = 0.0;
        let result = scroll.show_rows(ui, HEIGHT, self.count, |ui, rows| {
            self.range = rows.start.saturating_sub(24)..rows.end.saturating_add(24);
            // A conservative monospace width keeps every source column reachable.
            let width = (120.0 + self.columns as f32 * 8.0).max(ui.available_width());
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
                let (sign, color, fill) = match row.kind {
                    RowKind::Added => (
                        "+",
                        egui::Color32::from_rgb(150, 220, 170),
                        egui::Color32::from_rgba_unmultiplied(35, 95, 55, 65),
                    ),
                    RowKind::Deleted => (
                        "-",
                        egui::Color32::from_rgb(245, 160, 160),
                        egui::Color32::from_rgba_unmultiplied(115, 40, 40, 65),
                    ),
                    RowKind::Hunk => (
                        "",
                        egui::Color32::from_rgb(135, 185, 230),
                        egui::Color32::from_gray(35),
                    ),
                    _ => (
                        "",
                        ui.visuals().strong_text_color(),
                        egui::Color32::TRANSPARENT,
                    ),
                };
                ui.painter().rect_filled(rect, 0.0, fill);
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
                ui.painter()
                    .with_clip_rect(
                        egui::Rect::from_min_max(rect.min + egui::vec2(116.0, 0.0), rect.max)
                            .intersect(ui.clip_rect()),
                    )
                    .text(
                        rect.min + egui::vec2(116.0, HEIGHT / 2.0),
                        egui::Align2::LEFT_CENTER,
                        &row.text,
                        font,
                        color,
                    );
                if response
                    .on_hover_text("Click to copy this displayed line")
                    .clicked()
                {
                    ctx.copy_text(row.text.clone());
                }
            }
        });
        self.offset = result.state.offset.y;
        self.send(&ctx);
    }
}
