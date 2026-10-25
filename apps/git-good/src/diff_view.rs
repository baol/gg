//! Virtualized source rows with pinned identity, fixed gutters, and horizontal scrolling.
use crate::{
    bounded_text,
    diff_worker::{Mode, Update, Worker},
    reader,
};
use eframe::egui;
use gg_engine::{
    diff::{DiffRow, FileDiff, RowKind},
    file::{FileText, ReadingSide},
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
    columns: usize,
    info: Option<FileDiff>,
    error: Option<String>,
    range: Range<usize>,
    sent: Option<(u64, Range<usize>, Mode, ReadingSide)>,
    rect: egui::Rect,
    bounds: egui::Rect,
    last_scroll: f64,
    staged: Option<Update>,
    offset: f32,
    reset: bool,
    mode: Mode,
    side: ReadingSide,
    full: Option<FileText>,
    full_side: ReadingSide,
    cursor: Option<usize>,
    anchor: Option<usize>,
    hunks: Vec<usize>,
    groups: Vec<(usize, usize)>,
    jump: Option<usize>,
    restore_line: Option<usize>,
    edge_known: bool,
    old_label: &'static str,
    new_label: &'static str,
    before_endpoint: Option<String>,
    after_endpoint: Option<String>,
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
            mode: Mode::Diff,
            side: ReadingSide::After,
            full: None,
            full_side: ReadingSide::After,
            cursor: None,
            anchor: None,
            hunks: vec![],
            groups: vec![],
            jump: None,
            restore_line: None,
            edge_known: true,
            old_label: "Before",
            new_label: "After",
            before_endpoint: None,
            after_endpoint: None,
        }
    }
}

