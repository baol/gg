//! Shared partial-patch selection and index publication.
use gg_engine::{
    diff::{DiffRow, RowKind},
    status::WorkingRequest,
};
use gix::bstr::BStr;
use std::{
    ops::Range,
    path::{Component, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// One hunk to change. Empty `changes` selects the whole hunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub header: String,
    pub changes: Vec<Change>,
}

/// A selected side of a changed source line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Added(usize),
    Deleted(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    /// Keep selected new-side lines and unselected old-side lines.
    Stage,
    /// Restore selected old-side lines and keep unselected new-side lines.
    Unstage,
}

/// Partial changes must come from the exact comparison: with whitespace
/// ignored, hunks would not describe the bytes being written.
pub(crate) fn check_exact(options: gg_engine::diff::Options) -> Result<()> {
    if options.whitespace != gg_engine::diff::Whitespace::All {
        return Err("Show all whitespace changes to apply hunks or lines".into());
    }
    Ok(())
}

pub fn edits_from_headers(headers: &[String]) -> Vec<Edit> {
    headers
        .iter()
        .map(|header| Edit {
            header: header.clone(),
            changes: vec![],
        })
        .collect()
}

/// Rebuild one side of a diff by line ranges.
///
/// Unchanged gaps come from `before`; context rows are copied once; selected
/// changes follow `direction`, while omitted changes keep the opposite side.
pub(crate) fn reconstruct(
    before: &[u8],
    rows: &[DiffRow],
    edits: &[Edit],
    direction: Direction,
) -> std::result::Result<(Vec<u8>, bool), String> {
    if edits.is_empty() {
        return Err("Select at least one hunk or line".into());
    }
    let hunks = hunk_spans(rows)?;
    let mut selected = vec![false; rows.len()];
    for edit in edits {
        let hunk = hunks
            .iter()
            .find(|hunk| hunk.header == edit.header)
            .ok_or("Selected hunk is stale. Refresh the diff.")?;
        if edit.changes.is_empty() {
            for index in hunk.rows.clone() {
                if matches!(rows[index].kind, RowKind::Added | RowKind::Deleted) {
                    selected[index] = true;
                }
            }
        } else {
            for change in &edit.changes {
                let found = hunk
                    .rows
                    .clone()
                    .find(|index| row_matches(&rows[*index], *change))
                    .ok_or("Selected line is stale. Refresh the diff.")?;
                selected[found] = true;
            }
        }
    }
    let changes = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| matches!(row.kind, RowKind::Added | RowKind::Deleted))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if changes.is_empty() {
        return Err("No changes to apply".into());
    }
    let all = changes.iter().all(|index| selected[*index]);
    let old_lines: Vec<&[u8]> = before.split_inclusive(|byte| *byte == b'\n').collect();
    let mut patched = Vec::with_capacity(before.len());
    let mut old_cursor = 0;
    for hunk in &hunks {
        let gap = old_lines
            .get(old_cursor..hunk.old_index)
            .ok_or("Selected hunk is stale. Refresh the diff.")?;
        for line in gap {
            patched.extend_from_slice(line);
        }
        for index in hunk.rows.clone() {
            let row = &rows[index];
            let keep = match row.kind {
                RowKind::Context => true,
                RowKind::Deleted => match direction {
                    Direction::Stage => !selected[index],
                    Direction::Unstage => selected[index],
                },
                RowKind::Added => match direction {
                    Direction::Stage => selected[index],
                    Direction::Unstage => !selected[index],
                },
                RowKind::NoNewline | RowKind::Hunk => false,
            };
            if keep {
                patched.extend_from_slice(&row.bytes);
            }
        }
        old_cursor = hunk.old_index + hunk.old_count;
    }
    for line in old_lines
        .get(old_cursor..)
        .ok_or("Selected hunk is stale. Refresh the diff.")?
    {
        patched.extend_from_slice(line);
    }
    Ok((patched, all))
}

struct Hunk {
    header: String,
    old_index: usize,
    old_count: usize,
    rows: Range<usize>,
}

fn hunk_spans(rows: &[DiffRow]) -> std::result::Result<Vec<Hunk>, String> {
    let mut starts = vec![];
    for (index, row) in rows.iter().enumerate() {
        if row.kind == RowKind::Hunk {
            let (old_start, old_count, _, _) = parse_hunk_header(&row.text)
                .ok_or_else(|| format!("Unrecognized hunk header: {}", row.text))?;
            starts.push((index, row.text.clone(), old_start, old_count));
        }
    }
    Ok(starts
        .iter()
        .enumerate()
        .map(|(position, (index, header, old_start, old_count))| Hunk {
            header: header.clone(),
            old_index: if *old_count == 0 {
                *old_start
            } else {
                old_start - 1
            },
            old_count: *old_count,
            rows: index + 1
                ..starts
                    .get(position + 1)
                    .map(|next| next.0)
                    .unwrap_or(rows.len()),
        })
        .collect())
}

fn row_matches(row: &DiffRow, change: Change) -> bool {
    match change {
        Change::Added(line) => row.kind == RowKind::Added && row.new_line == Some(line),
        Change::Deleted(line) => row.kind == RowKind::Deleted && row.old_line == Some(line),
    }
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

pub(crate) fn relative_path(request: &WorkingRequest) -> Result<PathBuf> {
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

pub(crate) fn acquire(repo: &gix::Repository) -> Result<gix::lock::File> {
    Ok(gix::lock::File::acquire_to_update_resource(
        repo.index_path(),
        gix::lock::acquire::Fail::Immediately,
        None,
        0,
    )?)
}

pub(crate) fn check_conflicts(index: &gix::index::State, path: &BStr) -> Result<()> {
    if index.entries().iter().any(|entry| {
        entry.path(index) == path && entry.stage() != gix::index::entry::Stage::Unconflicted
    }) {
        return Err("Resolve this path's conflict before staging or unstaging".into());
    }
    Ok(())
}

pub(crate) fn check_collision(index: &gix::index::State, wanted: &[u8]) -> Result<()> {
    if index.entries().iter().any(|entry| {
        let path: &[u8] = entry.path(index).as_ref();
        (path.starts_with(wanted) && path.get(wanted.len()) == Some(&b'/'))
            || (wanted.starts_with(path) && wanted.get(path.len()) == Some(&b'/'))
    }) {
        return Err("Stage the related file/directory changes first".into());
    }
    Ok(())
}

pub(crate) fn publish(
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
