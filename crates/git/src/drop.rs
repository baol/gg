//! Discard selected working-tree changes while leaving the index untouched.
use crate::patch::{self, Direction, Edit};
use gg_engine::{
    diff::{self, DiffKind, MAX_BLOB_BYTES},
    status::{Area, Kind, WorkingRequest},
};
use gix::bstr::BStr;
use std::{
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
static TEMP: AtomicU64 = AtomicU64::new(0);

pub fn file(request: &WorkingRequest) -> std::result::Result<(), String> {
    apply_file(request).map_err(|e| format!("Drop {}: {e}", request.entry.path))
}

pub fn changes(request: &WorkingRequest, edits: &[Edit]) -> std::result::Result<(), String> {
    apply_changes(request, edits).map_err(|e| format!("Drop {}: {e}", request.entry.path))
}

fn apply_file(request: &WorkingRequest) -> Result<()> {
    if request.entry.area == Area::Conflict {
        return Err("Resolve this path's conflict before dropping changes".into());
    }
    if request.entry.area == Area::Staged {
        return Err("Unstage the path before dropping working changes".into());
    }
    let relative = patch::relative_path(request)?;
    let repo = gix::discover(&request.repository.path)?;
    let lock = patch::acquire(&repo)?;
    crate::status::validate(&request.repository)?;
    let index = repo.index_or_empty()?;
    if index.is_sparse() {
        return Err("Dropping requires an expanded index".into());
    }
    let path = BStr::new(&request.entry.path_bytes);
    patch::check_conflicts(&index, path)?;
    let raw_entry = index.entry_by_path(path);
    let index_entry = raw_entry.filter(|entry| {
        !entry
            .flags
            .contains(gix::index::entry::Flags::INTENT_TO_ADD)
    });
    if request.entry.area == Area::Unstaged && raw_entry.is_some() && index_entry.is_none() {
        return Err("Drop is unavailable for intent-to-add paths".into());
    }
    let root = &request.repository.path;
    if request.entry.area == Area::Untracked {
        if index.entry_by_path(path).is_some() {
            return Err("Path is now tracked. Refresh status.".into());
        }
        let captured = crate::working_diff::capture(root, &relative, None)?
            .ok_or("Working path is absent. Refresh status.")?;
        if crate::working_diff::capture(root, &relative, None)? != Some(captured) {
            return Err("Working path changed while dropping. Refresh status.".into());
        }
        std::fs::remove_file(root.join(&relative))?;
        drop(lock);
        return Ok(());
    }
    let entry = index_entry.ok_or("Path is no longer tracked. Refresh status.")?;
    let mode = entry.mode.bits() as u16;
    if mode == 0o160000 {
        return Err("Drop submodule changes inside the submodule".into());
    }
    if !matches!(mode, 0o100644 | 0o100755 | 0o120000) {
        return Err("Dropping this path requires a regular file or symlink".into());
    }
    let captured = crate::working_diff::capture(root, &relative, None)?;
    let bytes = repo.find_blob(entry.id)?.data.clone();
    let bytes = to_worktree(&repo, &relative, &bytes, mode)?;
    if crate::working_diff::capture(root, &relative, None)? != captured {
        return Err("Working path changed while dropping. Refresh status.".into());
    }
    write_path(root, &relative, &bytes, mode)?;
    drop(lock);
    Ok(())
}

fn apply_changes(request: &WorkingRequest, edits: &[Edit]) -> Result<()> {
    if edits.is_empty() {
        return Err("Select at least one hunk or line".into());
    }
    if request.entry.area != Area::Unstaged || request.entry.kind != Kind::Modified {
        return Err("Select an unstaged modification to drop changes".into());
    }
    let relative = patch::relative_path(request)?;
    let repo = gix::discover(&request.repository.path)?;
    let lock = patch::acquire(&repo)?;
    crate::status::validate(&request.repository)?;
    let index = repo.index_or_empty()?;
    if index.is_sparse() {
        return Err("Partial dropping requires an expanded index".into());
    }
    let path = BStr::new(&request.entry.path_bytes);
    patch::check_conflicts(&index, path)?;
    let entry = index
        .entry_by_path(path)
        .filter(|entry| {
            !entry
                .flags
                .contains(gix::index::entry::Flags::INTENT_TO_ADD)
        })
        .ok_or("Path is no longer tracked. Refresh status.")?;
    let mode = entry.mode.bits() as u16;
    if !matches!(mode, 0o100644 | 0o100755) {
        return Err("Partial dropping requires a regular text file".into());
    }
    let root = &request.repository.path;
    let captured = crate::working_diff::capture(root, &relative, Some(MAX_BLOB_BYTES as u64))?
        .ok_or("Working path is absent. Refresh status.")?;
    if captured.size > MAX_BLOB_BYTES as u64 {
        return Err("Partial dropping is limited to 2 MiB text files".into());
    }
    let capabilities = repo.filesystem_options()?;
    let effective = crate::working_diff::effective_mode(&capabilities, Some(mode), captured.mode);
    if effective != mode {
        return Err("Drop the whole file to discard mode or type changes".into());
    }
    let before = repo.find_blob(entry.id)?.data.clone();
    let (mut pipeline, _) = repo.filter_pipeline(None)?;
    let mut after = vec![];
    pipeline
        .convert_to_git(std::io::Cursor::new(&captured.bytes), &relative, &index)?
        .take((MAX_BLOB_BYTES + 1) as u64)
        .read_to_end(&mut after)?;
    let comparison = diff::compare(&before, &after, || false)?;
    if comparison.kind != DiffKind::Text {
        return Err("Partial dropping requires a text comparison".into());
    }
    if !comparison.complete {
        return Err("Partial dropping requires a complete comparison".into());
    }
    if !comparison.rows.iter().any(|row| {
        matches!(
            row.kind,
            gg_engine::diff::RowKind::Added | gg_engine::diff::RowKind::Deleted
        )
    }) {
        return Err("No working changes to drop".into());
    }
    let (patched, _) = patch::reconstruct(&before, &comparison.rows, edits, Direction::Unstage)?;
    let bytes = to_worktree(&repo, &relative, &patched, mode)?;
    let working_mode = captured.mode;
    if crate::working_diff::capture(root, &relative, Some(MAX_BLOB_BYTES as u64))? != Some(captured)
    {
        return Err("Working path changed while dropping. Refresh status.".into());
    }
    write_path(root, &relative, &bytes, working_mode)?;
    drop(lock);
    Ok(())
}

fn to_worktree(
    repo: &gix::Repository,
    relative: &Path,
    bytes: &[u8],
    mode: u16,
) -> Result<Vec<u8>> {
    if mode == 0o120000 {
        return Ok(bytes.to_vec());
    }
    let (mut pipeline, _) = repo.filter_pipeline(None)?;
    let mut outcome = pipeline.convert_to_worktree(
        bytes,
        gix::path::into_bstr(relative)?.as_ref(),
        gix::filter::plumbing::pipeline::convert::to_worktree::Options {
            can_delay: gix::filter::plumbing::driver::apply::Delay::Forbid,
            unknown_encoding:
                gix::filter::plumbing::pipeline::convert::to_worktree::UnknownEncoding::Fail,
        },
    )?;
    let mut out = vec![];
    outcome.read_to_end(&mut out)?;
    Ok(out)
}

fn write_path(root: &Path, relative: &Path, bytes: &[u8], mode: u16) -> Result<()> {
    let full = root.join(relative);
    let parent = full
        .parent()
        .filter(|parent| parent.is_dir())
        .ok_or("Parent directory is missing. Refresh status.")?;
    match mode {
        0o100644 | 0o100755 => {
            let temp = parent.join(format!(
                ".gg-drop-{}-{}",
                std::process::id(),
                TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            let result = (|| -> Result<()> {
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temp)?;
                file.write_all(bytes)?;
                file.sync_all()?;
                drop(file);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(
                        &temp,
                        std::fs::Permissions::from_mode(mode as u32 & 0o777),
                    )?;
                }
                replace(&temp, &full)?;
                Ok(())
            })();
            if result.is_err() {
                let _ = std::fs::remove_file(&temp);
            }
            result
        }
        #[cfg(unix)]
        0o120000 => {
            use std::os::unix::ffi::OsStrExt;
            let temp = parent.join(format!(
                ".gg-drop-{}-{}",
                std::process::id(),
                TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            let result = std::os::unix::fs::symlink(std::ffi::OsStr::from_bytes(bytes), &temp)
                .map_err(Into::into)
                .and_then(|()| replace(&temp, &full));
            if result.is_err() {
                let _ = std::fs::remove_file(&temp);
            }
            result
        }
        #[cfg(not(unix))]
        0o120000 => Err("Dropping symlinks is unsupported on this platform".into()),
        _ => Err("Dropping this path requires a regular file or symlink".into()),
    }
}

#[cfg(unix)]
fn replace(temp: &Path, full: &Path) -> Result<()> {
    std::fs::rename(temp, full)?;
    Ok(())
}

#[cfg(not(unix))]
fn replace(temp: &Path, full: &Path) -> Result<()> {
    if full.exists() {
        std::fs::remove_file(full)?;
    }
    std::fs::rename(temp, full)?;
    Ok(())
}
