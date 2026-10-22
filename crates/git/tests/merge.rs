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

fn head(path: &Path) -> Result<String> {
    Ok(String::from_utf8(git(path, &["rev-parse", "HEAD"])?)?
        .trim()
        .to_string())
}

fn parents(path: &Path) -> Result<usize> {
    Ok(
        String::from_utf8(git(path, &["log", "-1", "--format=%P"])?)?
            .split_whitespace()
            .count(),
    )
}

#[test]
fn fast_forward_and_up_to_date() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("b"), b"feature\n")?;
    git(p, &["add", "b"])?;
    git(p, &["commit", "-qm", "feature"])?;
    git(p, &["switch", "-q", "main"])?;
    let base = head(p)?;

    let receipt = gg_git::merge::merge(p, "feature", false)?;
    assert!(receipt.applied);
    assert!(!receipt.in_progress);
    assert_eq!(receipt.kind, Some(gg_engine::merge::Kind::FastForward));
    assert_eq!(std::fs::read(p.join("b"))?, b"feature\n");
    assert_eq!(
        String::from_utf8(git(p, &["log", "-1", "--format=%s"])?)?,
        "feature\n"
    );

    let receipt = gg_git::merge::merge(p, "feature", false)?;
    assert!(receipt.applied);
    assert_eq!(receipt.kind, Some(gg_engine::merge::Kind::UpToDate));
    assert_ne!(head(p)?, base);
    Ok(())
}

#[test]
fn diverged_merge_creates_a_merge_commit() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("b"), b"feature\n")?;
    git(p, &["add", "b"])?;
    git(p, &["commit", "-qm", "feature"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("c"), b"main\n")?;
    git(p, &["add", "c"])?;
    git(p, &["commit", "-qm", "main"])?;

    let receipt = gg_git::merge::merge(p, "feature", false)?;
    assert!(receipt.applied);
    assert!(!receipt.in_progress);
    assert_eq!(receipt.kind, Some(gg_engine::merge::Kind::MergeCommit));
    assert_eq!(parents(p)?, 2);
    assert!(p.join("b").exists());
    assert!(p.join("c").exists());
    Ok(())
}

#[test]
fn no_ff_forces_a_merge_commit() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("b"), b"feature\n")?;
    git(p, &["add", "b"])?;
    git(p, &["commit", "-qm", "feature"])?;
    git(p, &["switch", "-q", "main"])?;

    let receipt = gg_git::merge::merge(p, "feature", true)?;
    assert!(receipt.applied);
    assert_eq!(receipt.kind, Some(gg_engine::merge::Kind::MergeCommit));
    assert_eq!(parents(p)?, 2);
    Ok(())
}

#[test]
fn conflict_can_abort_and_continue() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("a"), b"feature\n")?;
    git(p, &["commit", "-qam", "feature"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("a"), b"main\n")?;
    git(p, &["commit", "-qam", "main"])?;
    let main_head = head(p)?;

    let receipt = gg_git::merge::merge(p, "feature", false)?;
    assert!(!receipt.applied);
    assert!(receipt.in_progress);
    assert_eq!(receipt.conflicts, vec!["a".to_string()]);
    assert_eq!(
        gg_git::mutation::state(p)?.map(|state| state.operation),
        Some(gg_engine::mutation::Operation::Merge)
    );

    gg_git::mutation::abort_operation(p, gg_engine::mutation::Operation::Merge)?;
    assert!(gg_git::mutation::state(p)?.is_none());
    assert_eq!(head(p)?, main_head);
    assert_eq!(std::fs::read(p.join("a"))?, b"main\n");

    gg_git::merge::merge(p, "feature", false)?;
    std::fs::write(p.join("a"), b"resolved\n")?;
    git(p, &["add", "a"])?;
    let continued = gg_git::mutation::continue_operation(p, gg_engine::mutation::Operation::Merge)?;
    assert!(continued.applied);
    assert!(!continued.in_progress);
    assert!(gg_git::mutation::state(p)?.is_none());
    assert_eq!(parents(p)?, 2);
    assert_eq!(std::fs::read(p.join("a"))?, b"resolved\n");
    Ok(())
}

#[test]
fn rejects_option_shaped_and_missing_references() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let error = match gg_git::merge::merge(p, "--abort", false) {
        Ok(_) => return Err("option-shaped merge unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("must not start"), "{error}");
    let error = match gg_git::merge::merge(p, "missing", false) {
        Ok(_) => return Err("missing merge unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("was not found"), "{error}");
    Ok(())
}
