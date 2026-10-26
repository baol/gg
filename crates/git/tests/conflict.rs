use std::{error::Error, path::Path, process::Command};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn git(path: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .args(["-c", "core.autocrlf=false"])
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
    Ok(output.stdout)
}

fn repo() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-b", "main"])?;
    git(dir.path(), &["config", "user.name", "Test"])?;
    git(dir.path(), &["config", "user.email", "test@example.org"])?;
    git(dir.path(), &["config", "core.autocrlf", "false"])?;
    std::fs::write(dir.path().join("a"), b"base\n")?;
    git(dir.path(), &["add", "a"])?;
    git(dir.path(), &["commit", "-qm", "base"])?;
    Ok(dir)
}

fn conflicting(dir: &Path) -> Result {
    git(dir, &["switch", "-qc", "feature"])?;
    std::fs::write(dir.join("a"), b"feature\n")?;
    git(dir, &["commit", "-qam", "feature"])?;
    git(dir, &["switch", "-q", "main"])?;
    std::fs::write(dir.join("a"), b"main\n")?;
    git(dir, &["commit", "-qam", "main"])?;
    let error = gg_git::merge::merge(dir, "feature", false);
    assert!(error.is_ok());
    Ok(())
}

#[test]
fn reads_and_selects_conflict_sides() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    assert_eq!(gg_git::conflict::unmerged(p)?, vec!["a".to_string()]);

    let content = gg_git::conflict::read(p, "a")?;
    assert_eq!(content.base.as_deref(), Some("base\n"));
    assert_eq!(content.ours.as_deref(), Some("main\n"));
    assert_eq!(content.theirs.as_deref(), Some("feature\n"));
    assert!(!content.binary);
    assert!(!content.truncated);

    gg_git::conflict::use_side(p, "a", gg_engine::conflict::Side::Theirs)?;
    assert_eq!(std::fs::read(p.join("a"))?, b"feature\n");
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    Ok(())
}

#[test]
fn stages_manually_resolved_content() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    std::fs::write(p.join("a"), b"resolved\n")?;
    gg_git::conflict::stage(p, "a")?;
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    assert_eq!(std::fs::read(p.join("a"))?, b"resolved\n");
    Ok(())
}

#[test]
fn add_add_conflict_has_no_base() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("new"), b"feature\n")?;
    git(p, &["add", "new"])?;
    git(p, &["commit", "-qm", "feature new"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("new"), b"main\n")?;
    git(p, &["add", "new"])?;
    git(p, &["commit", "-qm", "main new"])?;
    gg_git::merge::merge(p, "feature", false)?;

    let content = gg_git::conflict::read(p, "new")?;
    assert!(content.base.is_none());
    assert_eq!(content.ours.as_deref(), Some("main\n"));
    assert_eq!(content.theirs.as_deref(), Some("feature\n"));
    Ok(())
}

#[test]
fn rejects_unknown_paths_and_base_selection() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let error = match gg_git::conflict::read(p, "missing") {
        Ok(_) => return Err("missing conflict unexpectedly read".into()),
        Err(error) => error,
    };
    assert!(error.contains("No unmerged stages"), "{error}");
    let error = match gg_git::conflict::use_side(p, "a", gg_engine::conflict::Side::Base) {
        Ok(_) => return Err("base side unexpectedly selected".into()),
        Err(error) => error,
    };
    assert!(error.contains("Base content"), "{error}");
    Ok(())
}

#[test]
fn previewed_sides_publish_only_after_validation() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let snapshot = gg_git::conflict::snapshot(p, "a")?;
    assert!(snapshot.base.is_some());
    assert!(snapshot.ours.is_some());
    let ours = snapshot.ours.clone();
    assert!(ours.is_some());
    assert!(snapshot.theirs.is_some());
    assert_eq!(
        std::fs::read(p.join("a"))?,
        b"<<<<<<< HEAD\nmain\n=======\nfeature\n>>>>>>> feature\n"
    );

    gg_git::conflict::publish(p, "a", &snapshot, gg_engine::conflict::Resolution::Ours)?;
    assert_eq!(std::fs::read(p.join("a"))?, b"main\n");
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    let staged = git(p, &["ls-files", "-s", "--", "a"])?;
    let staged = String::from_utf8_lossy(&staged);
    assert!(
        staged.contains(ours.as_deref().unwrap_or_default()),
        "staged output must be the selected side: {staged}"
    );
    Ok(())
}