#[derive(Clone, Copy)]
enum Action {
    Mode(Mode),
    Side(ReadingSide),
    Change(isize),
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
        self.restore_line = self.current_line();
        self.path = path;
        self.change = change;
        self.working = None;
        self.generation += 1;
        self.error = None;
        self.staged = None;
        self.sent = None;
        self.full = None;
        self.cursor = None;
        self.anchor = None;
        self.hunks.clear();
        self.groups.clear();
        self.jump = None;
        if !self.side_available(self.side) {
            self.side = self.existing_side();
        }
        self.full_side = self.side;
        self.send(ctx);
        ctx.request_repaint();
    }

    /// Select a version edge. When `known` is false the earlier side is not
    /// resolved yet, so only the full current content is meaningful.
    pub fn select_edge(
        &mut self,
        path: PathBuf,
        change: Option<FileChange>,
        known: bool,
        ctx: &egui::Context,
    ) {
        let changed = self.edge_known != known;
        self.edge_known = known;
        self.select(path, change, ctx);
        if !known && self.mode != Mode::Full {
            self.set_mode(Mode::Full, ctx);
        }
        if changed {
            self.sent = None;
            self.send(ctx);
        }
    }

    /// Name the two inspection sides; history calls them Old and New.
    pub fn set_side_labels(&mut self, old: &'static str, new: &'static str) {
        self.old_label = old;
        self.new_label = new;
    }

    /// Name both exact comparison endpoints for the header.
    pub fn set_endpoints(&mut self, before: impl Into<String>, after: impl Into<String>) {
        self.before_endpoint = Some(before.into());
        self.after_endpoint = Some(after.into());
    }

    /// The current presentation and place, independent of the comparison.
    pub fn place(&self) -> reader::ReaderPlace {
        reader::ReaderPlace {
            mode: self.mode,
            side: self.side,
            cursor: self.cursor,
            anchor: self.anchor,
            offset: self.offset,
        }
    }

    /// Capture everything a consumer must restore when returning to this file.
    pub fn save_context(&self) -> Option<reader::ReaderContext> {
        let change = self.change.clone()?;
        let endpoint = |label: &'static str, side: &Option<gg_engine::review::FileSide>| match side
        {
            Some(side) => format!("{label} {}", side.id.chars().take(8).collect::<String>()),
            None => format!("{label} · absent"),
        };
        Some(reader::ReaderContext::new(
            self.path.clone(),
            change.clone(),
            self.before_endpoint
                .clone()
                .unwrap_or_else(|| endpoint(self.old_label, &change.old)),
            self.after_endpoint
                .clone()
                .unwrap_or_else(|| endpoint(self.new_label, &change.new)),
            self.place(),
        ))
    }

    /// Restore a captured context without inventing a second viewer.
    pub fn open_context(&mut self, context: &reader::ReaderContext, ctx: &egui::Context) {
        let same = self.path == context.path
            && self
                .change
                .as_ref()
                .map(|c| (&c.path_bytes, &c.old, &c.new))
                == Some((
                    &context.change.path_bytes,
                    &context.change.old,
                    &context.change.new,
                ));
        if !same {
            self.select(context.path.clone(), Some(context.change.clone()), ctx);
        }
        self.set_endpoints(context.before.label.clone(), context.after.label.clone());
        self.mode = context.place.mode;
        self.side = context.place.side;
        self.full_side = self.side;
        self.cursor = context.place.cursor;
        self.anchor = context.place.anchor;
        self.offset = context.place.offset;
        self.sent = None;
        self.send(ctx);
        ctx.request_repaint();
    }

    /// Capabilities the current comparison supports.
    pub fn capabilities(&self) -> reader::Capabilities {
        let present = |side: &Option<gg_engine::review::FileSide>| {
            side.as_ref().is_some_and(|side| side.mode != 0o160000)
        };
        let (old, new) = self
            .change
            .as_ref()
            .map(|change| (&change.old, &change.new))
            .unwrap_or((&None, &None));
        reader::Capabilities {
            diff: self.edge_known,
            full_before: present(old),
            full_after: present(new),
            export_before: self.displayed.is_some() && present(old),
            export_after: self.displayed.is_some() && present(new),
        }
    }

    fn side_label(&self, side: ReadingSide) -> &'static str {
        match side {
            ReadingSide::Before => self.old_label,
            ReadingSide::After => self.new_label,
        }
    }

    fn side_available(&self, side: ReadingSide) -> bool {
        let Some(change) = &self.change else {
            return true;
        };
        match side {
            ReadingSide::Before => change.old.is_some(),
            ReadingSide::After => change.new.is_some(),
        }
    }

    fn existing_side(&self) -> ReadingSide {
        match &self.change {
            Some(change) => {
                gg_engine::file::existing_side(change.old.as_ref(), change.new.as_ref())
            }
            None => ReadingSide::After,
        }
    }

    fn send(&mut self, ctx: &egui::Context) {
        let Some(change) = &self.change else {
            return;
        };
        let range = if self.displayed_generation == self.generation {
            self.range.clone()
        } else {
            0..64
        };
        let key = (self.generation, range.clone(), self.mode, self.side);
        if self.sent.as_ref() == Some(&key) {
            return;
        }
        if self.worker.is_none() {
            match Worker::start(ctx.clone()) {
                Ok(w) => self.worker = Some(w),
                Err(e) => {
                    self.error = Some(e);
                    return;
                }
            }
        }
        if let Some(worker) = &self.worker {
            // A mode switch or file change centers the first diff page on its line.
            let line = if self.mode == Mode::Diff {
                self.restore_line
            } else {
                None
            };
            if let Some(working) = &self.working {
                worker.request_working(
                    self.generation,
                    working.clone(),
                    range,
                    self.mode,
                    self.side,
                    line,
                );
            } else {
                worker.request(
                    self.generation,
                    self.path.clone(),
                    change.clone(),
                    range,
                    self.mode,
                    self.side,
                    line,
                );
            }
            self.sent = Some(key);
        }
    }

    pub fn accept(&mut self, update: Update, pinned: Option<usize>) {
        if update.generation != self.generation
            || update.mode != self.mode
            || update.side != self.side
        {
            return;
        }
        self.hunks = update.hunks;
        if self.mode == Mode::Full {
            if self.displayed_generation != update.generation || self.full_side != update.side {
                self.rows.clear();
                self.offset = 0.0;
                self.reset = true;
                self.range = 0..64;
                self.binary_view = crate::binary_preview::View::default();
                self.displayed = self.change.clone();
                self.displayed_path = self.path.clone();
                self.displayed_working = self.working.clone();
                self.displayed_generation = update.generation;
                self.full_side = update.side;
                self.full = None;
                self.cursor = None;
                self.anchor = None;
                self.groups.clear();
            }
            self.preview = None;
            match update.full {
                Some(Ok(text)) => {
                    self.count = text.lines.len();
                    self.columns = text
                        .lines
                        .iter()
                        .map(|line| line.text.chars().count())
                        .max()
                        .unwrap_or(0);
                    self.groups = text.changed_groups();
                    self.full = Some(text);
                    self.error = None;
                    self.adopt_position();
                }
                Some(Err(error)) => {
                    self.full = None;
                    self.groups.clear();
                    self.count = 0;
                    self.error = Some(error);
                }
                None => {}
            }
            if let Ok(info) = update.result {
                self.info = Some(info);
            }
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
                    self.cursor = None;
                    self.anchor = None;
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
                if let Some(line) = self.restore_line.take()
                    && let Some(index) = self.row_for_line(line)
                {
                    self.select_row(index);
                }
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn adopt_position(&mut self) {
        let Some(full) = &self.full else {
            return;
        };
        let count = full.lines.len();
        if count == 0 {
            self.cursor = None;
            self.anchor = None;
            return;
        }
        let target = if let Some(line) = self.restore_line.take() {
            line.saturating_sub(1).min(count - 1)
        } else if let Some(cursor) = self.cursor {
            cursor.min(count - 1)
        } else {
            full.lines.iter().position(|line| line.changed).unwrap_or(0)
        };
        self.cursor = Some(target);
        self.anchor = Some(target);
        self.jump = Some(target);
        self.offset = target as f32 * HEIGHT;
    }

    fn select_row(&mut self, index: usize) {
        self.cursor = Some(index);
        self.anchor = Some(index);
        self.jump = Some(index);
        self.offset = index as f32 * HEIGHT;
    }

    fn row_for_line(&self, line: usize) -> Option<usize> {
        self.rows
            .iter()
            .filter_map(|(index, row)| self.row_line(row).map(|n| (*index, n)))
            .min_by_key(|(_, n)| n.abs_diff(line))
            .map(|(index, _)| index)
    }

    fn row_line(&self, row: &DiffRow) -> Option<usize> {
        match self.side {
            ReadingSide::Before => row.old_line,
            ReadingSide::After => row.new_line.or(row.old_line),
        }
    }

    fn current_line(&self) -> Option<usize> {
        match self.mode {
            Mode::Full => self.cursor.map(|index| index + 1),
            Mode::Diff => {
                let index = self.cursor?;
                let row_line = |row: &DiffRow| self.row_line(row);
                self.rows
                    .get(&index)
                    .and_then(row_line)
                    .or_else(|| {
                        self.rows
                            .range(index..)
                            .find(|(_, row)| self.row_changed(row))
                            .and_then(|(_, row)| row_line(row))
                    })
                    .or_else(|| self.rows.range(index..).find_map(|(_, row)| row_line(row)))
                    .or_else(|| {
                        self.rows
                            .range(..index)
                            .rev()
                            .find_map(|(_, row)| row_line(row))
                    })
            }
        }
    }

    fn row_changed(&self, row: &DiffRow) -> bool {
        match self.side {
            ReadingSide::Before => row.kind == RowKind::Deleted,
            ReadingSide::After => row.kind == RowKind::Added,
        }
    }

    fn set_mode(&mut self, mode: Mode, ctx: &egui::Context) {
        if self.mode == mode {
            return;
        }
        let line = self.current_line();
        self.mode = mode;
        if mode == Mode::Full {
            if !self.side_available(self.side) {
                self.side = self.existing_side();
            }
            self.full_side = self.side;
            if let Some(line) = line {
                let index = line.saturating_sub(1);
                self.cursor = Some(index);
                self.anchor = Some(index);
                self.jump = Some(index);
            } else {
                // The diff page may not have arrived yet; adopt the first change instead.
                self.cursor = None;
                self.anchor = None;
                self.jump = None;
            }
            if self.full.is_some() {
                self.adopt_position();
            }
        } else {
            let line = line.unwrap_or(1);
            self.restore_line = Some(line);
            if let Some(index) = self.row_for_line(line) {
                self.select_row(index);
            } else if let Some(hunk) = self.hunks.first().copied() {
                self.select_row(hunk);
            }
        }
        self.sent = None;
        self.send(ctx);
        ctx.request_repaint();
    }

    fn set_side(&mut self, side: ReadingSide, ctx: &egui::Context) {
        if self.side == side || !self.side_available(side) {
            return;
        }
        let line = self.current_line().unwrap_or(1);
        self.side = side;
        self.full = None;
        self.full_side = side;
        if self.mode == Mode::Full {
            let index = line.saturating_sub(1);
            self.cursor = Some(index);
            self.anchor = Some(index);
            self.jump = Some(index);
        }
        self.sent = None;
        self.send(ctx);
        ctx.request_repaint();
    }

    /// One-based change number at or before the cursor. `None` means before the first change.
    fn change_number(&self) -> Option<usize> {
        let index = match self.mode {
            Mode::Full => {
                let line = self.cursor? + 1;
                self.groups.iter().rposition(|(start, _)| *start <= line)?
            }
            Mode::Diff => {
                let row = self.cursor?;
                self.hunks.iter().rposition(|hunk| *hunk <= row)?
            }
        };
        Some(index + 1)
    }

    fn change_total(&self) -> usize {
        match self.mode {
            Mode::Full => self.groups.len(),
            Mode::Diff => self.hunks.len(),
        }
    }

    fn change_position(&self) -> (usize, usize) {
        let total = self.change_total();
        if total == 0 {
            return (0, 0);
        }
        // Before the first change the next one is still change 1.
        (self.change_number().unwrap_or(1), total)
    }

    fn next_change(&self) -> Option<usize> {
        match self.mode {
            Mode::Full => {
                let Some(line) = self.cursor.map(|index| index + 1) else {
                    return self.groups.first().map(|(start, _)| *start);
                };
                self.groups
                    .iter()
                    .find(|(start, _)| *start > line)
                    .map(|(start, _)| *start)
            }
            Mode::Diff => {
                let Some(row) = self.cursor else {
                    return self.hunks.first().copied();
                };
                self.hunks.iter().find(|hunk| **hunk > row).copied()
            }
        }
    }

    fn previous_change(&self) -> Option<usize> {
        match self.mode {
            Mode::Full => {
                let line = self.cursor? + 1;
                self.groups
                    .iter()
                    .rev()
                    .find(|(_, end)| *end < line)
                    .map(|(start, _)| *start)
            }
            Mode::Diff => {
                let row = self.cursor?;
                self.hunks.iter().rev().find(|hunk| **hunk < row).copied()
            }
        }
    }

    fn jump_change(&mut self, direction: isize, ctx: &egui::Context) {
        let target = if direction > 0 {
            self.next_change()
        } else {
            self.previous_change()
        };
        let Some(target) = target else {
            return;
        };
        if self.mode == Mode::Full {
            self.select_row(target.saturating_sub(1));
            return;
        }
        self.cursor = Some(target);
        self.anchor = Some(target);
        self.jump = Some(target);
        self.offset = target as f32 * HEIGHT;
        self.range = target.saturating_sub(128)..target.saturating_add(128).min(self.count);
        self.sent = None;
        self.send(ctx);
    }

    fn move_cursor(&mut self, step: isize, shift: bool) {
        let Some(full) = &self.full else {
            return;
        };
        let count = full.lines.len();
        if count == 0 {
            return;
        }
        let current = self
            .cursor
            .unwrap_or_else(|| ((self.offset / HEIGHT).floor() as usize).min(count - 1));
        let next = (current as isize + step).clamp(0, count as isize - 1) as usize;
        if shift {
            if self.anchor.is_none() {
                self.anchor = Some(current);
            }
        } else {
            self.anchor = Some(next);
        }
        self.cursor = Some(next);
        let visible = (self.rect.height() / HEIGHT).floor().max(1.0) as usize;
        let top = (self.offset / HEIGHT).floor() as usize;
        if next < top || next >= top + visible {
            self.offset = next.saturating_sub(visible / 2) as f32 * HEIGHT;
            self.jump = Some(next);
        }
    }

    fn reading_keys(&mut self, ctx: &egui::Context) {
        let page = (self.rect.height() / HEIGHT).floor().max(2.0) as isize - 1;
        let shift = ctx.input(|input| input.modifiers.shift);
        let step = crate::navigation::step(ctx, page);
        if step != 0 {
            if self.mode == Mode::Full {
                self.move_cursor(step, shift);
            } else {
                self.offset = (self.offset + step as f32 * HEIGHT).clamp(
                    0.0,
                    (self.count as f32 * HEIGHT - self.rect.height()).max(0.0),
                );
                let row = (self.offset / HEIGHT).floor() as usize;
                if self.rows.contains_key(&row) {
                    self.cursor = Some(row);
                    if !shift {
                        self.anchor = Some(row);
                    }
                }
            }
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::OpenBracket)) {
            self.jump_change(-1, ctx);
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::CloseBracket))
        {
            self.jump_change(1, ctx);
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
        let Some(change) = self.displayed.as_ref().or(self.change.as_ref()).cloned() else {
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
            if let (Some(before), Some(after)) = (&self.before_endpoint, &self.after_endpoint) {
                bounded_text::field(
                    ui,
                    &format!("{before} → {after}"),
                    egui::FontId::monospace(11.0),
                    20.0,
                );
            }
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
            let capabilities = self.capabilities();
            ui.horizontal(|ui| {
                for (label, side, enabled) in [
                    ("Export before", &change.old, capabilities.export_before),
                    ("Export after", &change.new, capabilities.export_after),
                ] {
                    if ui
                        .add_enabled(enabled && !self.export.busy(), egui::Button::new(label))
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
        let mut actions = vec![];
        let capabilities = self.capabilities();
        ui.horizontal_wrapped(|ui| {
            let diff = ui
                .add_enabled(
                    capabilities.diff,
                    egui::Button::new("Diff").selected(self.mode == Mode::Diff),
                )
                .on_disabled_hover_text("Choose an earlier version to compare");
            if diff.clicked() {
                actions.push(Action::Mode(Mode::Diff));
            }
            if ui
                .selectable_label(self.mode == Mode::Full, "Full file")
                .clicked()
            {
                actions.push(Action::Mode(Mode::Full));
            }
            if self.mode == Mode::Full && self.edge_known {
                ui.separator();
                for side in [ReadingSide::Before, ReadingSide::After] {
                    if ui
                        .add_enabled(
                            side != self.side && self.side_available(side),
                            egui::Button::new(self.side_label(side)),
                        )
                        .clicked()
                    {
                        actions.push(Action::Side(side));
                    }
                }
            }
            ui.separator();
            if !self.edge_known {
                ui.weak("Earlier version not loaded");
                ui.add_enabled(false, egui::Button::new("< Prev change"));
                ui.add_enabled(false, egui::Button::new("Next change >"));
            } else {
                let (index, total) = self.change_position();
                let label = if total == 0 {
                    "No changes".to_string()
                } else if self.change_number().is_some() {
                    format!("Change {index} of {total}")
                } else {
                    format!("Before change 1 of {total}")
                };
                ui.weak(label);
                let previous = self.previous_change().is_some();
                let next = self.next_change().is_some();
                if ui
                    .add_enabled(previous, egui::Button::new("< Prev change"))
                    .clicked()
                {
                    actions.push(Action::Change(-1));
                }
                if ui
                    .add_enabled(next, egui::Button::new("Next change >"))
                    .clicked()
                {
                    actions.push(Action::Change(1));
                }
            }
        });
        for action in actions {
            match action {
                Action::Mode(mode) => self.set_mode(mode, &ctx),
                Action::Side(side) => self.set_side(side, &ctx),
                Action::Change(direction) => self.jump_change(direction, &ctx),
            }
        }
        if let Some(error) = &self.error {
            bounded_text::field(ui, error, egui::FontId::proportional(12.0), 22.0);
            if ui
                .button(if self.mode == Mode::Full {
                    "Retry file"
                } else {
                    "Retry diff"
                })
                .clicked()
            {
                self.generation += 1;
                self.sent = None;
                self.error = None;
                self.full = None;
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
        if self.mode == Mode::Full {
            if let Some(full) = &self.full {
                bounded_text::field(
                    ui,
                    &format!(
                        "{} side · {} bytes{}",
                        self.side_label(full.side),
                        full.size,
                        if full.complete { "" } else { " · bounded" }
                    ),
                    egui::FontId::proportional(11.0),
                    18.0,
                );
                bounded_text::field(
                    ui,
                    if pending { &status } else { &full.note },
                    egui::FontId::proportional(12.0),
                    22.0,
                );
            } else {
                bounded_text::field(
                    ui,
                    if self.error.is_some() {
                        "Full file unavailable"
                    } else {
                        "Loading full file…"
                    },
                    egui::FontId::proportional(12.0),
                    22.0,
                );
            }
        } else if let Some(info) = &self.info {
            bounded_text::field(
                ui,
                &format!(
                    "{}: {} bytes · {}: {} bytes{}",
                    self.old_label,
                    info.old_size,
                    self.new_label,
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
        if self.mode == Mode::Full {
            self.draw_full(ui, &ctx, keyboard);
            return;
        }
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
            self.reading_keys(&ctx);
            scroll = scroll.vertical_scroll_offset(self.offset);
        }
        if let Some(jump) = self.jump.take() {
            self.offset = (jump as f32 * HEIGHT).clamp(
                0.0,
                (self.count as f32 * HEIGHT - self.rect.height()).max(0.0),
            );
            scroll = scroll.vertical_scroll_offset(self.offset);
        }
        ui.spacing_mut().item_spacing.y = 0.0;
        let shift = ctx.input(|input| input.modifiers.shift);
        let mut cursor = self.cursor;
        let mut anchor = self.anchor;
        let result = scroll.show_rows(ui, HEIGHT, self.count, |ui, rows| {
            self.range = rows.start.saturating_sub(24)..rows.end.saturating_add(24);
            // A conservative monospace width keeps every source column reachable.
            let width = (120.0 + self.columns as f32 * 16.0).max(ui.available_width());
            let (lo, hi) = match (cursor, anchor) {
                (Some(a), Some(b)) => (a.min(b), a.max(b)),
                (Some(a), None) => (a, a),
                _ => (usize::MAX, 0),
            };
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
                if cursor == Some(index) {
                    ui.painter()
                        .rect_filled(rect, 0.0, ui.visuals().selection.bg_fill);
                } else if index >= lo && index <= hi {
                    ui.painter()
                        .rect_filled(rect, 0.0, crate::colors::selection(ui.visuals()));
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
                let response = response
                    .on_hover_text("Click to set the place · Double-click to copy this line");
                if response.clicked() && !response.double_clicked() {
                    cursor = Some(index);
                    if shift {
                        anchor = Some(anchor.unwrap_or(index));
                    } else {
                        anchor = Some(index);
                    }
                }
                if response.double_clicked() {
                    ctx.copy_text(row.text.clone());
                }
            }
        });
        self.offset = result.state.clamped_offset().y;
        self.cursor = cursor;
        self.anchor = anchor;
        self.send(&ctx);
    }

    fn draw_full(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, keyboard: bool) {
        if keyboard && crate::navigation::available(ctx) {
            self.reading_keys(ctx);
        }
        let Some(full) = &self.full else {
            ui.weak(if self.error.is_some() {
                "Full file unavailable"
            } else {
                "Loading full file…"
            });
            self.send(ctx);
            return;
        };
        if full.lines.is_empty() {
            ui.weak(if full.note.is_empty() {
                "Empty file"
            } else {
                &full.note
            });
            self.send(ctx);
            return;
        }
        let mut scroll = egui::ScrollArea::both()
            .id_salt(("file-full", self.displayed_generation))
            .auto_shrink([false, false]);
        if self.reset {
            scroll = scroll.scroll_offset(egui::Vec2::ZERO);
            self.reset = false;
        }
        if let Some(jump) = self.jump.take() {
            self.offset = (jump as f32 * HEIGHT).clamp(
                0.0,
                (self.count as f32 * HEIGHT - self.rect.height()).max(0.0),
            );
            scroll = scroll.vertical_scroll_offset(self.offset);
        }
        ui.spacing_mut().item_spacing.y = 0.0;
        let width = (70.0 + self.columns as f32 * 8.0).max(ui.available_width());
        let shift = ctx.input(|input| input.modifiers.shift);
        let side = self.side;
        let (cursor, anchor) = (self.cursor, self.anchor);
        let mut new_cursor = cursor;
        let mut new_anchor = anchor;
        let result = scroll.show_rows(ui, HEIGHT, self.count, |ui, rows| {
            let (lo, hi) = match (cursor, anchor) {
                (Some(a), Some(b)) => (a.min(b), a.max(b)),
                (Some(a), None) => (a, a),
                _ => (usize::MAX, 0),
            };
            for index in rows {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(width, HEIGHT), egui::Sense::click());
                let Some(line) = full.lines.get(index) else {
                    continue;
                };
                let (_, color, fill) = if line.changed {
                    crate::colors::diff(
                        ui.visuals(),
                        if side == ReadingSide::After {
                            RowKind::Added
                        } else {
                            RowKind::Deleted
                        },
                    )
                } else {
                    ("", ui.visuals().text_color(), egui::Color32::TRANSPARENT)
                };
                if cursor == Some(index) {
                    ui.painter()
                        .rect_filled(rect, 0.0, ui.visuals().selection.bg_fill);
                } else if index >= lo && index <= hi {
                    ui.painter()
                        .rect_filled(rect, 0.0, crate::colors::selection(ui.visuals()));
                } else if line.changed {
                    ui.painter().rect_filled(rect, 0.0, fill);
                }
                let font = egui::FontId::monospace(12.0);
                ui.painter().text(
                    rect.min + egui::vec2(40.0, HEIGHT / 2.0),
                    egui::Align2::RIGHT_CENTER,
                    line.number.to_string(),
                    egui::FontId::monospace(11.0),
                    ui.visuals().weak_text_color(),
                );
                if line.changed {
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            rect.min + egui::vec2(46.0, 3.0),
                            egui::vec2(3.0, HEIGHT - 6.0),
                        ),
                        1.0,
                        color,
                    );
                }
                let origin = rect.min + egui::vec2(58.0, HEIGHT / 2.0);
                let clip = egui::Rect::from_min_max(rect.min + egui::vec2(58.0, 0.0), rect.max)
                    .intersect(ui.clip_rect());
                let (start, end, advance) = ui.fonts_mut(|fonts| {
                    visible_text(
                        &line.text,
                        clip.left() - origin.x,
                        clip.right() - origin.x,
                        |ch| fonts.glyph_width(&font, ch),
                    )
                });
                ui.painter().with_clip_rect(clip).text(
                    origin + egui::vec2(advance, 0.0),
                    egui::Align2::LEFT_CENTER,
                    &line.text[start..end],
                    font,
                    color,
                );
                let response = response.on_hover_text(
                    "Click to set the place · Shift-click to extend · Double-click to copy",
                );
                if response.clicked() && !response.double_clicked() {
                    new_cursor = Some(index);
                    if shift {
                        new_anchor = Some(new_anchor.unwrap_or(index));
                    } else {
                        new_anchor = Some(index);
                    }
                }
                if response.double_clicked() {
                    ctx.copy_text(line.text.clone());
                }
            }
        });
        self.offset = result.state.clamped_offset().y;
        self.cursor = new_cursor;
        self.anchor = new_anchor;
        self.send(ctx);
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
    use gg_engine::{
        diff::{DiffKind, RowKind},
        file::{FileText, ReadingSide},
        review::FileSide,
    };
    use std::collections::BTreeSet;

    fn file(name: &str, before: bool, after: bool) -> FileChange {
        let side = || FileSide {
            id: "0".repeat(40),
            mode: 0o100644,
        };
        FileChange {
            path: name.into(),
            path_bytes: name.as_bytes().into(),
            kind: gg_engine::review::ChangeKind::Modified,
            old: before.then(side),
            new: after.then(side),
        }
    }

    fn diff_update(generation: u64, doc: FileDiff) -> Update {
        let hunks = doc
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.kind == RowKind::Hunk)
            .map(|(index, _)| index)
            .collect();
        Update {
            generation,
            mode: Mode::Diff,
            side: ReadingSide::After,
            preview: None,
            count: doc.rows.len(),
            columns: 5,
            page_start: 0,
            result: Ok(doc),
            full: None,
            hunks,
        }
    }

    fn full_update(generation: u64, side: ReadingSide, text: FileText) -> Update {
        Update {
            generation,
            mode: Mode::Full,
            side,
            preview: None,
            count: text.lines.len(),
            columns: 8,
            page_start: 0,
            result: Ok(FileDiff::summary(DiffKind::Text, 0, 0, "")),
            full: Some(Ok(text)),
            hunks: vec![0],
        }
    }

    #[test]
    fn selection_keeps_displayed_diff_until_replacement_arrives() {
        let ctx = egui::Context::default();
        let mut view = DiffView::default();
        view.select("/repo".into(), Some(file("a", false, true)), &ctx);
        let mut doc = FileDiff::summary(DiffKind::Text, 0, 1, "");
        doc.rows.push(DiffRow {
            kind: RowKind::Added,
            old_line: None,
            new_line: Some(1),
            text: "first".into(),
            bytes: b"first".to_vec(),
        });
        let update = diff_update(view.generation, doc);
        view.accept(update.clone(), None);
        view.select("/repo".into(), Some(file("b", false, true)), &ctx);
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
        let selection = file("a", false, true);
        view.change = Some(selection.clone());
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
        let update = diff_update(1, doc);
        view.accept(update.clone(), None);
        view.change = None; // Isolate rendered input from asynchronous repository reads.

        let frame = |view: &mut DiffView, events| {
            let _ = crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(720.0, 480.0, events),
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| view.draw_reading(ui));
                },
            );
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
            assert_eq!(
                view.displayed.as_ref().map(|f| &f.path),
                Some(&selection.path)
            );
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
        let output = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(720.0, 480.0, vec![]),
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| view.draw(ui));
            },
        );
        assert!(output.shapes.iter().any(|shape| {
            matches!(&shape.shape, egui::Shape::Text(text)
                if text.galley.job.text == "Before: 256 bytes · After: 268 bytes")
        }));
    }

    #[test]
    fn full_file_marks_changes_and_navigates_by_change() {
        let ctx = egui::Context::default();
        let mut view = DiffView {
            change: Some(file("a", true, true)),
            generation: 1,
            side: ReadingSide::After,
            mode: Mode::Full,
            ..Default::default()
        };
        let changed = BTreeSet::from([2usize, 5]);
        let text = FileText::from_bytes(
            ReadingSide::After,
            b"one\ntwo\nthree\nfour\nfive\n",
            &changed,
        );
        view.accept(full_update(1, ReadingSide::After, text), None);
        assert_eq!(view.count, 5);
        assert_eq!(view.mode, Mode::Full);
        assert_eq!(view.cursor, Some(1));
        view.cursor = Some(0);
        view.anchor = Some(0);
        assert_eq!(view.change_position(), (1, 2));
        view.cursor = Some(1);
        view.anchor = Some(1);
        view.jump_change(1, &ctx);
        assert_eq!(view.cursor, Some(4));
        view.jump_change(-1, &ctx);
        assert_eq!(view.cursor, Some(1));
        assert_eq!(view.change_position(), (1, 2));
    }

    #[test]
    fn diff_change_navigation_reaches_the_first_hunk_and_stops_at_the_ends() {
        let ctx = egui::Context::default();
        let mut view = DiffView {
            count: 200,
            hunks: vec![0, 40],
            ..Default::default()
        };
        assert_eq!(view.change_position(), (1, 2));
        assert_eq!(view.previous_change(), None);
        assert_eq!(view.next_change(), Some(0));

        view.jump_change(1, &ctx);
        assert_eq!(view.cursor, Some(0));
        assert_eq!(view.previous_change(), None);
        assert_eq!(view.next_change(), Some(40));

        view.jump_change(1, &ctx);
        assert_eq!(view.cursor, Some(40));
        assert_eq!(view.previous_change(), Some(0));
        assert_eq!(view.next_change(), None);
        view.jump_change(1, &ctx);
        assert_eq!(view.cursor, Some(40));

        view.jump_change(-1, &ctx);
        assert_eq!(view.cursor, Some(0));
        view.jump_change(-1, &ctx);
        assert_eq!(view.cursor, Some(0));
    }

    #[test]
    fn full_file_change_navigation_handles_both_boundaries() {
        let ctx = egui::Context::default();
        let mut view = DiffView {
            change: Some(file("a", true, true)),
            generation: 1,
            mode: Mode::Full,
            ..Default::default()
        };
        let body: Vec<u8> = (1..=50)
            .map(|line| format!("line {line}\n"))
            .collect::<String>()
            .into_bytes();
        let text =
            FileText::from_bytes(ReadingSide::After, &body, &BTreeSet::from([4usize, 5, 40]));
        view.accept(full_update(1, ReadingSide::After, text), None);

        view.cursor = Some(2); // line 3, before the first change.
        assert_eq!(view.change_position(), (1, 2));
        assert_eq!(view.previous_change(), None);
        assert_eq!(view.next_change(), Some(4));
        view.jump_change(1, &ctx);
        assert_eq!(view.cursor, Some(3));
        view.jump_change(1, &ctx);
        assert_eq!(view.cursor, Some(39));
        assert_eq!(view.previous_change(), Some(4));
        assert_eq!(view.next_change(), None);
        view.jump_change(1, &ctx);
        assert_eq!(view.cursor, Some(39));
        view.jump_change(-1, &ctx);
        assert_eq!(view.cursor, Some(3));
        view.jump_change(-1, &ctx);
        assert_eq!(view.cursor, Some(3));
    }

    #[test]
    fn change_key_scrolls_to_an_offscreen_hunk() {
        let ctx = egui::Context::default();
        let mut view = DiffView {
            change: Some(file("a", true, true)),
            generation: 1,
            ..Default::default()
        };
        let mut doc = FileDiff::summary(DiffKind::Text, 0, 200, "");
        doc.rows = (0..200)
            .map(|index| {
                let hunk = index == 0 || index == 40;
                DiffRow {
                    kind: if hunk {
                        RowKind::Hunk
                    } else {
                        RowKind::Context
                    },
                    old_line: (!hunk).then_some(index + 1),
                    new_line: (!hunk).then_some(index + 1),
                    text: if hunk { "@@" } else { "line" }.into(),
                    bytes: vec![],
                }
            })
            .collect();
        view.accept(diff_update(1, doc), None);
        view.change = None; // Isolate rendered input from asynchronous repository reads.
        let frame = |view: &mut DiffView, events| {
            let _ = crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(720.0, 480.0, events),
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| view.draw_reading(ui));
                },
            );
        };
        frame(&mut view, vec![]);
        assert_eq!(view.cursor, None);
        for target in [Some(0), Some(40)] {
            frame(
                &mut view,
                vec![egui::Event::Key {
                    key: egui::Key::CloseBracket,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert_eq!(view.cursor, target, "offset {}", view.offset);
        }
        assert!(
            view.offset >= 40.0 * HEIGHT - 1.0,
            "second change must scroll into view, offset {}",
            view.offset
        );
    }

    #[test]
    fn a_single_change_is_reachable_and_then_terminal_in_both_modes() {
        let ctx = egui::Context::default();
        let mut full = DiffView {
            change: Some(file("a", true, true)),
            generation: 1,
            mode: Mode::Full,
            ..Default::default()
        };
        let text = FileText::from_bytes(ReadingSide::After, b"one\ntwo\n", &BTreeSet::from([1]));
        full.accept(full_update(1, ReadingSide::After, text), None);
        full.cursor = None;
        full.anchor = None;
        assert_eq!(full.change_position(), (1, 1));
        assert_eq!(full.previous_change(), None);
        assert_eq!(full.next_change(), Some(1));
        full.jump_change(1, &ctx);
        assert_eq!(full.cursor, Some(0));
        assert_eq!(full.previous_change(), None);
        assert_eq!(full.next_change(), None);

        let mut diff = DiffView {
            count: 10,
            hunks: vec![0],
            ..Default::default()
        };
        assert_eq!(diff.previous_change(), None);
        assert_eq!(diff.next_change(), Some(0));
        diff.jump_change(1, &ctx);
        assert_eq!(diff.cursor, Some(0));
        assert_eq!(diff.previous_change(), None);
        assert_eq!(diff.next_change(), None);
    }

    #[test]
    fn restored_line_lands_on_the_centered_first_page() {
        let mut view = DiffView {
            change: Some(file("a", true, true)),
            generation: 2,
            ..Default::default()
        };
        view.restore_line = Some(500);
        let mut doc = FileDiff::summary(DiffKind::Text, 0, 1000, "");
        doc.rows = (371..627)
            .map(|index| DiffRow {
                kind: RowKind::Added,
                old_line: None,
                new_line: Some(index + 1),
                text: format!("row {index}"),
                bytes: vec![],
            })
            .collect();
        let mut update = diff_update(2, doc);
        update.count = 1000;
        update.page_start = 371;
        view.accept(update, None);
        assert_eq!(view.cursor, Some(499));
        assert_eq!(view.anchor, Some(499));
        assert_eq!(view.rows[&499].new_line, Some(500));
        assert!(!view.rows.contains_key(&0));
        assert_eq!(view.count, 1000);
    }

    #[test]
    fn mode_toggle_from_a_hunk_anchors_the_first_changed_line() {
        let ctx = egui::Context::default();
        let mut view = DiffView {
            change: Some(file("a", true, true)),
            generation: 1,
            ..Default::default()
        };
        view.rows.insert(
            0,
            DiffRow {
                kind: RowKind::Hunk,
                old_line: None,
                new_line: None,
                text: "@@".into(),
                bytes: vec![],
            },
        );
        view.rows.insert(
            1,
            DiffRow {
                kind: RowKind::Context,
                old_line: Some(1),
                new_line: Some(1),
                text: "one".into(),
                bytes: b"one\n".to_vec(),
            },
        );
        view.rows.insert(
            2,
            DiffRow {
                kind: RowKind::Deleted,
                old_line: Some(2),
                new_line: None,
                text: "old".into(),
                bytes: b"old\n".to_vec(),
            },
        );
        view.rows.insert(
            3,
            DiffRow {
                kind: RowKind::Added,
                old_line: None,
                new_line: Some(2),
                text: "new".into(),
                bytes: b"new\n".to_vec(),
            },
        );
        view.hunks = vec![0];
        view.cursor = Some(0);
        view.anchor = Some(0);
        assert_eq!(view.current_line(), Some(2));
        view.set_mode(Mode::Full, &ctx);
        assert_eq!(view.cursor, Some(1));
        assert_eq!(view.anchor, Some(1));
        // Without a loaded diff page, full mode waits for its first change.
        view.mode = Mode::Diff;
        view.rows.clear();
        view.cursor = None;
        view.anchor = None;
        view.set_mode(Mode::Full, &ctx);
        assert_eq!(view.cursor, None);
        let text =
            FileText::from_bytes(ReadingSide::After, b"one\nnew\n", &BTreeSet::from([2usize]));
        view.accept(full_update(1, ReadingSide::After, text), None);
        assert_eq!(view.cursor, Some(1));
    }

    #[test]
    fn full_file_side_switch_and_mode_toggle_keep_the_line() {
        let ctx = egui::Context::default();
        let mut view = DiffView {
            change: Some(file("a", true, true)),
            generation: 1,
            mode: Mode::Full,
            ..Default::default()
        };
        let changed = BTreeSet::from([3usize]);
        let after = FileText::from_bytes(ReadingSide::After, b"a\nb\nc\nd\n", &changed);
        view.accept(full_update(1, ReadingSide::After, after.clone()), None);
        view.cursor = Some(2);
        view.anchor = Some(2);
        let before = FileText::from_bytes(
            ReadingSide::Before,
            b"a\nb\nold\nc\nd\n",
            &BTreeSet::from([3usize]),
        );
        view.accept(full_update(1, ReadingSide::Before, before), None);
        // The accepted side is stale until the requested side arrives.
        assert_eq!(view.side, ReadingSide::After);
        view.set_side(ReadingSide::Before, &ctx);
        view.accept(
            full_update(
                1,
                ReadingSide::Before,
                FileText::from_bytes(
                    ReadingSide::Before,
                    b"a\nb\nold\nc\nd\n",
                    &BTreeSet::from([3usize]),
                ),
            ),
            None,
        );
        assert_eq!(view.side, ReadingSide::Before);
        assert_eq!(view.full_side, ReadingSide::Before);
        assert_eq!(view.cursor, Some(2));
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
        let mut update = diff_update(1, doc);
        view.accept(update.clone(), None);
        assert_eq!(view.count, 0);
        update.generation = 2;
        view.accept(update.clone(), Some(0));
        assert_eq!(view.count, 1);
        assert!(view.rows.is_empty());
        view.accept(update, None);
        assert_eq!(view.rows[&0].text, "new");
    }

    #[test]
    fn saved_reader_context_restores_place_and_endpoints() -> Result<(), String> {
        let ctx = egui::Context::default();
        let mut view = DiffView::default();
        let change = file("src/app.rs", true, true);
        view.change = Some(change);
        view.path = "repo".into();
        view.generation = 4;
        view.set_endpoints("main (11111111)", "feature (22222222)");
        view.mode = Mode::Full;
        view.side = ReadingSide::Before;
        view.cursor = Some(4);
        view.anchor = Some(7);
        view.offset = 88.0;
        let saved = view.save_context().ok_or("context")?;
        assert_eq!(saved.before.label, "main (11111111)");
        assert_eq!(saved.after.label, "feature (22222222)");
        assert_eq!(saved.change.path, "src/app.rs");
        assert_eq!(saved.place.mode, Mode::Full);
        assert_eq!(saved.place.side, ReadingSide::Before);
        assert_eq!(saved.place.cursor, Some(4));
        assert_eq!(saved.place.anchor, Some(7));
        assert_eq!(saved.place.offset, 88.0);

        view.mode = Mode::Diff;
        view.side = ReadingSide::After;
        view.cursor = None;
        view.anchor = None;
        view.offset = 0.0;
        view.open_context(&saved, &ctx);
        assert_eq!(view.mode, Mode::Full);
        assert_eq!(view.side, ReadingSide::Before);
        assert_eq!(view.cursor, Some(4));
        assert_eq!(view.anchor, Some(7));
        assert_eq!(view.offset, 88.0);
        assert_eq!(view.before_endpoint.as_deref(), Some("main (11111111)"));
        assert_eq!(view.after_endpoint.as_deref(), Some("feature (22222222)"));
        Ok(())
    }

    #[test]
    fn reader_capabilities_follow_the_exact_sides() {
        let mut view = DiffView {
            change: Some(file("src/app.rs", false, true)),
            ..Default::default()
        };
        let capabilities = view.capabilities();
        assert!(capabilities.diff);
        assert!(!capabilities.full_before);
        assert!(capabilities.full_after);
        assert!(!capabilities.export_before);
        assert!(!capabilities.export_after);

        view.displayed = view.change.clone();
        let capabilities = view.capabilities();
        assert!(capabilities.export_after);
        assert!(!capabilities.export_before);
    }

    #[test]
    fn the_reader_header_names_both_exact_endpoints() {
        let ctx = egui::Context::default();
        let mut view = DiffView {
            change: Some(file("src/app.rs", true, true)),
            ..Default::default()
        };
        view.set_endpoints("main (11111111)", "feature (22222222)");
        let output = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(720.0, 480.0, vec![]),
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| view.draw(ui));
            },
        );
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.job.text == "main (11111111) → feature (22222222)"
        )));
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
            against: None,
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
            let _ = crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(1120.0, 740.0, vec![]),
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| view.draw(ui));
                },
            );
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
