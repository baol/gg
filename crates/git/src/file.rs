//! Read one bounded blob side for the full-file reading surface.
use gg_engine::{
    diff::MAX_BLOB_BYTES,
    file::{FileKind, FileText, ReadingSide},
    review::{FileChange, FileSide},
};
use std::{collections::BTreeSet, path::Path};

fn check(cancelled: &impl Fn() -> bool) -> Result<(), String> {
    if cancelled() {
        Err("Operation cancelled".into())
    } else {
        Ok(())
    }
}

/// Read the selected side of an immutable comparison. Missing sides are explicit.
pub fn read(
    path: &Path,
    change: &FileChange,
    side: ReadingSide,
    changed: &BTreeSet<usize>,
    cancelled: impl Fn() -> bool,
) -> Result<FileText, String> {
    let file: Option<&FileSide> = match side {
        ReadingSide::Before => change.old.as_ref(),
        ReadingSide::After => change.new.as_ref(),
    };
    let Some(file) = file else {
        return Ok(FileText::missing(
            side,
            format!(
                "File does not exist on the {} side",
                side.label().to_lowercase()
            ),
        ));
    };
    let run = || -> Result<FileText, Box<dyn std::error::Error>> {
        let repo = gix::discover(path)?;
        check(&cancelled)?;
        if file.mode == 0o160000 {
            return Ok(FileText::summary(
                FileKind::Gitlink,
                side,
                0,
                "Gitlink: compare the referenced commit ID",
            ));
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
        check(&cancelled)?;
        let blob = repo.find_blob(id)?;
        check(&cancelled)?;
        let mut text = FileText::from_bytes(side, &blob.data, changed);
        if file.mode == 0o120000 && text.note.is_empty() {
            text.note = "Symlink target".into();
        }
        Ok(text)
    };
    run().map_err(|error| {
        format!(
            "Could not read the {} side of {} in {}: {error}",
            side.label().to_lowercase(),
            change.path,
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gg_engine::review::ChangeKind;

    #[test]
    fn missing_side_is_explicit() -> Result<(), String> {
        let change = FileChange {
            path: "added".into(),
            path_bytes: b"added".to_vec(),
            kind: ChangeKind::Added,
            old: None,
            new: None,
        };
        let text = read(
            Path::new("."),
            &change,
            ReadingSide::Before,
            &BTreeSet::new(),
            || false,
        )?;
        assert_eq!(text.kind, FileKind::Empty);
        assert!(text.note.contains("does not exist"));
        Ok(())
    }
}
