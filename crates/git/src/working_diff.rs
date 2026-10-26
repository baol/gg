//! Capture and validate selected working comparisons without writing Git objects.
use gg_engine::{
    diff::{self, DiffKind, FileDiff, MAX_BLOB_BYTES},
    file::{FileKind, FileText, ReadingSide},
    review::{ChangeKind, FileChange, FileSide},
    status::{Area, WorkingRequest},
};
use std::{
    collections::BTreeSet,
    io::Read,
    path::{Component, Path},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn read(
    request: &WorkingRequest,
    cancelled: impl Fn() -> bool,
) -> std::result::Result<FileDiff, String> {
    read_inner(request, &cancelled).map_err(|e| format!("Compare {}: {e}", request.entry.path))
}
fn read_inner(request: &WorkingRequest, cancelled: &dyn Fn() -> bool) -> Result<FileDiff> {
    let check = || -> Result<()> {
        if cancelled() {
            Err("Comparison cancelled".into())
        } else {
            Ok(())
        }
    };
    check()?;
    crate::status::validate(&request.repository)?;
    let repo = gix::discover(&request.repository.path)?;
    let relative = gix::path::from_bstr(gix::bstr::BStr::new(&request.entry.path_bytes))?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("Use a repository-relative file path".into());
    }
    let index = repo.index_or_empty()?;
    let index_entry = index.entry_by_path(gix::bstr::BStr::new(&request.entry.path_bytes));
    let index_side = index_entry
        .filter(|e| !e.flags.contains(gix::index::entry::Flags::INTENT_TO_ADD))
        .map(|e| FileSide {
            id: e.id.to_string(),
            mode: e.mode.bits() as u16,
        });
    if request.entry.area == Area::Conflict {
        return Ok(FileDiff::summary(
            DiffKind::Metadata,
            0,
            0,
            "Unmerged path: resolve the conflict before staging",
        ));
    }
    if request.entry.area == Area::Staged {
        let before = if let Some(base) = &request.repository.base_tree {
            let tree = repo.find_tree(gix::ObjectId::from_hex(base.as_bytes())?)?;
            tree.lookup_entry_by_path(&relative)?.map(|e| FileSide {
                id: e.id().to_string(),
                mode: e.mode().value(),
            })
        } else if let Some(head) = &request.repository.head {
            let tree = repo
                .find_commit(gix::ObjectId::from_hex(head.as_bytes())?)?
                .tree()?;
            tree.lookup_entry_by_path(&relative)?.map(|e| FileSide {
                id: e.id().to_string(),
                mode: e.mode().value(),
            })
        } else {
            None
        };
        let change = FileChange {
            path: request.entry.path.clone(),
            path_bytes: request.entry.path_bytes.clone(),
            kind: ChangeKind::Modified,
            old: before,
            new: index_side,
        };
        let result = crate::diff::read(&request.repository.path, &change, cancelled)?;
        check()?;
        crate::status::validate(&request.repository)?;
        return Ok(result);
    }
    if index_side.as_ref().is_some_and(|s| s.mode == 0o160000) {
        return Ok(FileDiff::summary(
            DiffKind::Gitlink,
            0,
            0,
            "Submodule working changes: inspect the submodule repository",
        ));
    }
    let full = request.repository.path.join(&relative);
    let captured = capture(
        &request.repository.path,
        &relative,
        Some(MAX_BLOB_BYTES as u64),
    )?;
    let fs = repo.filesystem_options()?;
    let effective_mode = captured
        .as_ref()
        .map(|new| effective_mode(&fs, index_side.as_ref().map(|side| side.mode), new.mode));
    check()?;
    let old_size = match &index_side {
        Some(side) if request.entry.area != Area::Untracked => repo
            .find_header(gix::ObjectId::from_hex(side.id.as_bytes())?)?
            .size(),
        _ => 0,
    };
    let note = match &captured {
        Some(_) => format!(
            "Working snapshot · mode {:06o}",
            effective_mode.unwrap_or(0)
        ),
        None => "Working file absent".into(),
    };
    let new_size = captured.as_ref().map_or(0, |c| c.size);
    let mut result = if old_size > MAX_BLOB_BYTES as u64 || new_size > MAX_BLOB_BYTES as u64 {
        FileDiff::summary(
            DiffKind::Limited,
            old_size,
            new_size,
            "Text budget: 2 MiB per side",
        )
    } else {
        let before = match &index_side {
            Some(side) if request.entry.area != Area::Untracked => repo
                .find_blob(gix::ObjectId::from_hex(side.id.as_bytes())?)?
                .data
                .clone(),
            _ => vec![],
        };
        let mut after = captured.as_ref().map_or_else(Vec::new, |c| c.bytes.clone());
        if effective_mode.is_some_and(|mode| mode != 0o120000) {
            let (mut pipeline, _) = repo.filter_pipeline(None)?;
            let mut normalized = vec![];
            pipeline
                .convert_to_git(std::io::Cursor::new(&after), &relative, &index)?
                .take((MAX_BLOB_BYTES + 1) as u64)
                .read_to_end(&mut normalized)?;
            after = normalized;
        }
        check()?;
        let mut diff = diff::compare(&before, &after, cancelled)?;
        if let (Some(old), Some(new_mode)) = (&index_side, effective_mode)
            && old.mode != new_mode
        {
            diff.note = format!(
                "Mode {:06o} to {:06o}{}{}",
                old.mode,
                new_mode,
                if diff.note.is_empty() { "" } else { " · " },
                diff.note
            );
        }
        diff
    };
    check()?;
    if capture(
        &request.repository.path,
        &relative,
        Some(MAX_BLOB_BYTES as u64),
    )? != captured
    {
        return Err(format!(
            "Working file changed while reading {}. Refresh status.",
            full.display()
        )
        .into());
    }
    crate::status::validate(&request.repository)?;
    result.note = format!(
        "{note}{}{}",
        if result.note.is_empty() { "" } else { " · " },
        result.note
    );
    Ok(result)
}

