//! Immutable blob access for previews and explicit export.
use gg_engine::review::FileSide;
use std::{io::Write, path::Path};

fn check(cancelled: &impl Fn() -> bool) -> Result<(), String> {
    if cancelled() {
        Err("Operation cancelled".into())
    } else {
        Ok(())
    }
}
/// Encoded size of one blob side, without reading its content.
pub fn size(path: &Path, side: &FileSide) -> Result<u64, String> {
    if side.mode == 0o160000 {
        return Ok(0);
    }
    let repo = gix::discover(path).map_err(|e| e.to_string())?;
    let id = gix::ObjectId::from_hex(side.id.as_bytes()).map_err(|e| e.to_string())?;
    Ok(repo.find_header(id).map_err(|e| e.to_string())?.size())
}

/// Preview reads have an explicit encoded-byte budget. None means summary/export only.
pub fn preview(
    path: &Path,
    side: &FileSide,
    limit: u64,
    cancelled: impl Fn() -> bool,
) -> Result<Option<Vec<u8>>, String> {
    check(&cancelled)?;
    if side.mode == 0o160000 {
        return Ok(None);
    }
    let repo = gix::discover(path).map_err(|e| e.to_string())?;
    let id = gix::ObjectId::from_hex(side.id.as_bytes()).map_err(|e| e.to_string())?;
    if repo.find_header(id).map_err(|e| e.to_string())?.size() > limit {
        return Ok(None);
    }
    check(&cancelled)?;
    let blob = repo.find_blob(id).map_err(|e| e.to_string())?;
    check(&cancelled)?;
    Ok(Some(blob.data.clone()))
}
/// Decode one selected object, then write exact bytes in cancellable 64-KiB chunks.
pub fn export(
    path: &Path,
    side: &FileSide,
    out: &mut impl Write,
    cancelled: impl Fn() -> bool,
) -> Result<u64, String> {
    check(&cancelled)?;
    if side.mode == 0o160000 {
        return Err("Gitlinks reference commits; select a blob to export".into());
    }
    let repo = gix::discover(path).map_err(|e| e.to_string())?;
    let id = gix::ObjectId::from_hex(side.id.as_bytes()).map_err(|e| e.to_string())?;
    check(&cancelled)?;
    let blob = repo.find_blob(id).map_err(|e| e.to_string())?;
    for chunk in blob.data.chunks(64 * 1024) {
        check(&cancelled)?;
        out.write_all(chunk).map_err(|e| e.to_string())?;
    }
    check(&cancelled)?;
    out.flush().map_err(|e| e.to_string())?;
    Ok(blob.data.len() as u64)
}

/// Keep repository metadata outside explicit export destinations, including linked worktrees.
pub fn check_destination(path: &Path, destination: &Path) -> Result<(), String> {
    let repo = gix::discover(path).map_err(|e| e.to_string())?;
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent.canonicalize().map_err(|e| e.to_string())?;
    for directory in [repo.git_dir(), repo.common_dir()] {
        let directory = directory.canonicalize().map_err(|e| e.to_string())?;
        if parent.starts_with(directory) {
            return Err("Choose an export destination outside Git metadata".into());
        }
    }
    Ok(())
}
