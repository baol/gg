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

fn subject(path: &Path) -> Result<String> {
    Ok(
        String::from_utf8(git(path, &["log", "-1", "--format=%s"])?)?
            .trim()
            .to_string(),
    )
}

fn diverged() -> Result<(tempfile::TempDir, String, String)> {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("b"), b"feature\n")?;
    git(p, &["add", "b"])?;
    git(p, &["commit", "-qm", "feature work"])?;
    let previous = head(p)?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("c"), b"main\n")?;
    git(p, &["add", "c"])?;
    git(p, &["commit", "-qm", "main work"])?;
    let main = head(p)?;
    git(p, &["switch", "-q", "feature"])?;
    Ok((dir, previous, main))
}

#[test]
fn rebase_rewrites_the_branch_and_records_the_previous_tip() -> Result {
    let (dir, previous, main) = diverged()?;
    let p = dir.path();
    let receipt = gg_git::rebase::rebase(p, "main", false)?;
    assert!(receipt.applied);
    assert!(!receipt.in_progress);
    assert_eq!(receipt.reference, "main");
    assert_eq!(receipt.commit.as_deref(), Some(main.as_str()));
    assert_eq!(receipt.previous.as_deref(), Some(previous.as_str()));
    let backup = receipt.backup.as_deref().ok_or("missing backup ref")?;
    assert!(backup.starts_with("refs/gg/backup/rebase/"), "{backup}");
    assert_eq!(
        String::from_utf8(git(p, &["rev-parse", backup])?)?.trim(),
        previous
    );
    assert_eq!(
        String::from_utf8(git(p, &["rev-parse", "HEAD^"])?)?.trim(),
        main
    );
    assert_eq!(subject(p)?, "feature work");
    assert_eq!(std::fs::read(p.join("b"))?, b"feature\n");
    assert_eq!(std::fs::read(p.join("c"))?, b"main\n");
    assert_eq!(
        String::from_utf8(git(p, &["log", "-1", "--format=%P"])?)?
            .split_whitespace()
            .count(),
        1
    );
    Ok(())
}

#[test]
fn rebase_accepts_tags_and_remote_tracking_refs() -> Result {
    let (dir, previous, main) = diverged()?;
    let p = dir.path();
    git(p, &["tag", "v1", main.as_str()])?;
    git(
        p,
        &["update-ref", "refs/remotes/origin/main", main.as_str()],
    )?;

    let receipt = gg_git::rebase::rebase(p, "v1", false)?;
    assert!(receipt.applied);
    assert_eq!(receipt.commit.as_deref(), Some(main.as_str()));
    assert_eq!(
        String::from_utf8(git(p, &["rev-parse", "HEAD^"])?)?.trim(),
        main
    );
    assert_ne!(head(p)?, previous);

    let current = head(p)?;
    let receipt = gg_git::rebase::rebase(p, "origin/main", false)?;
    assert!(receipt.applied);
    assert_eq!(receipt.commit.as_deref(), Some(main.as_str()));
    assert_eq!(head(p)?, current);
    Ok(())
}

#[test]
fn rebase_conflicts_can_abort_and_continue() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("a"), b"feature\n")?;
    git(p, &["commit", "-qam", "feature change"])?;
    let previous = head(p)?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("a"), b"main\n")?;
    git(p, &["commit", "-qam", "main change"])?;
    let main = head(p)?;
    git(p, &["switch", "-q", "feature"])?;

    let receipt = gg_git::rebase::rebase(p, "main", false)?;
    assert!(!receipt.applied);
    assert!(receipt.in_progress);
    assert_eq!(receipt.conflicts, vec!["a".to_string()]);
    assert_eq!(
        gg_git::mutation::state(p)?.map(|state| state.operation),
        Some(gg_engine::mutation::Operation::Rebase)
    );

    let error = match gg_git::rebase::rebase(p, "main", false) {
        Ok(_) => return Err("nested rebase unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("already in progress"), "{error}");

    gg_git::mutation::abort_operation(p, gg_engine::mutation::Operation::Rebase)?;
    assert!(gg_git::mutation::state(p)?.is_none());
    assert_eq!(head(p)?, previous);
    assert_eq!(std::fs::read(p.join("a"))?, b"feature\n");

    gg_git::rebase::rebase(p, "main", false)?;
    std::fs::write(p.join("a"), b"resolved\n")?;
    git(p, &["add", "a"])?;
    let continued =
        gg_git::mutation::continue_operation(p, gg_engine::mutation::Operation::Rebase)?;
    assert!(continued.applied);
    assert!(!continued.in_progress);
    assert!(gg_git::mutation::state(p)?.is_none());
    assert_eq!(
        String::from_utf8(git(p, &["rev-parse", "HEAD^"])?)?.trim(),
        main
    );
    assert_eq!(subject(p)?, "feature change");
    assert_eq!(std::fs::read(p.join("a"))?, b"resolved\n");
    Ok(())
}

#[test]
fn rebase_conflict_can_skip_the_commit() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("a"), b"feature\n")?;
    git(p, &["commit", "-qam", "feature change"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("a"), b"main\n")?;
    git(p, &["commit", "-qam", "main change"])?;
    let main = head(p)?;
    git(p, &["switch", "-q", "feature"])?;

    let receipt = gg_git::rebase::rebase(p, "main", false)?;
    assert!(receipt.in_progress);
    let skipped = gg_git::mutation::skip_operation(p, gg_engine::mutation::Operation::Rebase)?;
    assert!(skipped.applied);
    assert!(!skipped.in_progress);
    assert_eq!(head(p)?, main);
    assert_eq!(std::fs::read(p.join("a"))?, b"main\n");
    assert!(gg_git::mutation::state(p)?.is_none());
    Ok(())
}

#[test]
fn rebase_requires_a_clean_worktree_unless_autostash_is_enabled() -> Result {
    let (dir, _, _) = diverged()?;
    let p = dir.path();
    let before = head(p)?;
    std::fs::write(p.join("b"), b"dirty\n")?;

    let error = match gg_git::rebase::rebase(p, "main", false) {
        Ok(_) => return Err("dirty rebase unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("not clean"), "{error}");
    assert_eq!(head(p)?, before);

    git(p, &["config", "rebase.autoStash", "true"])?;
    let error = match gg_git::rebase::rebase(p, "main", false) {
        Ok(_) => return Err("configured autostash bypassed the explicit decision".into()),
        Err(error) => error,
    };
    assert!(error.contains("not clean"), "{error}");
    assert_eq!(head(p)?, before);

    let receipt = gg_git::rebase::rebase(p, "main", true)?;
    assert!(receipt.applied);
    assert_eq!(std::fs::read(p.join("b"))?, b"dirty\n");
    assert_eq!(
        String::from_utf8(git(p, &["rev-parse", "HEAD^"])?)?.trim(),
        receipt.commit.ok_or("missing upstream commit")?
    );
    Ok(())
}

#[test]
fn rebase_rejects_option_shaped_and_missing_references() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let before = head(p)?;
    let error = match gg_git::rebase::rebase(p, "--abort", false) {
        Ok(_) => return Err("option-shaped rebase unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("must not start"), "{error}");
    let error = match gg_git::rebase::rebase(p, "missing", false) {
        Ok(_) => return Err("missing rebase unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("was not found"), "{error}");
    assert_eq!(head(p)?, before);
    Ok(())
}
