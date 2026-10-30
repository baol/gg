//! Load only the selected immutable objects, checking headers before allocation.
use gg_engine::{
    diff::{self, DiffKind, FileDiff, MAX_BLOB_BYTES},
    review::{FileChange, FileSide},
};
use std::path::Path;
pub fn read(
    path: &Path,
    change: &FileChange,
    cancelled: impl Fn() -> bool,
) -> Result<FileDiff, String> {
    read_with(path, change, diff::Options::default(), cancelled)
}
/// Compare one change with the reader's context and whitespace options.
pub fn read_with(
    path: &Path,
    change: &FileChange,
    options: diff::Options,
    cancelled: impl Fn() -> bool,
) -> Result<FileDiff, String> {
    let run = || -> Result<FileDiff, Box<dyn std::error::Error>> {
        let repo = gix::discover(path)?;
        if cancelled() {
            return Err("Comparison cancelled".into());
        }
        let size = |side: &Option<FileSide>| -> Result<u64, Box<dyn std::error::Error>> {
            if cancelled() {
                return Err("Comparison cancelled".into());
            }
            match side {
                Some(s) if s.mode != 0o160000 => Ok(repo
                    .find_header(gix::ObjectId::from_hex(s.id.as_bytes())?)?
                    .size()),
                _ => Ok(0),
            }
        };
        let (old_size, new_size) = (size(&change.old)?, size(&change.new)?);
        if [&change.old, &change.new]
            .iter()
            .any(|s| s.as_ref().is_some_and(|s| s.mode == 0o160000))
        {
            return Ok(FileDiff::summary(
                DiffKind::Gitlink,
                old_size,
                new_size,
                "Gitlink: compare the referenced commit IDs",
            ));
        }
        let mode_note = match (&change.old, &change.new) {
            (Some(old), Some(new)) if old.mode != new.mode => {
                if old.mode & 0o170000 != new.mode & 0o170000 {
                    let kind = |mode| if mode == 0o120000 { "symlink" } else { "file" };
                    format!("Type changed: {} to {}", kind(old.mode), kind(new.mode))
                } else {
                    format!("Mode changed: {:06o} to {:06o}", old.mode, new.mode)
                }
            }
            _ if [&change.old, &change.new]
                .iter()
                .any(|side| side.as_ref().is_some_and(|side| side.mode == 0o120000)) =>
            {
                "Symlink target".into()
            }
            _ => String::new(),
        };
        if let (Some(old), Some(new)) = (&change.old, &change.new)
            && old.id == new.id
            && old.mode != new.mode
        {
            return Ok(FileDiff::summary(
                DiffKind::Metadata,
                old_size,
                new_size,
                format!("{mode_note} · content unchanged"),
            ));
        }
        if old_size > MAX_BLOB_BYTES as u64 || new_size > MAX_BLOB_BYTES as u64 {
            return Ok(FileDiff::summary(
                DiffKind::Limited,
                old_size,
                new_size,
                "Text budget: 2 MiB per side",
            ));
        }
        let bytes = |side: &Option<FileSide>| -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            if cancelled() {
                return Err("Comparison cancelled".into());
            }
            match side {
                Some(s) => Ok(repo
                    .find_blob(gix::ObjectId::from_hex(s.id.as_bytes())?)?
                    .data
                    .clone()),
                None => Ok(vec![]),
            }
        };
        let mut result = diff::compare_with(
            &bytes(&change.old)?,
            &bytes(&change.new)?,
            options,
            &cancelled,
        )?;
        if !mode_note.is_empty() {
            result.note = if result.note.is_empty() {
                mode_note
            } else {
                format!("{mode_note} · {}", result.note)
            };
        }
        Ok(result)
    };
    run().map_err(|e| {
        format!(
            "Could not compare {} in {}: {e}",
            change.path,
            path.display()
        )
    })
}
