//! Side-by-side staged/unstaged diffs with inline hunk and line actions.
use crate::bounded_text;
use eframe::egui;
use gg_engine::{
    diff::{DiffKind, DiffRow, FileDiff, RowKind},
    status::{Area, Kind, StatusEntry, StatusIdentity, WorkingRequest},
};
use gg_git::patch::{Change, Edit};
use std::{
    collections::BTreeSet,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, TryRecvError},
    },
    thread,
};

const HEIGHT: f32 = 20.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum PaneKind {
    Staged,
    Unstaged,
}

struct Part {
    pane: usize,
    result: Result<FileDiff, String>,
}

struct Load {
    rx: Receiver<Part>,
    cancel: Arc<AtomicBool>,
    remaining: usize,
}

impl Drop for Load {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

struct HunkInfo {
    header: usize,
    changes: Vec<usize>,
}

#[derive(Default)]
struct Pane {
    request: Option<WorkingRequest>,
    diff: Option<FileDiff>,
    error: Option<String>,
    loading: bool,
    selected: BTreeSet<usize>,
    hunks: Vec<HunkInfo>,
    columns: usize,
    offset: f32,
    restore: bool,
}

impl Pane {
    fn rows(&self) -> usize {
        self.diff.as_ref().map_or(0, |diff| diff.rows.len())
    }

    fn text(&self) -> bool {
        self.diff
            .as_ref()
            .is_some_and(|diff| diff.kind == DiffKind::Text && diff.complete)
    }

    fn can_line_edit(&self, kind: PaneKind) -> bool {
        if !self.text() {
            return false;
        }
        self.request.as_ref().is_some_and(|request| match kind {
            PaneKind::Staged => request.entry.area == Area::Staged,
            PaneKind::Unstaged => {
                request.entry.area == Area::Unstaged && request.entry.kind == Kind::Modified
            }
        })
    }

    fn can_whole_edit(&self, kind: PaneKind) -> bool {
        self.rows() > 0
            && self.request.as_ref().is_some_and(|request| match kind {
                PaneKind::Staged => request.entry.area == Area::Staged,
                PaneKind::Unstaged => {
                    matches!(request.entry.area, Area::Unstaged | Area::Untracked)
                }
            })
    }

