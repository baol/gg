//! Local Changes workspace: independent Unstaged/Staged lists beside one reader.
use crate::{
    bounded_text,
    status_worker::{Ranges, Update, Worker},
};
use eframe::egui;
use gg_engine::status::{Area, Kind, StatusEntry};
use std::{collections::BTreeMap, ops::Range, path::PathBuf};
const HEIGHT: f32 = 24.0;
const GROUP_HEADER: f32 = 24.0;
const WIDE_LAYOUT: f32 = 900.0;
type Comparison = (Vec<u8>, Area);
fn commit_reason(
    busy: bool,
    scanning: bool,
    message: &str,
    amend: bool,
    staged: usize,
) -> Option<&'static str> {
    if busy {
        Some("Wait for the current operation")
    } else if scanning {
        Some("Waiting for the status scan")
    } else if message.trim().is_empty() {
        Some("Enter a commit message")
    } else if !amend && staged == 0 {
        Some("Stage at least one file")
    } else {
        None
    }
}
fn group_name(list: usize) -> &'static str {
    if list == 0 { "Unstaged" } else { "Staged" }
}
fn area_badge(entry: &StatusEntry) -> Option<&'static str> {
    match entry.area {
        Area::Untracked => Some("Untracked"),
        Area::Conflict => Some("Conflict"),
        Area::Staged | Area::Unstaged => None,
    }
}

/// Session-level commit text and selection retained while the workspace is closed.
#[derive(Clone, Default)]
pub(crate) struct Draft {
    pub normal: String,
    pub amend: String,
    pub amend_mode: bool,
    pub amend_base: Option<String>,
    pub selected: Option<Comparison>,
}

struct List {
    rows: BTreeMap<usize, StatusEntry>,
    count: usize,
    viewport: Range<usize>,
    offset: f32,
    rect: egui::Rect,
    selected: Option<Comparison>,
    selected_row: Option<usize>,
    pending_row: Option<usize>,
    reveal: bool,
    restore: bool,
    collapsed: bool,
}
impl List {
    fn new() -> Self {
        Self {
            rows: BTreeMap::new(),
            count: 0,
            viewport: 0..64,
            offset: 0.0,
            rect: egui::Rect::NOTHING,
            selected: None,
            selected_row: None,
            pending_row: None,
            reveal: false,
            restore: false,
            collapsed: false,
        }
    }
}