/// Read one full-file side of the selected working comparison.
///
/// Staged reads the base tree and the index; unstaged and untracked reads the
/// index and the normalized working snapshot. Conflict entries have no
/// meaningful single side and return an explicit summary.
pub fn read_side(
    request: &WorkingRequest,
    side: ReadingSide,
    changed: &BTreeSet<usize>,
    cancelled: impl Fn() -> bool,
) -> std::result::Result<FileText, String> {
    read_side_inner(request, side, changed, &cancelled).map_err(|error| {
        format!(
            "Read {} of {}: {error}",
            side.label().to_lowercase(),
            request.entry.path
        )
    })
}

fn read_side_inner(
    request: &WorkingRequest,
    side: ReadingSide,
    changed: &BTreeSet<usize>,
    cancelled: &dyn Fn() -> bool,
) -> Result<FileText> {
    let check = || -> Result<()> {
        if cancelled() {
            Err("Read cancelled".into())
        } else {
            Ok(())
        }
    };
    check()?;
    crate::status::validate(&request.repository)?;
    let repo = gix::discover(&request.repository.path)?;
    let relative = gix::path::from_bstr(gix::bstr::BStr::new(&request.entry.path_bytes))?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("Use a repository-relative file path".into());
    }
    if request.entry.area == Area::Conflict {
        return Ok(FileText::missing(
            side,
            "Unmerged path: choosing a side is read-only; stage the resolution",
        ));
    }
    let index = repo.index_or_empty()?;
    let index_side = index
        .entry_by_path(gix::bstr::BStr::new(&request.entry.path_bytes))
        .filter(|entry| {
            !entry
                .flags
                .contains(gix::index::entry::Flags::INTENT_TO_ADD)
        })
        .map(|entry| FileSide {
            id: entry.id.to_string(),
            mode: entry.mode.bits() as u16,
        });
    let side = match (request.entry.area, side) {
        (Area::Staged, ReadingSide::Before) => {
            read_tree_side(request, &repo, &relative, side, changed, cancelled)?
        }
        (Area::Staged, ReadingSide::After) => {
            read_git_side(&repo, index_side.as_ref(), side, changed, cancelled)?
        }
        (Area::Untracked, ReadingSide::Before) => {
            FileText::missing(side, "Untracked: no earlier content")
        }
        (_, ReadingSide::Before) => {
            read_git_side(&repo, index_side.as_ref(), side, changed, cancelled)?
        }
        (_, ReadingSide::After) => read_working_side(
            request,
            &repo,
            &relative,
            index_side.as_ref(),
            side,
            changed,
            cancelled,
        )?,
    };
    check()?;
    crate::status::validate(&request.repository)?;
    Ok(side)
}

fn read_tree_side(
    request: &WorkingRequest,
    repo: &gix::Repository,
    relative: &Path,
    side: ReadingSide,
    changed: &BTreeSet<usize>,
    cancelled: &dyn Fn() -> bool,
) -> Result<FileText> {
    let before = if let Some(base) = &request.repository.base_tree {
        let tree = repo.find_tree(gix::ObjectId::from_hex(base.as_bytes())?)?;
        tree.lookup_entry_by_path(relative)?.map(|entry| FileSide {
            id: entry.id().to_string(),
            mode: entry.mode().value(),
        })
    } else if let Some(head) = &request.repository.head {
        let tree = repo
            .find_commit(gix::ObjectId::from_hex(head.as_bytes())?)?
            .tree()?;
        tree.lookup_entry_by_path(relative)?.map(|entry| FileSide {
            id: entry.id().to_string(),
            mode: entry.mode().value(),
        })
    } else {
        None
    };
    read_git_side(repo, before.as_ref(), side, changed, cancelled)
}