    fn can_drop_whole(&self) -> bool {
        self.rows() > 0
            && self.request.as_ref().is_some_and(|request| {
                matches!(request.entry.area, Area::Unstaged | Area::Untracked)
                    && !matches!(request.entry.kind, Kind::IntentToAdd | Kind::Conflicted)
            })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DropScope {
    Selected,
    Hunk,
    File,
}

enum Action {
    Hunk {
        pane: usize,
        edit: Edit,
    },
    Whole {
        pane: usize,
    },
    Drop {
        pane: usize,
        edit: Option<Edit>,
        scope: DropScope,
    },
}

#[derive(Default)]
pub struct StageView {
    path: String,
    panes: [Pane; 2],
    load: Option<Load>,
    mutation: Option<Receiver<Result<StatusIdentity, String>>>,
    mutation_label: Option<String>,
    drop_confirm: Option<Action>,
    error: Option<String>,
    changed: bool,
    narrow_focus: usize,
    narrow_split: bool,
}

impl StageView {
    pub fn select_working(
        &mut self,
        repository: StatusIdentity,
        entry: StatusEntry,
        companion: Option<StatusEntry>,
        ctx: &egui::Context,
    ) {
        self.path = entry.path.clone();
        self.panes = [
            Pane {
                request: Some(WorkingRequest {
                    repository: repository.clone(),
                    entry: staged_entry(&entry, companion.as_ref()),
                }),
                restore: true,
                ..Pane::default()
            },
            Pane {
                request: Some(WorkingRequest {
                    repository,
                    entry: unstaged_entry(&entry, companion.as_ref()),
                }),
                restore: true,
                ..Pane::default()
            },
        ];
        self.error = None;
        self.mutation = None;
        self.mutation_label = None;
        self.drop_confirm = None;
        self.narrow_focus = 0;
        self.narrow_split = false;
        self.reload(ctx);
    }

    pub fn reload(&mut self, ctx: &egui::Context) {
        let (Some(staged), Some(unstaged)) =
            (self.panes[0].request.clone(), self.panes[1].request.clone())
        else {
            return;
        };
        self.load = None; // Cancels any prior comparison.
        self.error = None;
        for pane in &mut self.panes {
            pane.loading = true;
            pane.error = None;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let thread_cancel = cancel.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let repaint = ctx.clone();
        match thread::Builder::new()
            .name("gg-stage-diff".into())
            .spawn(move || {
                let cancelled = {
                    let cancel = thread_cancel.clone();
                    move || cancel.load(Ordering::Relaxed)
                };
                let result = gg_git::working_diff::read(&staged, &cancelled);
                if cancelled() {
                    return;
                }
                if tx.send(Part { pane: 0, result }).is_err() {
                    return;
                }
                repaint.request_repaint();
                let result = gg_git::working_diff::read(&unstaged, &cancelled);
                if cancelled() {
                    return;
                }
                let _ = tx.send(Part { pane: 1, result });
                repaint.request_repaint();
            }) {
            Ok(_) => {
                self.load = Some(Load {
                    rx,
                    cancel,
                    remaining: 2,
                });
            }
            Err(e) => {
                for pane in &mut self.panes {
                    pane.loading = false;
                }
                self.error = Some(format!("Start diff: {e}"));
            }
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn pending(&self) -> bool {
        self.mutation.is_some()
    }

    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    #[cfg(test)]
    pub fn ready(&self) -> bool {
        self.panes.iter().all(|pane| !pane.loading)
    }

    #[cfg(test)]
    pub fn has_text(&self, needle: &str) -> bool {
        self.panes.iter().any(|pane| {
            pane.diff
                .as_ref()
                .is_some_and(|diff| diff.rows.iter().any(|row| row.text.contains(needle)))
        })
    }

    #[cfg(test)]
    pub fn pane_has_text(&self, pane: usize, needle: &str) -> bool {
        self.panes.get(pane).is_some_and(|pane| {
            pane.diff
                .as_ref()
                .is_some_and(|diff| diff.rows.iter().any(|row| row.text.contains(needle)))
        })
    }

    #[cfg(test)]
    pub fn pane_ready(&self, pane: usize) -> bool {
        self.panes
            .get(pane)
            .is_some_and(|pane| !pane.loading && pane.diff.is_some())
    }

    #[cfg(test)]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn poll(&mut self, ctx: &egui::Context) {
        let mut parts = vec![];
        let mut done = false;
        if let Some(load) = &mut self.load {
            while let Ok(part) = load.rx.try_recv() {
                parts.push(part);
                load.remaining = load.remaining.saturating_sub(1);
            }
            done = load.remaining == 0;
        }
        for part in parts {
            self.adopt(part.pane, part.result);
        }
        if done {
            self.load = None;
        }
        let result = self.mutation.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Disconnected) => {
                Some(Err("Mutation worker stopped. Refresh status.".into()))
            }
            Err(TryRecvError::Empty) => None,
        });
        if let Some(result) = result {
            self.mutation = None;
            self.mutation_label = None;
            match result {
                Ok(identity) => {
                    for pane in &mut self.panes {
                        if let Some(request) = &mut pane.request {
                            request.repository = identity.clone();
                        }
                    }
                    self.changed = true;
                    self.error = None;
                    self.reload(ctx);
                }
                Err(e) => self.error = Some(e),
            }
        }
    }

    fn adopt(&mut self, index: usize, result: Result<FileDiff, String>) {
        let Some(pane) = self.panes.get_mut(index) else {
            return;
        };
        pane.loading = false;
        match result {
            Ok(diff) => {
                pane.error = None;
                pane.columns = diff
                    .rows
                    .iter()
                    .map(|row| row.text.chars().count())
                    .max()
                    .unwrap_or(0);
                pane.hunks = hunk_infos(&diff.rows);
                let rows = diff.rows.len();
                if pane.offset > rows as f32 * HEIGHT {
                    pane.offset = (rows as f32 * HEIGHT - HEIGHT).max(0.0);
                }
                pane.diff = Some(diff);
                pane.selected.clear();
                pane.restore = true;
            }
            Err(e) => {
                pane.error = Some(e);
            }
        }
    }

    fn apply(&mut self, action: Action, ctx: &egui::Context) {
        if self.mutation.is_some() {
            return;
        }
        let pane_index = match &action {
            Action::Hunk { pane, .. } | Action::Whole { pane } | Action::Drop { pane, .. } => *pane,
        };
        let Some(request) = self
            .panes
            .get(pane_index)
            .and_then(|pane| pane.request.clone())
        else {
            return;
        };
        let path = request.repository.path.clone();
        let base_tree = request.repository.base_tree.clone();
        let label = match &action {
            Action::Drop { .. } => "Dropping…",
            Action::Hunk { .. } | Action::Whole { .. } if pane_index == 0 => "Unstaging…",
            Action::Hunk { .. } | Action::Whole { .. } => "Staging…",
        };
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let repaint = ctx.clone();
        match thread::Builder::new()
            .name("gg-stage-mutate".into())
            .spawn(move || {
                let result = match action {
                    Action::Hunk { pane, edit } => {
                        if pane == 0 {
                            gg_git::unstage::changes(&request, &[edit])
                        } else {
                            gg_git::stage::changes(&request, &[edit])
                        }
                    }
                    Action::Whole { pane } => {
                        if pane == 0 {
                            gg_git::unstage::file(&request)
                        } else {
                            gg_git::stage::file(&request)
                        }
                    }
                    Action::Drop { pane, edit, .. } => {
                        if pane != 1 {
                            Err("Drop is only available for working changes".into())
                        } else if let Some(edit) = edit {
                            gg_git::drop::changes(&request, &[edit])
                        } else {
                            gg_git::drop::file(&request)
                        }
                    }
                }
                .and_then(|()| {
                    gg_git::status::Session::open_with_base_tree(&path, base_tree)
                        .map(|session| session.identity)
                        .map_err(|e| format!("Refresh diff: {e}"))
                });
                let _ = tx.send(result);
                repaint.request_repaint();
            }) {
            Ok(_) => {
                self.mutation = Some(rx);
                self.mutation_label = Some(label.into());
                self.error = None;
                self.changed = true;
            }
            Err(e) => self.error = Some(format!("Start mutation: {e}")),
        }
    }

    pub fn draw(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.poll(ctx);
        if let Some(label) = &self.mutation_label {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.weak(label);
            });
        }
        if let Some(error) = &self.error {
            bounded_text::field(ui, error, egui::FontId::proportional(12.0), 22.0);
        }
        let mut action = self.drop_confirm.take();
        let mut confirmed = false;
        if let Some(confirm) = &action {
            let scope = match confirm {
                Action::Drop { scope, .. } => drop_scope(*scope),
                _ => "",
            };
            ui.horizontal(|ui| {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    format!(
                        "Discard {scope} from {}? The index is unchanged.",
                        self.path
                    ),
                );
                if ui
                    .add_enabled(
                        self.mutation.is_none(),
                        egui::Button::new("Discard").min_size(egui::vec2(86.0, 22.0)),
                    )
                    .clicked()
                {
                    confirmed = true;
                }
                if ui
                    .add_enabled(
                        self.mutation.is_none(),
                        egui::Button::new("Cancel").min_size(egui::vec2(70.0, 22.0)),
                    )
                    .clicked()
                {
                    action = None;
                }
            });
            ui.weak("Working-tree changes will be lost. The index is unchanged.");
        }
        let blocked = self.mutation.is_some() || action.is_some();
        if confirmed {
            if let Some(action) = action.take() {
                self.apply(action, ctx);
            }
        } else {
            self.drop_confirm = action;
        }
        let mut action = None;
        if ui.available_width() >= 860.0 {
            ui.columns(2, |columns| {
                draw_pane(
                    &mut self.panes[0],
                    PaneKind::Staged,
                    &mut columns[0],
                    blocked,
                    &mut action,
                );
                draw_pane(
                    &mut self.panes[1],
                    PaneKind::Unstaged,
                    &mut columns[1],
                    blocked,
                    &mut action,
                );
            });
        } else {
            let staged_changes = self.panes[0].hunks.len();
            let unstaged_changes = self.panes[1].hunks.len();
            if staged_changes == 0 && unstaged_changes > 0 {
                self.narrow_focus = 1;
            } else if unstaged_changes == 0 && staged_changes > 0 {
                self.narrow_focus = 0;
            }
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.narrow_focus,
                    0,
                    format!("Staged ({staged_changes})"),
                );
                ui.selectable_value(
                    &mut self.narrow_focus,
                    1,
                    format!("Unstaged ({unstaged_changes})"),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button(if self.narrow_split { "Focus" } else { "Split" })
                        .on_hover_text(if self.narrow_split {
                            "Show only the active comparison"
                        } else {
                            "Show both comparisons stacked"
                        })
                        .clicked()
                    {
                        self.narrow_split = !self.narrow_split;
                    }
                });
            });
            if self.narrow_split {
                let height = ui.available_height();
                let half = ((height - 8.0) / 2.0).max(72.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), half),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        draw_pane(
                            &mut self.panes[0],
                            PaneKind::Staged,
                            ui,
                            blocked,
                            &mut action,
                        );
                    },
                );
                ui.separator();
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        draw_pane(
                            &mut self.panes[1],
                            PaneKind::Unstaged,
                            ui,
                            blocked,
                            &mut action,
                        );
                    },
                );
            } else if self.narrow_focus == 0 {
                draw_pane(
                    &mut self.panes[0],
                    PaneKind::Staged,
                    ui,
                    blocked,
                    &mut action,
                );
            } else {
                draw_pane(
                    &mut self.panes[1],
                    PaneKind::Unstaged,
                    ui,
                    blocked,
                    &mut action,
                );
            }
        }
        if let Some(action) = action {
            match action {
                Action::Drop { .. } => self.drop_confirm = Some(action),
                _ => self.apply(action, ctx),
            }
        }
    }
}

