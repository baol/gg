//! Validated staging mutations behind the shared Local Changes reader.
//!
//! Loading and rendering belong to [`crate::diff_view::DiffView`]. This module
//! owns only the selected working comparison, the mutation worker, and the
//! conversion from reader row edits to the Git staging or discard operation.
use crate::diff_view::{StageEdit, StageScope};
use eframe::egui;
use gg_engine::status::{Area, StatusEntry, StatusIdentity, WorkingRequest};
use gg_git::patch::{Change, Edit};
use std::{
    sync::mpsc::{Receiver, TryRecvError},
    thread,
};

#[derive(Default)]
pub struct StageActions {
    request: Option<WorkingRequest>,
    mutation: Option<Receiver<Result<StatusIdentity, String>>>,
    label: Option<&'static str>,
    error: Option<String>,
    changed: Option<StatusIdentity>,
}

impl StageActions {
    /// Point the mutation engine at the comparison selected in the workspace.
    pub fn select(&mut self, repository: StatusIdentity, entry: StatusEntry) {
        self.request = Some(WorkingRequest { repository, entry });
        self.mutation = None;
        self.label = None;
        self.error = None;
        self.changed = None;
    }

    /// Leave the active comparison without requesting anything.
    pub fn clear(&mut self) {
        self.request = None;
        self.mutation = None;
        self.label = None;
        self.error = None;
        self.changed = None;
    }

    /// A mutation publication is pending.
    pub fn mutating(&self) -> bool {
        self.mutation.is_some()
    }

    pub fn label(&self) -> Option<&'static str> {
        self.label
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Take the repository identity published by the last successful mutation.
    pub fn take_changed(&mut self) -> Option<StatusIdentity> {
        self.changed.take()
    }

