use gg_engine::{
    review::{ChangeKind, FileChange, FileSide},
    status::{Area, Kind, StatusEntry, WorkingRequest},
};
use std::{error::Error, path::Path, process::Command};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

const OID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn git(path: &Path, args: &[&str]) -> Result {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.org")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.org")
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(())
}

fn output(path: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn pointer(oid: &str, size: u64) -> String {
    format!("version https://git-lfs.github.com/spec/v1\noid sha256:{oid}\nsize {size}\n")
}

fn repo() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-q", "-b", "main"])?;
    git(dir.path(), &["config", "user.name", "Test"])?;
    git(dir.path(), &["config", "user.email", "test@example.org"])?;
    git(dir.path(), &["config", "core.autocrlf", "false"])?;
    std::fs::write(dir.path().join("a"), b"base\n")?;
    git(dir.path(), &["add", "a"])?;
    git(dir.path(), &["commit", "-qm", "base"])?;
    Ok(dir)
}

fn change(path: &Path, old: Option<&str>, new: Option<&str>) -> Result<FileChange> {
    let side = |reference: &str| -> Result<FileSide> {
        Ok(FileSide {
            id: output(path, &["rev-parse", reference])?,
            mode: 0o100644,
        })
    };
    Ok(FileChange {
        path: "asset.bin".into(),
        path_bytes: b"asset.bin".to_vec(),
        kind: ChangeKind::Modified,
        old: old.map(side).transpose()?,
        new: new.map(side).transpose()?,
    })
}

#[test]
fn pointer_details_separate_oid_size_and_local_availability() -> Result {
    let dir = repo()?;
    std::fs::write(dir.path().join("asset.bin"), pointer(OID, 12345))?;
    git(dir.path(), &["add", "asset.bin"])?;
    git(dir.path(), &["commit", "-qm", "pointer"])?;
    let change = change(dir.path(), None, Some("HEAD:asset.bin"))?;
    let details = gg_git::lfs::read(dir.path(), &change)?;
    assert_eq!(details.path, "asset.bin");
    let after = details.after.as_ref().ok_or("missing after pointer")?;
    assert_eq!(after.oid, OID);
    assert_eq!(after.size, 12345);
    assert!(!after.available);

    let object = dir
        .path()
        .join(".git/lfs/objects")
        .join(&OID[..2])
        .join(&OID[2..4])
        .join(OID);
    std::fs::create_dir_all(object.parent().ok_or("missing parent")?)?;
    std::fs::write(&object, b"content")?;
    let details = gg_git::lfs::read(dir.path(), &change)?;
    let after = details.after.as_ref().ok_or("missing after pointer")?;
    assert!(after.available);
    assert_eq!(details.status_label(), "Object available");
    Ok(())
}

#[test]
fn before_and_after_pointers_and_plain_text_are_distinguished() -> Result {
    let dir = repo()?;
    std::fs::write(dir.path().join("asset.bin"), pointer(OID, 10))?;
    git(dir.path(), &["add", "asset.bin"])?;
    git(dir.path(), &["commit", "-qm", "first pointer"])?;
    let other = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
    std::fs::write(dir.path().join("asset.bin"), pointer(other, 20))?;
    git(dir.path(), &["commit", "-qam", "second pointer"])?;
    let change = change(dir.path(), Some("HEAD~1:asset.bin"), Some("HEAD:asset.bin"))?;
    let details = gg_git::lfs::read(dir.path(), &change)?;
    assert_eq!(details.before.as_ref().map(|p| p.size), Some(10));
    assert_eq!(details.after.as_ref().map(|p| p.size), Some(20));
    assert_eq!(details.after.as_ref().map(|p| p.oid.as_str()), Some(other));

    let plain = FileChange {
        path: "a".into(),
        path_bytes: b"a".to_vec(),
        kind: ChangeKind::Modified,
        old: None,
        new: Some(FileSide {
            id: output(dir.path(), &["rev-parse", "HEAD:a"])?,
            mode: 0o100644,
        }),
    };
    let details = gg_git::lfs::read(dir.path(), &plain)?;
    assert!(details.before.is_none() && details.after.is_none());
    assert_eq!(details.status_label(), "Not an LFS pointer");
    Ok(())
}

#[test]
fn malformed_pointer_is_reported_without_treating_text_as_lfs() -> Result {
    let dir = repo()?;
    std::fs::write(
        dir.path().join("asset.bin"),
        b"version https://git-lfs.github.com/spec/v1\noid sha256:short\nsize 1\n",
    )?;
    git(dir.path(), &["add", "asset.bin"])?;
    git(dir.path(), &["commit", "-qm", "bad pointer"])?;
    let change = change(dir.path(), None, Some("HEAD:asset.bin"))?;
    let details = gg_git::lfs::read(dir.path(), &change)?;
    assert!(details.after.is_none());
    assert!(
        details
            .note
            .as_deref()
            .is_some_and(|note| note.contains("64 hexadecimal")),
        "{:?}",
        details.note
    );
    Ok(())
}