fn staged_entry(primary: &StatusEntry, companion: Option<&StatusEntry>) -> StatusEntry {
    if primary.area == Area::Conflict {
        return StatusEntry::new(Area::Conflict, Kind::Conflicted, &primary.path_bytes);
    }
    if primary.area == Area::Staged {
        return primary.clone();
    }
    if let Some(entry) = companion.filter(|entry| entry.area == Area::Staged) {
        return entry.clone();
    }
    StatusEntry::new(Area::Staged, Kind::Modified, &primary.path_bytes)
}

fn unstaged_entry(primary: &StatusEntry, companion: Option<&StatusEntry>) -> StatusEntry {
    if matches!(primary.area, Area::Unstaged | Area::Untracked) {
        return primary.clone();
    }
    if let Some(entry) =
        companion.filter(|entry| matches!(entry.area, Area::Unstaged | Area::Untracked))
    {
        return entry.clone();
    }
    let area = if primary.area == Area::Conflict {
        Area::Conflict
    } else {
        Area::Unstaged
    };
    let kind = if area == Area::Conflict {
        Kind::Conflicted
    } else {
        Kind::Modified
    };
    StatusEntry::new(area, kind, &primary.path_bytes)
}

fn hunk_infos(rows: &[DiffRow]) -> Vec<HunkInfo> {
    let mut hunks = vec![];
    for (index, row) in rows.iter().enumerate() {
        if row.kind == RowKind::Hunk {
            hunks.push(HunkInfo {
                header: index,
                changes: vec![],
            });
        } else if matches!(row.kind, RowKind::Added | RowKind::Deleted)
            && let Some(hunk) = hunks.last_mut()
        {
            hunk.changes.push(index);
        }
    }
    hunks
}

