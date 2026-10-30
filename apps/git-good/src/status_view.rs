//! Local Changes workspace: independent Unstaged/Staged lists beside one reader.
use crate::{
    bounded_text,
    diff_view::{StageEdit, StageScope},
    status_worker::{Ranges, Update, Worker},
};
use eframe::egui;
use gg_engine::status::{Area, Kind, StatusEntry};
use std::{collections::BTreeMap, ops::Range, path::PathBuf};
const HEIGHT: f32 = 22.0;
const GROUP_HEADER: f32 = 24.0;
const WIDE_LAYOUT: f32 = 900.0;
type Comparison = (Vec<u8>, Area);

#[derive(Clone)]
enum StageFollow {
    /// After staging a whole file, select the next/previous unstaged entry.
    Advance {
        from: Vec<u8>,
        target: Option<Vec<u8>>,
    },
    /// After unstaging a whole file, follow that path into Unstaged.
    Move { path: Vec<u8> },
}
fn commit_reason<'a>(
    busy: bool,
    scanning: bool,
    message: &str,
    amend: bool,
    staged: usize,
    signing_error: Option<&'a str>,
) -> Option<&'a str> {
    if busy {
        Some("Wait for the current operation")
    } else if scanning {
        Some("Waiting for the status scan")
    } else if message.trim().is_empty() {
        Some("Enter a commit message")
    } else if !amend && staged == 0 {
        Some("Stage at least one file")
    } else if let Some(error) = signing_error {
        Some(error)
    } else {
        None
    }
}
fn group_name(list: usize) -> &'static str {
    if list == 0 { "Unstaged" } else { "Staged" }
}
/// One-character change marker for a status row; the full kind is in the tooltip.
fn kind_symbol(kind: Kind) -> &'static str {
    match kind {
        Kind::Added | Kind::IntentToAdd => "+",
        Kind::Deleted => "−",
        Kind::Modified => "~",
        Kind::TypeChanged => "T",
        Kind::Conflicted => "!",
    }
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
    pub sign: bool,
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
    stage: crate::stage_view::StageActions,
    reader: crate::diff_view::DiffView,
    drop_confirm: Option<crate::diff_view::StageEdit>,
    commit: Option<std::sync::mpsc::Receiver<Result<gg_git::commit::Receipt, String>>>,
    commit_done: Option<String>,
    last_commit: Option<String>,
    commit_warning: Option<String>,
    last_signed: bool,
    signing: Option<gg_engine::signature::Config>,
    sign_commit: bool,
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
    stage_follow: Option<StageFollow>,
    stage_follow_ready: bool,
    lists: [List; 2],
    focus: usize,
    total: usize,
    complete: bool,
    error: Option<String>,
    action_error: Option<String>,
    external_busy: Option<String>,
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
            stage: crate::stage_view::StageActions::default(),
            reader: crate::diff_view::DiffView::default(),
            drop_confirm: None,
            commit: None,
            commit_done: None,
            last_commit: None,
            commit_warning: None,
            last_signed: false,
            signing: None,
            sign_commit: false,
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
            stage_follow: None,
            stage_follow_ready: false,
            lists: [List::new(), List::new()],
            focus: 0,
            total: 0,
            complete: false,
            error: None,
            action_error: None,
            external_busy: None,
            replacing: false,
            active: None,
            active_entry: None,
            reader_rect: egui::Rect::NOTHING,
            form_rect: egui::Rect::NOTHING,
            last_scroll: f64::NEG_INFINITY,
        };
        view.signing = Some(gg_git::signature::config(&view.path));
        view.sign_commit = view
            .signing
            .as_ref()
            .is_some_and(|config| config.commit_signing);
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
            self.stage_follow = None;
            self.stage_follow_ready = false;
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
        if self.stage_follow_ready && self.complete {
            self.stage_follow_ready = false;
            if let Some(follow) = self.stage_follow.take() {
                self.apply_stage_follow(follow, ctx);
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
        if self.active.is_none() {
            return;
        }
        self.active_entry = None;
        self.stage.clear();
        self.reader.clear();
        self.drop_confirm = None;
        ctx.request_repaint();
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
        self.drop_confirm = None;
        if let Some(repository) = self.identity.clone() {
            self.stage.select(repository.clone(), entry.clone());
            self.reader.select_working(repository, entry, ctx);
        }
    }
    fn toggle_file(&mut self, index: usize, row: usize, ctx: &egui::Context) {
        if self.mutations_blocked() {
            return;
        }
        let Some(entry) = self.lists[index].rows.get(&row).cloned() else {
            return;
        };
        let eligible = match entry.area {
            Area::Staged => true,
            Area::Unstaged | Area::Untracked => {
                !matches!(entry.kind, Kind::IntentToAdd | Kind::Conflicted)
            }
            Area::Conflict => false,
        };
        if !eligible {
            return;
        }
        self.select_row(index, row, ctx);
        self.apply_stage_edit(
            StageEdit {
                scope: StageScope::File,
                discard: false,
                header: None,
                changes: Vec::new(),
            },
            ctx,
        );
    }

    fn next_unstaged_after(&self, path_bytes: &[u8]) -> Option<Vec<u8>> {
        let rows = self.lists[0]
            .rows
            .values()
            .filter(|entry| matches!(entry.area, Area::Unstaged | Area::Untracked))
            .collect::<Vec<_>>();
        let index = rows
            .iter()
            .position(|entry| entry.path_bytes == path_bytes)?;
        rows.get(index + 1)
            .or_else(|| index.checked_sub(1).and_then(|index| rows.get(index)))
            .map(|entry| entry.path_bytes.clone())
    }

    fn apply_stage_edit(&mut self, edit: StageEdit, ctx: &egui::Context) {
        self.stage_follow = None;
        self.stage_follow_ready = false;
        if edit.scope == StageScope::File
            && !edit.discard
            && let Some(entry) = self.active_entry.clone()
        {
            if entry.area == Area::Staged {
                self.stage_follow = Some(StageFollow::Move {
                    path: entry.path_bytes.clone(),
                });
            } else {
                let target = self.next_unstaged_after(&entry.path_bytes);
                self.stage_follow = Some(StageFollow::Advance {
                    from: entry.path_bytes.clone(),
                    target,
                });
            }
        }
        self.stage.apply(edit, ctx);
        if !self.stage.mutating() {
            self.stage_follow = None;
        }
    }

    fn apply_stage_follow(&mut self, follow: StageFollow, ctx: &egui::Context) {
        let path = match &follow {
            StageFollow::Advance { from, .. } => from.clone(),
            StageFollow::Move { path } => path.clone(),
        };
        if !self
            .active
            .as_ref()
            .is_some_and(|(active, _)| active == &path)
        {
            return;
        }
        match follow {
            StageFollow::Move { path } => {
                if let Some(row) = self.lists[0]
                    .rows
                    .iter()
                    .find(|(_, entry)| entry.path_bytes == path)
                    .map(|(row, _)| *row)
                {
                    self.select_row(0, row, ctx);
                    self.lists[0].reveal = true;
                    return;
                }
            }
            StageFollow::Advance { target, .. } => {
                if let Some(target) = target
                    && let Some(row) = self.lists[0]
                        .rows
                        .iter()
                        .find(|(_, entry)| entry.path_bytes == target)
                        .map(|(row, _)| *row)
                {
                    self.select_row(0, row, ctx);
                    self.lists[0].reveal = true;
                    return;
                }
                if let Some(row) = self.lists[0].rows.keys().next().copied() {
                    self.select_row(0, row, ctx);
                    self.lists[0].reveal = true;
                    return;
                }
                if let Some(row) = self.lists[1]
                    .rows
                    .iter()
                    .find(|(_, entry)| entry.path_bytes == path)
                    .map(|(row, _)| *row)
                {
                    self.select_row(1, row, ctx);
                    self.lists[1].reveal = true;
                    return;
                }
            }
        }
        if let Some(row) = self.lists[0].rows.keys().next().copied() {
            self.select_row(0, row, ctx);
        } else if let Some(row) = self.lists[1].rows.keys().next().copied() {
            self.select_row(1, row, ctx);
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
        let selection_changed = self.active_entry.as_ref() != Some(&entry);
        if let Some(identity) = self.identity.clone() {
            if selection_changed {
                self.stage.select(identity.clone(), entry.clone());
                self.reader.select_working(identity, entry.clone(), ctx);
            } else if reload {
                self.reader.refresh_working(identity, ctx);
            }
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
    pub(crate) fn busy(&self) -> bool {
        self.stage.mutating() || self.commit.is_some()
    }

    /// Mutations must wait while another view owns an operation, but browsing stays live.
    fn mutations_blocked(&self) -> bool {
        self.busy() || self.external_busy.is_some()
    }

    /// Disable incompatible mutations while another view owns an operation.
    pub fn set_external_busy(&mut self, reason: Option<String>) {
        self.external_busy = reason;
    }

    pub fn take_commit(&mut self) -> Option<String> {
        self.commit_done.take()
    }

    /// Canonical worktree identity used to keep drafts from crossing repositories.
    pub(crate) fn repository_key(&self) -> PathBuf {
        std::fs::canonicalize(&self.path).unwrap_or_else(|_| self.path.clone())
    }

    /// Capture the active draft before the workspace is dropped.
    pub(crate) fn take_submodule_request(&mut self) -> Option<crate::diff_view::SubmoduleRequest> {
        self.reader.take_submodule_request()
    }

    /// Take an LFS-details request raised by the reader.
    pub(crate) fn take_lfs_request(&mut self) -> Option<crate::diff_view::LfsRequest> {
        self.reader.take_lfs_request()
    }

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
            sign: self.sign_commit,
            selected: self.active.clone(),
        }
    }

    /// Restore a draft after reopening Local Changes for the same worktree.
    pub(crate) fn restore_draft(&mut self, draft: Draft, ctx: &egui::Context) {
        self.normal_draft = draft.normal;
        self.amend_draft = draft.amend;
        self.sign_commit = draft.sign;
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

    /// Put `message` in an empty commit box, such as a reopened commit's
    /// original message; a draft the user typed is kept.
    pub fn prefill_message(&mut self, message: &str) {
        if !self.amend && self.message.trim().is_empty() {
            self.message = message.to_string();
        }
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
        if self.mutations_blocked() || self.replacing {
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

    fn signing_error(&self) -> Option<String> {
        if !self.sign_commit {
            return None;
        }
        let config = self.signing.as_ref()?;
        if config.ready() {
            return None;
        }
        Some(
            config
                .note
                .clone()
                .unwrap_or_else(|| "Signing key or tool is unavailable.".into()),
        )
    }

    fn start_commit(&mut self, ctx: &egui::Context) {
        self.signing = Some(gg_git::signature::config(&self.path));
        let signing_error = self.signing_error();
        if commit_reason(
            self.mutations_blocked(),
            self.replacing || !self.complete || self.identity.is_none(),
            &self.message,
            self.amend,
            self.lists[1].count,
            signing_error.as_deref(),
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
        let sign = self.sign_commit;
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let repaint = ctx.clone();
        match std::thread::Builder::new()
            .name("gg-commit".into())
            .spawn(move || {
                let result = match (amend, sign) {
                    (true, true) => gg_git::commit::amend_signed(&repository, &message),
                    (true, false) => gg_git::commit::amend(&repository, &message),
                    (false, true) => gg_git::commit::create_signed(&repository, &message),
                    (false, false) => gg_git::commit::create(&repository, &message),
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
                    self.last_signed = receipt.signed;
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
                    self.signing = Some(gg_git::signature::config(&self.path));
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
    ) -> (Option<usize>, Option<usize>) {
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
            return (None, None);
        }
        let mut clicked = None;
        let mut toggled = None;
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
            return (None, None);
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
                    if response.double_clicked() && !busy {
                        toggled = Some(row);
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
                    let (symbol, symbol_color) = match entry.area {
                        Area::Untracked => ("?", area_color),
                        Area::Conflict => ("!", area_color),
                        _ => (kind_symbol(entry.kind), color),
                    };
                    let description = match area_badge(entry) {
                        Some(badge) => format!("{badge} · {}", entry.kind.label()),
                        None => entry.kind.label().to_string(),
                    };
                    let _ = response.clone().on_hover_text(description);
                    bounded_text::paint(
                        ui,
                        egui::Rect::from_min_size(rect.min, egui::vec2(14.0, HEIGHT)),
                        symbol,
                        egui::FontId::proportional(13.0),
                        symbol_color,
                    );
                    let path_x = 22.0;
                    bounded_text::paint(
                        ui,
                        egui::Rect::from_min_size(
                            rect.min + egui::vec2(path_x, 0.0),
                            egui::vec2((rect.width() - path_x).max(0.0), HEIGHT),
                        ),
                        entry.path.as_str(),
                        egui::FontId::proportional(12.0),
                        ui.visuals().text_color(),
                    );
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
        (clicked, toggled)
    }

    fn draw_lists_wide(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, height: f32) {
        let left = (ui.available_width() * 0.32).clamp(250.0, 380.0);
        ui.allocate_ui_with_layout(
            egui::vec2(left, height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                let each = ((height - 6.0) / 2.0 - GROUP_HEADER - 2.0).max(40.0);
                let (first_click, first_toggle) = self.draw_group(ui, 0, each, false, true);
                ui.add_space(2.0);
                let (second_click, second_toggle) = self.draw_group(ui, 1, each, false, true);
                if let Some(row) = first_click {
                    self.select_row(0, row, ctx);
                } else if let Some(row) = second_click {
                    self.select_row(1, row, ctx);
                }
                if let Some(row) = first_toggle {
                    self.toggle_file(0, row, ctx);
                } else if let Some(row) = second_toggle {
                    self.toggle_file(1, row, ctx);
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
        let (first_click, first_toggle) = self.draw_group(ui, 0, each, true, false);
        let (second_click, second_toggle) = self.draw_group(ui, 1, each, true, false);
        if let Some(row) = first_click {
            self.select_row(0, row, ctx);
        } else if let Some(row) = second_click {
            self.select_row(1, row, ctx);
        }
        if let Some(row) = first_toggle {
            self.toggle_file(0, row, ctx);
        } else if let Some(row) = second_toggle {
            self.toggle_file(1, row, ctx);
        }
    }

    fn draw_reader(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.reader_rect = ui.max_rect();
        if let Some(label) = self.stage.label() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.weak(label);
            });
        }
        if let Some(error) = self.stage.error().map(str::to_owned) {
            bounded_text::field(ui, &error, egui::FontId::proportional(12.0), 22.0);
        }
        if let Some(selected) = &self.active
            && self.active_entry.is_none()
        {
            bounded_text::field(
                ui,
                &gg_engine::review::display_path(&selected.0),
                egui::FontId::proportional(13.0),
                24.0,
            );
            ui.weak("This entry is no longer present. Select another entry.");
            return;
        }
        if self.active.is_none() {
            ui.weak("Select an Unstaged or Staged entry to read its comparison.");
            return;
        }
        let Some(entry) = self.active_entry.clone() else {
            return;
        };
        let mut confirm = self.drop_confirm.take();
        let mut confirmed = false;
        if let Some(edit) = &confirm {
            let scope = match edit.scope {
                crate::diff_view::StageScope::Selected => "the selected changes",
                crate::diff_view::StageScope::Hunk => "this hunk",
                crate::diff_view::StageScope::File => "all working changes",
            };
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    format!(
                        "Discard {scope} from {}? The index is unchanged.",
                        entry.path
                    ),
                );
                if ui
                    .add_enabled(!self.mutations_blocked(), egui::Button::new("Discard"))
                    .clicked()
                {
                    confirmed = true;
                }
                if ui
                    .add_enabled(!self.mutations_blocked(), egui::Button::new("Cancel"))
                    .clicked()
                {
                    confirm = None;
                }
            });
            ui.weak("Working-tree changes will be lost. The index is unchanged.");
        }
        if confirmed {
            if let Some(edit) = confirm.take() {
                self.stage.apply(edit, ctx);
            }
        } else {
            self.drop_confirm = confirm;
        }
        let discard = matches!(entry.area, Area::Unstaged | Area::Untracked)
            && !matches!(entry.kind, Kind::IntentToAdd | Kind::Conflicted);
        let verb = if entry.area == Area::Staged {
            "Unstage"
        } else {
            "Stage"
        };
        self.reader
            .set_staging(verb, discard, self.mutations_blocked());
        self.reader.draw(ui);
        self.reader.clear_staging();
        if let Some(edit) = self.reader.take_stage_edit() {
            if self.mutations_blocked() {
                // Another view owns the repository; drop the stale edit.
            } else if edit.discard {
                self.drop_confirm = Some(edit);
            } else {
                self.apply_stage_edit(edit, ctx);
            }
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
                "Amending {head} · {subject} · Base {base} · Staged is {head}'s parent → index; Unstage removes changes from the amended commit"
            ),
            egui::FontId::proportional(12.0),
            GROUP_HEADER,
        );
    }

    fn draw_commit_form(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let top = ui.cursor().top();
        let left = ui.max_rect().left();
        let right = ui.max_rect().right();
        let busy = self.mutations_blocked() || self.replacing;
        let scanning = self.replacing || !self.complete || self.identity.is_none();
        let signing_error = self.signing_error();
        let reason = commit_reason(
            busy,
            scanning,
            &self.message,
            self.amend,
            self.lists[1].count,
            signing_error.as_deref(),
        );
        let signing_ready = self.signing.as_ref().is_some_and(|config| config.ready());
        let signing_hint = self
            .signing
            .as_ref()
            .and_then(|config| config.note.clone())
            .unwrap_or_else(|| "Signing key or tool is unavailable.".into());
        let can_commit = reason.is_none();
        let message_height = if self.message_expanded { 118.0 } else { 54.0 };
        let mut commit_now = false;
        ui.horizontal(|ui| {
            let button_width = if self.amend { 112.0 } else { 88.0 };
            let width = (ui.available_width() - button_width - 150.0).max(160.0);
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
                    let mut sign = self.sign_commit;
                    let sign_response = ui.add_enabled(
                        !busy && signing_ready,
                        egui::Checkbox::new(&mut sign, "Sign"),
                    );
                    let sign_response = if signing_ready {
                        sign_response.on_hover_text(
                            "Create a signed commit using the configured format and key. A signing failure keeps the draft and index.",
                        )
                    } else {
                        sign_response.on_disabled_hover_text(&signing_hint)
                    };
                    if sign_response.changed() {
                        self.sign_commit = sign;
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
        if let Some(reason) = &self.external_busy {
            return format!(
                "{reason} · {} unstaged · {} staged",
                self.lists[0].count, self.lists[1].count
            );
        }
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
                "{} {}{} · {warning}",
                if self.last_amended {
                    "Amended"
                } else {
                    "Committed"
                },
                self.last_commit.as_deref().unwrap_or(""),
                if self.last_signed { " (signed)" } else { "" }
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
                "{} {id}{} · {remaining}",
                if self.last_amended {
                    "Amended"
                } else {
                    "Committed"
                },
                if self.last_signed { " (signed)" } else { "" }
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
        self.stage.poll();
        if self.stage.error().is_some() {
            self.stage_follow = None;
            self.stage_follow_ready = false;
        }
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
        if self.drop_confirm.is_some() {
            if crate::navigation::available(ctx)
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            {
                self.drop_confirm = None;
            }
        } else if !self.busy() && !self.replacing && crate::navigation::available(ctx) {
            back |= ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                    || i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
            });
            refresh |= ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::R));
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
                self.focus = 1 - self.focus;
                if self.lists[self.focus].count > 0 {
                    let row = self.lists[self.focus].selected_row.unwrap_or(0);
                    self.select_row(self.focus, row, ctx);
                    self.lists[self.focus].reveal = true;
                }
                crate::navigation::focus_content(ctx);
            }
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
        egui::Panel::bottom("working-status")
            .exact_size(25.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2((ui.available_width() - 82.0).max(120.0), 22.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            bounded_text::field(
                                ui,
                                &self.status_text(),
                                egui::FontId::proportional(12.0),
                                22.0,
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
                            .min_size(egui::vec2(74.0, 22.0)),
                        )
                        .clicked();
                });
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
        if let Some(identity) = self.stage.take_changed() {
            if self.stage_follow.is_some() {
                self.stage_follow_ready = true;
            }
            self.reader.refresh_working(identity, ctx);
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

    #[test]
    fn change_markers_are_single_distinct_characters() {
        let kinds = [
            Kind::Added,
            Kind::Modified,
            Kind::Deleted,
            Kind::TypeChanged,
            Kind::Conflicted,
        ];
        let symbols = kinds.map(kind_symbol);
        for symbol in symbols {
            assert_eq!(symbol.chars().count(), 1, "{symbol:?}");
        }
        let mut unique = symbols.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), kinds.len(), "{symbols:?}");
        assert_eq!(kind_symbol(Kind::IntentToAdd), kind_symbol(Kind::Added));
    }

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
            stage: crate::stage_view::StageActions::default(),
            reader: crate::diff_view::DiffView::default(),
            drop_confirm: None,
            commit: None,
            commit_done: None,
            last_commit: None,
            commit_warning: None,
            last_signed: false,
            signing: None,
            sign_commit: false,
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
            stage_follow: None,
            stage_follow_ready: false,
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
            external_busy: None,
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
    fn history_navigation_keeps_a_pending_commit_owned() {
        let ctx = egui::Context::default();
        let mut app = crate::Browser::new("/repository".into());
        let mut working = fixture(0, 1);
        let (_tx, rx) = std::sync::mpsc::sync_channel(1);
        working.commit = Some(rx);
        app.working = Some(working);

        app.close_to_history(&ctx);

        assert!(app.working.is_some());
        assert!(app.working.as_ref().is_some_and(|view| view.busy()));
        assert!(app.operation_busy());
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
                if (!reading && view.complete && !view.replacing && view.reader.ready())
                    || (reading && view.reader.ready() && view.reader.has_text(text))
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
        assert!(!view.reader.has_text("before"));
        Ok(())
    }
    #[test]
    fn tab_switches_keyboard_focus_between_the_file_groups() {
        let ctx = egui::Context::default();
        let mut view = fixture(2, 2);
        frame(&ctx, &mut view, 1120.0, vec![]);
        let tab = egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&ctx, &mut view, 1120.0, vec![tab.clone()]);
        assert_eq!(view.focus, 1);
        assert_eq!(
            view.active.as_ref().map(|(_, area)| *area),
            Some(Area::Staged)
        );
        frame(&ctx, &mut view, 1120.0, vec![tab]);
        assert_eq!(view.focus, 0);
        assert_eq!(
            view.active.as_ref().map(|(_, area)| *area),
            Some(Area::Unstaged)
        );
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
            assert!(
                view.lists[0].rect.top() < 42.0,
                "no duplicate workspace header: top={}",
                view.lists[0].rect.top()
            );
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
        assert!(view.reader.has_text("staged"));
        view.select_row(0, unstaged, &ctx);
        ready(&ctx, &mut view, 1120.0);
        assert_eq!(
            view.active.as_ref().map(|(_, area)| *area),
            Some(Area::Unstaged)
        );
        assert!(view.reader.has_text("working"));
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
    #[test]
    fn workspace_uses_one_reader_header_with_on_demand_details()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_area(&mut view, &ctx, Area::Unstaged);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.contains(&"File actions"));
        assert!(text.contains(&"index → working file"));
        assert!(!text.contains(&"View diff"));
        assert!(!text.iter().any(|line| line.starts_with("Unstaged:")));
        assert!(
            !text.contains(&"Show file details"),
            "low-frequency identity stays behind the action surface"
        );
        assert!(
            !text.iter().any(|line| line.starts_with("100644 ")),
            "object identity is not a permanent row"
        );

        let menu = text_pos(&output, "File actions").ok_or("missing File actions")?;
        click(&ctx, &mut view, 1120.0, menu + egui::vec2(5.0, 0.0));
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let details =
            text_pos(&output, "Show file details").ok_or("missing file details action")?;
        click(&ctx, &mut view, 1120.0, details + egui::vec2(5.0, 0.0));
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            text.iter()
                .any(|line| line.contains("Unstaged · Modified") && line.contains("index")),
            "details name the comparison without replacing the reader header: {text:?}"
        );
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
    fn run_at(
        ctx: &egui::Context,
        view: &mut StatusView,
        width: f32,
        events: Vec<egui::Event>,
        time: f64,
    ) -> egui::FullOutput {
        crate::tests::run_ui(
            ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 480.0),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| {
                view.draw_ui(ctx);
            },
        )
    }
    fn row_pos(
        output: &egui::FullOutput,
        view: &StatusView,
        list: usize,
        path: &str,
    ) -> Option<egui::Pos2> {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text)
                if text.galley.job.text == path && view.lists[list].rect.contains(text.pos) =>
            {
                Some(text.pos + egui::vec2(6.0, 6.0))
            }
            _ => None,
        })
    }
    fn double_click(ctx: &egui::Context, view: &mut StatusView, width: f32, pos: egui::Pos2) {
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
        let now = ctx.input(|input| input.time).max(100.0) + 1.0;
        run_at(
            ctx,
            view,
            width,
            vec![egui::Event::PointerMoved(pos), press.clone()],
            now,
        );
        run_at(ctx, view, width, vec![release.clone()], now + 0.05);
        run_at(ctx, view, width, vec![press], now + 0.10);
        run_at(
            ctx,
            view,
            width,
            vec![release, egui::Event::PointerGone],
            now + 0.15,
        );
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
    fn wait_text(
        ctx: &egui::Context,
        view: &mut StatusView,
        width: f32,
        label: &str,
    ) -> Result<egui::Pos2, Box<dyn std::error::Error>> {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let output = run(ctx, view, width, vec![]);
            if let Some(pos) = text_pos(&output, label) {
                return Ok(pos);
            }
            if std::time::Instant::now() > until {
                return Err(format!("missing {label}").into());
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
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
            assert!(
                std::time::Instant::now() < until,
                "working view timed out: busy={} complete={} replacing={} lists={}/{} active={:?} commit={} done={:?} error={:?}",
                view.busy(),
                view.complete,
                view.replacing,
                view.lists[0].count,
                view.lists[1].count,
                view.active,
                view.commit.is_some(),
                view.commit_done,
                view.error
            );
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    fn settle(ctx: &egui::Context, view: &mut StatusView, width: f32) {
        pump(ctx, view, width, |view| {
            view.complete && !view.replacing && !view.busy()
        });
    }
    fn ready(ctx: &egui::Context, view: &mut StatusView, width: f32) {
        pump(ctx, view, width, |view| view.reader.ready());
    }
    fn generated(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        git(path, &["init", "-q"])?;
        git(path, &["config", "core.autocrlf", "false"])?;
        git(path, &["config", "user.name", "Test"])?;
        git(path, &["config", "user.email", "test@example.org"])?;
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
    fn click_whole_file(
        ctx: &egui::Context,
        view: &mut StatusView,
        width: f32,
        label: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        pump(ctx, view, width, |view| {
            view.reader.ready() && view.complete && !view.replacing && !view.stage.mutating()
        });
        let button = wait_text(ctx, view, width, label)?;
        click(ctx, view, width, button + egui::vec2(5.0, 0.0));
        Ok(())
    }
    fn click_file_action(
        ctx: &egui::Context,
        view: &mut StatusView,
        width: f32,
        label: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        pump(ctx, view, width, |view| {
            view.reader.ready() && view.complete && !view.replacing && !view.stage.mutating()
        });
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let menu = loop {
            let output = run(ctx, view, width, vec![]);
            if let Some(menu) = text_pos(&output, "File actions") {
                break menu;
            }
            if std::time::Instant::now() > until {
                return Err("missing File actions".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        };
        click(ctx, view, width, menu + egui::vec2(5.0, 0.0));
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let item = loop {
            let output = run(ctx, view, width, vec![]);
            if let Some(item) = text_pos(&output, label) {
                break item;
            }
            if std::time::Instant::now() > until {
                return Err("missing file action item".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        };
        click(ctx, view, width, item + egui::vec2(5.0, 0.0));
        Ok(())
    }
    fn select_area(view: &mut StatusView, ctx: &egui::Context, area: Area) {
        let index = if area == Area::Staged { 1 } else { 0 };
        if view.lists[index].rows.contains_key(&0) {
            view.select_row(index, 0, ctx);
        }
    }

    #[test]
    fn local_changes_full_file_reads_both_working_sides() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_area(&mut view, &ctx, Area::Unstaged);
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let full = text_pos(&output, "Full file").ok_or("missing Full file")?;
        click(&ctx, &mut view, 1120.0, full + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.reader.pending() && view.reader.has_text("ONE")
        });
        assert!(view.reader.has_text("TWELVE"));
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let before = text_pos(&output, "Before").ok_or("missing Before side")?;
        click(&ctx, &mut view, 1120.0, before + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.reader.pending() && view.reader.has_text("twelve")
        });
        assert!(!view.reader.has_text("TWELVE"));
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let diff = text_pos(&output, "Diff").ok_or("missing Diff")?;
        click(&ctx, &mut view, 1120.0, diff + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.reader.pending() && view.reader.has_text("TWELVE")
        });
        Ok(())
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
            !view.reader.pending()
                && view.reader.ready()
                && !view.stage.mutating()
                && view.complete
                && !view.replacing
        });
        assert_eq!(
            view.active.as_ref().map(|(_, area)| *area),
            Some(Area::Unstaged)
        );
        assert_eq!(
            file(dir.path(), &["show", ":a"])?,
            b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n"
        );
        assert!(view.reader.has_text("TWELVE"));
        assert!(!view.reader.has_text("ONE"));
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
        for _ in 0..2 {
            let next = text_pos(&output, "Next >").ok_or("missing next change")?;
            click(&ctx, &mut view, 1120.0, next + egui::vec2(5.0, 0.0));
        }
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let line = text_pos(&output, "TWELVE").ok_or("missing added line")?;
        click(&ctx, &mut view, 1120.0, line + egui::vec2(5.0, 0.0));
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let button = text_pos(&output, "Stage 1").ok_or("missing selected-line action")?;
        click(&ctx, &mut view, 1120.0, button + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.reader.pending()
                && view.reader.ready()
                && !view.stage.mutating()
                && view.complete
                && !view.replacing
        });
        assert_eq!(
            file(dir.path(), &["show", ":a"])?,
            b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\nTWELVE\n"
        );
        assert!(view.reader.has_text("twelve"));
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
            !view.reader.pending() && !view.stage.mutating()
        });
        assert_eq!(
            file(dir.path(), &["show", ":a"])?,
            b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n"
        );
        click_whole_file(&ctx, &mut view, 1120.0, "Unstage")?;
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.reader.pending()
                && view.reader.ready()
                && !view.stage.mutating()
                && view.complete
                && !view.replacing
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
        click_whole_file(&ctx, &mut view, 1120.0, "Stage")?;
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.reader.pending()
                && view.complete
                && !view.replacing
                && view.lists[1].rows.values().any(|entry| entry.path == "a")
        });
        assert_eq!(file(p, &["show", ":a"])?, b"a-working\n");
        assert_eq!(file(p, &["show", ":b"])?, staged_b);
        assert_eq!(std::fs::read(p.join("b"))?, b"b-working\n");

        select_path(&mut view, &ctx, "a", Area::Staged);
        ready(&ctx, &mut view, 1120.0);
        click_whole_file(&ctx, &mut view, 1120.0, "Unstage")?;
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.reader.pending() && !view.stage.mutating() && view.complete && !view.replacing
        });
        assert_eq!(file(p, &["show", ":a"])?, b"a-base\n");
        assert_eq!(file(p, &["show", ":b"])?, staged_b);
        assert_eq!(std::fs::read(p.join("a"))?, b"a-working\n");
        Ok(())
    }

    #[test]
    fn double_click_stages_and_unstages_with_the_stage_button_and_advances()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let p = dir.path();
        git(p, &["init", "-q"])?;
        for name in ["a", "b", "c"] {
            std::fs::write(p.join(name), b"base\n")?;
        }
        git(p, &["add", "."])?;
        git(p, &["commit", "-qm", "base"])?;
        for name in ["a", "b", "c"] {
            std::fs::write(p.join(name), format!("{name} working\n"))?;
        }
        let ctx = egui::Context::default();
        let mut view = StatusView::new(p.into(), &ctx);
        settle(&ctx, &mut view, 1120.0);

        let output = run(&ctx, &mut view, 1120.0, vec![]);
        assert!(text_pos(&output, "Stage").is_some(), "own Stage button");
        let a = row_pos(&output, &view, 0, "a").ok_or("missing a row")?;
        double_click(&ctx, &mut view, 1120.0, a);
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.mutating() && view.complete && !view.replacing && view.lists[0].count == 2
        });
        assert_eq!(file(p, &["show", ":a"])?, b"a working\n");
        assert_eq!(
            view.active
                .as_ref()
                .map(|(bytes, area)| (String::from_utf8_lossy(bytes).into_owned(), *area)),
            Some(("b".into(), Area::Unstaged)),
            "the next unstaged file is selected"
        );

        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let b = row_pos(&output, &view, 0, "b").ok_or("missing b row")?;
        double_click(&ctx, &mut view, 1120.0, b);
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.mutating() && view.complete && !view.replacing && view.lists[0].count == 1
        });
        assert_eq!(
            view.active
                .as_ref()
                .map(|(bytes, area)| (String::from_utf8_lossy(bytes).into_owned(), *area)),
            Some(("c".into(), Area::Unstaged))
        );

        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let c = row_pos(&output, &view, 0, "c").ok_or("missing c row")?;
        double_click(&ctx, &mut view, 1120.0, c);
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.mutating() && view.complete && !view.replacing && view.lists[0].count == 0
        });
        assert_eq!(
            view.active
                .as_ref()
                .map(|(bytes, area)| (String::from_utf8_lossy(bytes).into_owned(), *area)),
            Some(("c".into(), Area::Staged)),
            "the last staged file stays selected in Staged"
        );

        let output = run(&ctx, &mut view, 1120.0, vec![]);
        assert!(text_pos(&output, "Unstage").is_some(), "own Unstage button");
        let c = row_pos(&output, &view, 1, "c").ok_or("missing staged c row")?;
        double_click(&ctx, &mut view, 1120.0, c);
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.mutating() && view.complete && !view.replacing && view.lists[0].count == 1
        });
        assert_eq!(
            file(p, &["diff", "--cached", "--name-only"])?,
            b"a\nb\n",
            "only c was unstaged"
        );
        assert_eq!(
            view.active
                .as_ref()
                .map(|(bytes, area)| (String::from_utf8_lossy(bytes).into_owned(), *area)),
            Some(("c".into(), Area::Unstaged)),
            "unstaging follows the same path into Unstaged"
        );

        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let stage = text_pos(&output, "Stage").ok_or("missing Stage button")?;
        click(&ctx, &mut view, 1120.0, stage + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.mutating()
                && view.complete
                && !view.replacing
                && view.lists[1].rows.values().any(|entry| entry.path == "c")
        });
        assert_eq!(file(p, &["show", ":c"])?, b"c working\n");

        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let unstage = text_pos(&output, "Unstage").ok_or("missing Unstage button")?;
        click(&ctx, &mut view, 1120.0, unstage + egui::vec2(5.0, 0.0));
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.stage.mutating()
                && view.complete
                && !view.replacing
                && view.lists[0].rows.values().any(|entry| entry.path == "c")
        });
        assert_eq!(file(p, &["diff", "--cached", "--name-only"])?, b"a\nb\n");
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
    fn an_external_operation_disables_incompatible_mutations() {
        let ctx = egui::Context::default();
        let mut view = fixture(1, 1);
        frame(&ctx, &mut view, 1120.0, vec![]);
        assert!(!view.busy());
        view.set_external_busy(Some("Wait for the history rewrite to finish".into()));
        assert!(!view.busy(), "browsing stays available");
        assert!(view.mutations_blocked());
        assert!(
            view.status_text()
                .contains("Wait for the history rewrite to finish")
        );
        assert_eq!(
            commit_reason(true, false, "message", false, 1, None),
            Some("Wait for the current operation")
        );
    }
    #[test]
    fn external_operation_keeps_browsing_but_blocks_file_actions()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_area(&mut view, &ctx, Area::Unstaged);
        ready(&ctx, &mut view, 1120.0);
        view.set_external_busy(Some("Wait for the history rewrite to finish".into()));
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let menu = text_pos(&output, "File actions").ok_or("missing File actions")?;
        click(&ctx, &mut view, 1120.0, menu + egui::vec2(5.0, 0.0));
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.contains(&"Copy path"));
        assert!(!text.contains(&"Stage file"));
        assert!(!text.contains(&"Discard file…"));
        assert!(!view.busy(), "selection and navigation stay available");
        assert!(view.mutations_blocked());
        Ok(())
    }
    #[test]
    fn commit_readiness_explains_disabled_states() {
        assert_eq!(
            commit_reason(false, false, "subject", false, 0, None),
            Some("Stage at least one file")
        );
        assert_eq!(
            commit_reason(false, false, "   ", false, 1, None),
            Some("Enter a commit message")
        );
        assert_eq!(
            commit_reason(false, true, "subject", true, 1, None),
            Some("Waiting for the status scan")
        );
        assert_eq!(
            commit_reason(true, false, "subject", false, 1, None),
            Some("Wait for the current operation")
        );
        assert_eq!(commit_reason(false, false, "subject", true, 0, None), None);
        assert_eq!(commit_reason(false, false, "subject", false, 1, None), None);
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
        ready(&ctx, &mut view, 1120.0);
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
                if text.galley.job.text.contains("HEAD parent → index")
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
        git(p, &["config", "user.name", "Test"])?;
        git(p, &["config", "user.email", "test@example.org"])?;
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
        ready(&ctx, &mut view, 1120.0);
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.job.text.contains("Amending")
                    && text.galley.job.text.contains(&format!("parent {}", &parent[..7]))
        )));
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text) if text.galley.job.text.contains("HEAD parent → index")
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
        click_whole_file(&ctx, &mut view, 1120.0, "Unstage")?;
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing
        });
        assert_eq!(file(p, &["show", ":a"])?, b"base\n");
        assert!(view.lists[1].rows.values().all(|entry| entry.path != "a"));
        assert!(view.lists[0].rows.values().any(|entry| entry.path == "a"));

        select_path(&mut view, &ctx, "a", Area::Unstaged);
        ready(&ctx, &mut view, 1120.0);
        click_whole_file(&ctx, &mut view, 1120.0, "Stage")?;
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
    fn signing_readiness_blocks_the_commit_without_unsigned_fallback()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        git(dir.path(), &["add", "a"])?;
        git(dir.path(), &["config", "gpg.format", "openpgp"])?;
        git(
            dir.path(),
            &[
                "config",
                "user.signingkey",
                "DEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF",
            ],
        )?;
        git(
            dir.path(),
            &[
                "config",
                "gpg.program",
                if cfg!(windows) {
                    "gg-no-such-gpg.exe"
                } else {
                    "gg-no-such-gpg"
                },
            ],
        )?;
        git(dir.path(), &["config", "commit.gpgsign", "true"])?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        assert!(view.sign_commit, "commit.gpgsign starts the sign choice on");
        assert!(view.signing_error().is_some());
        view.message = "signed UI".into();
        let head = file(dir.path(), &["rev-parse", "HEAD"])?;
        let index = std::fs::read(dir.path().join(".git/index"))?;
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        assert!(text_pos(&output, "Sign").is_some(), "Sign is visible");

        view.start_commit(&ctx);
        assert!(
            view.commit.is_none(),
            "a blocked signing setup must not start an unsigned commit"
        );
        assert_eq!(view.message, "signed UI");
        assert_eq!(file(dir.path(), &["rev-parse", "HEAD"])?, head);
        assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);

        view.sign_commit = false;
        view.start_commit(&ctx);
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.busy() && view.complete && !view.replacing && view.commit_done.is_some()
        });
        assert!(!view.last_signed);
        assert_ne!(file(dir.path(), &["rev-parse", "HEAD"])?, head);
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
        pump(&ctx, &mut view, 1120.0, |view| view.reader.ready());
        let output = run(&ctx, &mut view, 1120.0, vec![]);
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.iter().any(|line| line.contains("Unmerged path")));
        assert!(
            !text
                .iter()
                .any(|line| line.contains("Stage hunk") || line.contains("Unstage hunk")),
            "conflict content is read-only until Stage resolution"
        );
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
        pump(&ctx, &mut view, 1120.0, |view| {
            view.lists[0]
                .rows
                .iter()
                .any(|(_, entry)| entry.path_bytes == selected.0 && entry.area == selected.1)
        });
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
        pump(&ctx, &mut restored, 1120.0, |view| {
            view.lists[0]
                .rows
                .iter()
                .any(|(_, entry)| entry.path_bytes == selected.0 && entry.area == selected.1)
        });
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
    fn staging_the_only_file_follows_it_into_staged() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        generated(dir.path())?;
        let ctx = egui::Context::default();
        let mut view = StatusView::new(dir.path().into(), &ctx);
        settle(&ctx, &mut view, 1120.0);
        select_path(&mut view, &ctx, "a", Area::Unstaged);
        ready(&ctx, &mut view, 1120.0);
        click_whole_file(&ctx, &mut view, 1120.0, "Stage")?;
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.reader.pending() && view.complete && !view.replacing && view.lists[0].count == 0
        });
        assert_eq!(view.lists[0].count, 0);
        assert_eq!(
            view.active
                .as_ref()
                .map(|(bytes, area)| (String::from_utf8_lossy(bytes).into_owned(), *area)),
            Some(("a".into(), Area::Staged))
        );
        assert_eq!(
            view.lists[1]
                .selected_row
                .and_then(|row| view.lists[1].rows.get(&row)),
            view.lists[1].rows.values().find(|entry| entry.path == "a"),
            "the staged counterpart is selected"
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
        assert!(text.iter().any(|line| line.contains("No unstaged changes")));
        assert!(
            !text.iter().any(|line| line.contains("no longer present")),
            "following the staged file must not leave a missing comparison"
        );
        assert!(text.contains(&"a"), "the staged path is retained");
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
        for _ in 0..2 {
            let next = text_pos(&output, "Next >").ok_or("missing next change")?;
            click(&ctx, &mut view, 1120.0, next + egui::vec2(5.0, 0.0));
        }
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
            !view.reader.pending() && !view.stage.mutating()
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
            assert!(view.drop_confirm.is_some());
            frame(
                &ctx,
                &mut view,
                width,
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert!(view.drop_confirm.is_none(), "Escape cancels the discard");
            assert_eq!(std::fs::read(dir.path().join("a"))?, working);
            assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);

            let output = run(&ctx, &mut view, width, vec![]);
            let drop = text_pos(&output, "Discard hunk").ok_or("missing drop hunk")?;
            click(&ctx, &mut view, width, drop + egui::vec2(5.0, 0.0));
            let cancel = wait_text(&ctx, &mut view, width, "Cancel")?;
            click(&ctx, &mut view, width, cancel + egui::vec2(5.0, 0.0));
            assert_eq!(std::fs::read(dir.path().join("a"))?, working);
            assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);

            let output = run(&ctx, &mut view, width, vec![]);
            let drop = text_pos(&output, "Discard hunk").ok_or("missing drop hunk")?;
            click(&ctx, &mut view, width, drop + egui::vec2(5.0, 0.0));
            let discard = wait_text(&ctx, &mut view, width, "Discard")?;
            click(&ctx, &mut view, width, discard + egui::vec2(5.0, 0.0));
            pump(&ctx, &mut view, width, |view| {
                !view.reader.pending()
                    && view.reader.ready()
                    && !view.stage.mutating()
                    && view.complete
                    && !view.replacing
                    && view.reader.has_text("TWELVE")
            });
            assert_eq!(
                std::fs::read(dir.path().join("a"))?,
                b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n"
            );
            assert_eq!(std::fs::read(dir.path().join(".git/index"))?, index);

            click_file_action(&ctx, &mut view, width, "Discard file…")?;
            let discard = wait_text(&ctx, &mut view, width, "Discard")?;
            click(&ctx, &mut view, width, discard + egui::vec2(5.0, 0.0));
            pump(&ctx, &mut view, width, |view| {
                !view.reader.pending() && view.reader.ready() && view.lists[0].count == 0
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
        pump(&ctx, &mut view, 1120.0, |view| {
            !view.reader.pending() && !view.stage.mutating()
        });
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
