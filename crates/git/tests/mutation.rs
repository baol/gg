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

fn subject(path: &Path) -> Result<String> {
    Ok(
        String::from_utf8(git(path, &["log", "-1", "--format=%s"])?)?
            .trim()
            .to_string(),
    )
}

fn status(path: &Path) -> Result<String> {
    Ok(String::from_utf8(git(path, &["status", "--porcelain"])?)?)
}

#[test]
fn cherry_pick_applies_a_commit() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("b"), b"feature\n")?;
    git(p, &["add", "b"])?;
    git(p, &["commit", "-qm", "feature work"])?;
    git(p, &["switch", "-q", "main"])?;
    let base = String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?;

    let receipt = gg_git::mutation::cherry_pick(p, "feature")?;
    assert!(receipt.applied);
    assert!(!receipt.in_progress);
    assert!(receipt.conflicts.is_empty());
    assert_eq!(std::fs::read(p.join("b"))?, b"feature\n");
    assert_eq!(
        String::from_utf8(git(p, &["rev-parse", "HEAD^"])?)?,
        base,
        "cherry-pick parent"
    );
    assert_eq!(subject(p)?, "feature work");
    assert!(gg_git::mutation::state(p)?.is_none());
    Ok(())
}

#[test]
fn cherry_pick_conflict_can_abort_and_continue() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "other"])?;
    std::fs::write(p.join("a"), b"other\n")?;
    git(p, &["commit", "-qam", "other"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("a"), b"main\n")?;
    git(p, &["commit", "-qam", "main"])?;
    let main_head = String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?;

    let receipt = gg_git::mutation::cherry_pick(p, "other")?;
    assert!(!receipt.applied);
    assert!(receipt.in_progress);
    assert_eq!(receipt.conflicts, vec!["a".to_string()]);
    assert!(status(p)?.contains("UU a"));
    assert!(gg_git::mutation::state(p)?.is_some());

    let aborted = gg_git::mutation::abort_operation(p, gg_engine::mutation::Operation::CherryPick)?;
    assert!(!aborted.in_progress);
    assert!(gg_git::mutation::state(p)?.is_none());
    assert_eq!(git(p, &["rev-parse", "HEAD"])?, main_head.as_bytes());
    assert_eq!(std::fs::read(p.join("a"))?, b"main\n");

    gg_git::mutation::cherry_pick(p, "other")?;
    std::fs::write(p.join("a"), b"resolved\n")?;
    git(p, &["add", "a"])?;
    let continued =
        gg_git::mutation::continue_operation(p, gg_engine::mutation::Operation::CherryPick)?;
    assert!(continued.applied);
    assert!(!continued.in_progress);
    assert!(gg_git::mutation::state(p)?.is_none());
    assert_eq!(subject(p)?, "other");
    Ok(())
}

#[test]
fn revert_inverts_a_commit_and_can_skip_a_conflict() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"one\n")?;
    git(p, &["commit", "-qam", "one"])?;
    let first = String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?;
    std::fs::write(p.join("a"), b"two\n")?;
    git(p, &["commit", "-qam", "two"])?;

    let receipt = gg_git::mutation::revert(p, first.trim())?;
    assert!(!receipt.applied);
    assert!(receipt.in_progress);
    assert_eq!(receipt.conflicts, vec!["a".to_string()]);
    assert!(status(p)?.contains("UU a"));
    assert!(gg_git::mutation::state(p)?.is_some());

    let skipped = gg_git::mutation::skip_operation(p, gg_engine::mutation::Operation::Revert)?;
    assert!(!skipped.in_progress);
    assert!(gg_git::mutation::state(p)?.is_none());
    assert_eq!(std::fs::read(p.join("a"))?, b"two\n");
    assert_eq!(subject(p)?, "two");

    let clean = gg_git::mutation::revert(p, "HEAD")?;
    assert!(clean.applied);
    assert!(!clean.in_progress);
    assert_eq!(std::fs::read(p.join("a"))?, b"one\n");
    assert!(subject(p)?.starts_with("Revert"));
    Ok(())
}

#[test]
fn rejects_option_shaped_and_missing_commits() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let error = match gg_git::mutation::cherry_pick(p, "--abort") {
        Ok(_) => return Err("option-shaped cherry-pick unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("must be a revision"), "{error}");
    let error = match gg_git::mutation::revert(p, "missing") {
        Ok(_) => return Err("missing revert unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("was not found"), "{error}");
    assert!(gg_git::mutation::state(p)?.is_none());
    Ok(())
}