fn draw_pane(
    pane: &mut Pane,
    kind: PaneKind,
    ui: &mut egui::Ui,
    pending: bool,
    action: &mut Option<Action>,
) {
    let index = match kind {
        PaneKind::Staged => 0,
        PaneKind::Unstaged => 1,
    };
    let (title, route, verb) = match kind {
        PaneKind::Staged => (
            "Staged",
            if pane
                .request
                .as_ref()
                .is_some_and(|request| request.repository.base_tree.is_some())
            {
                "parent to index"
            } else {
                "HEAD to index"
            },
            "Unstage",
        ),
        PaneKind::Unstaged => ("Unstaged", "index to working", "Stage"),
    };
    ui.horizontal(|ui| {
        ui.strong(title);
        ui.weak(route);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if pane.can_whole_edit(kind)
                && ui
                    .add_enabled(
                        !pending,
                        egui::Button::new(format!("{verb} file")).min_size(egui::vec2(88.0, 20.0)),
                    )
                    .clicked()
            {
                *action = Some(Action::Whole { pane: index });
            }
            if kind == PaneKind::Unstaged
                && pane.can_drop_whole()
                && ui
                    .add_enabled(
                        !pending,
                        egui::Button::new("Discard file").min_size(egui::vec2(76.0, 20.0)),
                    )
                    .on_hover_text(
                        "Discard all working changes in this file; the index is unchanged",
                    )
                    .clicked()
            {
                *action = Some(Action::Drop {
                    pane: index,
                    edit: None,
                    scope: DropScope::File,
                });
            }
        });
    });
    let note = if let Some(error) = &pane.error {
        error.clone()
    } else if pane.loading {
        if pane.diff.is_some() {
            "Refreshing…".into()
        } else {
            "Loading…".into()
        }
    } else if let Some(diff) = &pane.diff {
        if diff.rows.is_empty() {
            if kind == PaneKind::Staged {
                "No staged changes".into()
            } else {
                "No unstaged changes".into()
            }
        } else {
            let mut note = diff.note.clone();
            if note.is_empty() {
                note = format!("{} to {} bytes", diff.old_size, diff.new_size);
            }
            note
        }
    } else {
        "Loading…".into()
    };
    bounded_text::field(
        ui,
        &note,
        egui::FontId::proportional(11.0),
        if pane.error.is_some() { 22.0 } else { 18.0 },
    );
    ui.separator();
    let Some(diff) = &pane.diff else {
        return;
    };
    if diff.rows.is_empty() {
        return;
    }
    let can_edit = pane.can_line_edit(kind);
    let mut scroll = egui::ScrollArea::both()
        .id_salt(("stage-pane", index))
        .auto_shrink([false, false]);
    if pane.restore {
        scroll = scroll.vertical_scroll_offset(pane.offset);
        pane.restore = false;
    }
    ui.spacing_mut().item_spacing.y = 0.0;
    let font = egui::FontId::monospace(12.0);
    let Pane {
        diff,
        selected,
        hunks,
        columns,
        offset,
        ..
    } = pane;
    let Some(diff) = diff else {
        return;
    };
    let width = (120.0 + *columns as f32 * 8.0).max(ui.available_width());
    let out = scroll.show_rows(ui, HEIGHT, diff.rows.len(), |ui, rows| {
        for index in rows {
            let row = &diff.rows[index];
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(width, HEIGHT),
                egui::Sense::click(),
            );
            let (sign, color, fill) = crate::colors::diff(ui.visuals(), row.kind);
            ui.painter().rect_filled(rect, 0.0, fill);
            if selected.contains(&index) {
                ui.painter()
                    .rect_filled(rect, 0.0, crate::colors::selection(ui.visuals()));
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(rect.min, egui::vec2(3.0, HEIGHT)),
                    0.0,
                    ui.visuals().selection.stroke.color,
                );
            }
            let mut button_clicked = false;
            let mut drop_clicked = false;
            if row.kind == RowKind::Hunk && can_edit {
                let hunk = hunks.iter().find(|hunk| hunk.header == index);
                let chosen = hunk.map_or(0, |hunk| {
                    hunk.changes
                        .iter()
                        .filter(|line| selected.contains(line))
                        .count()
                });
                let label = if chosen == 0 {
                    format!("{verb} hunk")
                } else {
                    format!("{verb} {chosen}")
                };
                let button = egui::Rect::from_min_size(
                    rect.min + egui::vec2(4.0, 1.0),
                    egui::vec2(104.0, HEIGHT - 2.0),
                );
                if pending {
                    ui.painter().rect(
                        button,
                        3.0,
                        ui.visuals().widgets.noninteractive.bg_fill,
                        ui.visuals().widgets.noninteractive.bg_stroke,
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        button.center(),
                        egui::Align2::CENTER_CENTER,
                        &label,
                        egui::FontId::proportional(11.0),
                        ui.visuals().weak_text_color(),
                    );
                } else {
                    let response = ui
                        .interact(
                            button,
                            ui.scope_id().with(("stage-hunk", index_kind(kind), index)),
                            egui::Sense::click(),
                        )
                        .on_hover_text(
                            "Apply the selected changed lines, or the whole hunk when none are selected",
                        );
                    button_clicked = response.clicked();
                    let fill = if response.hovered() {
                        ui.visuals().widgets.hovered.bg_fill
                    } else {
                        ui.visuals().widgets.inactive.bg_fill
                    };
                    ui.painter().rect(
                        button,
                        3.0,
                        fill,
                        ui.visuals().widgets.inactive.fg_stroke,
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        button.center(),
                        egui::Align2::CENTER_CENTER,
                        &label,
                        egui::FontId::proportional(11.0),
                        ui.visuals().strong_text_color(),
                    );
                    if button_clicked {
                        let edit = hunk.map(|hunk| {
                            let lines = hunk
                                .changes
                                .iter()
                                .copied()
                                .filter(|line| selected.contains(line))
                                .collect::<Vec<_>>();
                            Edit {
                                header: row.text.clone(),
                                changes: if lines.is_empty() {
                                    vec![]
                                } else {
                                    lines
                                        .into_iter()
                                        .filter_map(|line| match diff.rows[line].kind {
                                            RowKind::Added => diff.rows[line]
                                                .new_line
                                                .map(Change::Added),
                                            RowKind::Deleted => diff.rows[line]
                                                .old_line
                                                .map(Change::Deleted),
                                            _ => None,
                                        })
                                        .collect()
                                },
                            }
                        });
                        if let Some(edit) = edit {
                            *action = Some(Action::Hunk {
                                pane: index_kind(kind),
                                edit,
                            });
                        }
                    }
                }
                if kind == PaneKind::Unstaged
                    && let Some(hunk) = hunk
                {
                    let chosen = hunk
                        .changes
                        .iter()
                        .any(|line| selected.contains(line));
                    let label = if chosen {
                        "Discard selected"
                    } else {
                        "Discard hunk"
                    };
                    let button = egui::Rect::from_min_size(
                        egui::pos2(rect.right() - 108.0, rect.top() + 1.0),
                        egui::vec2(104.0, HEIGHT - 2.0),
                    );
                    if pending {
                        ui.painter().rect(
                            button,
                            3.0,
                            ui.visuals().widgets.noninteractive.bg_fill,
                            ui.visuals().widgets.noninteractive.bg_stroke,
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(
                            button.center(),
                            egui::Align2::CENTER_CENTER,
                            label,
                            egui::FontId::proportional(11.0),
                            ui.visuals().weak_text_color(),
                        );
                    } else {
                        let response = ui
                            .interact(
                                button,
                                ui.scope_id().with(("drop-hunk", index)),
                                egui::Sense::click(),
                            )
                            .on_hover_text(
                                "Discard the selected change block, or the whole hunk when none is selected",
                            );
                        drop_clicked = response.clicked();
                        let fill = if response.hovered() {
                            ui.visuals().widgets.hovered.bg_fill
                        } else {
                            ui.visuals().widgets.inactive.bg_fill
                        };
                        ui.painter().rect(
                            button,
                            3.0,
                            fill,
                            ui.visuals().widgets.inactive.fg_stroke,
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(
                            button.center(),
                            egui::Align2::CENTER_CENTER,
                            label,
                            egui::FontId::proportional(11.0),
                            ui.visuals().strong_text_color(),
                        );
                        if drop_clicked {
                            *action = Some(Action::Drop {
                                pane: index_kind(kind),
                                edit: Some(Edit {
                                    header: row.text.clone(),
                                    changes: drop_changes(&diff.rows, hunk, selected),
                                }),
                                scope: if chosen {
                                    DropScope::Selected
                                } else {
                                    DropScope::Hunk
                                },
                            });
                        }
                    }
                }
            }
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
            let text_right = if kind == PaneKind::Unstaged
                && can_edit
                && row.kind == RowKind::Hunk
            {
                rect.right() - 112.0
            } else {
                rect.right()
            };
            let clip = egui::Rect::from_min_max(
                rect.min + egui::vec2(116.0, 0.0),
                egui::pos2(text_right, rect.bottom()),
            )
            .intersect(ui.clip_rect());
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
                font.clone(),
                color,
            );
            let hover = if can_edit && matches!(row.kind, RowKind::Added | RowKind::Deleted) {
                "Click to select this changed line for partial staging"
            } else if can_edit && row.kind == RowKind::Hunk {
                "Click to select all changed lines in this hunk"
            } else {
                "Click to copy this displayed line"
            };
            if !pending
                && response.on_hover_text(hover).clicked()
                && !button_clicked
                && !drop_clicked
            {
                if can_edit && row.kind == RowKind::Hunk {
                    if let Some(hunk) = hunks.iter().find(|hunk| hunk.header == index) {
                        let all = hunk.changes.iter().all(|line| selected.contains(line));
                        if all {
                            for line in &hunk.changes {
                                selected.remove(line);
                            }
                        } else {
                            selected.extend(hunk.changes.iter().copied());
                        }
                    }
                } else if can_edit
                    && matches!(row.kind, RowKind::Added | RowKind::Deleted)
                {
                    if !selected.remove(&index) {
                        selected.insert(index);
                    }
                } else {
                    ui.ctx().copy_text(row.text.clone());
                }
            }
        }
    });
    *offset = out.state.clamped_offset().y;
}