#[test]
fn publish_rejects_changed_stages_or_working_bytes() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let stale = gg_git::conflict::snapshot(p, "a")?;
    gg_git::conflict::use_side(p, "a", gg_engine::conflict::Side::Theirs)?;
    assert!(
        gg_git::conflict::publish(p, "a", &stale, gg_engine::conflict::Resolution::Ours,).is_err()
    );
    assert_eq!(std::fs::read(p.join("a"))?, b"feature\n");

    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let stale = gg_git::conflict::snapshot(p, "a")?;
    std::fs::write(p.join("a"), b"external\n")?;
    assert!(
        gg_git::conflict::publish(p, "a", &stale, gg_engine::conflict::Resolution::Working,)
            .is_err()
    );
    let fresh = gg_git::conflict::snapshot(p, "a")?;
    gg_git::conflict::publish(p, "a", &fresh, gg_engine::conflict::Resolution::Working)?;
    assert_eq!(std::fs::read(p.join("a"))?, b"external\n");
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    Ok(())
}

#[test]
fn base_resolution_writes_the_common_ancestor() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let snapshot = gg_git::conflict::snapshot(p, "a")?;
    gg_git::conflict::publish(p, "a", &snapshot, gg_engine::conflict::Resolution::Base)?;
    assert_eq!(std::fs::read(p.join("a"))?, b"base\n");
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    Ok(())
}

#[test]
fn snapshot_rejects_paths_outside_the_repository() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    assert!(gg_git::conflict::snapshot(p, "../a").is_err());
    Ok(())
}

#[test]
fn oversized_working_files_validate_without_a_preview() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    std::fs::write(p.join("a"), vec![b'a'; 2 * 1024 * 1024 + 1])?;
    let snapshot = gg_git::conflict::snapshot(p, "a")?;
    assert!(snapshot.working.is_none());
    assert!(snapshot.working_hash.is_some());
    assert!(
        snapshot
            .working_note
            .as_deref()
            .is_some_and(|note| note.contains("bytes"))
    );
    gg_git::conflict::publish(p, "a", &snapshot, gg_engine::conflict::Resolution::Working)?;
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    Ok(())
}

#[test]
fn publishing_reports_a_partial_worktree_write() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let snapshot = gg_git::conflict::snapshot(p, "a")?;
    std::fs::write(p.join(".git/index.lock"), b"locked")?;
    let error =
        match gg_git::conflict::publish(p, "a", &snapshot, gg_engine::conflict::Resolution::Ours) {
            Err(error) => error,
            Ok(()) => return Err("locked index must fail".into()),
        };
    assert!(error.contains("written but staging failed"), "{error}");
    assert_eq!(std::fs::read(p.join("a"))?, b"main\n");
    std::fs::remove_file(p.join(".git/index.lock"))?;
    assert!(!gg_git::conflict::unmerged(p)?.is_empty());
    Ok(())
}

#[test]
fn non_regular_working_paths_keep_the_side_preview_explicit() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    std::fs::remove_file(p.join("a"))?;
    std::fs::create_dir(p.join("a"))?;
    let snapshot = gg_git::conflict::snapshot(p, "a")?;
    assert!(snapshot.working.is_none());
    assert!(
        snapshot
            .working_note
            .as_deref()
            .is_some_and(|note| note.contains("not a regular file"))
    );
    let error =
        match gg_git::conflict::publish(p, "a", &snapshot, gg_engine::conflict::Resolution::Ours) {
            Err(error) => error,
            Ok(()) => return Err("a directory cannot be replaced by rename".into()),
        };
    assert!(error.contains("Write working file"), "{error}");
    assert!(p.join("a").is_dir());
    Ok(())
}

#[test]
fn oversized_sides_publish_and_changed_bytes_are_detected() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    git(p, &["config", "user.name", "Test"])?;
    git(p, &["config", "user.email", "test@example.org"])?;
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    git(p, &["commit", "-qm", "base"])?;
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("a"), vec![b'f'; 2 * 1024 * 1024 + 1])?;
    git(p, &["commit", "-qam", "feature"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("a"), vec![b'm'; 2 * 1024 * 1024 + 1])?;
    git(p, &["commit", "-qam", "main"])?;
    let _ = gg_git::merge::merge(p, "feature", false);
    assert!(!gg_git::conflict::unmerged(p)?.is_empty());

    let snapshot = gg_git::conflict::snapshot(p, "a")?;
    gg_git::conflict::publish(p, "a", &snapshot, gg_engine::conflict::Resolution::Ours)?;
    let resolved = std::fs::read(p.join("a"))?;
    assert_eq!(resolved.len(), 2 * 1024 * 1024 + 1);
    assert!(resolved.iter().all(|byte| *byte == b'm'));
    assert!(gg_git::conflict::unmerged(p)?.is_empty());

    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let mut oversized = vec![b'a'; 2 * 1024 * 1024 + 1];
    std::fs::write(p.join("a"), &oversized)?;
    let snapshot = gg_git::conflict::snapshot(p, "a")?;
    oversized[100] = b'b';
    std::fs::write(p.join("a"), &oversized)?;
    assert!(
        gg_git::conflict::publish(p, "a", &snapshot, gg_engine::conflict::Resolution::Working,)
            .is_err(),
        "a changed byte in a large file must invalidate the preview"
    );
    Ok(())
}