    pub fn poll(&mut self) {
        let result = self.mutation.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Disconnected) => {
                Some(Err("Mutation worker stopped. Refresh status.".into()))
            }
            Err(TryRecvError::Empty) => None,
        });
        if let Some(result) = result {
            self.mutation = None;
            self.label = None;
            match result {
                Ok(identity) => {
                    if let Some(request) = &mut self.request {
                        request.repository = identity.clone();
                    }
                    self.changed = Some(identity);
                    self.error = None;
                }
                Err(error) => self.error = Some(error),
            }
        }
    }

    /// Apply one reader edit. Duplicate submissions are refused while pending.
    pub fn apply(&mut self, edit: StageEdit, ctx: &egui::Context) {
        self.apply_all(vec![edit], ctx);
    }

    /// Apply reader edits as one mutation: one whole-file edit, or hunk and
    /// line edits across several hunks. All edits share one direction.
    pub fn apply_all(&mut self, edits: Vec<StageEdit>, ctx: &egui::Context) {
        if self.mutation.is_some() {
            return;
        }
        let Some(first) = edits.first() else {
            return;
        };
        let discard = first.discard;
        let whole = first.scope == StageScope::File;
        if edits
            .iter()
            .any(|edit| edit.discard != discard || (edit.scope == StageScope::File) != whole)
            || (whole && edits.len() > 1)
        {
            self.error = Some("Apply one kind of change at a time.".into());
            return;
        }
        self.error = None;
        let Some(request) = self.request.clone() else {
            return;
        };
        let area = request.entry.area;
        if area == Area::Conflict {
            self.error = Some("Resolve the conflict before staging or discarding.".into());
            return;
        }
        if discard && area == Area::Staged {
            self.error = Some("Discard is only available for working changes.".into());
            return;
        }
        let patch = if whole {
            None
        } else {
            let mut patch = Vec::with_capacity(edits.len());
            for edit in &edits {
                let Some(header) = edit.header.clone() else {
                    self.error =
                        Some("The selected hunk is no longer loaded. Refresh the diff.".into());
                    return;
                };
                let changes = edit
                    .changes
                    .iter()
                    .filter_map(|(kind, line)| match kind {
                        gg_engine::diff::RowKind::Added => Some(Change::Added(*line)),
                        gg_engine::diff::RowKind::Deleted => Some(Change::Deleted(*line)),
                        _ => None,
                    })
                    .collect();
                patch.push(Edit { header, changes });
            }
            Some(patch)
        };
        self.label = Some(if discard {
            "Discarding…"
        } else if area == Area::Staged {
            "Unstaging…"
        } else {
            "Staging…"
        });
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let repaint = ctx.clone();
        let result = thread::Builder::new()
            .name("gg-stage-mutate".into())
            .spawn(move || {
                let operation = match (area, discard, patch) {
                    (Area::Staged, true, _) => {
                        Err("Discard is only available for working changes".into())
                    }
                    (Area::Staged, false, None) => gg_git::unstage::file(&request),
                    (Area::Staged, false, Some(edits)) => {
                        gg_git::unstage::changes(&request, &edits)
                    }
                    (Area::Unstaged | Area::Untracked, true, None) => gg_git::drop::file(&request),
                    (Area::Unstaged | Area::Untracked, true, Some(edits)) => {
                        gg_git::drop::changes(&request, &edits)
                    }
                    (Area::Unstaged | Area::Untracked, false, None) => {
                        gg_git::stage::file(&request)
                    }
                    (Area::Unstaged | Area::Untracked, false, Some(edits)) => {
                        gg_git::stage::changes(&request, &edits)
                    }
                    (Area::Conflict, _, _) => Err("Resolve the conflict before staging.".into()),
                };
                let result = operation.and_then(|()| {
                    gg_git::status::Session::open_with_base_tree(
                        &request.repository.path,
                        request.repository.base_tree.clone(),
                    )
                    .map(|session| session.identity)
                    .map_err(|error| format!("Refresh diff: {error}"))
                });
                let _ = tx.send(result);
                repaint.request_repaint();
            });
        match result {
            Ok(_) => self.mutation = Some(rx),
            Err(error) => {
                self.label = None;
                self.error = Some(format!("Start mutation: {error}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(area: Area) -> WorkingRequest {
        WorkingRequest {
            repository: StatusIdentity {
                path: "/nonexistent".into(),
                head: None,
                index: String::new(),
                base_tree: None,
            },
            entry: StatusEntry::new(area, gg_engine::status::Kind::Modified, b"f.txt"),
        }
    }

    #[test]
    fn conflicts_and_staged_discards_are_refused_before_spawning() {
        let ctx = egui::Context::default();
        let mut actions = StageActions {
            request: Some(request(Area::Conflict)),
            ..StageActions::default()
        };
        actions.apply(
            StageEdit {
                scope: StageScope::File,
                discard: false,
                header: None,
                changes: vec![],
            },
            &ctx,
        );
        assert!(!actions.mutating());
        assert!(
            actions
                .error()
                .is_some_and(|error| error.contains("conflict"))
        );

        let mut actions = StageActions {
            request: Some(request(Area::Staged)),
            ..StageActions::default()
        };
        actions.apply(
            StageEdit {
                scope: StageScope::File,
                discard: true,
                header: None,
                changes: vec![],
            },
            &ctx,
        );
        assert!(!actions.mutating());
        assert!(
            actions
                .error()
                .is_some_and(|error| error.contains("Discard"))
        );
    }

    #[test]
    fn a_hunk_edit_requires_its_header() {
        let ctx = egui::Context::default();
        let mut actions = StageActions {
            request: Some(request(Area::Unstaged)),
            ..StageActions::default()
        };
        actions.apply(
            StageEdit {
                scope: StageScope::Hunk,
                discard: false,
                header: None,
                changes: vec![],
            },
            &ctx,
        );
        assert!(!actions.mutating());
        assert!(
            actions
                .error()
                .is_some_and(|error| error.contains("Refresh"))
        );
    }
}
