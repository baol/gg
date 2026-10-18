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
    let run = || -> Result<FileDiff, Box<dyn std::error::Error>> {
        let repo = gix::discover(path)?;
        if cancelled() {
            return Err("Comparison cancelled".into());
        }
        let size = |side: &Option<FileSide>| -> Result<u64, Box<dyn std::error::Error>> {
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
        Ok(diff::compare(
            &bytes(&change.old)?,
            &bytes(&change.new)?,
            &cancelled,
        )?)
    };
    run().map_err(|e| {
        format!(
            "Could not compare {} in {}: {e}",
            change.path,
            path.display()
        )
    })
}
