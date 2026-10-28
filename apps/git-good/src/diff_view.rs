//! Virtualized source rows with pinned identity, fixed gutters, and horizontal scrolling.
use crate::{
    blame_worker, bounded_text,
    diff_worker::{Mode, Update, Worker},
    reader,
};
use eframe::egui;
use gg_engine::{
    blame::Blame,
    diff::{DiffKind, DiffRow, FileDiff, RowKind},
    file::{FileText, ReadingSide},
    review::FileChange,
    status::{Area, Kind, StatusEntry, StatusIdentity, WorkingRequest},
};
use std::{collections::BTreeMap, ops::Range, path::PathBuf};

const HEIGHT: f32 = 20.0;

/// Scope of one staging command emitted by the reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageScope {
    Selected,
    Hunk,
    File,
}

/// One validated staging or discard command, expressed independently of the
/// mutation crate. `changes` are `(row kind, old-or-new side line)` pairs.
#[derive(Clone)]
pub struct StageEdit {
    pub scope: StageScope,
    pub discard: bool,
    pub header: Option<String>,
    pub changes: Vec<(RowKind, usize)>,
}

#[derive(Clone, Copy)]
struct StageConfig {
    verb: &'static str,
    can_discard: bool,
    pending: bool,
}

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
    displayed_mode: Mode,
    displayed_side: ReadingSide,
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
    reveal: bool,
    attribution: bool,
    blame_revision: Option<String>,
    blame_before: Option<String>,
    blame_generation: u64,
    blame_worker: Option<blame_worker::Worker>,
    blame: Option<Blame>,
    blame_error: Option<String>,
    blame_loading: bool,
    line_history: Option<(usize, usize)>,
    open_commit: Option<(String, String)>,
    submodule_request: Option<SubmoduleRequest>,
    lfs_request: Option<FileChange>,
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
    restoring: Option<reader::ReaderPlace>,
    stage: Option<StageConfig>,
    stage_selected: BTreeMap<usize, (RowKind, usize)>,
    stage_edit: Option<StageEdit>,
    details_open: bool,
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
            displayed_mode: Mode::Diff,
            displayed_side: ReadingSide::After,
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
            reveal: false,
            attribution: false,
            blame_revision: None,
            blame_before: None,
            blame_generation: 0,
            blame_worker: None,
            blame: None,
            blame_error: None,
            blame_loading: false,
            line_history: None,
            open_commit: None,
            submodule_request: None,
            lfs_request: None,
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
            restoring: None,
            stage: None,
            stage_selected: BTreeMap::new(),
            stage_edit: None,
            details_open: false,
        }
    }
}

#[derive(Clone, Copy)]
enum Action {
    Mode(Mode),
    Side(ReadingSide),
    Change(isize),
    Attribution,
}

