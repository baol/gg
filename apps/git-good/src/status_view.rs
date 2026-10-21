//! Compact, paged working status with stable interaction boundaries.
use crate::{
    bounded_text,
    status_worker::{Update, Worker},
};
use eframe::egui;
use gg_engine::status::{Area, Kind, StatusEntry};
use std::{collections::BTreeMap, ops::Range, path::PathBuf};
const HEIGHT: f32 = 24.0;
pub struct StatusView {
    path: PathBuf,
    identity: Option<gg_engine::status::StatusIdentity>,
    diff: crate::diff_view::DiffView,
    reading: bool,
    mutation: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
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
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let ctx = ctx.clone();
        match std::thread::Builder::new()
            .name("unstage".into())
            .spawn(move || {
                let result =
                    gg_git::unstage::file(&gg_engine::status::WorkingRequest { repository, entry });
                let _ = tx.send(result);
                ctx.request_repaint();
            }) {
            Ok(_) => {
                self.mutation = Some(rx);
                self.error = None;
            }
            Err(e) => self.error = Some(format!("Start unstage: {e}")),
        }
    }
    fn draw_diff(&mut self, ctx: &egui::Context) {
        let mut back = crate::navigation::available(ctx)
            && ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                    || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
            });
        let mut refresh = false;
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
                        egui::vec2((ui.available_width() - 125.0).max(100.0), 26.0),
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
                    refresh |= ui
                        .add_sized([114.0, 26.0], egui::Button::new("Refresh status"))
                        .clicked();
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| self.diff.draw_reading(ui));
        bounded_text::show_disclosure(ctx);
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
                Some(Err("Unstage worker stopped. Refresh status.".into()))
            }
            Err(_) => None,
        }) {
            self.mutation = None;
            match result {
                Ok(()) => self.start(ctx, true),
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
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                unstage |= ui
                    .add_enabled(
                        self.mutation.is_none()
                            && !self.replacing
                            && self.error.is_none()
                            && self
                                .selected
                                .and_then(|r| self.rows.get(&r))
                                .is_some_and(|e| e.area == Area::Staged),
                        egui::Button::new(if self.mutation.is_some() {
                            "Unstaging…"
                        } else {
                            "Unstage file"
                        })
                        .min_size(egui::vec2(100.0, 26.0)),
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
}
