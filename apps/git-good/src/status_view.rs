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
    StageHunks,
    Unstage,
}
pub struct StatusView {
    path: PathBuf,
    identity: Option<gg_engine::status::StatusIdentity>,
    diff: crate::diff_view::DiffView,
    reading: bool,
    mutation: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    mutation_kind: Option<Mutation>,
    worker: Option<Worker>,
    pending: Option<Worker>,
    staged: Option<Update>,
    rows: BTreeMap<usize, StatusEntry>,
    count: usize,
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
            diff: crate::diff_view::DiffView::default(),
            reading: false,
            mutation: None,
            mutation_kind: None,
            worker: None,
            pending: None,
            staged: None,
            rows: BTreeMap::new(),
            count: 0,
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
        // A fresh scan can observe new working bytes with the same HEAD and index.
        if replacing {
            self.diff = crate::diff_view::DiffView::default();
        }
        match Worker::start(self.path.clone(), ctx.clone()) {
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
        self.complete = update.complete;
        // Paging a retained scan must preserve an action failure until explicit retry.
        self.error = update.error.or_else(|| self.action_error.clone());
        self.rows.extend(update.rows);
        let keep = self.viewport.start.saturating_sub(256)..self.viewport.end.saturating_add(256);
        self.rows.retain(|row, _| keep.contains(row));
        while self.rows.len() > 768 {
            self.rows.pop_last();
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
            self.diff
                .select_working(gg_engine::status::WorkingRequest { repository, entry }, ctx);
            self.reading = true;
        }
    }
    fn unstage(&mut self, ctx: &egui::Context) {
        if self.mutation.is_some() || self.replacing {
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
        if self.mutation.is_some() || self.replacing {
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
    fn stage_hunks(
        &mut self,
        ctx: &egui::Context,
        request: gg_engine::status::WorkingRequest,
        headers: Vec<String>,
    ) {
        if self.mutation.is_some() || self.replacing {
            return;
        }
        self.mutate(ctx, Mutation::StageHunks, "stage hunk", move || {
            gg_git::stage::hunks(&request, &headers)
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
    fn draw_diff(&mut self, ctx: &egui::Context) {
        let mut back = crate::navigation::available(ctx)
            && ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                    || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
            });
        let mut refresh = false;
        let mut stage_hunk = false;
        egui::TopBottomPanel::top("working-diff-header")
            .exact_height(42.0)
            .show(ctx, |ui| {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    back |= ui
                        .add_sized([100.0, 26.0], egui::Button::new("Back to files"))
                        .clicked();
                    ui.strong("Working diff");
                    ui.allocate_ui_with_layout(
                        egui::vec2((ui.available_width() - 230.0).max(100.0), 26.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            bounded_text::field(
                                ui,
                                &self.path.display().to_string(),
                                egui::FontId::proportional(12.0),
                                26.0,
                            );
                        },
                    );
                    stage_hunk |= ui
                        .add_enabled(
                            self.mutation.is_none() && self.diff.stageable_hunk().is_some(),
                            egui::Button::new(
                                if self.mutation_kind == Some(Mutation::StageHunks) {
                                    "Staging…"
                                } else {
                                    "Stage hunk"
                                },
                            )
                            .min_size(egui::vec2(96.0, 26.0)),
                        )
                        .on_disabled_hover_text("Select an unstaged hunk header")
                        .clicked();
                    refresh |= ui
                        .add_sized([114.0, 26.0], egui::Button::new("Refresh status"))
                        .clicked();
                });
            });
        if let Some(error) = &self.error {
            egui::TopBottomPanel::top("working-diff-error")
                .exact_height(23.0)
                .show(ctx, |ui| {
                    bounded_text::field(ui, error, egui::FontId::proportional(12.0), 22.0);
                });
        }
        egui::CentralPanel::default().show(ctx, |ui| self.diff.draw_reading(ui));
        bounded_text::show_disclosure(ctx);
        if stage_hunk
            && let Some((request, header)) = self
                .diff
                .stageable_hunk()
                .map(|(request, header)| (request.clone(), header.to_string()))
        {
            self.stage_hunks(ctx, request, vec![header]);
        }
        if back || refresh {
            self.reading = false;
            self.restore = true;
        }
        if refresh {
            self.diff = crate::diff_view::DiffView::default();
            self.start(ctx, true);
        }
    }
    pub fn draw(&mut self, ctx: &egui::Context) -> bool {
        if let Some(result) = self.mutation.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Some(Err("Mutation worker stopped. Refresh status.".into()))
            }
            Err(_) => None,
        }) {
            let completed = self.mutation_kind.take();
            self.mutation = None;
            match result {
                Ok(()) => {
                    if completed == Some(Mutation::StageHunks) {
                        self.reading = false;
                        self.restore = true;
                    }
                    self.start(ctx, true);
                }
                Err(e) => {
                    self.action_error = Some(e.clone());
                    self.error = Some(e);
                }
            }
        }
        if self.reading {
            self.draw_diff(ctx);
            return false;
        }
        let (now, scrolling, pointer_down, hovered) = ctx.input(|i| {
            (
                i.time,
                i.raw_scroll_delta != egui::Vec2::ZERO || i.smooth_scroll_delta != egui::Vec2::ZERO,
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
        if self.mutation.is_none() && crate::navigation::available(ctx) {
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
        egui::TopBottomPanel::top("working-header")
            .exact_height(42.0)
            .show(ctx, |ui| {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    back |= ui
                        .add_enabled(
                            self.mutation.is_none(),
                            egui::Button::new("Back").min_size(egui::vec2(64.0, 26.0)),
                        )
                        .clicked();
                    ui.strong("Status");
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
                            self.mutation.is_none(),
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
        egui::TopBottomPanel::bottom("working-status")
            .exact_height(25.0)
            .show(ctx, |ui| {
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
                } else if self.replacing {
                    format!(
                        "Refreshing · showing previous results · {} entries",
                        self.count
                    )
                } else if self.complete && self.count == 0 {
                    "Clean working tree".into()
                } else {
                    format!(
                        "{} entries{}",
                        self.count,
                        if self.complete { "" } else { " · scanning…" }
                    )
                };
                bounded_text::field(ui, &text, egui::FontId::proportional(12.0), 22.0);
            });
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
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                stage |= ui
                    .add_enabled(
                        self.mutation.is_none()
                            && !self.replacing
                            && self.error.is_none()
                            && matches!(selected_area, Some(Area::Unstaged | Area::Untracked)),
                        egui::Button::new(stage_label).min_size(egui::vec2(82.0, 26.0)),
                    )
                    .clicked();
                unstage |= ui
                    .add_enabled(
                        self.mutation.is_none()
                            && !self.replacing
                            && self.error.is_none()
                            && selected_area == Some(Area::Staged),
                        egui::Button::new(unstage_label).min_size(egui::vec2(100.0, 26.0)),
                    )
                    .clicked();
                open |= ui
                    .add_enabled(
                        self.mutation.is_none()
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
                        if response.clicked() && self.mutation.is_none() {
                            response.surrender_focus();
                            self.selected = Some(row);
                        }
                        if response.double_clicked() && self.mutation.is_none() {
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
            self.offset = out.state.offset.y;
        });
        if let Some(worker) = &self.worker {
            worker.request(self.viewport.clone());
        }
        bounded_text::show_disclosure(ctx);
        if self.mutation.is_some() {
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
            diff: crate::diff_view::DiffView::default(),
            reading: false,
            mutation: None,
            mutation_kind: None,
            worker: None,
            pending: None,
            staged: None,
            rows: (0..count.min(256)).map(|i| (i, entry(i))).collect(),
            count,
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
            },
            rows: (0..count.min(256)).map(|i| (i, entry(i))).collect(),
            count,
            complete,
            error: None,
        }
    }
    fn frame(ctx: &egui::Context, view: &mut StatusView, width: f32, events: Vec<egui::Event>) {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 480.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                view.draw(ctx);
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
                    || (reading && view.diff.rows.values().any(|row| row.text.contains(text)))
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
        assert!(
            !view
                .diff
                .rows
                .values()
                .any(|row| row.text.contains("before"))
        );
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
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(720.0, 480.0),
                )),
                ..Default::default()
            },
            |ctx| {
                view.draw(ctx);
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
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 480.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                view.draw(ctx);
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
    #[test]
    fn hunk_header_click_stages_and_returns_to_the_refreshed_list()
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
        git(&["init", "-q"])?;
        let base = b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n";
        std::fs::write(dir.path().join("a"), base)?;
        git(&["add", "a"])?;
        git(&[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
            "commit",
            "-qm",
            "base",
        ])?;
        let working =
            b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n";
        std::fs::write(dir.path().join("a"), working)?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            frame(&ctx, &mut view, 1120.0, vec![]);
            if view.complete && !view.replacing && view.mutation.is_none() {
                break;
            }
            assert!(std::time::Instant::now() < until, "status timed out");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(view.rows[&0].area, Area::Unstaged);
        view.open_diff(&ctx);
        let header = loop {
            frame(&ctx, &mut view, 1120.0, vec![]);
            if let Some(header) = view
                .diff
                .rows
                .values()
                .find(|row| row.kind == gg_engine::diff::RowKind::Hunk)
                .map(|row| row.text.clone())
            {
                break header;
            }
            assert!(std::time::Instant::now() < until, "diff timed out");
            std::thread::sleep(std::time::Duration::from_millis(2));
        };
        for width in [720.0, 1120.0] {
            let output = run(&ctx, &mut view, width, vec![]);
            assert!(
                text_pos(&output, "Stage hunk").is_some(),
                "missing stage hunk at {width}"
            );
        }
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let pos = text_pos(&output, &header).ok_or("missing hunk header")?;
        click(&ctx, &mut view, 1120.0, pos + egui::vec2(5.0, 0.0));
        assert_eq!(view.diff.selected_hunk(), Some(header.as_str()));
        assert!(view.diff.stageable_hunk().is_some());
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let button = text_pos(&output, "Stage hunk").ok_or("missing stage hunk button")?;
        click(&ctx, &mut view, 1120.0, button + egui::vec2(5.0, 0.0));
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            frame(&ctx, &mut view, 1120.0, vec![]);
            if view.mutation.is_none() && !view.replacing && view.complete && !view.reading {
                break;
            }
            assert!(std::time::Instant::now() < until, "hunk stage timed out");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(!view.reading);
        assert!(view.rows.values().any(|entry| entry.area == Area::Staged));
        assert!(view.rows.values().any(|entry| entry.area == Area::Unstaged));
        assert_eq!(std::fs::read(dir.path().join("a"))?, working);

        let row = view
            .rows
            .iter()
            .find(|(_, entry)| entry.area == Area::Unstaged)
            .map(|(row, _)| *row)
            .ok_or("remaining unstaged row")?;
        view.selected = Some(row);
        view.open_diff(&ctx);
        let header = loop {
            frame(&ctx, &mut view, 1120.0, vec![]);
            if let Some(header) = view
                .diff
                .rows
                .values()
                .find(|row| row.kind == gg_engine::diff::RowKind::Hunk)
                .map(|row| row.text.clone())
            {
                break header;
            }
            assert!(
                std::time::Instant::now() < until,
                "remaining diff timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(2));
        };
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let hunk = text_pos(&output, &header).ok_or("missing remaining hunk")?;
        click(&ctx, &mut view, 1120.0, hunk + egui::vec2(5.0, 0.0));
        let index = std::fs::read(dir.path().join(".git/index"))?;
        std::fs::write(dir.path().join(".git/index.lock"), "owner")?;
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let button = text_pos(&output, "Stage hunk").ok_or("missing stage hunk button")?;
        click(&ctx, &mut view, 1120.0, button + egui::vec2(5.0, 0.0));
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            frame(&ctx, &mut view, 1120.0, vec![]);
            if view.mutation.is_none() {
                break;
            }
            assert!(std::time::Instant::now() < until, "hunk failure timed out");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(view.reading);
        assert!(
            view.error
                .as_ref()
                .is_some_and(|error| error.contains("Stage hunks"))
        );
        assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);
        assert_eq!(std::fs::read(dir.path().join("a"))?, working);
        std::fs::remove_file(dir.path().join(".git/index.lock"))?;
        Ok(())
    }
}
