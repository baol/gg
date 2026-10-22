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
    diff: crate::diff_view::DiffView,
    show_diff: bool,
    restore_files: bool,
    replacing: bool,
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
        };
        view.retry(ctx);
        view
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
        self.request.path = comparison.path.clone();
        self.request.revision = comparison.commit.clone();
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
    fn metadata(&mut self, ui: &mut egui::Ui) {
        let Some(change) = self.selected.and_then(|row| self.rows.get(&row)) else {
            ui.weak("Select a file");
            return;
        };
        if ui.button("View diff").clicked() {
            self.show_diff = true;
        }
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
        let native_text = std::str::from_utf8(&change.path_bytes);
        if ui
            .button(if native_text.is_ok() {
                "Copy path"
            } else {
                "Copy escaped path"
            })
            .clicked()
        {
            ui.ctx()
                .copy_text(native_text.unwrap_or(&change.path).to_owned());
        }
    }
    /// Return true when Back is explicitly requested.
    pub(crate) fn draw_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
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
        let narrow = ctx.content_rect().width() < 1040.0;
        let mut back = false;
        let mut reveal = false;
        if crate::navigation::available(ctx) && !self.diff.export_open() {
            back = ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                    || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
            });
            if back && self.show_diff {
                self.show_diff = false;
                self.restore_files = true;
                back = false;
            }
            if self.selected.is_some()
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
        if (!narrow || self.show_diff)
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
            .exact_size(42.0)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui
                        .button(if self.show_diff {
                            "Back to files"
                        } else {
                            "Back to history"
                        })
                        .clicked()
                    {
                        if self.show_diff {
                            self.show_diff = false;
                            self.restore_files = true;
                        } else {
                            back = true;
                        }
                    }
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
                    if let Some(comparison) = &self.comparison {
                        let parents = comparison.parents.clone();
                        if parents.len() > 1 {
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
                        } else {
                            ui.weak(if parents.is_empty() {
                                "Empty tree"
                            } else {
                                "Parent 1"
                            });
                        }
                    }
                });
            });
        egui::Panel::top("review-identity")
            .exact_size(64.0)
            .show(ui, |ui| {
                bounded_text::field(ui, &self.subject, egui::FontId::proportional(16.0), 26.0);
                let base = self
                    .comparison
                    .as_ref()
                    .map(|c| c.parent.as_deref().unwrap_or("empty tree"))
                    .unwrap_or("resolving parent");
                bounded_text::field(
                    ui,
                    &format!(
                        "{} · {} · parent {}",
                        self.request.path.display(),
                        self.request.revision,
                        base
                    ),
                    egui::FontId::monospace(11.0),
                    22.0,
                );
            });
        egui::Panel::bottom("review-status")
            .exact_size(26.0)
            .show(ui, |ui| {
                let pending = format!(
                    "Loading parent {} · showing previous comparison",
                    self.request.parent.unwrap_or(1)
                );
                let failure = self.error.as_ref().map(|error| {
                    if self.replacing {
                        format!(
                            "Parent {} failed · showing previous comparison · {error}",
                            self.request.parent.unwrap_or(1)
                        )
                    } else {
                        error.clone()
                    }
                });
                let text = failure.as_deref().unwrap_or(if self.replacing {
                    &pending
                } else if self.show_diff && self.diff.binary() {
                    "Scroll to inspect · Export either side · Esc: files"
                } else if self.show_diff {
                    crate::navigation::READING_HINT
                } else if self.complete {
                    "Up/Down / j k: select · PgUp/PgDn: page · Esc: history"
                } else {
                    "Reading changed files…"
                });
                bounded_text::field(ui, text, egui::FontId::proportional(12.0), 18.0);
            });
        if self.show_diff {
            egui::CentralPanel::default().show(ui, |ui| self.diff.draw_reading(ui));
            bounded_text::show_disclosure(ctx);
            return back;
        }
        if !narrow {
            egui::Panel::right("review-file-diff")
                .default_size(640.0)
                .size_range(400.0..=(ctx.content_rect().width() - 320.0).max(400.0))
                .show(ui, |ui| self.diff.draw(ui));
        } else {
            egui::Panel::bottom("review-file-info-narrow")
                .exact_size(170.0)
                .show(ui, |ui| self.metadata(ui));
        }
        egui::CentralPanel::default().show(ui, |ui| {
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
            if self.restore_files {
                scroll = scroll.vertical_scroll_offset(self.offset);
                self.restore_files = false;
            }
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
                        egui::pos2(rect.left() + 80.0, rect.bottom()),
                    );
                    let path = egui::Rect::from_min_max(
                        egui::pos2(status.right() + 8.0, rect.top()),
                        rect.max,
                    );
                    if let Some(change) = change {
                        let (label, color) = crate::colors::change(ui.visuals(), &change.kind);
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
                            response.surrender_focus();
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
            self.offset = result.state.clamped_offset().y;
            self.page = (result.inner_rect.height() / ROW).floor().max(1.0) as usize;
        });
        if (!narrow || self.show_diff)
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