fn read_git_side(
    repo: &gix::Repository,
    file: Option<&FileSide>,
    side: ReadingSide,
    changed: &BTreeSet<usize>,
    cancelled: &dyn Fn() -> bool,
) -> Result<FileText> {
    let Some(file) = file else {
        return Ok(FileText::missing(side, "No content on this side"));
    };
    if file.mode == 0o160000 {
        return Ok(FileText::summary(
            FileKind::Gitlink,
            side,
            0,
            "Gitlink: compare the referenced commit ID",
        ));
    }
    if cancelled() {
        return Err("Read cancelled".into());
    }
    let id = gix::ObjectId::from_hex(file.id.as_bytes())?;
    let size = repo.find_header(id)?.size();
    if size > MAX_BLOB_BYTES as u64 {
        return Ok(FileText::summary(
            FileKind::Limited,
            side,
            size,
            "Text budget: 2 MiB per file",
        ));
    }
    if cancelled() {
        return Err("Read cancelled".into());
    }
    let blob = repo.find_blob(id)?;
    if cancelled() {
        return Err("Read cancelled".into());
    }
    let mut text = FileText::from_bytes(side, &blob.data, changed);
    if file.mode == 0o120000 && text.note.is_empty() {
        text.note = "Symlink target".into();
    }
    Ok(text)
}

fn read_working_side(
    request: &WorkingRequest,
    repo: &gix::Repository,
    relative: &Path,
    index_side: Option<&FileSide>,
    side: ReadingSide,
    changed: &BTreeSet<usize>,
    cancelled: &dyn Fn() -> bool,
) -> Result<FileText> {
    if index_side.is_some_and(|side| side.mode == 0o160000) {
        return Ok(FileText::summary(
            FileKind::Gitlink,
            side,
            0,
            "Submodule working changes: inspect the submodule repository",
        ));
    }
    let captured = capture(
        &request.repository.path,
        relative,
        Some(MAX_BLOB_BYTES as u64),
    )?;
    let Some(captured) = captured else {
        return Ok(FileText::missing(side, "Working file absent"));
    };
    if captured.size > MAX_BLOB_BYTES as u64 {
        return Ok(FileText::summary(
            FileKind::Limited,
            side,
            captured.size,
            "Text budget: 2 MiB per side",
        ));
    }
    if cancelled() {
        return Err("Read cancelled".into());
    }
    let filesystem = repo.filesystem_options()?;
    let mode = effective_mode(&filesystem, index_side.map(|side| side.mode), captured.mode);
    let mut bytes = captured.bytes;
    if mode != 0o120000 {
        let index = repo.index_or_empty()?;
        let (mut pipeline, _) = repo.filter_pipeline(None)?;
        let mut normalized = vec![];
        pipeline
            .convert_to_git(std::io::Cursor::new(&bytes), relative, &index)?
            .take((MAX_BLOB_BYTES + 1) as u64)
            .read_to_end(&mut normalized)?;
        bytes = normalized;
    }
    let mut text = FileText::from_bytes(side, &bytes, changed);
    if mode == 0o120000 && text.note.is_empty() {
        text.note = "Symlink target".into();
    } else if text.note.is_empty() {
        text.note = format!("Working snapshot · mode {mode:06o}");
    }
    Ok(text)
}
#[derive(PartialEq)]
pub(crate) struct Capture {
    pub(crate) bytes: Vec<u8>,
    pub(crate) size: u64,
    pub(crate) mode: u16,
    modified: Option<std::time::SystemTime>,
}
pub(crate) fn capture(root: &Path, relative: &Path, limit: Option<u64>) -> Result<Option<Capture>> {
    let mut parent = root.to_path_buf();
    let components: Vec<_> = relative.components().collect();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        parent.push(component.as_os_str());
        match std::fs::symlink_metadata(&parent) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err("Working path crosses a symlink. Refresh status.".into());
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        }
    }
    let path = root.join(relative);
    let meta = match std::fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let (bytes, mode, size) = if meta.file_type().is_symlink() {
        let target = std::fs::read_link(&path)?;
        let bytes = gix::path::into_bstr(target)?.to_vec();
        let size = bytes.len() as u64;
        (bytes, 0o120000, size)
    } else if meta.is_file() {
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            if meta.permissions().mode() & 0o111 != 0 {
                0o100755
            } else {
                0o100644
            }
        };
        #[cfg(not(unix))]
        let mode = 0o100644;
        let mut bytes = vec![];
        if limit.is_none_or(|limit| meta.len() <= limit) {
            let file = std::fs::File::open(&path)?;
            match limit {
                Some(limit) => {
                    file.take(limit + 1).read_to_end(&mut bytes)?;
                }
                None => {
                    let mut file = file;
                    file.read_to_end(&mut bytes)?;
                }
            }
        }
        (bytes, mode, meta.len())
    } else {
        return Err("Selected path is not a regular file or symlink. Refresh status.".into());
    };
    Ok(Some(Capture {
        bytes,
        size,
        mode,
        modified: meta.modified().ok(),
    }))
}

/// Resolve the index mode for captured working content on this filesystem.
pub(crate) fn effective_mode(
    capabilities: &gix::fs::Capabilities,
    previous: Option<u16>,
    mode: u16,
) -> u16 {
    match previous {
        Some(previous)
            if (!capabilities.symlink && previous == 0o120000 && mode & 0o170000 == 0o100000)
                || (!capabilities.executable_bit
                    && previous & 0o170000 == 0o100000
                    && mode & 0o170000 == 0o100000) =>
        {
            previous
        }
        _ => mode,
    }
}
