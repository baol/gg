//! Stage working paths or selected hunks/lines under the index lock.
use crate::patch::{self, Direction, Edit};
use gg_engine::{
    diff::{self, DiffKind, MAX_BLOB_BYTES},
    status::{Area, Kind, WorkingRequest},
};
use gix::bstr::BStr;
use std::io::Read;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn file(request: &WorkingRequest) -> std::result::Result<(), String> {
    apply_file(request).map_err(|e| format!("Stage {}: {e}", request.entry.path))
}
pub fn hunks(request: &WorkingRequest, selected: &[String]) -> std::result::Result<(), String> {
    changes(request, &patch::edits_from_headers(selected))
        .map_err(|e| format!("Stage hunks for {}: {e}", request.entry.path))
}
pub fn changes(request: &WorkingRequest, edits: &[Edit]) -> std::result::Result<(), String> {
    changes_with(request, edits, diff::Options::default())
}
/// Stage hunks and lines picked in a diff shown with `options`; the hunk
/// headers must come from a comparison with the same context.
pub fn changes_with(
    request: &WorkingRequest,
    edits: &[Edit],
    options: diff::Options,
) -> std::result::Result<(), String> {
    apply_changes(request, edits, options).map_err(|e| format!("Stage {}: {e}", request.entry.path))
}
fn apply_file(request: &WorkingRequest) -> Result<()> {
    match request.entry.area {
        Area::Unstaged | Area::Untracked => {}
        Area::Staged => return Err("Path is already staged".into()),
        Area::Conflict => return Err("Resolve this path's conflict before staging".into()),
    }
    let relative = patch::relative_path(request)?;
    let repo = gix::discover(&request.repository.path)?;
    let lock = patch::acquire(&repo)?;
    crate::status::validate(&request.repository)?;
    let mut index = (**repo.index_or_empty()?).clone();
    if index.is_sparse() {
        return Err("Stage requires an expanded index".into());
    }
    let path = BStr::new(&request.entry.path_bytes);
    patch::check_conflicts(&index, path)?;
    let previous = index.entry_by_path(path).map(|e| e.mode.bits() as u16);
    match request.entry.area {
        Area::Unstaged if previous.is_none() => {
            return Err("Path is no longer tracked. Refresh status.".into());
        }
        Area::Untracked if previous.is_some() => {
            return Err("Path is already tracked. Refresh status.".into());
        }
        _ => {}
    }
    if previous == Some(0o160000) && request.entry.kind != Kind::Deleted {
        return Err("Submodule changes: stage inside the submodule".into());
    }
    patch::check_collision(&index, &request.entry.path_bytes)?;
    let captured = crate::working_diff::capture(&request.repository.path, &relative, None)?;
    if request.entry.kind == Kind::Deleted {
        if captured.is_some() {
            return Err("Working path reappeared after the status scan. Refresh status.".into());
        }
        index.remove_entries(|_, p, _| p == path);
    } else {
        let captured = captured
            .as_ref()
            .ok_or("Working path is absent. Refresh status.")?;
        let capabilities = repo.filesystem_options()?;
        let mode = crate::working_diff::effective_mode(&capabilities, previous, captured.mode);
        let mut bytes = captured.bytes.clone();
        if mode != 0o120000 {
            let (mut pipeline, _) = repo.filter_pipeline(None)?;
            let mut normalized = vec![];
            pipeline
                .convert_to_git(std::io::Cursor::new(&bytes), &relative, &index)?
                .read_to_end(&mut normalized)?;
            bytes = normalized;
        }
        let id = repo.write_blob(&bytes)?.detach();
        let full = request.repository.path.join(&relative);
        let stat = gix::index::entry::Stat::from_fs(
            &gix::index::fs::Metadata::from_path_no_follow(&full)?,
        )?;
        index.remove_entries(|_, p, _| p == path);
        index.dangerously_push_entry(
            stat,
            id,
            gix::index::entry::Flags::empty(),
            gix::index::entry::Mode::from_bits_truncate(mode as u32),
            path,
        );
    }
    if crate::working_diff::capture(&request.repository.path, &relative, None)? != captured {
        return Err("Working path changed while staging. Refresh status.".into());
    }
    patch::publish(index, lock, request)
}
fn apply_changes(request: &WorkingRequest, edits: &[Edit], options: diff::Options) -> Result<()> {
    if edits.is_empty() {
        return Err("Select at least one hunk or line".into());
    }
    patch::check_exact(options)?;
    if request.entry.area != Area::Unstaged || request.entry.kind != Kind::Modified {
        return Err("Select an unstaged modification to stage changes".into());
    }
    let relative = patch::relative_path(request)?;
    let repo = gix::discover(&request.repository.path)?;
    let lock = patch::acquire(&repo)?;
    crate::status::validate(&request.repository)?;
    let mut index = (**repo.index_or_empty()?).clone();
    if index.is_sparse() {
        return Err("Partial staging requires an expanded index".into());
    }
    let path = BStr::new(&request.entry.path_bytes);
    patch::check_conflicts(&index, path)?;
    patch::check_collision(&index, &request.entry.path_bytes)?;
    let (previous_mode, previous_id) = index
        .entry_by_path(path)
        .filter(|entry| {
            !entry
                .flags
                .contains(gix::index::entry::Flags::INTENT_TO_ADD)
        })
        .map(|entry| (entry.mode.bits() as u16, entry.id))
        .ok_or("Path is no longer tracked. Refresh status.")?;
    if !matches!(previous_mode, 0o100644 | 0o100755) {
        return Err("Partial staging requires a regular text file".into());
    }
    let root = &request.repository.path;
    let captured = crate::working_diff::capture(root, &relative, Some(MAX_BLOB_BYTES as u64))?
        .ok_or("Working path is absent. Refresh status.")?;
    if captured.size > MAX_BLOB_BYTES as u64 {
        return Err("Partial staging is limited to 2 MiB text files".into());
    }
    let capabilities = repo.filesystem_options()?;
    let mode =
        crate::working_diff::effective_mode(&capabilities, Some(previous_mode), captured.mode);
    if mode != previous_mode {
        return Err("Stage the whole file to record mode or type changes".into());
    }
    let before = repo.find_blob(previous_id)?.data.clone();
    let (mut pipeline, _) = repo.filter_pipeline(None)?;
    let mut after = vec![];
    pipeline
        .convert_to_git(std::io::Cursor::new(&captured.bytes), &relative, &index)?
        .take((MAX_BLOB_BYTES + 1) as u64)
        .read_to_end(&mut after)?;
    let comparison = diff::compare_with(&before, &after, options, || false)?;
    if comparison.kind != DiffKind::Text {
        return Err("Partial staging requires a text comparison".into());
    }
    if !comparison.complete {
        return Err("Partial staging requires a complete comparison".into());
    }
    if !comparison.rows.iter().any(|row| {
        matches!(
            row.kind,
            gg_engine::diff::RowKind::Added | gg_engine::diff::RowKind::Deleted
        )
    }) {
        return Err("No changes to stage".into());
    }
    let (patched, all) = patch::reconstruct(&before, &comparison.rows, edits, Direction::Stage)?;
    if all && patched != after {
        return Err("Selected changes are stale. Refresh the diff.".into());
    }
    let id = repo.write_blob(&patched)?.detach();
    // Partial staging must leave stat data unset so status compares the remaining content.
    let stat = if all {
        gix::index::entry::Stat::from_fs(&gix::index::fs::Metadata::from_path_no_follow(
            &root.join(&relative),
        )?)?
    } else {
        gix::index::entry::Stat::default()
    };
    index.remove_entries(|_, p, _| p == path);
    index.dangerously_push_entry(
        stat,
        id,
        gix::index::entry::Flags::empty(),
        gix::index::entry::Mode::from_bits_truncate(mode as u32),
        path,
    );
    if crate::working_diff::capture(root, &relative, Some(MAX_BLOB_BYTES as u64))? != Some(captured)
    {
        return Err("Working path changed while staging. Refresh status.".into());
    }
    patch::publish(index, lock, request)
}