#[derive(Clone, Debug)]
pub struct SubmoduleRequest {
    pub path: String,
    pub path_bytes: Vec<u8>,
    pub before: Option<String>,
    pub after: Option<String>,
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
        self.restoring = None;
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
        self.reveal = true;
        self.blame = None;
        self.blame_error = None;
        self.blame_loading = false;
        self.blame_generation += 1;
        self.open_commit = None;
        self.submodule_request = None;
        self.lfs_request = None;
        self.sent = None;
        self.full = None;
        self.cursor = None;
        self.anchor = None;
        self.hunks.clear();
        self.groups.clear();
        self.jump = None;
        self.stage_selected.clear();
        self.stage_edit = None;
        self.details_open = false;
        if !self.side_available(self.side) {
            self.side = self.existing_side();
        }
        self.full_side = self.side;
        self.send(ctx);
        ctx.request_repaint();
    }

    /// Name the commit whose content owns the After side. `None` disables
    /// attribution, such as for working-tree content.
    #[cfg(test)]
    pub fn set_attribution_revision(&mut self, revision: Option<String>) {
        self.set_attribution_revisions(revision, None);
    }

    /// Name the commits that own the After and Before sides. Either may be
    /// absent when that side has no committed content.
    pub fn set_attribution_revisions(&mut self, after: Option<String>, before: Option<String>) {
        if self.blame_revision == after && self.blame_before == before {
            return;
        }
        self.blame_revision = after;
        self.blame_before = before;
        self.blame = None;
        self.blame_error = None;
        self.blame_loading = false;
        self.blame_generation += 1;
    }

    /// Whether the attribution overlay is currently requested.
    #[cfg(test)]
    pub fn attribution_active(&self) -> bool {
        self.attribution
    }

    /// Take a line-history request raised by the attribution strip.
    pub fn take_line_history(&mut self) -> Option<(usize, usize)> {
        self.line_history.take()
    }

    /// Take an Open commit request raised by the attribution strip.
    pub fn take_open_commit(&mut self) -> Option<(String, String)> {
        self.open_commit.take()
    }

    /// Take a submodule-details request raised by File actions.
    pub fn take_submodule_request(&mut self) -> Option<SubmoduleRequest> {
        self.submodule_request.take()
    }

    /// Take an LFS-details request raised by File actions.
    pub fn take_lfs_request(&mut self) -> Option<FileChange> {
        self.lfs_request.take()
    }

    fn is_lfs(&self) -> bool {
        let pointer = |text: &str| text.starts_with("version https://git-lfs.github.com/spec/v1");
        self.info
            .as_ref()
            .is_some_and(|info| info.rows.iter().any(|row| pointer(&row.text)))
            || self.rows.values().any(|row| pointer(&row.text))
            || self
                .full
                .as_ref()
                .is_some_and(|full| full.lines.iter().any(|line| pointer(&line.text)))
    }

    fn request_submodule(&self, change: &FileChange) -> Option<SubmoduleRequest> {
        let gitlink = |side: &Option<gg_engine::review::FileSide>| {
            side.as_ref().is_some_and(|side| side.mode == 0o160000)
        };
        if !gitlink(&change.old)
            && !gitlink(&change.new)
            && self
                .info
                .as_ref()
                .is_none_or(|info| info.kind != DiffKind::Gitlink)
        {
            return None;
        }
        Some(SubmoduleRequest {
            path: change.path.clone(),
            path_bytes: change.path_bytes.clone(),
            before: change
                .old
                .as_ref()
                .filter(|side| side.mode == 0o160000)
                .map(|side| side.id.clone()),
            after: change
                .new
                .as_ref()
                .filter(|side| side.mode == 0o160000)
                .map(|side| side.id.clone()),
        })
    }

    /// Whether attribution can be shown for the current comparison.
    pub fn attribution_available(&self) -> bool {
        let revision = match self.side {
            ReadingSide::After => &self.blame_revision,
            ReadingSide::Before => &self.blame_before,
        };
        self.mode == Mode::Full && self.working.is_none() && revision.is_some()
    }

    /// Toggle the attribution overlay and start its bounded read.
    pub fn toggle_attribution(&mut self, ctx: &egui::Context) {
        if !self.attribution && self.mode != Mode::Full {
            self.set_mode(Mode::Full, ctx);
        }
        if !self.attribution && !self.attribution_available() {
            return;
        }
        self.attribution = !self.attribution;
        if self.attribution {
            if self.blame.is_none() && !self.blame_loading {
                self.request_blame(ctx);
            }
        } else {
            self.blame = None;
            self.blame_error = None;
            self.blame_loading = false;
            self.blame_generation += 1;
        }
    }

    fn request_blame(&mut self, ctx: &egui::Context) {
        let revision = match self.side {
            ReadingSide::After => self.blame_revision.clone(),
            ReadingSide::Before => self.blame_before.clone(),
        };
        let Some(revision) = revision else {
            return;
        };
        let Some(change) = self.change.as_ref() else {
            return;
        };
        if self.working.is_some() {
            self.blame_error = Some("Attribution is available for committed content.".into());
            self.blame_loading = false;
            return;
        }
        if self.blame_worker.is_none() {
            match blame_worker::Worker::start(ctx.clone()) {
                Ok(worker) => self.blame_worker = Some(worker),
                Err(error) => {
                    self.blame_error = Some(error);
                    self.blame_loading = false;
                    return;
                }
            }
        }
        self.blame_generation += 1;
        self.blame = None;
        self.blame_error = None;
        self.blame_loading = true;
        if let Some(worker) = &self.blame_worker {
            worker.request(
                self.blame_generation,
                self.path.clone(),
                revision,
                change.path_bytes.clone(),
            );
        }
        ctx.request_repaint();
    }

    fn poll_blame(&mut self) {
        let Some(update) = self
            .blame_worker
            .as_ref()
            .and_then(blame_worker::Worker::poll)
        else {
            return;
        };
        if update.generation != self.blame_generation {
            return;
        }
        self.blame_loading = false;
        match update.result {
            Ok(blame) => {
                self.blame = Some(blame);
                self.blame_error = None;
            }
            Err(error) => {
                self.blame = None;
                self.blame_error = Some(error);
            }
        }
    }

    fn sync_blame(&mut self, ctx: &egui::Context) {
        if self.attribution
            && self.blame.is_none()
            && !self.blame_loading
            && self.blame_error.is_none()
        {
            self.request_blame(ctx);
        }
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

    /// Name the two inspection sides. History and review both use Before/After.
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
        let mut context = reader::ReaderContext::new(
            self.path.clone(),
            change.clone(),
            self.before_endpoint
                .clone()
                .unwrap_or_else(|| endpoint(self.old_label, &change.old)),
            self.after_endpoint
                .clone()
                .unwrap_or_else(|| endpoint(self.new_label, &change.new)),
            self.place(),
        );
        context.edge_known = self.edge_known;
        Some(context)
    }

    /// Restore a captured context without inventing a second viewer.
    pub fn open_context(&mut self, context: &reader::ReaderContext, ctx: &egui::Context) {
        // Restore the saved edge capability. Consumer-specific side labels are
        // reset by the caller before selecting a new source.
        self.edge_known = context.edge_known;
        self.old_label = "Before";
        self.new_label = "After";
        let same = self.path == context.repository
            && self
                .change
                .as_ref()
                .map(|c| (&c.path_bytes, &c.old, &c.new))
                == Some((
                    &context.change.path_bytes,
                    &context.change.old,
                    &context.change.new,
                ));
        let mut place = context.place.clone();
        if !same {
            self.mode = place.mode;
            self.side = place.side;
            if !self.side_available(self.side) {
                self.side = self.existing_side();
                place.side = self.side;
            }
            self.select(
                context.repository.clone(),
                Some(context.change.clone()),
                ctx,
            );
        } else {
            let side_changed = self.side != place.side;
            self.mode = place.mode;
            self.side = place.side;
            if !self.side_available(self.side) {
                self.side = self.existing_side();
                place.side = self.side;
            }
            self.full_side = self.side;
            if side_changed || self.side != place.side {
                self.full = None;
            }
        }
        self.set_endpoints(context.before.label.clone(), context.after.label.clone());
        self.cursor = place.cursor;
        self.anchor = place.anchor;
        self.offset = place.offset;
        self.restoring = Some(place);
        self.sent = None;
        self.send(ctx);
        ctx.request_repaint();
    }

    /// Select the active Staged, Unstaged, Untracked, or Conflict comparison.
    pub fn select_working(
        &mut self,
        repository: StatusIdentity,
        entry: StatusEntry,
        ctx: &egui::Context,
    ) {
        let same = self
            .working
            .as_ref()
            .is_some_and(|working| working.repository == repository && working.entry == entry);
        if same {
            return;
        }
        self.restoring = None;
        self.restore_line = self.current_line();
        self.path = repository.path.clone();
        self.change = Some(FileChange {
            path: entry.path.clone(),
            path_bytes: entry.path_bytes.clone(),
            kind: match entry.kind {
                Kind::Added => gg_engine::review::ChangeKind::Added,
                Kind::Deleted => gg_engine::review::ChangeKind::Deleted,
                Kind::TypeChanged => gg_engine::review::ChangeKind::TypeChanged,
                _ => gg_engine::review::ChangeKind::Modified,
            },
            old: None,
            new: None,
        });
        self.working = Some(WorkingRequest { repository, entry });
        self.generation += 1;
        self.error = None;
        self.staged = None;
        self.reveal = true;
        self.blame = None;
        self.blame_error = None;
        self.blame_loading = false;
        self.blame_generation += 1;
        self.open_commit = None;
        self.submodule_request = None;
        self.lfs_request = None;
        self.sent = None;
        self.full = None;
        self.cursor = None;
        self.anchor = None;
        self.hunks.clear();
        self.groups.clear();
        self.jump = None;
        self.stage_selected.clear();
        self.stage_edit = None;
        self.before_endpoint = None;
        self.after_endpoint = None;
        self.side = self.existing_side();
        self.full_side = self.side;
        self.send(ctx);
        ctx.request_repaint();
    }

    /// Leave the file source without keeping any comparison.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Enable the staged/working action overlay for this frame.
    pub fn set_staging(&mut self, verb: &'static str, can_discard: bool, pending: bool) {
        self.stage = Some(StageConfig {
            verb,
            can_discard,
            pending,
        });
    }

    /// Disable row actions when no staging entry is selected.
    pub fn clear_staging(&mut self) {
        self.stage = None;
    }

    /// Take one action emitted by the staging overlay.
    pub fn take_stage_edit(&mut self) -> Option<StageEdit> {
        self.stage_edit.take()
    }

    /// Refresh the same working comparison after a successful mutation.
    pub fn refresh_working(&mut self, repository: StatusIdentity, ctx: &egui::Context) {
        let Some(working) = &mut self.working else {
            return;
        };
        working.repository = repository;
        self.generation += 1;
        self.sent = None;
        self.error = None;
        self.send(ctx);
        ctx.request_repaint();
    }

    fn working_note_detail(note: &str) -> &str {
        note.strip_prefix("Working snapshot · mode ")
            .and_then(|rest| rest.split_once(" · "))
            .map(|(_, detail)| detail)
            .unwrap_or("")
    }

    fn working_route(&self, working: &WorkingRequest) -> String {
        match working.entry.area {
            Area::Staged => {
                if working.repository.base_tree.is_some() {
                    "HEAD parent → index".into()
                } else if let Some(head) = &working.repository.head {
                    let short = head.get(..8).unwrap_or(head);
                    format!("HEAD {short} → index")
                } else {
                    "empty → index".into()
                }
            }
            Area::Unstaged => "index → working file".into(),
            Area::Untracked => "empty → working file".into(),
            Area::Conflict => "Unmerged path · resolve the conflict before staging".into(),
        }
    }

    fn working_side_available(entry: &StatusEntry, side: ReadingSide) -> bool {
        if entry.area == Area::Conflict {
            return false;
        }
        match (entry.kind, side) {
            (Kind::Added, ReadingSide::Before) | (Kind::Deleted, ReadingSide::After) => false,
            (_, ReadingSide::Before) => entry.area != Area::Untracked,
            (_, ReadingSide::After) => true,
        }
    }

    fn stage_text(&self) -> bool {
        self.info
            .as_ref()
            .is_some_and(|info| info.kind == DiffKind::Text && info.complete)
    }

    fn content_pending(&self) -> bool {
        self.displayed_generation != self.generation
            || self.displayed_mode != self.mode
            || (self.mode == Mode::Full && self.displayed_side != self.side)
    }

    fn content_settled(&self) -> bool {
        !self.content_pending()
    }

    fn can_line_edit(&self) -> bool {
        let Some(working) = &self.working else {
            return false;
        };
        self.content_settled()
            && self.stage.is_some()
            && self.mode == Mode::Diff
            && self.stage_text()
            && matches!(
                (working.entry.area, working.entry.kind),
                (Area::Staged, _) | (Area::Unstaged, Kind::Modified)
            )
    }

    fn can_whole_edit(&self) -> bool {
        self.content_settled()
            && self.stage.is_some_and(|stage| !stage.pending)
            && self.info.is_some()
            && self.working.as_ref().is_some_and(|working| {
                matches!(
                    working.entry.area,
                    Area::Staged | Area::Unstaged | Area::Untracked
                )
            })
    }

    fn can_drop_whole(&self) -> bool {
        self.content_settled()
            && self
                .stage
                .is_some_and(|stage| stage.can_discard && !stage.pending)
            && self.working.as_ref().is_some_and(|working| {
                matches!(working.entry.area, Area::Unstaged | Area::Untracked)
                    && !matches!(working.entry.kind, Kind::IntentToAdd | Kind::Conflicted)
            })
    }

    fn hunk_bounds(&self, header: usize) -> Range<usize> {
        let end = self
            .hunks
            .iter()
            .find(|hunk| **hunk > header)
            .copied()
            .unwrap_or(self.count);
        header.saturating_add(1)..end
    }

    fn toggle_hunk(&mut self, header: usize) {
        let bounds = self.hunk_bounds(header);
        let rows: Vec<(usize, RowKind, usize)> = self
            .rows
            .range(bounds.clone())
            .filter_map(|(row, row_data)| match row_data.kind {
                RowKind::Added => row_data.new_line.map(|line| (*row, row_data.kind, line)),
                RowKind::Deleted => row_data.old_line.map(|line| (*row, row_data.kind, line)),
                _ => None,
            })
            .collect();
        if rows.is_empty() {
            return;
        }
        let all = rows
            .iter()
            .all(|(row, _, _)| self.stage_selected.contains_key(row));
        for (row, kind, line) in rows {
            if all {
                self.stage_selected.remove(&row);
            } else {
                self.stage_selected.insert(row, (kind, line));
            }
        }
    }

    fn hunk_edit(&self, header: usize, discard: bool) -> StageEdit {
        let bounds = self.hunk_bounds(header);
        let chosen: Vec<(usize, RowKind, usize)> = self
            .stage_selected
            .iter()
            .filter(|(row, _)| bounds.contains(row))
            .map(|(row, (kind, line))| (*row, *kind, *line))
            .collect();
        let changes = if chosen.is_empty() {
            vec![]
        } else if discard {
            // Expand each touched block so a discard cannot remove half of a
            // replacement that shares the block.
            let picked: std::collections::BTreeSet<usize> =
                chosen.iter().map(|(row, _, _)| *row).collect();
            type Block = (Vec<usize>, Vec<(RowKind, usize)>);
            let mut blocks: Vec<Block> = vec![];
            let mut current: (Vec<usize>, Vec<(RowKind, usize)>) = (vec![], vec![]);
            let mut last: Option<usize> = None;
            for (row, data) in self.rows.range(bounds.clone()) {
                let line = match data.kind {
                    RowKind::Added => data.new_line.map(|line| (data.kind, line)),
                    RowKind::Deleted => data.old_line.map(|line| (data.kind, line)),
                    _ => None,
                };
                match line {
                    Some(line) => {
                        if !last.is_some_and(|last| *row == last + 1) && !current.0.is_empty() {
                            blocks.push(std::mem::take(&mut current));
                        }
                        current.0.push(*row);
                        current.1.push(line);
                        last = Some(*row);
                    }
                    None => {
                        if !current.0.is_empty() {
                            blocks.push(std::mem::take(&mut current));
                        }
                        last = None;
                    }
                }
            }
            if !current.0.is_empty() {
                blocks.push(current);
            }
            blocks
                .into_iter()
                .filter(|(rows, _)| rows.iter().any(|row| picked.contains(row)))
                .flat_map(|(_, lines)| lines)
                .collect()
        } else {
            chosen
                .into_iter()
                .map(|(_, kind, line)| (kind, line))
                .collect()
        };
        StageEdit {
            scope: if changes.is_empty() {
                StageScope::Hunk
            } else {
                StageScope::Selected
            },
            discard,
            header: self.rows.get(&header).map(|row| row.text.clone()),
            changes,
        }
    }

    fn file_edit(&self, discard: bool) -> StageEdit {
        StageEdit {
            scope: StageScope::File,
            discard,
            header: None,
            changes: vec![],
        }
    }

    /// Reapply the restored place after an asynchronous update settles.
    fn apply_restoring(&mut self) {
        let Some(place) = &self.restoring else {
            return;
        };
        if place.mode != self.mode || place.side != self.side {
            return;
        }
        self.cursor = place.cursor;
        self.anchor = place.anchor;
        self.offset = place.offset;
        self.jump = None;
        self.restore_line = None;
    }

    /// Capabilities the current comparison supports.
    pub fn capabilities(&self) -> reader::Capabilities {
        if let Some(working) = &self.working {
            let before = Self::working_side_available(&working.entry, ReadingSide::Before);
            let after = Self::working_side_available(&working.entry, ReadingSide::After);
            return reader::Capabilities {
                diff: working.entry.area != Area::Conflict,
                full_before: before,
                full_after: after,
                export_before: false,
                export_after: false,
            };
        }
        let present = |side: &Option<gg_engine::review::FileSide>| {
            side.as_ref().is_some_and(|side| side.mode != 0o160000)
        };
        let (old, new) = self
            .change
            .as_ref()
            .map(|change| (&change.old, &change.new))
            .unwrap_or((&None, &None));
        let (displayed_old, displayed_new) = self
            .displayed
            .as_ref()
            .map(|change| (&change.old, &change.new))
            .unwrap_or((&None, &None));
        reader::Capabilities {
            diff: self.edge_known,
            full_before: old.is_some(),
            full_after: new.is_some(),
            export_before: self.displayed.is_some() && present(displayed_old),
            export_after: self.displayed.is_some() && present(displayed_new),
        }
    }

    fn side_label(&self, side: ReadingSide) -> &'static str {
        match side {
            ReadingSide::Before => self.old_label,
            ReadingSide::After => self.new_label,
        }
    }

    fn side_available(&self, side: ReadingSide) -> bool {
        if let Some(working) = &self.working {
            return Self::working_side_available(&working.entry, side);
        }
        let Some(change) = &self.change else {
            return true;
        };
        match side {
            ReadingSide::Before => change.old.is_some(),
            ReadingSide::After => change.new.is_some(),
        }
    }

    fn existing_side(&self) -> ReadingSide {
        if let Some(working) = &self.working {
            return if Self::working_side_available(&working.entry, ReadingSide::After) {
                ReadingSide::After
            } else {
                ReadingSide::Before
            };
        }
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
        self.displayed_side = update.side;
        if self.mode == Mode::Full {
            if self.displayed_mode != Mode::Full
                || self.displayed_generation != update.generation
                || self.full_side != update.side
            {
                self.rows.clear();
                self.offset = 0.0;
                self.reset = true;
                self.range = 0..64;
                self.binary_view = crate::binary_preview::View::default();
                self.displayed = self.change.clone();
                self.displayed_path = self.path.clone();
                self.displayed_working = self.working.clone();
                self.displayed_generation = update.generation;
                self.displayed_mode = Mode::Full;
                self.displayed_side = update.side;
                self.full_side = update.side;
                self.full = None;
                self.cursor = None;
                self.anchor = None;
                self.groups.clear();
                self.stage_selected.clear();
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
            self.apply_restoring();
            return;
        }
        match update.result {
            Ok(mut info) => {
                if self.displayed_mode != Mode::Diff
                    || self.displayed_generation != update.generation
                {
                    self.rows.clear();
                    self.offset = 0.0;
                    self.reset = true;
                    self.range = 0..64;
                    self.binary_view = crate::binary_preview::View::default();
                    self.displayed = self.change.clone();
                    self.displayed_path = self.path.clone();
                    self.displayed_working = self.working.clone();
                    self.displayed_generation = update.generation;
                    self.displayed_mode = Mode::Diff;
                    self.displayed_side = update.side;
                    self.cursor = None;
                    self.anchor = None;
                    self.stage_selected.clear();
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
        self.apply_restoring();
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
        self.restoring = None;
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
        self.restoring = None;
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
        self.restoring = None;
        let line = self.current_line().unwrap_or(1);
        self.side = side;
        self.blame = None;
        self.blame_error = None;
        self.blame_loading = false;
        self.blame_generation += 1;
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
        self.restoring = None;
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
        self.restoring = None;
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

    /// One file header: exact path, one scoped action menu, and the exact
    /// endpoints. Low-frequency identity and commands are not permanent rows.
    fn draw_header(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, change: &FileChange) {
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 104.0).max(80.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::hover());
            bounded_text::paint(
                ui,
                rect,
                &change.path,
                egui::FontId::proportional(14.0),
                ui.visuals().text_color(),
            );
            ui.menu_button("File actions", |ui| {
                let capabilities = self.capabilities();
                for (label, side, enabled) in [
                    (
                        "Export before…",
                        change.old.as_ref(),
                        capabilities.export_before,
                    ),
                    (
                        "Export after…",
                        change.new.as_ref(),
                        capabilities.export_after,
                    ),
                ] {
                    if ui
                        .add_enabled(enabled && !self.export.busy(), egui::Button::new(label))
                        .clicked()
                        && let Some(side) = side
                    {
                        self.export.choose(
                            self.displayed_path.clone(),
                            side.clone(),
                            format!("{} · {}", label.trim_end_matches('…'), change.path),
                        );
                        ui.close();
                    }
                }
                if self.can_whole_edit() {
                    let verb = self.stage.map_or("Stage", |stage| stage.verb);
                    if ui.button(format!("{verb} file")).clicked() {
                        self.stage_edit = Some(self.file_edit(false));
                        ui.close();
                    }
                }
                if self.can_drop_whole() && ui.button("Discard file…").clicked() {
                    self.stage_edit = Some(self.file_edit(true));
                    ui.close();
                }
                let native = std::str::from_utf8(&change.path_bytes).ok();
                if ui
                    .button(if native.is_some() {
                        "Copy path"
                    } else {
                        "Copy escaped path"
                    })
                    .clicked()
                {
                    ctx.copy_text(native.unwrap_or(&change.path).to_owned());
                    ui.close();
                }
                if self.working.is_none() && self.is_lfs() {
                    ui.separator();
                    if ui.button("LFS details…").clicked() {
                        self.lfs_request = Some(change.clone());
                        ui.close();
                    }
                }
                if let Some(request) = self.request_submodule(change) {
                    ui.separator();
                    if ui.button("Submodule details…").clicked() {
                        self.submodule_request = Some(request);
                        ui.close();
                    }
                }
                ui.separator();
                if ui
                    .button(if self.details_open {
                        "Hide file details"
                    } else {
                        "Show file details"
                    })
                    .clicked()
                {
                    self.details_open = !self.details_open;
                    ui.close();
                }
            });
        });
        let displayed_working = self
            .displayed_working
            .clone()
            .or_else(|| self.working.clone());
        if let Some(working) = &displayed_working {
            bounded_text::field(
                ui,
                &self.working_route(working),
                egui::FontId::monospace(11.0),
                20.0,
            );
        } else if (self.displayed.is_none() || self.displayed_generation == self.generation)
            && let (Some(before), Some(after)) = (&self.before_endpoint, &self.after_endpoint)
        {
            bounded_text::field(
                ui,
                &format!("{} {before} → {} {after}", self.old_label, self.new_label),
                egui::FontId::monospace(11.0),
                20.0,
            );
        }
        if !self.details_open {
            return;
        }
        let side = |side: &Option<gg_engine::review::FileSide>| {
            side.as_ref()
                .map(|side| format!("{:06o} {}", side.mode, side.id))
                .unwrap_or_else(|| "absent".into())
        };
        let sizes = self
            .info
            .as_ref()
            .map(|info| format!("{} → {} bytes", info.old_size, info.new_size))
            .unwrap_or_default();
        let text = if let Some(working) = &displayed_working {
            let base = if working.entry.area == Area::Staged {
                format!(
                    " · base {} → index {}",
                    working
                        .repository
                        .head
                        .as_deref()
                        .map(default_short)
                        .unwrap_or("unborn"),
                    default_short(&working.repository.index)
                )
            } else {
                format!(" · index {}", default_short(&working.repository.index))
            };
            let note = self
                .info
                .as_ref()
                .map(|info| info.note.trim())
                .filter(|note| !note.is_empty())
                .map(|note| format!(" · {note}"))
                .unwrap_or_default();
            format!(
                "{} · {} · {sizes}{base}{note}",
                working.entry.area.label(),
                working.entry.kind.label(),
            )
        } else {
            format!(
                "{}: {} · {} bytes · {}: {} · {} bytes",
                self.old_label,
                side(&change.old),
                self.info.as_ref().map_or(0, |info| info.old_size),
                self.new_label,
                side(&change.new),
                self.info.as_ref().map_or(0, |info| info.new_size)
            )
        };
        bounded_text::field(ui, &text, egui::FontId::monospace(10.0), 18.0);
    }

    /// One short state line for completeness, notes, and loading. Sizes and
    /// object identity live in File details instead of permanent rows.
    fn draw_status(&mut self, ui: &mut egui::Ui) {
        let pending = self.content_pending();
        let mut parts: Vec<String> = vec![];
        if self.mode == Mode::Full {
            if let Some(full) = &self.full {
                if !full.complete {
                    parts.push("bounded".into());
                }
                if pending {
                    parts.push("loading · previous content retained".into());
                } else if !full.note.is_empty() && !full.note.starts_with("Working snapshot") {
                    parts.push(full.note.clone());
                }
            } else if self.error.is_some() {
                parts.push("Full file unavailable".into());
            } else {
                parts.push("Loading full file…".into());
            }
        } else if let Some(info) = &self.info {
            if !info.complete {
                parts.push("text comparison incomplete".into());
            }
            let note = info.note.trim();
            let detail = if self.working.is_some() {
                Self::working_note_detail(note)
            } else {
                note
            };
            if !pending && !detail.is_empty() && detail != "Empty file" {
                parts.push(detail.to_owned());
            }
            if pending {
                parts.push("loading · previous diff retained".into());
            }
        } else if self.error.is_some() {
            parts.push("Diff unavailable".into());
        } else {
            parts.push("Loading selected-file diff…".into());
        }
        if self.attribution {
            if self.blame_loading {
                parts.push("Loading attribution…".into());
            } else if let Some(error) = &self.blame_error {
                parts.push(format!(
                    "Attribution unavailable: {}",
                    error.lines().next().unwrap_or_default()
                ));
            }
        }
        if parts.is_empty() {
            return;
        }
        bounded_text::field(
            ui,
            &parts.join(" · "),
            egui::FontId::proportional(11.0),
            20.0,
        );
    }

    fn draw_attribution(&mut self, ui: &mut egui::Ui) {
        let Some(line) = self
            .cursor
            .and_then(|index| self.blame.as_ref().and_then(|blame| blame.line(index + 1)))
            .cloned()
        else {
            return;
        };
        let range = match (self.cursor, self.anchor) {
            (Some(cursor), Some(anchor)) => (cursor.min(anchor) + 1, cursor.max(anchor) + 1),
            (Some(cursor), None) => (cursor + 1, cursor + 1),
            _ => (line.line, line.line),
        };
        ui.horizontal_wrapped(|ui| {
            ui.strong(line.author.clone());
            ui.weak(line.email.clone());
            ui.weak(relative_time(line.time));
            ui.weak(line.short_commit());
            ui.label(line.summary.clone());
            if ui.button("Open commit").clicked() {
                self.open_commit = Some((line.commit.clone(), line.summary.clone()));
            }
            if ui.button("Line history").clicked() {
                self.line_history = Some(range);
            }
        });
    }

    fn draw_inner(&mut self, ui: &mut egui::Ui, keyboard: bool) {
        self.bounds = ui.max_rect();
        let ctx = ui.ctx().clone();
        self.poll_blame();
        self.sync_blame(&ctx);
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
            self.restoring = None;
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
        // A newly selected file is the exception: at narrow widths the file
        // list disappears and the resting pointer can already be over the
        // reader, so the explicit selection must reveal its content.
        let reveal = self.reveal;
        if ((quiet && (!hovering || reveal))
            || (self.info.is_some() && self.displayed_generation == self.generation))
            && let Some(update) = self.staged.clone()
        {
            self.accept(update, pinned);
            self.reveal = false;
            if pinned.is_none() {
                self.staged = None;
            }
        }
        let Some(change) = self.displayed.as_ref().or(self.change.as_ref()).cloned() else {
            ui.weak("Select a file to read its diff");
            return;
        };
        self.draw_header(ui, &ctx, &change);
        let mut actions = vec![];
        let capabilities = self.capabilities();
        ui.horizontal_wrapped(|ui| {
            let diff = ui
                .add_enabled(
                    capabilities.diff,
                    egui::Button::new("Diff").selected(self.mode == Mode::Diff),
                )
                .on_disabled_hover_text(if self.working.is_some() {
                    "This path has no comparable sides"
                } else {
                    "Choose an earlier version to compare"
                });
            if diff.clicked() {
                actions.push(Action::Mode(Mode::Diff));
            }
            if ui
                .selectable_label(self.mode == Mode::Full, "Full file")
                .clicked()
            {
                actions.push(Action::Mode(Mode::Full));
            }
            if self.mode == Mode::Full {
                let available = self.attribution_available();
                let label = if self.attribution {
                    "Hide attribution"
                } else {
                    "Show attribution"
                };
                let response = ui
                    .add_enabled(available, egui::Button::new(label))
                    .on_disabled_hover_text(if self.working.is_some() {
                        "Attribution is available for committed content"
                    } else {
                        "This side has no committed revision"
                    });
                if response.clicked() {
                    actions.push(Action::Attribution);
                }
            }
            if self.mode == Mode::Full && self.edge_known {
                ui.separator();
                for side in [ReadingSide::Before, ReadingSide::After] {
                    let available = match side {
                        ReadingSide::Before => capabilities.full_before,
                        ReadingSide::After => capabilities.full_after,
                    };
                    let response = ui.add_enabled(
                        side != self.side && available,
                        egui::Button::new(self.side_label(side)),
                    );
                    if response.clicked() {
                        actions.push(Action::Side(side));
                    }
                }
            }
            ui.separator();
            if !self.edge_known {
                ui.weak("Earlier version not loaded");
                ui.add_enabled(false, egui::Button::new("< Prev"));
                ui.add_enabled(false, egui::Button::new("Next >"));
            } else {
                let (index, total) = self.change_position();
                if total == 0 {
                    if self
                        .info
                        .as_ref()
                        .is_none_or(|info| info.note.trim().is_empty())
                    {
                        ui.weak("No changes");
                    }
                } else if total > 1 {
                    let label = if self.change_number().is_some() {
                        format!("Change {index} of {total}")
                    } else {
                        format!("Before change 1 of {total}")
                    };
                    ui.weak(label);
                }
                let previous = self.previous_change().is_some();
                let next = self.next_change().is_some();
                if ui
                    .add_enabled(previous, egui::Button::new("< Prev"))
                    .on_hover_text("Previous change")
                    .clicked()
                {
                    actions.push(Action::Change(-1));
                }
                if ui
                    .add_enabled(next, egui::Button::new("Next >"))
                    .on_hover_text("Next change")
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
                Action::Attribution => self.toggle_attribution(&ctx),
            }
        }
        self.draw_status(ui);
        if self.attribution && self.mode == Mode::Full {
            self.draw_attribution(ui);
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
        if self.working.is_some()
            && let Some(info) = &self.info
            && info.kind != DiffKind::Text
        {
            ui.weak("No inline preview · open Full file to inspect one side");
            self.send(&ctx);
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
        let staging = self.stage;
        let can_edit = self.can_line_edit();
        let can_discard = can_edit && staging.is_some_and(|stage| stage.can_discard);
        let pending = staging.is_some_and(|stage| stage.pending);
        let verb = staging.map_or("Stage", |stage| stage.verb);
        let mut cursor = self.cursor;
        let mut anchor = self.anchor;
        let mut hunk_toggle = None;
        let mut hunk_action: Option<(usize, bool)> = None;
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
                let selected = self.stage_selected.contains_key(&index);
                if selected {
                    ui.painter()
                        .rect_filled(rect, 0.0, crate::colors::selection(ui.visuals()));
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(rect.min, egui::vec2(3.0, HEIGHT)),
                        0.0,
                        ui.visuals().selection.stroke.color,
                    );
                } else if cursor == Some(index) {
                    ui.painter()
                        .rect_filled(rect, 0.0, ui.visuals().selection.bg_fill);
                } else if index >= lo && index <= hi {
                    ui.painter()
                        .rect_filled(rect, 0.0, crate::colors::selection(ui.visuals()));
                }
                let mut button_clicked = false;
                let mut discard_clicked = false;
                if can_edit && row.kind == RowKind::Hunk {
                    let bounds = self.hunk_bounds(index);
                    let chosen = self
                        .stage_selected
                        .keys()
                        .filter(|row| bounds.contains(row))
                        .count();
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
                                ui.scope_id().with(("stage-hunk", index)),
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
                            hunk_action = Some((index, false));
                        }
                    }
                    if can_discard {
                        let label = if chosen == 0 {
                            "Discard hunk"
                        } else {
                            "Discard selected"
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
                            discard_clicked = response.clicked();
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
                            if discard_clicked {
                                hunk_action = Some((index, true));
                            }
                        }
                    }
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
                let text_right = if can_discard && row.kind == RowKind::Hunk {
                    rect.right() - 112.0
                } else {
                    rect.right()
                };
                let clip = egui::Rect::from_min_max(
                    rect.min + egui::vec2(116.0, 0.0),
                    egui::pos2(text_right, rect.bottom()),
                )
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
                let hover = if can_edit {
                    match row.kind {
                        RowKind::Hunk => "Click to select all changed lines in this hunk",
                        RowKind::Added | RowKind::Deleted => {
                            "Click to select this changed line for partial staging"
                        }
                        _ => "Click to set the place · Double-click to copy this line",
                    }
                } else {
                    "Click to set the place · Double-click to copy this line"
                };
                let response = response.on_hover_text(hover);
                if response.clicked() && !response.double_clicked() && !pending {
                    if button_clicked || discard_clicked {
                        // The inline hunk button owns this click.
                    } else if can_edit && row.kind == RowKind::Hunk {
                        hunk_toggle = Some(index);
                    } else if can_edit && matches!(row.kind, RowKind::Added | RowKind::Deleted)
                    {
                        let line = match row.kind {
                            RowKind::Added => row.new_line,
                            RowKind::Deleted => row.old_line,
                            _ => None,
                        };
                        if let Some(line) = line
                            && self.stage_selected.remove(&index).is_none()
                        {
                            self.stage_selected.insert(index, (row.kind, line));
                        }
                    } else {
                        self.restoring = None;
                        cursor = Some(index);
                        anchor = Some(if shift {
                            anchor.unwrap_or(index)
                        } else {
                            index
                        });
                    }
                }
                if response.double_clicked() && !can_edit {
                    ctx.copy_text(row.text.clone());
                }
            }
        });
        if let Some(header) = hunk_toggle {
            self.toggle_hunk(header);
        }
        if let Some((header, discard)) = hunk_action {
            self.stage_edit = Some(self.hunk_edit(header, discard));
            self.stage_selected.clear();
        }
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
        let blame = self.attribution.then_some(self.blame.as_ref()).flatten();
        let gutter = if blame.is_some() { 156.0 } else { 0.0 };
        let width = (70.0 + gutter + self.columns as f32 * 8.0).max(ui.available_width());
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
                if let Some(entry) = blame.and_then(|blame| blame.line(line.number)) {
                    let clip = egui::Rect::from_min_max(
                        rect.min + egui::vec2(4.0, 0.0),
                        rect.min + egui::vec2(gutter - 6.0, HEIGHT),
                    )
                    .intersect(ui.clip_rect());
                    ui.painter().with_clip_rect(clip).text(
                        rect.min + egui::vec2(4.0, HEIGHT / 2.0),
                        egui::Align2::LEFT_CENTER,
                        format!("{} · {}", entry.author, entry.short_commit()),
                        egui::FontId::proportional(10.0),
                        ui.visuals().weak_text_color(),
                    );
                }
                let font = egui::FontId::monospace(12.0);
                ui.painter().text(
                    rect.min + egui::vec2(40.0 + gutter, HEIGHT / 2.0),
                    egui::Align2::RIGHT_CENTER,
                    line.number.to_string(),
                    egui::FontId::monospace(11.0),
                    ui.visuals().weak_text_color(),
                );
                if line.changed {
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            rect.min + egui::vec2(46.0 + gutter, 3.0),
                            egui::vec2(3.0, HEIGHT - 6.0),
                        ),
                        1.0,
                        color,
                    );
                }
                let origin = rect.min + egui::vec2(58.0 + gutter, HEIGHT / 2.0);
                let clip =
                    egui::Rect::from_min_max(rect.min + egui::vec2(58.0 + gutter, 0.0), rect.max)
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

