//! Restore whole index paths or selected staged hunks/lines from pinned HEAD.
use crate::patch::{self, Direction, Edit};
use gg_engine::{
    diff::{self, DiffKind},
    status::{Area, WorkingRequest},
};
use gix::bstr::BStr;

pub fn file(request: &WorkingRequest) -> Result<(), String> {
    apply_file(request).map_err(|e| format!("Unstage {}: {e}", request.entry.path))
}
pub fn hunks(request: &WorkingRequest, selected: &[String]) -> Result<(), String> {
    changes(request, &patch::edits_from_headers(selected))
        .map_err(|e| format!("Unstage hunks for {}: {e}", request.entry.path))
}
pub fn changes(request: &WorkingRequest, edits: &[Edit]) -> Result<(), String> {
    apply_changes(request, edits).map_err(|e| format!("Unstage {}: {e}", request.entry.path))
}

fn apply_file(request: &WorkingRequest) -> Result<(), Box<dyn std::error::Error>> {
    if request.entry.area != Area::Staged {
        return Err("Select a staged file".into());
    }
    let relative = patch::relative_path(request)?;
    let repo = gix::discover(&request.repository.path)?;
    let lock = patch::acquire(&repo)?;
    crate::status::validate(&request.repository)?;
    let mut index = (**repo.index_or_empty()?).clone();
    if index.is_sparse() {
        return Err("Unstage requires an expanded index".into());
    }
    let path = BStr::new(&request.entry.path_bytes);
    patch::check_conflicts(&index, path)?;
    let before = head_side(request, &repo, &relative)?;
    if before.is_some() {
        patch::check_collision(&index, &request.entry.path_bytes)?;
    }
    index.remove_entries(|_, p, _| p == path);
    if let Some((id, mode)) = before {
        index.dangerously_push_entry(
            Default::default(),
            id,
            gix::index::entry::Flags::empty(),
            gix::index::entry::Mode::from_bits_truncate(mode as u32),
            path,
        );
    }
    patch::publish(index, lock, request)
}

fn apply_changes(
    request: &WorkingRequest,
    edits: &[Edit],
) -> Result<(), Box<dyn std::error::Error>> {
    if edits.is_empty() {
        return Err("Select at least one hunk or line".into());
    }
    if request.entry.area != Area::Staged {
        return Err("Select a staged file".into());
    }
    let relative = patch::relative_path(request)?;
    let repo = gix::discover(&request.repository.path)?;
    let lock = patch::acquire(&repo)?;
    crate::status::validate(&request.repository)?;
    let mut index = (**repo.index_or_empty()?).clone();
    if index.is_sparse() {
        return Err("Partial unstaging requires an expanded index".into());
    }
    let path = BStr::new(&request.entry.path_bytes);
    patch::check_conflicts(&index, path)?;
    let before = head_side(request, &repo, &relative)?;
    if before.is_some() {
        patch::check_collision(&index, &request.entry.path_bytes)?;
    }
    let index_entry = index.entry_by_path(path).filter(|entry| {
        !entry
            .flags
            .contains(gix::index::entry::Flags::INTENT_TO_ADD)
    });
    let after_mode = index_entry.map(|entry| entry.mode.bits() as u16);
    let before_mode = before.map(|(_, mode)| mode);
    if before_mode.is_some_and(|mode| !matches!(mode, 0o100644 | 0o100755))
        || after_mode.is_some_and(|mode| !matches!(mode, 0o100644 | 0o100755))
    {
        return Err("Partial unstaging requires a regular text file".into());
    }
    if let (Some(old), Some(new)) = (before_mode, after_mode)
        && old != new
    {
        return Err("Unstage the whole file to record mode or type changes".into());
    }
    let before_bytes = match before {
        Some((id, _)) => repo.find_blob(id)?.data.clone(),
        None => vec![],
    };
    let after_bytes = match index_entry {
        Some(entry) => repo.find_blob(entry.id)?.data.clone(),
        None => vec![],
    };
    let comparison = diff::compare(&before_bytes, &after_bytes, || false)?;
    if comparison.kind != DiffKind::Text {
        return Err("Partial unstaging requires a text comparison".into());
    }
    if !comparison.complete {
        return Err("Partial unstaging requires a complete comparison".into());
    }
    if !comparison.rows.iter().any(|row| {
        matches!(
            row.kind,
            gg_engine::diff::RowKind::Added | gg_engine::diff::RowKind::Deleted
        )
    }) {
        return Err("No staged changes to unstage".into());
    }
    let (patched, _) =
        patch::reconstruct(&before_bytes, &comparison.rows, edits, Direction::Unstage)?;
    let mode = before_mode
        .or(after_mode)
        .ok_or("No staged changes to unstage")?;
    index.remove_entries(|_, p, _| p == path);
    if let Some((id, _)) = before
        && patched == before_bytes
    {
        index.dangerously_push_entry(
            Default::default(),
            id,
            gix::index::entry::Flags::empty(),
            gix::index::entry::Mode::from_bits_truncate(mode as u32),
            path,
        );
    } else if before.is_none() && patched.is_empty() {
        // Removing the last staged addition leaves no index entry to restore.
    } else {
        let id = repo.write_blob(&patched)?.detach();
        index.dangerously_push_entry(
            Default::default(),
            id,
            gix::index::entry::Flags::empty(),
            gix::index::entry::Mode::from_bits_truncate(mode as u32),
            path,
        );
    }
    patch::publish(index, lock, request)
}

fn head_side(
    request: &WorkingRequest,
    repo: &gix::Repository,
    relative: &std::path::Path,
) -> Result<Option<(gix::ObjectId, u16)>, Box<dyn std::error::Error>> {
    let Some(head) = &request.repository.head else {
        return Ok(None);
    };
    let side = repo
        .find_commit(gix::ObjectId::from_hex(head.as_bytes())?)?
        .tree()?
        .lookup_entry_by_path(relative)?
        .map(|entry| (entry.id().detach(), entry.mode().value()));
    if side.is_some_and(|(_, mode)| mode == 0o040000) {
        return Err("Select an individual file; this path is a directory in HEAD".into());
    }
    Ok(side)
}
