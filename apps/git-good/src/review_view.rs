//! A review context keeps history input and viewport state untouched.
use crate::{
    bounded_text,
    review_worker::{Update, Worker},
};
use eframe::egui;
use gg_engine::review::*;
use std::{collections::BTreeMap, ops::Range};
const ROW: f32 = 26.0;
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
        };
        view.retry(ctx);
        view
    }
    fn retry(&mut self, ctx: &egui::Context) {
        self.worker = None;
        self.staged = None;
        self.error = None;
        self.complete = false;
        match Worker::start(self.request.clone(), ctx.clone()) {
            Ok(worker) => {
                worker.request(self.viewport.clone());
                self.worker = Some(worker);
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn metadata(&self, ui: &mut egui::Ui) {
        let Some(change) = self.selected.and_then(|row| self.rows.get(&row)) else {
            ui.weak("Select a file");
            return;
        };
        ui.weak("SELECTED FILE");
        ui.add_space(8.0);
        bounded_text::field(ui, &change.path, egui::FontId::proportional(14.0), 24.0);
        ui.add_space(8.0);
        for (label, side) in [("Before", &change.old), ("After", &change.new)] {
            let text = side
                .as_ref()
                .map(|s| format!("{label}  {:06o}  {}", s.mode, s.id))
                .unwrap_or_else(|| format!("{label}  absent"));
            bounded_text::field(ui, &text, egui::FontId::monospace(11.0), 22.0);
        }
        if ui.button("Copy path").clicked() {
            ui.ctx().copy_text(change.path.clone());
        }
    }
    /// Return true when Back is explicitly requested.
    pub fn draw(&mut self, ctx: &egui::Context) -> bool {
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
        if let Some(update) = &self.staged {
            for (row, change) in &update.rows {
                if pinned != Some(*row) || self.rows.contains_key(row) {
                    self.rows.entry(*row).or_insert_with(|| change.clone());
                }
            }
            if quiet && !scrollbar {
                self.count = update.count;
                self.complete = update.complete;
                self.error = update.error.clone();
                self.comparison = Some(update.comparison.clone());
                if self.selected.is_none() && self.count > 0 && pinned.is_none() {
                    self.selected = Some(0);
                }
            }
            if quiet && pinned.is_none() && !scrollbar {
                self.staged = None;
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
        let mut reveal = false;
        if !ctx.wants_keyboard_input() {
            back = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
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
                if ctx.input_mut(|i| i.consume_key(modifiers, key)) && self.count > 0 {
                    self.selected = Some(
                        self.selected
                            .unwrap_or(0)
                            .saturating_add_signed(step)
                            .min(self.count - 1),
                    );
                    reveal = true;
                }
            }
        }
        egui::TopBottomPanel::top("review-header")
            .exact_height(42.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    back |= ui.button("Back to history").clicked();
                    ui.separator();
                    ui.strong("Changed files");
                    if self.error.is_some() && ui.button("Retry").clicked() {
                        self.retry(ctx);
                    }
                    let label = format!(
                        "{}{} files",
                        self.count,
                        if self.complete { "" } else { "+" }
                    );
                    ui.weak(label);
                });
            });
        egui::TopBottomPanel::top("review-identity")
            .exact_height(64.0)
            .show(ctx, |ui| {
                bounded_text::field(ui, &self.subject, egui::FontId::proportional(16.0), 26.0);
                let base = self
                    .comparison
                    .as_ref()
                    .map(|c| c.parent.as_deref().unwrap_or("empty tree"))
                    .unwrap_or("resolving parent");
                bounded_text::field(
                    ui,
                    &format!(
                        "{} · {} ← {}",
                        self.request.path.display(),
                        self.request.revision,
                        base
                    ),
                    egui::FontId::monospace(11.0),
                    22.0,
                );
            });
        egui::TopBottomPanel::bottom("review-status")
            .exact_height(26.0)
            .show(ctx, |ui| {
                let text = self.error.as_deref().unwrap_or(if self.complete {
                    "Up/Down / j k: select · PgUp/PgDn: page · Esc: history"
                } else {
                    "Reading changed files…"
                });
                bounded_text::field(ui, text, egui::FontId::proportional(12.0), 18.0);
            });
        if ctx.content_rect().width() >= 1040.0 {
            egui::SidePanel::right("review-file-info")
                .default_width(310.0)
                .width_range(280.0..=400.0)
                .show(ctx, |ui| self.metadata(ui));
        } else {
            egui::TopBottomPanel::bottom("review-file-info-narrow")
                .exact_height(145.0)
                .show(ctx, |ui| self.metadata(ui));
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_sized([86.0, ROW], egui::Label::new("STATUS"));
                ui.weak("PATH");
            });
            self.rect = ui.available_rect_before_wrap();
            if self.count == 0 {
                ui.weak(if self.complete {
                    "This comparison has no changed files."
                } else {
                    "Reading comparison…"
                });
                return;
            }
            let mut scroll = egui::ScrollArea::vertical().id_salt("review-files");
            if reveal && let Some(row) = self.selected {
                let height = ui.available_height();
                let top = row as f32 * ROW;
                let offset = if top < self.offset {
                    top
                } else if top + ROW > self.offset + height {
                    top + ROW - height
                } else {
                    self.offset
                };
                scroll = scroll.vertical_scroll_offset(
                    offset.clamp(0.0, (self.count as f32 * ROW - height).max(0.0)),
                );
            }
            ui.spacing_mut().item_spacing.y = 0.0;
            let result = scroll.show_rows(ui, ROW, self.count, |ui, rows| {
                self.viewport = rows.start.saturating_sub(24)..rows.end.saturating_add(24);
                self.row_rects.clear();
                for row in rows {
                    let change = self.rows.get(&row);
                    let response = ui.add_sized(
                        [ui.available_width(), ROW],
                        egui::Button::new("")
                            .fill(if self.selected == Some(row) {
                                ui.visuals().selection.bg_fill.gamma_multiply(0.55)
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
                        egui::pos2(rect.left() + 80.0, rect.bottom()),
                    );
                    let path = egui::Rect::from_min_max(
                        egui::pos2(status.right() + 8.0, rect.top()),
                        rect.max,
                    );
                    if let Some(change) = change {
                        let (label, color) = match change.kind {
                            ChangeKind::Added => {
                                ("+ Added", egui::Color32::from_rgb(120, 200, 150))
                            }
                            ChangeKind::Deleted => {
                                ("− Deleted", egui::Color32::from_rgb(240, 140, 140))
                            }
                            ChangeKind::Modified => {
                                ("~ Modified", egui::Color32::from_rgb(220, 195, 125))
                            }
                            ChangeKind::TypeChanged => {
                                ("↔ Type", egui::Color32::from_rgb(170, 160, 230))
                            }
                        };
                        bounded_text::paint(
                            ui,
                            status,
                            label,
                            egui::FontId::proportional(12.0),
                            color,
                        );
                        bounded_text::paint(
                            ui,
                            path,
                            &change.path,
                            egui::FontId::proportional(13.0),
                            ui.visuals().strong_text_color(),
                        );
                        if response.clicked() {
                            self.selected = Some(row);
                        }
                    } else {
                        bounded_text::paint(
                            ui,
                            path,
                            "Loading…",
                            egui::FontId::proportional(12.0),
                            ui.visuals().weak_text_color(),
                        );
                    }
                }
            });
            self.offset = result.state.offset.y;
            self.page = (result.inner_rect.height() / ROW).floor().max(1.0) as usize;
        });
        if let Some(worker) = &self.worker {
            worker.request(self.viewport.clone());
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
    #[test]
    fn compact_rows_and_placeholder_identity_survive_arrival() -> Result<(), String> {
        for (width, height) in [(720.0, 480.0), (1120.0, 740.0)] {
            let ctx = egui::Context::default();
            let mut view = Review::fixture(&ctx);
            let frame = |view: &mut Review, events, time| {
                let mut input = crate::tests::frame_input(width, height, events);
                input.time = Some(time);
                ctx.run(input, |ctx| {
                    view.draw(ctx);
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
}
