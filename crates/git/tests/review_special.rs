use gg_engine::{
    diff::DiffKind,
    review::{ChangeKind, FileChange, ReviewReader, ReviewRequest, ReviewStep},
};
use std::{
    error::Error,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn git(path: &Path, args: &[&str], input: &[u8]) -> Result<String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.org")
        .env("GIT_COMMITTER_EMAIL", "test@example.org")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("stdin missing")?
        .write_all(input)?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}
fn tree(path: &Path, entries: &[(&[u8], &str, &str)]) -> Result<String> {
    let mut bytes = vec![];
    for (name, mode, id) in entries {
        let kind = if *mode == "160000" { "commit" } else { "blob" };
        bytes.extend(format!("{mode} {kind} {id}\t").as_bytes());
        bytes.extend(*name);
        bytes.push(0);
    }
    git(path, &["mktree", "-z", "--missing"], &bytes)
}
fn changes(path: &Path, before: &str, after: &str) -> Result<Vec<FileChange>> {
    let parent = git(path, &["commit-tree", before, "-m", "before"], b"")?;
    let commit = git(
        path,
        &["commit-tree", after, "-p", &parent, "-m", "after"],
        b"",
    )?;
    let mut reader = gg_git::review::ReviewSession::open(&ReviewRequest {
        path: path.into(),
        revision: commit,
        parent: None,
    })?;
    let mut result = vec![];
    loop {
        match reader.step()? {
            ReviewStep::Change(c) => result.push(c),
            ReviewStep::Complete => return Ok(result),
            ReviewStep::Pending => {}
        }
    }
}
#[test]
fn executable_mode_change_is_explicit_without_reading_content() -> Result {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init"], b"")?;
    let blob = git(dir.path(), &["hash-object", "-w", "--stdin"], b"\0binary")?;
    let before = tree(dir.path(), &[(b"file", "100644", &blob)])?;
    let after = tree(dir.path(), &[(b"file", "100755", &blob)])?;
    let files = changes(dir.path(), &before, &after)?;
    assert_eq!(files[0].kind, ChangeKind::Modified);
    let calls = std::cell::Cell::new(0);
    let diff = gg_git::diff::read(dir.path(), &files[0], || {
        calls.set(calls.get() + 1);
        calls.get() > 3
    })?;
    assert_eq!(diff.kind, DiffKind::Metadata);
    assert_eq!(calls.get(), 3);
    assert!(diff.complete && diff.rows.is_empty());
    assert!(diff.note.contains("100644") && diff.note.contains("100755"));
    assert!(diff.note.contains("content unchanged"));
    Ok(())
}
#[test]
fn symlink_type_change_and_gitlink_keep_exact_sides() -> Result {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init"], b"")?;
    let old = git(dir.path(), &["hash-object", "-w", "--stdin"], b"old target")?;
    let new = git(
        dir.path(),
        &["hash-object", "-w", "--stdin"],
        b"../new target",
    )?;
    let first = "1".repeat(40);
    let second = "2".repeat(40);
    let before = tree(
        dir.path(),
        &[
            (b"link", "120000", &old),
            (b"module", "160000", &first),
            (b"type", "100644", &old),
        ],
    )?;
    let after = tree(
        dir.path(),
        &[
            (b"link", "120000", &new),
            (b"module", "160000", &second),
            (b"type", "120000", &new),
        ],
    )?;
    let files = changes(dir.path(), &before, &after)?;
    let link = gg_git::diff::read(dir.path(), &files[0], || false)?;
    assert_eq!(link.kind, DiffKind::Text);
    assert!(link.note.contains("Symlink target"));
    assert!(link.rows.iter().any(|r| r.bytes == b"../new target"));
    let module = gg_git::diff::read(dir.path(), &files[1], || false)?;
    assert_eq!(module.kind, DiffKind::Gitlink);
    assert_eq!(
        files[1].old.as_ref().map(|s| s.id.as_str()),
        Some(first.as_str())
    );
    assert_eq!(
        files[1].new.as_ref().map(|s| s.id.as_str()),
        Some(second.as_str())
    );
    assert!(module.complete && module.rows.is_empty());
    assert_eq!(files[2].kind, ChangeKind::TypeChanged);
    let changed = gg_git::diff::read(dir.path(), &files[2], || false)?;
    assert!(changed.note.contains("file to symlink"));
    Ok(())
}
#[test]
fn unusual_paths_keep_distinct_one_line_labels_and_original_bytes() -> Result {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init"], b"")?;
    let blob = git(dir.path(), &["hash-object", "-w", "--stdin"], b"data")?;
    let before = tree(dir.path(), &[])?;
    let names: [&[u8]; 4] = [b"line\nname", b"raw\xfe", b"raw\xff", b"raw\\xff"];
    let entries: Vec<_> = names
        .iter()
        .map(|n| (*n, "100644", blob.as_str()))
        .collect();
    let after = tree(dir.path(), &entries)?;
    let files = changes(dir.path(), &before, &after)?;
    let labels: std::collections::BTreeSet<_> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(labels.len(), 4);
    assert!(labels.iter().all(|label| !label.contains('\n')));
    for name in names {
        assert!(files.iter().any(|f| f.path_bytes == name));
    }
    Ok(())
}

#[test]
fn cancellation_stops_before_the_next_object_header() -> Result {
    use gg_engine::review::FileSide;
    use std::cell::Cell;
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init"], b"")?;
    let blob = git(dir.path(), &["hash-object", "-w", "--stdin"], b"data")?;
    let file = FileChange {
        path: "file".into(),
        path_bytes: b"file".to_vec(),
        kind: ChangeKind::Modified,
        old: Some(FileSide {
            id: blob,
            mode: 0o100644,
        }),
        new: Some(FileSide {
            id: "f".repeat(40),
            mode: 0o100644,
        }),
    };
    let calls = Cell::new(0);
    let result = gg_git::diff::read(dir.path(), &file, || {
        calls.set(calls.get() + 1);
        calls.get() == 3
    });
    let error = result.err().ok_or("expected cancellation")?;
    assert!(error.contains("Comparison cancelled"), "{error}");
    assert_eq!(calls.get(), 3);
    Ok(())
}
