//! Stage working paths or selected hunks under the index lock.
use gg_engine::{
    diff::{self, DiffKind, MAX_BLOB_BYTES, RowKind},
    status::{Area, Kind, WorkingRequest},
};
use gix::bstr::BStr;
use std::{
    io::Read,
    path::{Component, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn file(request: &WorkingRequest) -> std::result::Result<(), String> {
    apply_file(request).map_err(|e| format!("Stage {}: {e}", request.entry.path))
}
pub fn hunks(request: &WorkingRequest, selected: &[String]) -> std::result::Result<(), String> {
    apply_hunks(request, selected)
        .map_err(|e| format!("Stage hunks for {}: {e}", request.entry.path))
}
fn apply_file(request: &WorkingRequest) -> Result<()> {
    match request.entry.area {
        Area::Unstaged | Area::Untracked => {}
        Area::Staged => return Err("Path is already staged".into()),
        Area::Conflict => return Err("Resolve this path's conflict before staging".into()),
    }
    let relative = relative_path(request)?;
    let repo = gix::discover(&request.repository.path)?;
    let lock = acquire(&repo)?;
    crate::status::validate(&request.repository)?;
    let mut index = (**repo.index_or_empty()?).clone();
    if index.is_sparse() {
        return Err("Stage requires an expanded index".into());
    }
    let path = BStr::new(&request.entry.path_bytes);
    check_conflicts(&index, path)?;
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
    check_collision(&index, &request.entry.path_bytes)?;
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
    publish(index, lock, request)
}
fn apply_hunks(request: &WorkingRequest, selected: &[String]) -> Result<()> {
    if selected.is_empty() {
        return Err("Select at least one hunk".into());
    }
    if request.entry.area != Area::Unstaged || request.entry.kind != Kind::Modified {
        return Err("Select an unstaged modification to stage hunks".into());
    }
    let relative = relative_path(request)?;
    let repo = gix::discover(&request.repository.path)?;
    let lock = acquire(&repo)?;
    crate::status::validate(&request.repository)?;
    let mut index = (**repo.index_or_empty()?).clone();
    if index.is_sparse() {
        return Err("Hunk staging requires an expanded index".into());
    }
    let path = BStr::new(&request.entry.path_bytes);
    check_conflicts(&index, path)?;
    let (previous_mode, previous_id) = index
        .entry_by_path(path)
        .map(|e| (e.mode.bits() as u16, e.id))
        .ok_or("Path is no longer tracked. Refresh status.")?;
    if previous_mode == 0o160000 {
        return Err("Submodule changes: stage inside the submodule".into());
    }
    if !matches!(previous_mode, 0o100644 | 0o100755) {
        return Err("Hunk staging requires a regular text file".into());
    }
    let root = &request.repository.path;
    let captured = crate::working_diff::capture(root, &relative, Some(MAX_BLOB_BYTES as u64))?
        .ok_or("Working path is absent. Refresh status.")?;
    if captured.size > MAX_BLOB_BYTES as u64 {
        return Err("Hunk staging is limited to 2 MiB text files".into());
    }
    let capabilities = repo.filesystem_options()?;
    let mode =
        crate::working_diff::effective_mode(&capabilities, Some(previous_mode), captured.mode);
    if mode != previous_mode || captured.mode == 0o120000 {
        return Err("Stage the whole file to record mode or type changes".into());
    }
    let before = repo.find_blob(previous_id)?.data.clone();
    let mut after = captured.bytes.clone();
    let (mut pipeline, _) = repo.filter_pipeline(None)?;
    let mut normalized = vec![];
    pipeline
        .convert_to_git(std::io::Cursor::new(&after), &relative, &index)?
        .take((MAX_BLOB_BYTES + 1) as u64)
        .read_to_end(&mut normalized)?;
    after = normalized;
    let comparison = diff::compare(&before, &after, || false)?;
    if comparison.kind != DiffKind::Text {
        return Err("Hunk staging requires a text comparison".into());
    }
    if !comparison.complete {
        return Err("Hunk staging requires a complete comparison".into());
    }
    let headers: Vec<&str> = comparison
        .rows
        .iter()
        .filter(|row| row.kind == RowKind::Hunk)
        .map(|row| row.text.as_str())
        .collect();
    if headers.is_empty() {
        return Err("No changes to stage".into());
    }
    let mut wanted = vec![false; headers.len()];
    for header in selected {
        let position = headers
            .iter()
            .position(|candidate| *candidate == header)
            .ok_or("Selected hunk is stale. Refresh the diff.")?;
        wanted[position] = true;
    }
    let all = wanted.iter().all(|wanted| *wanted);
    // Rebuild the index side by line ranges: unchanged gaps and unselected hunks keep the old side.
    let hunks: Vec<_> = comparison
        .rows
        .iter()
        .filter(|row| row.kind == RowKind::Hunk)
        .map(|row| {
            parse_hunk_header(&row.text)
                .ok_or_else(|| format!("Unrecognized hunk header: {}", row.text))
        })
        .collect::<std::result::Result<Vec<_>, String>>()?;
    let old_lines: Vec<&[u8]> = before.split_inclusive(|b| *b == b'\n').collect();
    let new_lines: Vec<&[u8]> = after.split_inclusive(|b| *b == b'\n').collect();
    let mut patched = Vec::with_capacity(after.len());
    let mut old_cursor = 0;
    for (position, &(old_start, old_count, new_start, new_count)) in hunks.iter().enumerate() {
        let old_index = if old_count == 0 {
            old_start
        } else {
            old_start - 1
        };
        let new_index = if new_count == 0 {
            new_start
        } else {
            new_start - 1
        };
        let stale = || "Selected hunk is stale. Refresh the diff.";
        let gap = old_lines.get(old_cursor..old_index).ok_or_else(stale)?;
        for line in gap {
            patched.extend_from_slice(line);
        }
        let lines = if wanted[position] {
            new_lines
                .get(new_index..new_index + new_count)
                .ok_or_else(stale)?
        } else {
            old_lines
                .get(old_index..old_index + old_count)
                .ok_or_else(stale)?
        };
        for line in lines {
            patched.extend_from_slice(line);
        }
        old_cursor = old_index + old_count;
    }
    for line in old_lines
        .get(old_cursor..)
        .ok_or("Selected hunk is stale. Refresh the diff.")?
    {
        patched.extend_from_slice(line);
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
    publish(index, lock, request)
}
fn parse_hunk_header(header: &str) -> Option<(usize, usize, usize, usize)> {
    let rest = header.strip_prefix("@@ -")?;
    let (old, rest) = rest.split_once(" +")?;
    let new = rest.split(" @@").next()?;
    let parse = |range: &str| -> Option<(usize, usize)> {
        let (start, count) = range.split_once(',').unwrap_or((range, "1"));
        Some((start.parse().ok()?, count.parse().ok()?))
    };
    let (old_start, old_count) = parse(old)?;
    let (new_start, new_count) = parse(new)?;
    Some((old_start, old_count, new_start, new_count))
}
fn relative_path(request: &WorkingRequest) -> Result<PathBuf> {
    let relative = gix::path::from_bstr(BStr::new(&request.entry.path_bytes))?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("Use a repository-relative file path".into());
    }
    Ok(relative.into_owned())
}
fn acquire(repo: &gix::Repository) -> Result<gix::lock::File> {
    Ok(gix::lock::File::acquire_to_update_resource(
        repo.index_path(),
        gix::lock::acquire::Fail::Immediately,
        None,
        0,
    )?)
}
fn check_conflicts(index: &gix::index::State, path: &BStr) -> Result<()> {
    if index
        .entries()
        .iter()
        .any(|e| e.path(index) == path && e.stage() != gix::index::entry::Stage::Unconflicted)
    {
        return Err("Resolve this path's conflict before staging".into());
    }
    Ok(())
}
fn check_collision(index: &gix::index::State, wanted: &[u8]) -> Result<()> {
    if index.entries().iter().any(|e| {
        let path: &[u8] = e.path(index).as_ref();
        (path.starts_with(wanted) && path.get(wanted.len()) == Some(&b'/'))
            || (wanted.starts_with(path) && wanted.get(path.len()) == Some(&b'/'))
    }) {
        return Err("Stage the related file/directory changes first".into());
    }
    Ok(())
}
fn publish(
    mut index: gix::index::File,
    mut lock: gix::lock::File,
    request: &WorkingRequest,
) -> Result<()> {
    index.sort_entries();
    index.remove_tree();
    index.write_to(&mut lock, Default::default())?;
    // HEAD can change independently of the index lock.
    crate::status::validate(&request.repository)?;
    lock.commit()?;
    Ok(())
}