pub struct StatusView {
    path: PathBuf,
    identity: Option<gg_engine::status::StatusIdentity>,
    stage: crate::stage_view::StageView,
    commit: Option<std::sync::mpsc::Receiver<Result<gg_git::commit::Receipt, String>>>,
    commit_done: Option<String>,
    last_commit: Option<String>,
    commit_warning: Option<String>,
    message: String,
    normal_draft: String,
    amend_draft: String,
    amend: bool,
    amend_base: Option<String>,
    amend_base_commit: Option<String>,
    amend_subject: Option<String>,
    message_expanded: bool,
    pending_selection: Option<Comparison>,
    last_amended: bool,
    worker: Option<Worker>,
    pending: Option<Worker>,
    staged: Option<Update>,
    lists: [List; 2],
    focus: usize,
    total: usize,
    complete: bool,
    error: Option<String>,
    action_error: Option<String>,
    replacing: bool,
    active: Option<Comparison>,
    active_entry: Option<StatusEntry>,
    reader_rect: egui::Rect,
    form_rect: egui::Rect,
    last_scroll: f64,
}
impl StatusView {
    pub fn new(path: PathBuf, ctx: &egui::Context) -> Self {
        let mut view = Self {
            path,
            identity: None,
            stage: crate::stage_view::StageView::default(),
            commit: None,
            commit_done: None,
            last_commit: None,
            commit_warning: None,
            message: String::new(),
            normal_draft: String::new(),
            amend_draft: String::new(),
            amend: false,
            amend_base: None,
            amend_base_commit: None,
            amend_subject: None,
            message_expanded: false,
            pending_selection: None,
            last_amended: false,
            worker: None,
            pending: None,
            staged: None,
            lists: [List::new(), List::new()],
            focus: 0,
            total: 0,
            complete: false,
            error: None,
            action_error: None,
            replacing: false,
            active: None,
            active_entry: None,
            reader_rect: egui::Rect::NOTHING,
            form_rect: egui::Rect::NOTHING,
            last_scroll: f64::NEG_INFINITY,
        };
        view.start(ctx, false);
        view
    }
    fn start(&mut self, ctx: &egui::Context, replacing: bool) {
        self.staged = None;
        self.error = None;
        self.action_error = None;
        self.replacing = replacing;
        match Worker::start(self.path.clone(), ctx.clone(), self.amend_base.clone()) {
            Ok(worker) => {
                if replacing {
                    // A refresh supersedes the previous scan; do not wait for a
                    // parked worker that may never publish another update.
                    self.worker = None;
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
    fn restart(&mut self, ctx: &egui::Context) {
        self.worker = None;
        self.pending = None;
        for list in &mut self.lists {
            list.rows.clear();
            list.count = 0;
            list.restore = true;
        }
        self.total = 0;
        self.complete = false;
        self.staged = None;
        self.start(ctx, false);
    }
    fn accept(&mut self, safe: bool, ctx: &egui::Context) {
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
        let replaced = self.replacing;
        if replaced {
            self.worker = self.pending.take();
            for list in &mut self.lists {
                list.rows.clear();
                list.restore = true;
            }
            self.replacing = false;
        }
        let selection = update.selection.clone();
        self.path = update.identity.path.clone();
        self.total = update.count;
        self.lists[0].count = update.unstaged_count();
        self.lists[1].count = update.staged;
        self.identity = Some(update.identity);
        self.complete = update.complete;
        // Paging a retained scan must preserve an action failure until explicit retry.
        self.error = update.error.or_else(|| self.action_error.clone());
        for (index, rows) in update.rows.into_iter().enumerate() {
            self.lists[index].rows.extend(rows);
        }
        if let Some((list, row, entry)) = &selection {
            self.lists[*list].rows.insert(*row, entry.clone());
        }
        self.prune_rows(selection.as_ref().map(|(list, row, _)| (*list, *row)));
        if self.lists[self.focus].count == 0 && self.lists[1 - self.focus].count > 0 {
            self.focus = 1 - self.focus;
        }
        self.restore_selection_rows();
        if let Some(index) = (0..2).find(|index| self.lists[*index].pending_row.is_some()) {
            let row = self.lists[index].pending_row.unwrap_or(0);
            if self.lists[index].rows.contains_key(&row) {
                self.select_row(index, row, ctx);
            } else {
                self.lists[index].selected_row = Some(row);
                self.lists[index].reveal = true;
            }
        }
        if let Some((list, row, entry)) = &selection {
            self.lists[*list].selected_row = Some(*row);
            self.lists[*list].selected = Some((entry.path_bytes.clone(), entry.area));
            self.focus = *list;
        }
        if let Some(selected) = self.pending_selection.clone()
            && let Some((list, row)) = self.find_row(&selected)
        {
            self.select_row(list, row, ctx);
            self.lists[list].reveal = true;
            self.pending_selection = None;
        } else if self.pending_selection.is_some() && self.complete {
            // The reviewed path vanished from the scan. Keep it as an explicit
            // empty comparison instead of showing a different file as current.
            self.active = self.pending_selection.take();
            self.active_entry = None;
            self.mark_missing(ctx);
        }
        if self.active.is_none() && self.pending_selection.is_none() && self.total > 0 {
            let (list, row) = if self.lists[0].count > 0 {
                (0, 0)
            } else {
                (1, 0)
            };
            if self.lists[list].rows.contains_key(&row) {
                self.select_row(list, row, ctx);
            }
        }
        let missing = self.complete
            && self
                .active
                .clone()
                .is_some_and(|selected| self.find_entry(&selected).is_none());
        if missing {
            self.mark_missing(ctx);
        } else {
            self.sync_active(ctx, replaced);
        }
    }
    fn mark_missing(&mut self, ctx: &egui::Context) {
        let Some(selected) = self.active.clone() else {
            return;
        };
        let Some(identity) = self.identity.clone() else {
            return;
        };
        self.active_entry = None;
        self.stage
            .select_missing(identity, selected.0, selected.1, ctx);
    }
    fn prune_rows(&mut self, pinned: Option<(usize, usize)>) {
        for index in 0..2 {
            let list = &mut self.lists[index];
            let keep =
                list.viewport.start.saturating_sub(256)..list.viewport.end.saturating_add(256);
            list.rows
                .retain(|row, _| keep.contains(row) || pinned == Some((index, *row)));
        }
        while self.lists[0].rows.len() + self.lists[1].rows.len() > 768 {
            let index = if self.lists[0].rows.len() >= self.lists[1].rows.len() {
                0
            } else {
                1
            };
            let victim = self.lists[index]
                .rows
                .keys()
                .copied()
                .find(|row| pinned != Some((index, *row)));
            let Some(row) = victim else {
                break;
            };
            self.lists[index].rows.remove(&row);
        }
    }
    fn restore_selection_rows(&mut self) {
        for list in &mut self.lists {
            list.selected_row = list.selected.as_ref().and_then(|selected| {
                list.rows
                    .iter()
                    .find(|(_, entry)| entry.path_bytes == selected.0 && entry.area == selected.1)
                    .map(|(row, _)| *row)
            });
        }
    }
    fn find_row(&self, selected: &Comparison) -> Option<(usize, usize)> {
        for (index, list) in self.lists.iter().enumerate() {
            if let Some((row, _)) = list
                .rows
                .iter()
                .find(|(_, entry)| entry.path_bytes == selected.0 && entry.area == selected.1)
            {
                return Some((index, *row));
            }
        }
        None
    }
    fn find_entry(&self, selected: &Comparison) -> Option<StatusEntry> {
        self.lists
            .iter()
            .flat_map(|list| list.rows.values())
            .find(|entry| entry.path_bytes == selected.0 && entry.area == selected.1)
            .cloned()
    }
    fn companion(&self, entry: &StatusEntry) -> Option<StatusEntry> {
        self.lists
            .iter()
            .flat_map(|list| list.rows.values())
            .find(|candidate| {
                candidate.path_bytes == entry.path_bytes && candidate.area != entry.area
            })
            .cloned()
    }
    fn select_row(&mut self, index: usize, row: usize, ctx: &egui::Context) {
        let Some(entry) = self.lists[index].rows.get(&row).cloned() else {
            self.focus = index;
            self.lists[index].selected = None;
            self.lists[index].selected_row = Some(row);
            self.lists[index].pending_row = Some(row);
            self.lists[index].reveal = true;
            return;
        };
        self.focus = index;
        self.lists[index].pending_row = None;
        let selected = (entry.path_bytes.clone(), entry.area);
        if self.active.as_ref() == Some(&selected) && self.lists[index].selected_row == Some(row) {
            return;
        }
        self.lists[index].selected_row = Some(row);
        self.lists[index].selected = Some(selected.clone());
        self.active = Some(selected);
        self.active_entry = Some(entry.clone());
        if let Some(repository) = self.identity.clone() {
            let companion = self.companion(&entry);
            self.stage.select_working(repository, entry, companion, ctx);
        }
    }
    fn sync_active(&mut self, ctx: &egui::Context, reload: bool) {
        let Some(selected) = self.active.clone() else {
            return;
        };
        let kind = self
            .active_entry
            .as_ref()
            .filter(|entry| entry.path_bytes == selected.0 && entry.area == selected.1)
            .map(|entry| entry.kind)
            .unwrap_or(Kind::Modified);
        let entry = self
            .find_entry(&selected)
            .unwrap_or_else(|| StatusEntry::new(selected.1, kind, &selected.0));
        let companion = self.companion(&entry);
        let repository_changed = self.stage.repository() != self.identity.as_ref();
        let selection_changed = self.active_entry.as_ref() != Some(&entry);
        if let Some(identity) = self.identity.clone()
            && (repository_changed || selection_changed)
        {
            self.stage
                .select_working(identity, entry.clone(), companion, ctx);
        } else if reload {
            self.stage.reload(ctx);
        }
        self.active_entry = Some(entry);
    }
    fn fill_unhovered(&mut self, pinned: Option<(usize, usize)>) {
        if self.replacing {
            return;
        }
        if let Some(update) = &self.staged {
            for (index, rows) in update.rows.iter().enumerate() {
                let count = if index == 0 {
                    update.unstaged_count()
                } else {
                    update.staged
                };
                for (row, entry) in rows {
                    if *row < count && pinned != Some((index, *row)) {
                        self.lists[index].rows.insert(*row, entry.clone());
                    }
                }
            }
        }
        self.prune_rows(pinned);
    }
    fn busy(&self) -> bool {
        self.stage.mutating() || self.commit.is_some()
    }

    pub fn take_commit(&mut self) -> Option<String> {
        self.commit_done.take()
    }

    /// Canonical worktree identity used to keep drafts from crossing repositories.
    pub(crate) fn repository_key(&self) -> PathBuf {
        std::fs::canonicalize(&self.path).unwrap_or_else(|_| self.path.clone())
    }

    /// Capture the active draft before the workspace is dropped.
    pub(crate) fn capture_draft(&self) -> Draft {
        let (normal, amend) = if self.amend {
            (self.normal_draft.clone(), self.message.clone())
        } else {
            (self.message.clone(), self.amend_draft.clone())
        };
        Draft {
            normal,
            amend,
            amend_mode: self.amend,
            amend_base: self.amend_base.clone(),
            selected: self.active.clone(),
        }
    }

    /// Restore a draft after reopening Local Changes for the same worktree.
    pub(crate) fn restore_draft(&mut self, draft: Draft, ctx: &egui::Context) {
        self.normal_draft = draft.normal;
        self.amend_draft = draft.amend;
        self.pending_selection = draft.selected;
        self.message = self.normal_draft.clone();
        let Some(base) = draft.amend_base else {
            self.restart(ctx);
            return;
        };
        if !draft.amend_mode
            || gg_git::status::amend_base_tree(&self.path).ok().as_deref() != Some(base.as_str())
        {
            self.restart(ctx);
            return;
        }
        self.amend = true;
        self.amend_base = Some(base);
        self.amend_base_commit = gg_git::commit::head_parent(&self.path).ok().flatten();
        let head = gg_git::commit::head_message(&self.path).ok();
        self.amend_subject = head
            .as_deref()
            .and_then(|message| message.lines().next())
            .map(str::to_string);
        self.message = if self.amend_draft.is_empty() {
            head.unwrap_or_default()
        } else {
            self.amend_draft.clone()
        };
        self.amend_draft = self.message.clone();
        self.restart(ctx);
    }

    #[cfg(test)]
    pub(crate) fn draft(&self) -> &str {
        &self.message
    }

    #[cfg(test)]
    pub(crate) fn set_draft(&mut self, message: String) {
        self.message = message;
    }

    /// Refresh the status scan without discarding the amend mode, draft, lists, or reader.
    pub fn refresh(&mut self, ctx: &egui::Context) {
        if self.amend {
            match gg_git::status::amend_base_tree(&self.path) {
                Ok(base) => {
                    self.amend_base = Some(base);
                    self.amend_base_commit = gg_git::commit::head_parent(&self.path).ok().flatten();
                }
                Err(error) => {
                    self.amend = false;
                    self.amend_base = None;
                    self.amend_base_commit = None;
                    self.action_error = Some(error);
                }
            }
        }
        self.start(ctx, true);
    }

    /// Enable amend mode, load the current HEAD message, and compare the index with HEAD's parent.
    pub fn start_amend(&mut self, ctx: &egui::Context) -> Result<(), String> {
        if self.busy() || self.replacing {
            return Err("Wait for the current operation to finish.".into());
        }
        let base = gg_git::status::amend_base_tree(&self.path)?;
        let parent = gg_git::commit::head_parent(&self.path)?;
        let message = gg_git::commit::head_message(&self.path)?;
        if !self.amend {
            self.normal_draft = self.message.clone();
        }
        self.amend_subject = Some(
            message
                .lines()
                .next()
                .unwrap_or("(empty message)")
                .to_string(),
        );
        self.message = if self.amend_draft.trim().is_empty() {
            message
        } else {
            self.amend_draft.clone()
        };
        self.amend_draft = self.message.clone();
        self.amend = true;
        self.amend_base = Some(base);
        self.amend_base_commit = parent;
        self.error = None;
        self.action_error = None;
        self.restart(ctx);
        Ok(())
    }

    fn start_commit(&mut self, ctx: &egui::Context) {
        if commit_reason(
            self.busy(),
            self.replacing || !self.complete || self.identity.is_none(),
            &self.message,
            self.amend,
            self.lists[1].count,
        )
        .is_some()
        {
            return;
        }
        let Some(repository) = self.identity.clone() else {
            self.error = Some("Status has not finished loading".into());
            return;
        };
        let message = self.message.clone();
        let amend = self.amend;
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let repaint = ctx.clone();
        match std::thread::Builder::new()
            .name("gg-commit".into())
            .spawn(move || {
                let result = if amend {
                    gg_git::commit::amend(&repository, &message)
                } else {
                    gg_git::commit::create(&repository, &message)
                };
                let _ = tx.send(result);
                repaint.request_repaint();
            }) {
            Ok(_) => {
                self.commit = Some(rx);
                self.error = None;
                self.action_error = None;
            }
            Err(e) => self.error = Some(format!("Start commit: {e}")),
        }
    }

    fn poll_commit(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.commit.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Some(Err("Commit worker stopped. Refresh status.".into()))
            }
            Err(_) => None,
        }) {
            self.commit = None;
            match result {
                Ok(receipt) => {
                    self.last_commit = Some(receipt.id.get(..7).unwrap_or(&receipt.id).to_string());
                    self.last_amended = receipt.amended;
                    if receipt.amended {
                        self.amend_draft.clear();
                    } else {
                        self.normal_draft.clear();
                    }
                    self.amend = false;
                    self.amend_base = None;
                    self.amend_base_commit = None;
                    self.amend_subject = None;
                    self.commit_done = Some(receipt.id.clone());
                    self.commit_warning = if receipt.warnings.is_empty() {
                        None
                    } else {
                        Some(receipt.warnings.join(" · "))
                    };
                    // Clear only the submitted draft; the other mode keeps its text.
                    self.message = if receipt.amended {
                        self.normal_draft.clone()
                    } else {
                        String::new()
                    };
                    self.start(ctx, true);
                }
                Err(e) => {
                    self.start(ctx, true);
                    self.action_error = Some(e.clone());
                    self.error = Some(e);
                }
            }
        }
    }

    fn commit_form_height(&self) -> f32 {
        let amend = if self.amend { GROUP_HEADER } else { 0.0 };
        if self.message_expanded {
            150.0 + amend
        } else {
            82.0 + amend
        }
    }

    fn draw_group(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        content_height: f32,
        collapsible: bool,
        shrink: bool,
    ) -> Option<usize> {
        let busy = self.busy() || self.replacing;
        let complete = self.complete;
        let active = self.focus == index;
        let list = &mut self.lists[index];
        let count = list.count;
        ui.horizontal(|ui| {
            if collapsible {
                let arrow = if list.collapsed { "+" } else { "-" };
                if ui
                    .small_button(arrow)
                    .on_hover_text(if list.collapsed {
                        "Show this group"
                    } else {
                        "Collapse this group"
                    })
                    .clicked()
                {
                    list.collapsed = !list.collapsed;
                }
            }
            ui.strong(format!("{} ({count})", group_name(index)));
        });
        if collapsible && list.collapsed {
            list.rect = egui::Rect::NOTHING;
            return None;
        }
        let mut clicked = None;
        if count == 0 {
            let empty_height = HEIGHT;
            list.rect = egui::Rect::from_min_size(
                ui.cursor().min,
                egui::vec2(ui.available_width(), empty_height),
            );
            ui.allocate_ui_with_layout(
                egui::vec2(list.rect.width(), empty_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.weak(if index == 0 && complete {
                        "No unstaged changes."
                    } else if index == 1 && complete {
                        "No staged changes."
                    } else {
                        "Nothing to show."
                    });
                },
            );
            return None;
        }
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt(("working-group", index))
            .max_height(content_height)
            .auto_shrink([false, shrink]);
        if !shrink {
            scroll = scroll.min_scrolled_height(content_height);
        }
        if list.restore {
            scroll = scroll.vertical_scroll_offset(list.offset);
            list.restore = false;
        }
        if list.reveal
            && let Some(row) = list.selected_row
        {
            let row_top = row as f32 * HEIGHT;
            let row_bottom = row_top + HEIGHT;
            let offset = if row_top < list.offset {
                row_top
            } else if row_bottom > list.offset + content_height {
                row_bottom - content_height
            } else {
                list.offset
            };
            scroll = scroll.vertical_scroll_offset(offset.max(0.0));
            list.reveal = false;
        }
        ui.spacing_mut().item_spacing.y = 0.0;
        let out = scroll.show_rows(ui, HEIGHT, count, |ui, visible| {
            list.viewport = visible.start.saturating_sub(16)..visible.end.saturating_add(16);
            for row in visible {
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), HEIGHT),
                    egui::Sense::click(),
                );
                if list.selected_row == Some(row) {
                    let fill = if active {
                        crate::colors::selection(ui.visuals())
                    } else {
                        crate::colors::selection(ui.visuals()).gamma_multiply(0.35)
                    };
                    ui.painter().rect_filled(rect, 0.0, fill);
                }
                if let Some(entry) = list.rows.get(&row) {
                    if response.clicked() && !busy {
                        response.surrender_focus();
                        clicked = Some(row);
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
                    let area_color = if entry.area == Area::Conflict {
                        ui.visuals().error_fg_color
                    } else {
                        ui.visuals().warn_fg_color
                    };
                    if let Some(badge) = area_badge(entry) {
                        bounded_text::paint(
                            ui,
                            egui::Rect::from_min_size(rect.min, egui::vec2(68.0, HEIGHT)),
                            badge,
                            egui::FontId::proportional(12.0),
                            area_color,
                        );
                    }
                    let kind_x = if area_badge(entry).is_some() {
                        74.0
                    } else {
                        0.0
                    };
                    let path_x = kind_x + 78.0;
                    for (x, width, text, color) in [
                        (kind_x, path_x - kind_x - 6.0, entry.kind.label(), color),
                        (
                            path_x,
                            rect.width() - path_x,
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
                            egui::FontId::proportional(12.0),
                            color,
                        );
                    }
                } else {
                    ui.painter().text(
                        rect.left_center(),
                        egui::Align2::LEFT_CENTER,
                        "Loading…",
                        egui::FontId::proportional(12.0),
                        ui.visuals().weak_text_color(),
                    );
                }
            }
        });
        list.rect = out.inner_rect;
        list.offset = out.state.clamped_offset().y;
        clicked
    }

    fn draw_lists_wide(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, height: f32) {
        let left = (ui.available_width() * 0.32).clamp(250.0, 380.0);
        ui.allocate_ui_with_layout(
            egui::vec2(left, height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                let each = ((height - 6.0) / 2.0 - GROUP_HEADER - 2.0).max(40.0);
                let first = self.draw_group(ui, 0, each, false, true);
                ui.add_space(2.0);
                let second = self.draw_group(ui, 1, each, false, true);
                if let Some(row) = first {
                    self.select_row(0, row, ctx);
                } else if let Some(row) = second {
                    self.select_row(1, row, ctx);
                }
            },
        );
    }

    fn draw_lists_narrow(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, budget: f32) {
        let visible = self
            .lists
            .iter()
            .filter(|list| !list.collapsed)
            .count()
            .max(1);
        let each = ((budget / visible as f32) - GROUP_HEADER - 2.0).max(16.0);
        let first = self.draw_group(ui, 0, each, true, false);
        let second = self.draw_group(ui, 1, each, true, false);
        if let Some(row) = first {
            self.select_row(0, row, ctx);
        } else if let Some(row) = second {
            self.select_row(1, row, ctx);
        }
    }

    fn draw_reader(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.reader_rect = ui.max_rect();
        if self.active.is_some() {
            self.stage.draw(ui, ctx);
        } else {
            ui.weak("Select an Unstaged or Staged entry to read its comparison.");
        }
    }

    fn draw_amend_banner(&self, ui: &mut egui::Ui) {
        let head = self
            .identity
            .as_ref()
            .and_then(|identity| identity.head.as_deref())
            .and_then(|id| id.get(..7))
            .unwrap_or("HEAD");
        let base = self
            .amend_base_commit
            .as_deref()
            .and_then(|id| id.get(..7))
            .map(|id| format!("parent {id}"))
            .unwrap_or_else(|| "root (empty tree)".into());
        let subject = self.amend_subject.as_deref().unwrap_or("(empty message)");
        bounded_text::field(
            ui,
            &format!(
                "Amending {head} · {subject} · Base {base} · Staged is {head}'s parent -> index; Unstage removes changes from the amended commit"
            ),
            egui::FontId::proportional(12.0),
            GROUP_HEADER,
        );
    }

    fn draw_commit_form(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let top = ui.cursor().top();
        let left = ui.max_rect().left();
        let right = ui.max_rect().right();
        let busy = self.busy() || self.replacing;
        let scanning = self.replacing || !self.complete || self.identity.is_none();
        let reason = commit_reason(
            busy,
            scanning,
            &self.message,
            self.amend,
            self.lists[1].count,
        );
        let can_commit = reason.is_none();
        let message_height = if self.message_expanded { 118.0 } else { 54.0 };
        let mut commit_now = false;
        ui.horizontal(|ui| {
            let button_width = if self.amend { 112.0 } else { 88.0 };
            let width = (ui.available_width() - button_width - 80.0).max(160.0);
            ui.add_sized(
                [width, message_height],
                egui::TextEdit::multiline(&mut self.message)
                    .hint_text("Commit message")
                    .desired_rows(2),
            );
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let mut amend = self.amend;
                    if ui
                        .add_enabled(!busy, egui::Checkbox::new(&mut amend, "Amend"))
                        .on_hover_text(
                            "Show the commit's contents as staged. Unstage removes changes from the amended commit; working files stay unchanged.",
                        )
                        .changed()
                    {
                        if amend {
                            if let Err(error) = self.start_amend(ctx) {
                                self.action_error = Some(error);
                            }
                        } else {
                            if self.amend {
                                self.amend_draft = self.message.clone();
                            }
                            self.amend = false;
                            self.amend_base = None;
                            self.amend_base_commit = None;
                            self.amend_subject = None;
                            self.message = self.normal_draft.clone();
                            self.restart(ctx);
                        }
                    }
                    if ui
                        .small_button(if self.message_expanded {
                            "Collapse"
                        } else {
                            "Expand"
                        })
                        .on_hover_text("Expand the commit message editor")
                        .clicked()
                    {
                        self.message_expanded = !self.message_expanded;
                    }
                });
                let label = if self.commit.is_some() {
                    "Committing…"
                } else if self.amend {
                    "Amend commit"
                } else {
                    "Commit"
                };
                let response = ui.add_enabled(
                    can_commit,
                    egui::Button::new(label).min_size(egui::vec2(button_width, 26.0)),
                );
                let response = if let Some(reason) = reason {
                    response.on_hover_text(reason)
                } else {
                    response
                };
                ui.weak(reason.unwrap_or(if self.amend {
                    "Ready to amend"
                } else {
                    "Ready to commit the index"
                }));
                commit_now |= response.clicked();
            });
        });
        if commit_now {
            self.start_commit(ctx);
        }
        let bottom = ui.cursor().top().max(top + 1.0);
        self.form_rect = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));
    }

    fn draw_right(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, height: f32) {
        let form = self.commit_form_height();
        let reader_height = (height - form - 8.0).max(80.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), reader_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| self.draw_reader(ui, ctx),
        );
        if self.amend {
            self.draw_amend_banner(ui);
        }
        self.draw_commit_form(ui, ctx);
    }

    fn draw_commit_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.add_space(3.0);
        if self.amend {
            self.draw_amend_banner(ui);
        }
        self.draw_commit_form(ui, ctx);
    }

    fn status_text(&self) -> String {
        if let Some(error) = &self.error {
            return format!(
                "{}{}",
                if self.replacing {
                    "Previous results · "
                } else {
                    "Incomplete · "
                },
                error
            );
        }
        if let Some(warning) = &self.commit_warning {
            return format!(
                "{} {} · {warning}",
                if self.last_amended {
                    "Amended"
                } else {
                    "Committed"
                },
                self.last_commit.as_deref().unwrap_or("")
            );
        }
        if let Some(id) = &self.last_commit {
            let remaining = if self.complete && self.total == 0 {
                "Clean working tree".to_string()
            } else {
                format!(
                    "{} unstaged · {} staged",
                    self.lists[0].count, self.lists[1].count
                )
            };
            return format!(
                "{} {id} · {remaining}",
                if self.last_amended {
                    "Amended"
                } else {
                    "Committed"
                }
            );
        }
        if self.replacing {
            return format!(
                "Refreshing · showing previous results · {} unstaged · {} staged",
                self.lists[0].count, self.lists[1].count
            );
        }
        if self.complete && self.total == 0 {
            "Clean working tree".into()
        } else {
            format!(
                "{} unstaged · {} staged{}",
                self.lists[0].count,
                self.lists[1].count,
                if self.complete { "" } else { " · scanning…" }
            )
        }
    }

    pub(crate) fn draw_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        self.poll_commit(ctx);
        let (now, scrolling, pointer_down, hovered) = ctx.input(|i| {
            (
                i.time,
                i.smooth_scroll_delta != egui::Vec2::ZERO,
                i.pointer.any_down(),
                i.pointer.hover_pos().is_some_and(|point| {
                    self.lists[0].rect.contains(point) || self.lists[1].rect.contains(point)
                }),
            )
        });
        if scrolling {
            self.last_scroll = now;
            ctx.request_repaint_after(std::time::Duration::from_millis(210));
        }
        let safe = !hovered && !pointer_down && now - self.last_scroll > 0.2;
        let pinned = ctx.input(|i| i.pointer.hover_pos()).and_then(|point| {
            for (index, list) in self.lists.iter().enumerate() {
                if list.rect.contains(point) {
                    return Some((
                        index,
                        ((point.y - list.rect.top() + list.offset) / HEIGHT)
                            .floor()
                            .max(0.0) as usize,
                    ));
                }
            }
            None
        });
        if self.replacing
            && let Some(Ok(update)) = self.worker.as_ref().and_then(Worker::poll)
        {
            let counts = [update.unstaged_count(), update.staged];
            for (index, rows) in update.rows.into_iter().enumerate() {
                let count = counts[index];
                self.lists[index]
                    .rows
                    .extend(rows.into_iter().filter(|(row, _)| {
                        *row < count && pinned != Some((index, *row)) && !pointer_down
                    }));
            }
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
        self.accept(safe, ctx);
        self.prune_rows(pinned);
        if !safe && !pointer_down {
            self.fill_unhovered(pinned);
        }
        let mut back = false;
        let mut refresh = false;
        if !self.busy() && !self.replacing && crate::navigation::available(ctx) {
            back |= ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                    || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
            });
            refresh |= ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::R));
            let page = (self.lists[self.focus].rect.height() / HEIGHT)
                .floor()
                .max(2.0) as isize
                - 1;
            let step = crate::navigation::step(ctx, page);
            if step != 0 && self.lists[self.focus].count > 0 {
                let current = self.lists[self.focus].selected_row.unwrap_or(0);
                let row = current
                    .saturating_add_signed(step)
                    .min(self.lists[self.focus].count - 1);
                self.select_row(self.focus, row, ctx);
                self.lists[self.focus].reveal = true;
            }
        }
        egui::Panel::top("working-header")
            .exact_size(42.0)
            .show(ui, |ui| {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.strong("Local Changes");
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
                            !self.busy(),
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
        egui::Panel::bottom("working-status")
            .exact_size(25.0)
            .show(ui, |ui| {
                bounded_text::field(
                    ui,
                    &self.status_text(),
                    egui::FontId::proportional(12.0),
                    22.0,
                );
            });
        let wide = ui.available_width() >= WIDE_LAYOUT;
        if !wide {
            egui::Panel::bottom("working-commit")
                .exact_size(self.commit_form_height() + 8.0)
                .show(ui, |ui| self.draw_commit_panel(ui, ctx));
        }
        egui::CentralPanel::default().show(ui, |ui| {
            let height = ui.available_height();
            if wide {
                ui.horizontal_top(|ui| {
                    self.draw_lists_wide(ui, ctx, height);
                    ui.separator();
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| self.draw_right(ui, ctx, height),
                    );
                });
            } else {
                let budget = (height * 0.32).clamp(96.0, 180.0);
                self.draw_lists_narrow(ui, ctx, budget);
                ui.separator();
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.draw_reader(ui, ctx),
                );
            }
        });
        if self.stage.take_changed() {
            self.start(ctx, true);
        }
        let ranges = Ranges {
            unstaged: self.lists[0].viewport.clone(),
            staged: self.lists[1].viewport.clone(),
            selected: self
                .active
                .clone()
                .or_else(|| self.pending_selection.clone()),
        };
        if let Some(worker) = &self.worker {
            worker.request(ranges.clone());
        }
        if let Some(worker) = &self.pending {
            worker.request(ranges);
        }
        bounded_text::show_disclosure(ctx);
        if self.busy() {
            return false;
        }
        if refresh {
            self.start(ctx, true);
        }
        back
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(list: usize, row: usize) -> StatusEntry {
        let area = if list == 0 {
            Area::Unstaged
        } else {
            Area::Staged
        };
        StatusEntry::new(
            area,
            Kind::Modified,
            format!("{list}-{row:06}-long-path-界.rs").as_bytes(),
        )
    }
    fn fixture(unstaged: usize, staged: usize) -> StatusView {
        let rows = |list: usize, count: usize| {
            (0..count.min(256))
                .map(|row| (row, entry(list, row)))
                .collect::<BTreeMap<_, _>>()
        };
        StatusView {
            path: "/repository".into(),
            identity: Some(gg_engine::status::StatusIdentity {
                path: "/repository".into(),
                head: Some("a".repeat(40)),
                index: "index".into(),
                base_tree: None,
            }),
            stage: crate::stage_view::StageView::default(),
            commit: None,
            commit_done: None,
            last_commit: None,
            commit_warning: None,
            message: String::new(),
            normal_draft: String::new(),
            amend_draft: String::new(),
            amend: false,
            amend_base: None,
            amend_base_commit: None,
            amend_subject: None,
            message_expanded: false,
            pending_selection: None,
            last_amended: false,
            worker: None,
            pending: None,
            staged: None,
            lists: [
                List {
                    rows: rows(0, unstaged),
                    count: unstaged,
                    ..List::new()
                },
                List {
                    rows: rows(1, staged),
                    count: staged,
                    ..List::new()
                },
            ],
            focus: 0,
            total: unstaged + staged,
            complete: true,
            error: None,
            action_error: None,
            replacing: false,
            active: None,
            active_entry: None,
            reader_rect: egui::Rect::NOTHING,
            form_rect: egui::Rect::NOTHING,
            last_scroll: f64::NEG_INFINITY,
        }
    }
    fn update(unstaged: usize, staged: usize, complete: bool) -> Update {
        Update {
            identity: gg_engine::status::StatusIdentity {
                path: "/repository".into(),
                head: None,
                index: "absent".into(),
                base_tree: None,
            },
            rows: [
                (0..unstaged.min(256))
                    .map(|row| (row, entry(0, row)))
                    .collect(),
                (0..staged.min(256))
                    .map(|row| (row, entry(1, row)))
                    .collect(),
            ],
            count: unstaged + staged,
            staged,
            complete,
            error: None,
            selection: None,
        }
    }
    fn frame(ctx: &egui::Context, view: &mut StatusView, width: f32, events: Vec<egui::Event>) {
        let _ = crate::tests::run_ui(
            ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 480.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                view.draw_ui(ctx);
            },
        );
    }
    #[test]
    fn hovered_rows_and_refresh_results_wait_for_safe_adoption() {
        let ctx = egui::Context::default();
        let mut view = fixture(10, 0);
        frame(&ctx, &mut view, 720.0, vec![]);
        view.staged = Some(update(1000, 0, true));
        let point = view.lists[0].rect.center();
        frame(
            &ctx,
            &mut view,
            720.0,
            vec![egui::Event::PointerMoved(point)],
        );
        assert_eq!(view.lists[0].count, 10);
        frame(&ctx, &mut view, 720.0, vec![egui::Event::PointerGone]);
        assert_eq!(view.lists[0].count, 1000);
        view.replacing = true;
        view.lists[0].selected = Some((entry(0, 8).path_bytes.clone(), entry(0, 8).area));
        view.lists[0].selected_row = Some(8);
        view.lists[0].offset = 60.0;
        view.staged = Some(update(2, 0, false));
        view.accept(true, &ctx);
        assert_eq!((view.lists[0].count, view.lists[0].offset), (1000, 60.0));
        let mut failure = update(2, 0, false);
        failure.error = Some("read failed".into());
        view.staged = Some(failure);
        view.accept(true, &ctx);
        assert_eq!(view.lists[0].count, 1000);
        assert!(view.error.is_some());
        view.staged = Some(update(0, 0, true));
        view.accept(false, &ctx);
        assert_eq!(view.lists[0].count, 1000);
        view.accept(true, &ctx);
        assert_eq!(view.total, 0);
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
                if (!reading && view.complete && !view.replacing && view.stage.pane_ready(1))
                    || (reading && view.stage.ready() && view.stage.has_text(text))
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
        assert!(matches!(
            view.active.as_ref().map(|(_, area)| *area),
            Some(Area::Unstaged | Area::Untracked)
        ));
        wait(&mut view, true, "before")?;
        std::fs::write(dir.path().join("a"), b"after\n")?;
        view.start(&ctx, true);
        wait(&mut view, false, "")?;
        assert_eq!(view.identity, identity);
        wait(&mut view, true, "after")?;
        assert!(!view.stage.has_text("before"));
        Ok(())
    }
    #[test]
    fn workspace_preserves_independent_list_selection_and_scroll() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut view = fixture(1000, 1000);
            view.lists[0].selected = Some((entry(0, 20).path_bytes.clone(), Area::Unstaged));
            view.lists[0].selected_row = Some(20);
            view.lists[0].offset = 200.0;
            view.lists[0].restore = true;
            view.lists[1].selected = Some((entry(1, 10).path_bytes.clone(), Area::Staged));
            view.lists[1].selected_row = Some(10);
            view.lists[1].offset = 80.0;
            view.lists[1].restore = true;
            frame(&ctx, &mut view, width, vec![]);
            let first = view.lists[0].offset;
            let second = view.lists[1].offset;
            assert!((first - 200.0).abs() < 1.0);
            assert!((second - 80.0).abs() < 1.0);
            frame(
                &ctx,
                &mut view,
                width,
                vec![egui::Event::Key {
                    key: egui::Key::ArrowDown,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert_eq!(view.lists[0].selected_row, Some(21));
            assert_eq!(view.lists[1].selected_row, Some(10));
            assert!((view.lists[1].offset - second).abs() < 1.0);
        }
    }
    #[test]
    fn loading_rows_fill_around_the_hovered_slot_without_changing_extent() {
        let mut view = fixture(1000, 0);
        view.lists[0].rows.clear();
        view.lists[0].viewport = 500..532;
        let mut incoming = update(2000, 0, false);
        incoming.rows[0] = (500..532).map(|row| (row, entry(0, row))).collect();
        view.staged = Some(incoming);
        view.fill_unhovered(Some((0, 510)));
        assert_eq!(view.lists[0].count, 1000);
        assert!(!view.lists[0].rows.contains_key(&510));
        assert!(view.lists[0].rows.contains_key(&509) && view.lists[0].rows.contains_key(&511));
        view.accept(true, &egui::Context::default());
        assert_eq!(view.lists[0].count, 2000);
        assert!(view.lists[0].rows.contains_key(&510));
        assert!(view.lists[0].rows.len() + view.lists[1].rows.len() <= 768);
    }
    #[test]
    fn keyboard_pages_keep_last_page_stable_at_both_widths() {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut view = fixture(1000, 0);
            view.lists[0].rows.insert(998, entry(0, 998));
            view.lists[0].rows.insert(999, entry(0, 999));
            frame(&ctx, &mut view, width, vec![]);
            assert!(view.lists[0].rect.top() >= 42.0);
            view.lists[0].selected_row = Some(998);
            view.lists[0].reveal = true;
            frame(&ctx, &mut view, width, vec![]);
            let event = egui::Event::Key {
                key: egui::Key::ArrowDown,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            frame(&ctx, &mut view, width, vec![event]);
            assert_eq!(view.lists[0].selected_row, Some(999));
            let offset = view.lists[0].offset;
            for _ in 0..3 {
                frame(&ctx, &mut view, width, vec![]);
                assert!((view.lists[0].offset - offset).abs() < 1.0);
            }
            let event = egui::Event::Key {
                key: egui::Key::PageUp,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            frame(&ctx, &mut view, width, vec![event]);
            assert!(view.lists[0].selected_row.is_some_and(|row| row < 999));
        }
    }
    #[test]
    fn wide_and_narrow_geometry_place_both_groups_above_or_beside_the_reader() {
        for (width, height) in [(1120.0, 740.0), (720.0, 480.0)] {
            let ctx = egui::Context::default();
            let mut view = fixture(2, 2);
            let _ = crate::tests::run_ui(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, height),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    view.draw_ui(ctx);
                },
            );
            assert_ne!(view.lists[0].rect, egui::Rect::NOTHING);
            assert_ne!(view.lists[1].rect, egui::Rect::NOTHING);
            assert!(view.lists[0].rect.top() < view.lists[1].rect.top());
            assert!(view.reader_rect.top() < view.form_rect.top());
            assert!(view.form_rect.bottom() <= height - 20.0);
            if width >= WIDE_LAYOUT {
                assert!(view.lists[0].rect.right() <= view.reader_rect.left());
                assert!(view.lists[1].rect.right() <= view.reader_rect.left());
                assert!(view.lists[0].rect.top() < view.reader_rect.bottom());
                assert!(view.lists[0].rect.bottom() > view.reader_rect.top());
            } else {
                assert!(view.lists[0].rect.bottom() <= view.lists[1].rect.top() + 30.0);
                assert!(view.lists[1].rect.bottom() <= view.reader_rect.top());
                assert!(view.reader_rect.top() < view.form_rect.top());
            }
        }
    }
    #[test]
    fn partially_staged_path_is_in_both_groups_and_selects_distinct_comparisons()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let p = dir.path();
        git(p, &["init", "-q"])?;
        std::fs::write(p.join("a"), b"base\n")?;
        git(p, &["add", "a"])?;
        git(p, &["commit", "-qm", "base"])?;
        std::fs::write(p.join("a"), b"staged\n")?;
        git(p, &["add", "a"])?;
        std::fs::write(p.join("a"), b"working\n")?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(p.into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        let staged = view.lists[1]
            .rows
            .iter()
            .find(|(_, entry)| entry.path == "a")
            .map(|(row, _)| *row)
            .ok_or("missing staged entry")?;
        let unstaged = view.lists[0]
            .rows
            .iter()
            .find(|(_, entry)| entry.path == "a")
            .map(|(row, _)| *row)
            .ok_or("missing unstaged entry")?;
        assert_eq!(
            view.lists[0].rows.get(&0).map(|entry| entry.path.as_str()),
            Some("a"),
            "working rows use list coordinates"
        );
        view.select_row(1, staged, &ctx);
        ready(&ctx, &mut view, 1120.0);
        assert_eq!(
            view.active.as_ref().map(|(_, area)| *area),
            Some(Area::Staged)
        );
        assert_eq!(view.stage.active_pane(), 0);
        assert!(view.stage.pane_has_text(0, "staged"));
        view.select_row(0, unstaged, &ctx);
        ready(&ctx, &mut view, 1120.0);
        assert_eq!(
            view.active.as_ref().map(|(_, area)| *area),
            Some(Area::Unstaged)
        );
        assert_eq!(view.stage.active_pane(), 1);
        assert!(view.stage.pane_has_text(1, "working"));
        Ok(())
    }
    #[test]
    fn workspace_renders_both_headers_and_active_reader_actions()
    -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let mut view = fixture(1, 1);
        view.identity = None;
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.iter().any(|line| line.starts_with("Unstaged (")));
        assert!(text.iter().any(|line| line.starts_with("Staged (")));
        assert!(!text.contains(&"View diff"));
        assert!(!text.contains(&"Split"));
        assert!(!text.contains(&"Review & stage"));
        assert_eq!(view.active, None);
        Ok(())
    }
    fn run(
        ctx: &egui::Context,
        view: &mut StatusView,
        width: f32,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        crate::tests::run_ui(
            ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 480.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                view.draw_ui(ctx);
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
        run(ctx, view, width, vec![release, egui::Event::PointerGone]);
    }
    fn git(path: &std::path::Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.org")
            .env("GIT_COMMITTER_EMAIL", "test@example.org")
            .output()?;
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        Ok(())
    }
    fn file(path: &std::path::Path, args: &[&str]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()?;
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        Ok(out.stdout)
    }
    fn pump(
        ctx: &egui::Context,
        view: &mut StatusView,
        width: f32,
        mut done: impl FnMut(&StatusView) -> bool,
    ) {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            frame(ctx, view, width, vec![]);
            if done(view) {
                return;
            }
            assert!(std::time::Instant::now() < until, "working view timed out");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    fn settle(ctx: &egui::Context, view: &mut StatusView, width: f32) {
        pump(ctx, view, width, |view| {
            view.complete && !view.replacing && !view.busy()
        });
    }
    fn ready(ctx: &egui::Context, view: &mut StatusView, width: f32) {
        pump(ctx, view, width, |view| {
            view.stage.ready() && view.stage.pane_ready(0) && view.stage.pane_ready(1)
        });
    }
    fn generated(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        git(path, &["init", "-q"])?;
        git(path, &["config", "core.autocrlf", "false"])?;
        std::fs::write(
            path.join("a"),
            b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n",
        )?;
        git(path, &["add", "a"])?;
        git(path, &["commit", "-qm", "base"])?;
        std::fs::write(
            path.join("a"),
            b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n",
        )?;
        Ok(())
    }
    fn select_path(view: &mut StatusView, ctx: &egui::Context, path: &str, area: Area) {
        let index = if area == Area::Staged { 1 } else { 0 };
        if let Some(row) = view.lists[index]
            .rows
            .iter()
            .find(|(_, entry)| entry.path == path && entry.area == area)
            .map(|(row, _)| *row)
        {
            view.select_row(index, row, ctx);
        }
    }
    fn select_area(view: &mut StatusView, ctx: &egui::Context, area: Area) {
        let index = if area == Area::Staged { 1 } else { 0 };
        if view.lists[index].rows.contains_key(&0) {
            view.select_row(index, 0, ctx);
        }
    }

    #[test]
    fn inline_hunk_button_moves_the_hunk_and_keeps_the_workspace()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let working = std::fs::read(dir.path().join("a"))?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_area(&mut view, &ctx, Area::Unstaged);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let button = text_pos(&output, "Stage hunk").ok_or("missing stage hunk")?;
        click(&ctx, &mut view, 1120.0, button + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.pending() && view.stage.ready() && view.stage.pane_has_text(0, "ONE")
        });
        assert_eq!(
            view.active.as_ref().map(|(_, area)| *area),
            Some(Area::Unstaged)
        );
        assert_eq!(
            file(dir.path(), &["show", ":a"])?,
            b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n"
        );
        assert!(view.stage.pane_has_text(1, "TWELVE"));
        assert!(!view.stage.pane_has_text(1, "ONE"));
        assert_eq!(std::fs::read(dir.path().join("a"))?, working);
        Ok(())
    }

    #[test]
    fn inline_selected_line_stages_only_that_line_and_keeps_the_workspace()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let working = std::fs::read(dir.path().join("a"))?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_area(&mut view, &ctx, Area::Unstaged);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let line = text_pos(&output, "TWELVE").ok_or("missing added line")?;
        click(&ctx, &mut view, 1120.0, line + egui::vec2(5.0, 0.0));
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let button = text_pos(&output, "Stage 1").ok_or("missing selected-line action")?;
        click(&ctx, &mut view, 1120.0, button + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.pending() && view.stage.ready() && view.stage.pane_has_text(0, "TWELVE")
        });
        assert_eq!(
            file(dir.path(), &["show", ":a"])?,
            b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\nTWELVE\n"
        );
        assert!(view.stage.pane_has_text(1, "twelve"));
        assert_eq!(std::fs::read(dir.path().join("a"))?, working);
        Ok(())
    }

    #[test]
    fn inline_unstage_restores_head_and_keeps_the_workspace()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        git(dir.path(), &["add", "a"])?;
        std::fs::write(
            dir.path().join("a"),
            b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE!\n",
        )?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_area(&mut view, &ctx, Area::Staged);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let button = text_pos(&output, "Unstage hunk").ok_or("missing unstage hunk")?;
        click(&ctx, &mut view, 1120.0, button + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.pending() && view.stage.ready()
        });
        assert_eq!(
            file(dir.path(), &["show", ":a"])?,
            b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n"
        );
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let button = text_pos(&output, "Unstage file").ok_or("missing unstage all")?;
        click(&ctx, &mut view, 1120.0, button + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.pending() && view.stage.ready() && !view.stage.pane_has_text(0, "TWELVE")
        });
        assert_eq!(
            file(dir.path(), &["show", ":a"])?,
            b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n"
        );
        Ok(())
    }

    #[test]
    fn whole_file_actions_preserve_unrelated_index_and_working_data()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let p = dir.path();
        git(p, &["init", "-q"])?;
        std::fs::write(p.join("a"), b"a-base\n")?;
        std::fs::write(p.join("b"), b"b-base\n")?;
        git(p, &["add", "."])?;
        git(p, &["commit", "-qm", "base"])?;
        std::fs::write(p.join("b"), b"b-staged\n")?;
        git(p, &["add", "b"])?;
        std::fs::write(p.join("a"), b"a-working\n")?;
        std::fs::write(p.join("b"), b"b-working\n")?;
        let staged_b = file(p, &["show", ":b"])?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(p.into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_path(&mut view, &ctx, "a", Area::Unstaged);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let stage = text_pos(&output, "Stage file").ok_or("missing stage file")?;
        click(&ctx, &mut view, 1120.0, stage + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.pending() && view.complete && !view.replacing
        });
        assert_eq!(file(p, &["show", ":a"])?, b"a-working\n");
        assert_eq!(file(p, &["show", ":b"])?, staged_b);
        assert_eq!(std::fs::read(p.join("b"))?, b"b-working\n");

        select_path(&mut view, &ctx, "a", Area::Staged);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let unstage = text_pos(&output, "Unstage file").ok_or("missing unstage file")?;
        click(&ctx, &mut view, 1120.0, unstage + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.pending() && view.complete && !view.replacing
        });
        assert_eq!(file(p, &["show", ":a"])?, b"a-base\n");
        assert_eq!(file(p, &["show", ":b"])?, staged_b);
        assert_eq!(std::fs::read(p.join("a"))?, b"a-working\n");
        Ok(())
    }

    #[test]
    fn commit_panel_is_visible_at_both_widths() -> Result<(), Box<dyn std::error::Error>> {
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let mut view = fixture(2, 0);
            view.message = "A commit".into();
            let output = run(&ctx, &mut view, width, vec![]);
            assert!(
                text_pos(&output, "Commit").is_some(),
                "missing commit at {width}"
            );
        }
        Ok(())
    }

    #[test]
    fn commit_readiness_explains_disabled_states() {
        assert_eq!(
            commit_reason(false, false, "subject", false, 0),
            Some("Stage at least one file")
        );
        assert_eq!(
            commit_reason(false, false, "   ", false, 1),
            Some("Enter a commit message")
        );
        assert_eq!(
            commit_reason(false, true, "subject", true, 1),
            Some("Waiting for the status scan")
        );
        assert_eq!(
            commit_reason(true, false, "subject", false, 1),
            Some("Wait for the current operation")
        );
        assert_eq!(commit_reason(false, false, "subject", true, 0), None);
        assert_eq!(commit_reason(false, false, "subject", false, 1), None);
    }

    #[test]
    fn commit_editor_expands_and_collapses() {
        let ctx = egui::Context::default();
        let mut view = fixture(2, 0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let expand = text_pos(&output, "Expand");
        assert!(expand.is_some(), "missing expand button");
        click(
            &ctx,
            &mut view,
            1120.0,
            expand.unwrap_or_default() + egui::vec2(5.0, 0.0),
        );
        assert!(view.message_expanded);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let collapse = text_pos(&output, "Collapse");
        assert!(collapse.is_some(), "missing collapse button");
        click(
            &ctx,
            &mut view,
            1120.0,
            collapse.unwrap_or_default() + egui::vec2(5.0, 0.0),
        );
        assert!(!view.message_expanded);
    }

    #[test]
    fn amend_panel_loads_head_names_the_base_and_rewrites_the_commit()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        git(dir.path(), &["add", "a"])?;
        let previous = file(dir.path(), &["rev-parse", "HEAD"])?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        view.start_amend(&ctx)?;
        settle(&ctx, &mut view, 1120.0);
        assert_eq!(view.message, "base");
        assert!(view.amend);
        assert!(view.amend_base_commit.is_none(), "root amend has no parent");
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.job.text.contains("Amending")
                    && text.galley.job.text.contains("base")
                    && text.galley.job.text.contains("root (empty tree)")
        )));
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.job.text.contains("HEAD parent -> index")
        )));
        let amend = text_pos(&output, "Amend commit").ok_or("missing amend button")?;
        click(&ctx, &mut view, 1120.0, amend + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing && view.commit_done.is_some()
        });
        assert!(view.take_commit().is_some());
        assert!(!view.amend);
        assert_eq!(
            file(dir.path(), &["show", "-s", "--format=%s", "HEAD"])?,
            b"base\n"
        );
        assert_eq!(
            file(dir.path(), &["show", "HEAD:a"])?,
            b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n"
        );
        assert_eq!(file(dir.path(), &["rev-list", "--count", "HEAD"])?, b"1\n");
        assert_ne!(file(dir.path(), &["rev-parse", "HEAD"])?, previous);
        assert!(
            std::fs::read_dir(dir.path().join(".git/refs/gg/backup/amend"))?
                .next()
                .is_some()
        );
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        assert!(
            output.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::epaint::Shape::Text(text)
                    if text.galley.job.text.starts_with("Amended ")
            )),
            "missing amended status line"
        );
        Ok(())
    }

    #[test]
    fn normal_and_amend_drafts_stay_separate() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        view.message = "normal draft".into();
        view.start_amend(&ctx)?;
        assert!(view.amend);
        assert_eq!(view.normal_draft, "normal draft");
        assert_eq!(view.message, "base");
        view.message = "amend draft".into();

        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let checkbox = text_pos(&output, "Amend").ok_or("missing amend checkbox")?;
        click(&ctx, &mut view, 1120.0, checkbox + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing && !view.amend
        });
        assert_eq!(view.message, "normal draft");
        assert_eq!(view.amend_draft, "amend draft");

        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let checkbox = text_pos(&output, "Amend").ok_or("missing amend checkbox")?;
        click(&ctx, &mut view, 1120.0, checkbox + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing && view.amend
        });
        assert_eq!(view.message, "amend draft");
        Ok(())
    }

    #[test]
    fn draft_restores_the_selected_comparison() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let ctx = egui::Context::default();
        let mut first = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut first, 1120.0);
        select_path(&mut first, &ctx, "a", Area::Unstaged);
        first.set_draft("kept".into());
        let draft = first.capture_draft();
        let mut second = StatusView::new(dir.path().into(), &ctx);
        second.restore_draft(draft, &ctx);
        settle(&ctx, &mut second, 1120.0);
        assert_eq!(second.draft(), "kept");
        assert_eq!(
            second
                .active
                .as_ref()
                .map(|(bytes, area)| (String::from_utf8_lossy(bytes).into_owned(), *area)),
            Some(("a".into(), Area::Unstaged))
        );
        second.set_draft("still kept".into());
        second.start(&ctx, true);
        settle(&ctx, &mut second, 1120.0);
        assert_eq!(second.draft(), "still kept");
        assert_eq!(
            second
                .active
                .as_ref()
                .map(|(bytes, area)| (String::from_utf8_lossy(bytes).into_owned(), *area)),
            Some(("a".into(), Area::Unstaged))
        );
        Ok(())
    }

    #[test]
    fn amend_view_shows_commit_contents_and_restages_through_unstage_and_stage()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let p = dir.path();
        git(p, &["init", "-q"])?;
        std::fs::write(p.join("a"), b"base\n")?;
        git(p, &["add", "a"])?;
        git(p, &["commit", "-qm", "base"])?;
        std::fs::write(p.join("a"), b"commit\n")?;
        git(p, &["commit", "-qam", "second"])?;
        std::fs::write(p.join("a"), b"working\n")?;

        let ctx = egui::Context::default();
        let mut view = StatusView::new(p.into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        view.start_amend(&ctx)?;
        settle(&ctx, &mut view, 1120.0);
        assert_eq!(view.message, "second");
        let parent = String::from_utf8(file(p, &["rev-parse", "HEAD^"])?)?
            .trim()
            .to_string();
        assert_eq!(view.amend_base_commit.as_deref(), Some(parent.as_str()));
        select_path(&mut view, &ctx, "a", Area::Staged);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.job.text.contains("Amending")
                    && text.galley.job.text.contains(&format!("parent {}", &parent[..7]))
        )));
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text) if text.galley.job.text.contains("HEAD parent -> index")
        )));
        assert!(
            view.lists[1].rows.values().any(|entry| entry.path == "a"),
            "the amended commit's contents are missing"
        );
        assert!(view.lists[0].rows.values().any(|entry| entry.path == "a"));

        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let checkbox = text_pos(&output, "Amend").ok_or("missing amend checkbox")?;
        click(&ctx, &mut view, 1120.0, checkbox + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing
        });
        assert!(!view.amend);
        assert!(view.amend_base.is_none());
        assert!(view.lists[1].rows.values().all(|entry| entry.path != "a"));
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let checkbox = text_pos(&output, "Amend").ok_or("missing amend checkbox")?;
        click(&ctx, &mut view, 1120.0, checkbox + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing
        });
        assert!(view.amend);
        assert!(view.amend_base.is_some());
        assert!(view.lists[1].rows.values().any(|entry| entry.path == "a"));

        select_path(&mut view, &ctx, "a", Area::Staged);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let unstage = text_pos(&output, "Unstage file").ok_or("missing unstage file")?;
        click(&ctx, &mut view, 1120.0, unstage + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing
        });
        assert_eq!(file(p, &["show", ":a"])?, b"base\n");
        assert!(view.lists[1].rows.values().all(|entry| entry.path != "a"));
        assert!(view.lists[0].rows.values().any(|entry| entry.path == "a"));

        select_path(&mut view, &ctx, "a", Area::Unstaged);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let stage = text_pos(&output, "Stage file").ok_or("missing stage file")?;
        click(&ctx, &mut view, 1120.0, stage + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing
        });
        assert_eq!(file(p, &["show", ":a"])?, b"working\n");

        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let amend = text_pos(&output, "Amend commit").ok_or("missing amend button")?;
        click(&ctx, &mut view, 1120.0, amend + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing && view.commit_done.is_some()
        });
        assert!(view.take_commit().is_some());
        assert!(!view.amend);
        assert!(view.amend_base.is_none());
        assert_eq!(file(p, &["show", "HEAD:a"])?, b"working\n");
        assert_eq!(file(p, &["rev-list", "--count", "HEAD"])?, b"2\n");
        Ok(())
    }

    #[test]
    fn commit_panel_commits_the_index_and_reports_hook_failures()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        git(dir.path(), &["add", "a"])?;
        let index = std::fs::read(dir.path().join(".git/index"))?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        view.message = "UI commit".into();
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let commit = text_pos(&output, "Commit").ok_or("missing commit button")?;
        click(&ctx, &mut view, 1120.0, commit + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing && view.commit_done.is_some()
        });
        assert!(view.take_commit().is_some());
        assert_eq!(
            file(dir.path(), &["show", "-s", "--format=%s", "HEAD"])?,
            b"UI commit\n"
        );
        assert!(scan_is_clean(dir.path())?);
        assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);

        std::fs::write(dir.path().join("a"), b"next\n")?;
        git(dir.path(), &["add", "a"])?;
        view.start(&ctx, true);
        settle(&ctx, &mut view, 1120.0);
        let blocked = std::fs::read(dir.path().join(".git/index"))?;
        let road = dir.path().join(".git/hooks/pre-commit");
        std::fs::create_dir_all(road.parent().ok_or("missing hooks dir")?)?;
        std::fs::write(&road, "#!/bin/sh\necho blocked >&2\nexit 4\n")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&road, std::fs::Permissions::from_mode(0o755))?;
        }
        view.message = "should fail".into();
        view.start_commit(&ctx);
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing && view.error.is_some()
        });
        assert!(
            view.error
                .as_ref()
                .is_some_and(|e| e.contains("pre-commit"))
        );
        assert!(view.error.as_ref().is_some_and(|e| e.contains("preserved")));
        assert_eq!(view.message, "should fail");
        assert_eq!(std::fs::read(dir.path().join(".git/index"))?, blocked);
        assert_eq!(file(dir.path(), &["rev-list", "--count", "HEAD"])?, b"2\n");
        Ok(())
    }

    #[test]
    fn conflict_row_reads_an_unmerged_summary_and_route() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        let p = dir.path();
        git(p, &["init", "-q", "-b", "main"])?;
        git(p, &["config", "core.autocrlf", "false"])?;
        std::fs::write(p.join("a"), b"base\n")?;
        git(p, &["add", "a"])?;
        git(p, &["commit", "-qm", "base"])?;
        git(p, &["checkout", "-qb", "side"])?;
        std::fs::write(p.join("a"), b"side\n")?;
        git(p, &["add", "a"])?;
        git(p, &["commit", "-qm", "side"])?;
        git(p, &["checkout", "-q", "main"])?;
        std::fs::write(p.join("a"), b"main\n")?;
        git(p, &["add", "a"])?;
        git(p, &["commit", "-qm", "main"])?;
        let merge = std::process::Command::new("git")
            .arg("-C")
            .arg(p)
            .args(["merge", "side"])
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.org")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.org")
            .output()?;
        assert!(!merge.status.success());

        let ctx = egui::Context::default();
        let mut view = StatusView::new(p.into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_path(&mut view, &ctx, "a", Area::Conflict);
        assert!(
            view.lists[0]
                .rows
                .values()
                .any(|entry| entry.area == Area::Conflict),
            "conflict row is missing"
        );
        pump(&ctx, &mut view, 1120.0, |view| view.stage.ready());
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.iter().any(|line| line.contains("unmerged path")));
        assert!(text.iter().any(|line| line.contains("Unmerged path:")));
        assert!(!text.iter().any(|line| line == &"No unstaged changes"));
        Ok(())
    }

    #[test]
    fn staged_away_untracked_entry_keeps_an_empty_comparison()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let p = dir.path();
        git(p, &["init", "-q"])?;
        std::fs::write(p.join("new"), b"untracked\n")?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(p.into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_path(&mut view, &ctx, "new", Area::Untracked);
        ready(&ctx, &mut view, 1120.0);
        let identity = view.identity.clone().ok_or("missing identity")?;
        let entry = view.lists[0]
            .rows
            .values()
            .find(|entry| entry.path == "new")
            .cloned()
            .ok_or("missing untracked entry")?;
        gg_git::stage::file(&gg_engine::status::WorkingRequest {
            repository: identity,
            entry,
        })?;
        view.refresh(&ctx);
        settle(&ctx, &mut view, 1120.0);
        assert_eq!(
            view.active.as_ref().map(|(_, area)| *area),
            Some(Area::Untracked)
        );
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.iter().any(|line| line.contains("no longer present")));
        assert!(!text.iter().any(|line| line.contains("Stage file")));
        Ok(())
    }

    #[test]
    fn refresh_and_draft_reselection_find_off_page_entries()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let p = dir.path();
        git(p, &["init", "-q"])?;
        for index in 0..300 {
            std::fs::write(p.join(format!("file-{index:04}")), b"content\n")?;
        }
        let ctx = egui::Context::default();
        let mut view = StatusView::new(p.into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        let session = gg_git::status::Session::open(p)?;
        let mut scan = Vec::new();
        session.scan(&std::sync::atomic::AtomicBool::new(false), |entry| {
            scan.push((entry.path_bytes.clone(), entry.area));
            Ok(())
        })?;
        let (bytes, area) = scan.get(250).cloned().ok_or("missing row 250")?;
        let entry =
            gg_engine::status::StatusEntry::new(area, gg_engine::status::Kind::Modified, &bytes);
        let selected = (bytes.clone(), area);
        assert!(!view.lists[0].rows.contains_key(&250));
        view.lists[0].selected_row = Some(250);
        view.lists[0].selected = Some(selected.clone());
        view.active = Some(selected.clone());
        view.active_entry = Some(entry.clone());
        view.focus = 0;
        view.refresh(&ctx);
        settle(&ctx, &mut view, 1120.0);
        let row = view
            .find_entry(&selected)
            .and_then(|_| {
                view.lists[0]
                    .rows
                    .iter()
                    .find(|(_, entry)| entry.path_bytes == selected.0 && entry.area == selected.1)
                    .map(|(row, _)| *row)
            })
            .ok_or("selected entry did not load")?;
        assert_eq!(view.lists[0].selected_row, Some(row));
        assert_eq!(view.active, Some(selected.clone()));

        let draft = view.capture_draft();
        let mut restored = StatusView::new(p.into(), &ctx);
        restored.restore_draft(draft, &ctx);
        settle(&ctx, &mut restored, 1120.0);
        let row = restored.lists[0]
            .rows
            .iter()
            .find(|(_, entry)| entry.path_bytes == selected.0 && entry.area == selected.1)
            .map(|(row, _)| *row)
            .ok_or("draft selection did not load")?;
        assert_eq!(restored.lists[0].selected_row, Some(row));
        assert_eq!(restored.active, Some(selected));
        Ok(())
    }

    fn scan_is_clean(path: &std::path::Path) -> Result<bool, Box<dyn std::error::Error>> {
        let session = gg_git::status::Session::open(path)?;
        let mut clean = true;
        session.scan(&std::sync::atomic::AtomicBool::new(false), |_| {
            clean = false;
            Ok(())
        })?;
        Ok(clean)
    }

    #[test]
    fn selected_entry_disappearing_retains_path_with_empty_comparison()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_path(&mut view, &ctx, "a", Area::Unstaged);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let stage = text_pos(&output, "Stage file").ok_or("missing stage file")?;
        click(&ctx, &mut view, 1120.0, stage + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.pending() && view.complete && !view.replacing && view.lists[0].count == 0
        });
        assert_eq!(view.lists[0].count, 0);
        assert_eq!(
            view.active
                .as_ref()
                .map(|(bytes, area)| (String::from_utf8_lossy(bytes).into_owned(), *area)),
            Some(("a".into(), Area::Unstaged))
        );
        assert_eq!(view.stage.path(), "a");
        assert_eq!(view.stage.active_pane(), 1);
        assert_eq!(view.stage.pane_rows(1), 0, "stale working diff");
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.job.text.contains("No unstaged changes")
        )));
        select_path(&mut view, &ctx, "a", Area::Staged);
        ready(&ctx, &mut view, 1120.0);
        assert_eq!(
            view.active.as_ref().map(|(_, area)| *area),
            Some(Area::Staged)
        );
        Ok(())
    }

    #[test]
    fn selected_line_drop_reverts_the_whole_change_block() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let index = std::fs::read(dir.path().join(".git/index"))?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_area(&mut view, &ctx, Area::Unstaged);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let line = text_pos(&output, "TWELVE").ok_or("missing selected line")?;
        click(&ctx, &mut view, 1120.0, line + egui::vec2(5.0, 0.0));
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let drop = text_pos(&output, "Discard selected").ok_or("missing selected drop")?;
        click(&ctx, &mut view, 1120.0, drop + egui::vec2(5.0, 0.0));
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let discard = text_pos(&output, "Discard").ok_or("missing discard")?;
        click(&ctx, &mut view, 1120.0, discard + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.pending() && view.stage.ready()
        });
        assert_eq!(
            std::fs::read(dir.path().join("a"))?,
            b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n"
        );
        assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);
        Ok(())
    }

    #[test]
    fn drop_requires_confirmation_and_leaves_the_index_alone()
    -> Result<(), Box<dyn std::error::Error>> {
        for width in [720.0, 1120.0] {
            let dir = tempfile::tempdir()?;
            generated(dir.path())?;
            let index = std::fs::read(dir.path().join(".git/index"))?;
            let working = std::fs::read(dir.path().join("a"))?;
            let ctx = egui::Context::default();
            let mut view = StatusView::new(dir.path().into(), &ctx);
            settle(&ctx, &mut view, width);
            select_area(&mut view, &ctx, Area::Unstaged);
            ready(&ctx, &mut view, width);

            let output = run(&ctx, &mut view, width, vec![]);
            let drop = text_pos(&output, "Discard hunk").ok_or("missing drop hunk")?;
            click(&ctx, &mut view, width, drop + egui::vec2(5.0, 0.0));
            let output = run(&ctx, &mut view, width, vec![]);
            let cancel = text_pos(&output, "Cancel").ok_or("missing drop cancel")?;
            click(&ctx, &mut view, width, cancel + egui::vec2(5.0, 0.0));
            assert_eq!(std::fs::read(dir.path().join("a"))?, working);
            assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);

            let output = run(&ctx, &mut view, width, vec![]);
            let drop = text_pos(&output, "Discard hunk").ok_or("missing drop hunk")?;
            click(&ctx, &mut view, width, drop + egui::vec2(5.0, 0.0));
            let output = run(&ctx, &mut view, width, vec![]);
            let discard = text_pos(&output, "Discard").ok_or("missing discard")?;
            click(&ctx, &mut view, width, discard + egui::vec2(5.0, 0.0));
            pump(&ctx, &mut view, width, |view| {
                !view.stage.pending()
                    && view.stage.ready()
                    && view.stage.pane_has_text(1, "TWELVE")
                    && !view.stage.pane_has_text(1, "ONE")
            });
            assert_eq!(
                std::fs::read(dir.path().join("a"))?,
                b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n"
            );
            assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);

            let output = run(&ctx, &mut view, width, vec![]);
            let drop = text_pos(&output, "Discard file").ok_or("missing drop all")?;
            click(&ctx, &mut view, width, drop + egui::vec2(5.0, 0.0));
            let output = run(&ctx, &mut view, width, vec![]);
            let discard = text_pos(&output, "Discard").ok_or("missing discard all")?;
            click(&ctx, &mut view, width, discard + egui::vec2(5.0, 0.0));
            pump(&ctx, &mut view, width, |view| {
                !view.stage.pending() && view.stage.ready() && view.lists[0].count == 0
            });
            assert_eq!(
                std::fs::read(dir.path().join("a"))?,
                b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n"
            );
            assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);
            assert_eq!(
                view.active.as_ref().map(|(_, area)| *area),
                Some(Area::Unstaged),
                "the reviewed path stays selected with an empty comparison"
            );
        }
        Ok(())
    }

    #[test]
    fn inline_action_failure_keeps_the_workspace_and_index()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_area(&mut view, &ctx, Area::Unstaged);
        ready(&ctx, &mut view, 1120.0);
        let index = std::fs::read(dir.path().join(".git/index"))?;
        std::fs::write(dir.path().join(".git/index.lock"), "owner")?;
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let button = text_pos(&output, "Stage hunk").ok_or("missing stage hunk")?;
        click(&ctx, &mut view, 1120.0, button + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| !view.stage.pending());
        assert!(
            view.stage
                .error()
                .is_some_and(|error| error.contains("Stage"))
        );
        assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);
        std::fs::remove_file(dir.path().join(".git/index.lock"))?;
        Ok(())
    }
}