fn relative_time(seconds: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default();
    let delta = (now - seconds).max(0);
    if delta < 60 {
        "just now".into()
    } else if delta < 3_600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86_400 {
        format!("{}h ago", delta / 3_600)
    } else {
        format!("{}d ago", delta / 86_400)
    }
}

fn default_short(value: &str) -> &str {
    if value.is_empty() {
        "none"
    } else {
        value.get(..8).unwrap_or(value)
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
impl DiffView {
    pub fn ready(&self) -> bool {
        self.content_settled()
    }

    pub fn pending(&self) -> bool {
        self.content_pending()
    }

    pub fn has_text(&self, needle: &str) -> bool {
        self.rows.values().any(|row| row.text.contains(needle))
            || self
                .full
                .as_ref()
                .is_some_and(|full| full.lines.iter().any(|line| line.text.contains(needle)))
    }

    pub fn rows_len(&self) -> usize {
        if self.mode == Mode::Full {
            self.full.as_ref().map_or(0, |full| full.lines.len())
        } else {
            self.count
        }
    }
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
    fn a_new_selection_reveals_while_the_pointer_rests_on_the_reader() {
        let ctx = egui::Context::default();
        let mut view = DiffView::default();
        let selection = file("a", false, true);
        view.change = Some(selection);
        view.generation = 1;
        let mut doc = FileDiff::summary(DiffKind::Text, 0, 100, "");
        doc.rows = (0..20)
            .map(|i| DiffRow {
                kind: RowKind::Added,
                old_line: None,
                new_line: Some(i + 1),
                text: format!("row {i}"),
                bytes: vec![],
            })
            .collect();
        view.accept(diff_update(1, doc.clone()), None);
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
        let pointer = view.rect.center();
        frame(&mut view, vec![egui::Event::PointerMoved(pointer)]);
        view.generation = 2;
        view.last_scroll = f64::NEG_INFINITY;
        view.reveal = true;
        view.staged = Some(diff_update(2, doc));
        frame(&mut view, vec![]);
        assert_eq!(view.displayed_generation, 2);
        assert!(!view.reveal);
    }

    #[test]
    fn lfs_pointer_text_exposes_details_request() {
        let change = FileChange {
            path: "asset.bin".into(),
            path_bytes: b"asset.bin".to_vec(),
            kind: gg_engine::review::ChangeKind::Modified,
            old: None,
            new: Some(FileSide {
                id: "a".repeat(40),
                mode: 0o100644,
            }),
        };
        let mut doc = FileDiff::summary(DiffKind::Text, 1, 1, "");
        doc.rows.push(DiffRow {
            kind: RowKind::Context,
            old_line: Some(1),
            new_line: Some(1),
            text: "version https://git-lfs.github.com/spec/v1".into(),
            bytes: Vec::new(),
        });
        let mut view = DiffView {
            change: Some(change.clone()),
            info: Some(doc),
            ..Default::default()
        };
        assert!(view.is_lfs());
        view.lfs_request = Some(change);
        assert!(view.take_lfs_request().is_some());
    }

    #[test]
    fn gitlink_change_exposes_recorded_before_and_after_ids() {
        let side = FileSide {
            id: "a".repeat(40),
            mode: 0o160000,
        };
        let change = FileChange {
            path: "vendor/lib".into(),
            path_bytes: b"vendor/lib".to_vec(),
            kind: gg_engine::review::ChangeKind::Modified,
            old: Some(side.clone()),
            new: Some(FileSide {
                id: "b".repeat(40),
                mode: 0o160000,
            }),
        };
        let view = DiffView {
            change: Some(change.clone()),
            info: Some(FileDiff::summary(DiffKind::Gitlink, 0, 0, "")),
            ..Default::default()
        };
        let request = view.request_submodule(&change);
        assert!(request.is_some(), "gitlink request");
        if let Some(request) = request {
            assert_eq!(request.path, "vendor/lib");
            assert_eq!(request.before.as_deref(), Some("a".repeat(40).as_str()));
            assert_eq!(request.after.as_deref(), Some("b".repeat(40).as_str()));
        }
    }

    #[test]
    fn attribution_column_and_selected_line_metadata_render() {
        let ctx = egui::Context::default();
        let mut view = DiffView::default();
        let change = file("a.txt", true, true);
        view.displayed = Some(change);
        view.mode = Mode::Full;
        view.side = ReadingSide::After;
        view.displayed_mode = Mode::Full;
        view.displayed_side = ReadingSide::After;
        view.displayed_generation = 1;
        view.generation = 1;
        view.count = 2;
        view.full = Some(FileText {
            kind: gg_engine::file::FileKind::Text,
            side: ReadingSide::After,
            size: 8,
            complete: true,
            note: String::new(),
            lines: vec![
                gg_engine::file::FileLine {
                    number: 1,
                    text: "one".into(),
                    bytes: b"one\n".to_vec(),
                    changed: false,
                },
                gg_engine::file::FileLine {
                    number: 2,
                    text: "two".into(),
                    bytes: b"two\n".to_vec(),
                    changed: true,
                },
            ],
        });
        view.attribution = true;
        view.blame_revision = Some("HEAD".into());
        view.blame = Some(gg_engine::blame::Blame {
            revision: "HEAD".into(),
            path: "a.txt".into(),
            path_bytes: b"a.txt".to_vec(),
            start_line: 1,
            end_line: 2,
            lines: vec![
                gg_engine::blame::Line {
                    line: 1,
                    origin_line: 1,
                    commit: "a".repeat(40),
                    author: "Ada".into(),
                    email: "ada@example.org".into(),
                    time: 1,
                    summary: "first".into(),
                },
                gg_engine::blame::Line {
                    line: 2,
                    origin_line: 2,
                    commit: "b".repeat(40),
                    author: "Grace".into(),
                    email: "grace@example.org".into(),
                    time: 1,
                    summary: "second".into(),
                },
            ],
            complete: true,
            note: None,
        });
        view.cursor = Some(0);
        let output = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(720.0, 480.0, vec![]),
            |ui| view.draw(ui),
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            text.iter().any(|line| line.contains("Ada · aaaaaaaa")),
            "{text:?}"
        );
        assert!(
            text.iter().any(|line| line.contains("Grace · bbbbbbbb")),
            "{text:?}"
        );
        for expected in ["Ada", "ada@example.org", "first"] {
            assert!(
                text.iter().any(|line| line.contains(expected)),
                "missing {expected}: {text:?}"
            );
        }

        view.set_attribution_revision(None);
        assert!(!view.attribution_available());
        view.set_attribution_revision(Some("HEAD".into()));
        assert!(view.attribution_available());

        view.side = ReadingSide::Before;
        view.set_attribution_revisions(None, Some("parent".into()));
        assert!(view.attribution_available());

        view.side = ReadingSide::After;
        view.set_attribution_revisions(Some("HEAD".into()), None);
        view.attribution = false;
        view.change = None;
        view.toggle_attribution(&ctx);
        assert!(view.attribution_active());
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
            details_open: true,
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
                if text.galley.job.text.contains("Before")
                    && text.galley.job.text.contains("256 bytes")
                    && text.galley.job.text.contains("After")
                    && text.galley.job.text.contains("268 bytes"))
        }));
    }

    #[test]
    fn working_summaries_replace_a_misleading_no_changes_state() {
        use gg_engine::status::{Area, Kind, StatusEntry, StatusIdentity, WorkingRequest};
        let ctx = egui::Context::default();
        let draw = |view: &mut DiffView| {
            crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(720.0, 480.0, vec![]),
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        view.draw(ui);
                    });
                },
            )
        };
        let working = || WorkingRequest {
            repository: StatusIdentity {
                path: "/repo".into(),
                head: None,
                index: "index".into(),
                base_tree: None,
            },
            entry: StatusEntry::new(Area::Unstaged, Kind::Modified, b"a"),
        };
        let mut binary = DiffView {
            change: Some(file("a", true, true)),
            working: Some(working()),
            info: Some(FileDiff::summary(
                DiffKind::Binary,
                0,
                12,
                "Working snapshot · mode 100644 · Binary content",
            )),
            generation: 1,
            displayed_generation: 1,
            displayed_mode: Mode::Diff,
            ..Default::default()
        };
        let output = draw(&mut binary);
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.iter().any(|line| line.contains("Binary content")));
        assert!(!text.contains(&"No changes"));

        let mut mode = DiffView {
            change: Some(file("a", true, true)),
            working: Some(working()),
            info: Some(FileDiff {
                kind: DiffKind::Text,
                old_size: 1,
                new_size: 1,
                complete: true,
                note: "Working snapshot · mode 100755 · Mode 100644 to 100755".into(),
                rows: vec![],
            }),
            details_open: true,
            generation: 1,
            displayed_generation: 1,
            displayed_mode: Mode::Diff,
            ..Default::default()
        };
        let output = draw(&mut mode);
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            text.iter()
                .any(|line| line.contains("Mode 100644 to 100755"))
        );
        assert!(
            text.iter()
                .any(|line| line.contains("Working snapshot · mode 100755"))
        );
        assert!(!text.contains(&"No changes"));
    }

    #[test]
    fn staging_edits_follow_the_hunk_and_selected_lines() {
        use gg_engine::status::{Area, Kind, StatusEntry, StatusIdentity, WorkingRequest};
        let ctx = egui::Context::default();
        let mut view = DiffView {
            change: Some(file("a", true, true)),
            working: Some(WorkingRequest {
                repository: StatusIdentity {
                    path: "/repo".into(),
                    head: None,
                    index: "index".into(),
                    base_tree: None,
                },
                entry: StatusEntry::new(Area::Unstaged, Kind::Modified, b"a"),
            }),
            info: Some(FileDiff {
                kind: DiffKind::Text,
                old_size: 1,
                new_size: 1,
                complete: true,
                note: String::new(),
                rows: vec![],
            }),
            hunks: vec![0, 4],
            count: 7,
            stage: Some(StageConfig {
                verb: "Stage",
                can_discard: true,
                pending: false,
            }),
            ..Default::default()
        };
        let row = |kind, old_line, new_line| DiffRow {
            kind,
            old_line,
            new_line,
            text: String::new(),
            bytes: vec![],
        };
        for (index, row) in [
            row(RowKind::Hunk, None, None),
            row(RowKind::Deleted, Some(1), None),
            row(RowKind::Added, None, Some(1)),
            row(RowKind::Added, None, Some(2)),
            row(RowKind::Hunk, None, None),
            row(RowKind::Added, None, Some(3)),
        ]
        .into_iter()
        .enumerate()
        {
            view.rows.insert(index, row);
        }

        let whole = view.hunk_edit(0, false);
        assert_eq!(whole.scope, StageScope::Hunk);
        assert!(whole.changes.is_empty(), "the whole hunk is implicit");
        assert_eq!(whole.header.as_deref(), Some(""));

        view.stage_selected.insert(2, (RowKind::Added, 1));
        let selected = view.hunk_edit(0, false);
        assert_eq!(selected.scope, StageScope::Selected);
        assert_eq!(selected.changes, vec![(RowKind::Added, 1)]);

        let discard = view.hunk_edit(0, true);
        assert!(discard.discard);
        assert_eq!(
            discard.changes,
            vec![
                (RowKind::Deleted, 1),
                (RowKind::Added, 1),
                (RowKind::Added, 2)
            ],
            "a discard expands the touched change block"
        );

        view.stage_selected.clear();
        view.stage_selected.insert(5, (RowKind::Added, 3));
        let other = view.hunk_edit(4, false);
        assert_eq!(other.scope, StageScope::Selected);
        assert_eq!(other.changes, vec![(RowKind::Added, 3)]);
        assert!(!other.discard);

        let file = view.file_edit(true);
        assert_eq!(file.scope, StageScope::File);
        assert!(file.discard);
        assert!(file.changes.is_empty());

        view.generation = 2;
        view.displayed_generation = 1;
        assert!(
            !view.can_whole_edit() && !view.can_drop_whole() && !view.can_line_edit(),
            "actions wait for the displayed comparison to match the request"
        );
        view.displayed_generation = 2;
        assert!(view.can_whole_edit() && view.can_drop_whole() && view.can_line_edit());
        view.stage = Some(StageConfig {
            verb: "Stage",
            can_discard: true,
            pending: true,
        });
        assert!(
            !view.can_whole_edit() && !view.can_drop_whole(),
            "whole-file actions wait for the pending mutation"
        );
        let _ = ctx;
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
                if text.galley.job.text == "Before main (11111111) → After feature (22222222)"
        )));
    }

    #[test]
    fn restored_place_survives_a_settling_full_file_update() {
        let mut view = DiffView {
            change: Some(file("src/app.rs", true, true)),
            path: "repo".into(),
            generation: 1,
            mode: Mode::Full,
            side: ReadingSide::After,
            restoring: Some(reader::ReaderPlace {
                mode: Mode::Full,
                side: ReadingSide::After,
                cursor: Some(9),
                anchor: Some(12),
                offset: 240.0,
            }),
            ..Default::default()
        };
        let text = FileText::from_bytes(
            ReadingSide::After,
            b"01\n02\n03\n04\n05\n06\n07\n08\n09\n10\n11\n12\n13\n14\n15\n",
            &BTreeSet::new(),
        );
        view.accept(full_update(1, ReadingSide::After, text), None);
        assert_eq!(view.cursor, Some(9));
        assert_eq!(view.anchor, Some(12));
        assert_eq!(view.offset, 240.0);
        assert_eq!(view.jump, None);
    }

    #[test]
    fn open_context_keeps_place_when_the_identity_is_adopted() {
        let ctx = egui::Context::default();
        let mut view = DiffView {
            change: Some(file("a.rs", true, true)),
            path: "repo".into(),
            generation: 1,
            mode: Mode::Full,
            side: ReadingSide::After,
            ..Default::default()
        };
        let context = reader::ReaderContext::new(
            "repo".into(),
            file("b.rs", true, true),
            "old version",
            "new version",
            reader::ReaderPlace {
                mode: Mode::Full,
                side: ReadingSide::After,
                cursor: Some(40),
                anchor: Some(45),
                offset: 800.0,
            },
        );
        view.open_context(&context, &ctx);
        let text = FileText::from_bytes(
            ReadingSide::After,
            b"01\n02\n03\n04\n05\n06\n07\n08\n09\n10\n11\n12\n13\n14\n15\n16\n17\n18\n19\n20\n21\n22\n23\n24\n25\n26\n27\n28\n29\n30\n31\n32\n33\n34\n35\n36\n37\n38\n39\n40\n41\n42\n43\n44\n45\n46\n47\n48\n49\n50\n",
            &BTreeSet::new(),
        );
        let generation = view.generation;
        view.accept(full_update(generation, ReadingSide::After, text), None);
        assert_eq!(view.change.as_ref().map(|c| c.path.as_str()), Some("b.rs"));
        assert_eq!(view.cursor, Some(40));
        assert_eq!(view.anchor, Some(45));
        assert_eq!(view.offset, 800.0);
        assert_eq!(view.jump, None);
    }

    #[test]
    fn open_context_clears_full_content_from_the_other_side() {
        let ctx = egui::Context::default();
        let mut view = DiffView {
            change: Some(file("a.rs", true, true)),
            path: "repo".into(),
            mode: Mode::Full,
            side: ReadingSide::After,
            full: Some(FileText::from_bytes(
                ReadingSide::After,
                b"after\n",
                &BTreeSet::new(),
            )),
            ..Default::default()
        };
        let context = reader::ReaderContext::new(
            "repo".into(),
            file("a.rs", true, true),
            "main",
            "feature",
            reader::ReaderPlace {
                mode: Mode::Full,
                side: ReadingSide::Before,
                cursor: Some(0),
                anchor: Some(0),
                offset: 0.0,
            },
        );
        view.open_context(&context, &ctx);
        assert_eq!(view.side, ReadingSide::Before);
        assert!(view.full.is_none());
    }

    #[test]
    fn export_capabilities_follow_the_displayed_sides() {
        let mut view = DiffView {
            change: Some(file("src/app.rs", true, true)),
            displayed: Some(file("src/app.rs", false, true)),
            ..Default::default()
        };
        let capabilities = view.capabilities();
        assert!(!capabilities.export_before);
        assert!(capabilities.export_after);

        view.displayed = Some(file("src/app.rs", true, false));
        let capabilities = view.capabilities();
        assert!(capabilities.export_before);
        assert!(!capabilities.export_after);
    }

    #[test]
    fn pending_endpoint_labels_wait_for_matching_content() {
        let ctx = egui::Context::default();
        let mut view = DiffView {
            change: Some(file("src/app.rs", true, true)),
            displayed: Some(file("src/app.rs", true, true)),
            generation: 1,
            ..Default::default()
        };
        view.set_endpoints("older (aaaaaaaa)", "newer (bbbbbbbb)");
        let draw = |view: &mut DiffView| {
            crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(720.0, 480.0, vec![]),
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| view.draw(ui));
                },
            )
        };
        let output = draw(&mut view);
        assert!(!output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.job.text
                    == "Before older (aaaaaaaa) → After newer (bbbbbbbb)"
        )));
        view.displayed_generation = 1;
        let output = draw(&mut view);
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text)
                if text.galley.job.text
                    == "Before older (aaaaaaaa) → After newer (bbbbbbbb)"
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