fn drop_changes(rows: &[DiffRow], hunk: &HunkInfo, selected: &BTreeSet<usize>) -> Vec<Change> {
    let chosen = hunk.changes.iter().any(|line| selected.contains(line));
    if !chosen {
        return vec![];
    }
    let mut blocks: Vec<Vec<usize>> = vec![];
    for line in &hunk.changes {
        match blocks.last_mut() {
            Some(block) if block.last().is_some_and(|last| *line == last + 1) => {
                block.push(*line);
            }
            _ => blocks.push(vec![*line]),
        }
    }
    blocks
        .into_iter()
        .filter(|block| block.iter().any(|line| selected.contains(line)))
        .flatten()
        .filter_map(|line| match rows[line].kind {
            RowKind::Added => rows[line].new_line.map(Change::Added),
            RowKind::Deleted => rows[line].old_line.map(Change::Deleted),
            _ => None,
        })
        .collect()
}

fn drop_scope(scope: DropScope) -> &'static str {
    match scope {
        DropScope::Selected => "the selected changes",
        DropScope::Hunk => "this hunk",
        DropScope::File => "all working changes",
    }
}

fn index_kind(kind: PaneKind) -> usize {
    match kind {
        PaneKind::Staged => 0,
        PaneKind::Unstaged => 1,
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

    #[test]
    fn conflicted_paths_never_open_an_editable_pane() {
        let conflict = StatusEntry::new(Area::Conflict, Kind::Conflicted, b"conflicted.txt");
        let staged = staged_entry(&conflict, None);
        let unstaged = unstaged_entry(&conflict, None);
        assert_eq!(staged.area, Area::Conflict);
        assert_eq!(unstaged.area, Area::Conflict);
        let pane = Pane {
            request: Some(WorkingRequest {
                repository: StatusIdentity {
                    path: "/nonexistent".into(),
                    head: None,
                    index: String::new(),
                    base_tree: None,
                },
                entry: staged,
            }),
            ..Pane::default()
        };
        assert!(!pane.can_line_edit(PaneKind::Staged));
        assert!(!pane.can_whole_edit(PaneKind::Staged));
    }
}
