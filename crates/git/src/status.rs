//! Callback-based status traversal: consumers control buffering and cancellation.
use gg_engine::status::{Area, Kind, StatusEntry, StatusIdentity};
use gix::{bstr::BStr, status::plumbing::index_as_worktree as iw};
use std::{
    collections::HashSet,
    ops::ControlFlow,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Session {
    repo: gix::Repository,
    pub identity: StatusIdentity,
}
impl Session {
    pub fn open(path: &Path) -> Result<Self, String> {
        Self::open_with_base_tree(path, None)
    }

    /// Open status with an explicit tree for the staged side.
    pub fn open_with_base_tree(path: &Path, base_tree: Option<String>) -> Result<Self, String> {
        let repo =
            gix::discover(path).map_err(|e| format!("Read status {}: {e}", path.display()))?;
        let workdir = repo
            .workdir()
            .ok_or_else(|| format!("Status requires a working tree: {}", path.display()))?
            .to_path_buf();
        if let Some(base) = &base_tree {
            repo.find_tree(
                gix::ObjectId::from_hex(base.as_bytes())
                    .map_err(|e| format!("Read amend base {base}: {e}"))?,
            )
            .map_err(|e| format!("Read amend base {base}: {e}"))?;
        }
        let head = repo
            .head()
            .map_err(|e| e.to_string())?
            .id()
            .map(|id| id.to_string());
        let index = index_fingerprint(&repo)?;
        Ok(Self {
            repo,
            identity: StatusIdentity {
                path: workdir,
                head,
                index,
                base_tree,
            },
        })
    }

    /// Open status against the amend base: the parent tree of HEAD, or the empty tree for a root commit.
    pub fn open_amend(path: &Path) -> Result<Self, String> {
        Self::open_with_base_tree(path, Some(amend_base_tree(path)?))
    }
    pub fn scan(
        &self,
        stop: &AtomicBool,
        mut emit: impl FnMut(StatusEntry) -> Result<(), String>,
    ) -> Result<(), String> {
        self.scan_inner(stop, &mut emit)
            .map_err(|e| format!("Read status {}: {e}", self.identity.path.display()))
    }
    fn scan_inner(
        &self,
        stop: &AtomicBool,
        emit: &mut dyn FnMut(StatusEntry) -> Result<(), String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let repo = &self.repo;
        validate(&self.identity)?;
        let (pipeline, index) = repo.filter_pipeline(None)?;
        let excluded: HashSet<Vec<u8>> = index
            .entries()
            .iter()
            .filter(|e| {
                e.stage() != gix::index::entry::Stage::Unconflicted
                    || e.flags.contains(gix::index::entry::Flags::INTENT_TO_ADD)
            })
            .map(|e| e.path(&index).to_vec())
            .collect();
        let tree = match (&self.identity.base_tree, &self.identity.head) {
            (Some(base), _) => gix::ObjectId::from_hex(base.as_bytes())?,
            (None, Some(head)) => repo
                .find_commit(gix::ObjectId::from_hex(head.as_bytes())?)?
                .tree_id()?
                .detach(),
            (None, None) => gix::ObjectId::empty_tree(repo.object_hash()),
        };
        let mut pathspec = repo.pathspec(
            false,
            None::<&str>,
            false,
            &index,
            gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
        )?;
        let mut error = None;
        repo.tree_index_status(
            &tree,
            &index,
            Some(&mut pathspec),
            gix::status::tree_index::TrackRenames::Disabled,
            |change, _, _| {
                if stop.load(Ordering::Relaxed) {
                    return Ok(ControlFlow::Break(()));
                }
                use gix::diff::index::ChangeRef;
                let kind = match &change {
                    ChangeRef::Addition { .. } => Kind::Added,
                    ChangeRef::Deletion { .. } => Kind::Deleted,
                    ChangeRef::Modification {
                        previous_entry_mode,
                        entry_mode,
                        ..
                    } if (previous_entry_mode.bits() & 0o170000)
                        != (entry_mode.bits() & 0o170000) =>
                    {
                        Kind::TypeChanged
                    }
                    _ => Kind::Modified,
                };
                if !excluded.contains::<[u8]>(change.location().as_ref())
                    && let Err(e) = emit(StatusEntry::new(Area::Staged, kind, change.location()))
                {
                    error = Some(e);
                    return Ok(ControlFlow::Break(()));
                }
                Ok(ControlFlow::Continue(()))
            },
        )?;
        if let Some(e) = error {
            return Err(e.into());
        }
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let (filter, stack) = pipeline.into_parts();
        let mut sink = Sink {
            emit,
            stop,
            error: None,
        };
        gix::status::plumbing::index_as_worktree(
            &index,
            &self.identity.path,
            &mut sink,
            iw::traits::FastEq,
            gix::status::index_worktree::BuiltinSubmoduleStatus::new(
                repo.clone().into_sync(),
                Default::default(),
            )?,
            repo.objects.clone().into_arc()?,
            &mut gix::progress::Discard,
            iw::Context {
                pathspec: pathspec.into_parts().0,
                stack,
                filter,
                should_interrupt: stop,
            },
            iw::Options {
                fs: repo.filesystem_options()?,
                thread_limit: Some(1),
                stat: repo.stat_options()?,
                fscache: false,
            },
        )?;
        if let Some(e) = sink.error.take() {
            return Err(e.into());
        }
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        repo.dirwalk(
            &index,
            None::<&BStr>,
            stop,
            repo.dirwalk_options()?
                .empty_patterns_match_prefix(false)
                .emit_untracked(gix::dir::walk::EmissionMode::Matching)
                .emit_ignored(None)
                .emit_empty_directories(false),
            &mut sink,
        )?;
        if let Some(e) = sink.error {
            return Err(e.into());
        }
        if !stop.load(Ordering::Relaxed) {
            validate(&self.identity)?;
        }
        Ok(())
    }
}
struct Sink<'a> {
    emit: &'a mut dyn FnMut(StatusEntry) -> Result<(), String>,
    stop: &'a AtomicBool,
    error: Option<String>,
}
impl Sink<'_> {
    fn push(&mut self, entry: StatusEntry) {
        if !self.stop.load(Ordering::Relaxed)
            && self.error.is_none()
            && let Err(e) = (self.emit)(entry)
        {
            self.error = Some(e);
            self.stop.store(true, Ordering::Relaxed);
        }
    }
}
impl<'index> iw::VisitEntry<'index> for Sink<'_> {
    type ContentChange = ();
    type SubmoduleStatus = gix::submodule::Status;
    fn visit_entry(
        &mut self,
        _: &'index [gix::index::Entry],
        _: &'index gix::index::Entry,
        _: usize,
        path: &'index BStr,
        status: iw::EntryStatus<(), Self::SubmoduleStatus>,
    ) {
        let (area, kind) = match status {
            iw::EntryStatus::NeedsUpdate(_) => return,
            iw::EntryStatus::Conflict { .. } => (Area::Conflict, Kind::Conflicted),
            iw::EntryStatus::IntentToAdd => (Area::Unstaged, Kind::IntentToAdd),
            iw::EntryStatus::Change(change) => (
                Area::Unstaged,
                match change {
                    iw::Change::Removed => Kind::Deleted,
                    iw::Change::Type { .. } => Kind::TypeChanged,
                    _ => Kind::Modified,
                },
            ),
        };
        self.push(StatusEntry::new(area, kind, path));
    }
}
impl gix::dir::walk::Delegate for Sink<'_> {
    fn emit(
        &mut self,
        entry: gix::dir::EntryRef<'_>,
        _: Option<gix::dir::entry::Status>,
    ) -> gix::dir::walk::Action {
        if entry.status == gix::dir::entry::Status::Untracked {
            self.push(StatusEntry::new(
                Area::Untracked,
                Kind::Added,
                &entry.rela_path,
            ));
        }
        if self.stop.load(Ordering::Relaxed) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

/// Return the tree an amended commit is compared against: HEAD's parent, or the empty tree.
pub fn amend_base_tree(path: &Path) -> Result<String, String> {
    let repo = gix::discover(path).map_err(|e| format!("Read status {}: {e}", path.display()))?;
    if repo.workdir().is_none() {
        return Err(format!("Amend requires a working tree: {}", path.display()));
    }
    let commit = repo.head_commit().map_err(|e| {
        format!("There is no commit to amend. Create the first commit instead: {e}")
    })?;
    let tree = match commit.parent_ids().next() {
        Some(parent) => repo
            .find_commit(parent.detach())
            .map_err(|e| format!("Read parent commit: {e}"))?
            .tree_id()
            .map_err(|e| format!("Read parent tree: {e}"))?
            .detach(),
        None => gix::ObjectId::empty_tree(repo.object_hash()),
    };
    Ok(tree.to_string())
}

fn index_fingerprint(repo: &gix::Repository) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = match std::fs::File::open(repo.index_path()) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok("absent".into()),
        Err(e) => return Err(e.to_string()),
    };
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let len = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if len == 0 {
            break;
        }
        hash.update(&buffer[..len]);
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
pub fn validate(identity: &StatusIdentity) -> Result<(), String> {
    let current = Session::open(&identity.path)?.identity;
    if current.head != identity.head || current.index != identity.index {
        return Err("Repository changed since this status scan. Refresh status.".into());
    }
    Ok(())
}