fn working(path: &Path, area: Area, kind: Kind, file: &str) -> Result<WorkingRequest> {
    Ok(WorkingRequest {
        repository: gg_git::status::Session::open(path)?.identity,
        entry: StatusEntry::new(area, kind, file.as_bytes()),
    })
}

#[test]
fn working_staged_unstaged_and_untracked_pointers_use_their_own_sides() -> Result {
    let dir = repo()?;
    let first = "1111111111111111111111111111111111111111111111111111111111111111";
    let second = "2222222222222222222222222222222222222222222222222222222222222222";
    let third = "3333333333333333333333333333333333333333333333333333333333333333";
    let untracked = "4444444444444444444444444444444444444444444444444444444444444444";
    std::fs::write(dir.path().join("asset.bin"), pointer(first, 10))?;
    git(dir.path(), &["add", "asset.bin"])?;
    git(dir.path(), &["commit", "-qm", "first pointer"])?;

    std::fs::write(dir.path().join("asset.bin"), pointer(second, 20))?;
    git(dir.path(), &["add", "asset.bin"])?;
    let staged = gg_git::lfs::read_working(
        dir.path(),
        &working(dir.path(), Area::Staged, Kind::Modified, "asset.bin")?,
    )?;
    assert_eq!(staged.before.as_ref().map(|p| p.oid.as_str()), Some(first));
    assert_eq!(staged.after.as_ref().map(|p| p.oid.as_str()), Some(second));
    assert!(
        matches!(staged.status_label(), "Object missing" | "LFS unavailable"),
        "{}",
        staged.status_label()
    );

    std::fs::write(dir.path().join("asset.bin"), pointer(third, 30))?;
    let unstaged = gg_git::lfs::read_working(
        dir.path(),
        &working(dir.path(), Area::Unstaged, Kind::Modified, "asset.bin")?,
    )?;
    assert_eq!(
        unstaged.before.as_ref().map(|p| p.oid.as_str()),
        Some(second)
    );
    assert_eq!(unstaged.after.as_ref().map(|p| p.oid.as_str()), Some(third));

    std::fs::write(dir.path().join("new.bin"), pointer(untracked, 40))?;
    let new = gg_git::lfs::read_working(
        dir.path(),
        &working(dir.path(), Area::Untracked, Kind::Added, "new.bin")?,
    )?;
    assert!(new.before.is_none());
    assert_eq!(new.after.as_ref().map(|p| p.oid.as_str()), Some(untracked));
    Ok(())
}

#[test]
fn plain_working_text_and_conflicts_do_not_masquerade_as_pointers() -> Result {
    let dir = repo()?;
    std::fs::write(dir.path().join("plain.bin"), b"ordinary text\n")?;
    let plain = gg_git::lfs::read_working(
        dir.path(),
        &working(dir.path(), Area::Untracked, Kind::Added, "plain.bin")?,
    )?;
    assert!(plain.before.is_none() && plain.after.is_none());
    assert_eq!(plain.status_label(), "Not an LFS pointer");

    let conflict = gg_git::lfs::read_working(
        dir.path(),
        &working(dir.path(), Area::Conflict, Kind::Conflicted, "asset.bin")?,
    )?;
    assert!(conflict.target().is_none());
    assert!(
        conflict
            .note
            .as_deref()
            .is_some_and(|note| note.contains("resolve the conflict")),
        "{:?}",
        conflict.note
    );
    Ok(())
}

#[test]
fn bounded_object_preview_reads_only_an_available_small_object() -> Result {
    let dir = repo()?;
    let oid = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    let object = gg_git::lfs::object_path(dir.path(), oid).ok_or("missing object path")?;
    assert!(gg_git::lfs::read_object(dir.path(), oid, 1024)?.is_none());
    std::fs::create_dir_all(object.parent().ok_or("missing parent")?)?;
    std::fs::write(&object, b"hello")?;
    assert_eq!(
        gg_git::lfs::read_object(dir.path(), oid, 16)?,
        Some(b"hello".to_vec())
    );
    let error = match gg_git::lfs::read_object(dir.path(), oid, 3) {
        Err(error) => error,
        Ok(_) => return Err("over-limit preview unexpectedly succeeded".into()),
    };
    assert!(error.contains("limited to 3 B"), "{error}");
    assert!(gg_git::lfs::object_path(dir.path(), "not-an-oid").is_none());
    Ok(())
}
