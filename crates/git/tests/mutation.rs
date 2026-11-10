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

fn hook(path: &Path, name: &str, body: &str) -> Result {
    let hooks = path.join(".git/hooks");
    std::fs::create_dir_all(&hooks)?;
    let file = hooks.join(name);
    std::fs::write(&file, format!("#!/bin/sh\n{body}\n"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
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
fn several_commits_are_picked_oldest_first_in_one_cherry_pick() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature"])?;
    for name in ["b", "c"] {
        std::fs::write(p.join(name), format!("{name}\n"))?;
        git(p, &["add", name])?;
        git(p, &["commit", "-qm", &format!("add {name}")])?;
    }
    git(p, &["switch", "-q", "main"])?;
    let receipt = gg_git::mutation::cherry_pick_many(p, &["feature~1", "feature"])?;
    assert!(receipt.applied && !receipt.in_progress);
    assert_eq!(
        String::from_utf8(git(p, &["log", "--format=%s", "-3"])?)?,
        "add c\nadd b\nbase\n"
    );
    assert!(gg_git::mutation::cherry_pick_many(p, &[]).is_err());
    assert!(gg_git::mutation::cherry_pick_many(p, &["feature", "--abort"]).is_err());
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
fn recovery_failures_are_errors() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let error =
        match gg_git::mutation::abort_operation(p, gg_engine::mutation::Operation::CherryPick) {
            Ok(_) => return Err("abort without an operation unexpectedly succeeded".into()),
            Err(error) => error,
        };
    assert!(error.contains("Abort cherry-pick failed"), "{error}");

    git(p, &["switch", "-qc", "other"])?;
    std::fs::write(p.join("a"), b"other\n")?;
    git(p, &["commit", "-qam", "other"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("a"), b"main\n")?;
    git(p, &["commit", "-qam", "main"])?;
    gg_git::mutation::cherry_pick(p, "other")?;
    std::fs::write(p.join("a"), b"resolved\n")?;
    git(p, &["add", "a"])?;
    hook(p, "commit-msg", "echo hook-blocked >&2\nexit 7")?;

    let error =
        match gg_git::mutation::continue_operation(p, gg_engine::mutation::Operation::CherryPick) {
            Ok(_) => return Err("continue with a failing hook unexpectedly succeeded".into()),
            Err(error) => error,
        };
    assert!(error.contains("hook-blocked"), "{error}");
    assert!(gg_git::mutation::state(p)?.is_some());
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

/// `main` with `topic` (files `t`, `t2`) merged in by a merge commit, after
/// `main` gained `c` (Pro Git 7.8, "Undoing Merges").
fn merged_topic() -> Result<tempfile::TempDir> {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "topic"])?;
    for name in ["t", "t2"] {
        std::fs::write(p.join(name), b"topic\n")?;
        git(p, &["add", name])?;
        git(p, &["commit", "-qm", &format!("topic {name}")])?;
    }
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("c"), b"main\n")?;
    git(p, &["add", "c"])?;
    git(p, &["commit", "-qm", "main c"])?;
    git(
        p,
        &[
            "merge",
            "-q",
            "--no-ff",
            "topic",
            "-m",
            "Merge branch 'topic'",
        ],
    )?;
    Ok(dir)
}

/// INT-10: a merge commit is not cherry-picked; nothing changes.
#[test]
fn cherry_pick_of_a_merge_is_refused() -> Result {
    let dir = merged_topic()?;
    let p = dir.path();
    let merge = String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?;
    git(p, &["switch", "-qc", "other", "HEAD~1"])?;
    let before = String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?;
    let error = gg_git::mutation::cherry_pick(p, merge.trim()).err();
    assert_eq!(
        error.as_deref(),
        Some("Use the Merge action for merge operations.")
    );
    let error = gg_git::mutation::cherry_pick_many(p, &["main~1", merge.trim()]).err();
    assert_eq!(
        error.as_deref(),
        Some("Use the Merge action for merge operations.")
    );
    assert_eq!(String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?, before);
    assert_eq!(status(p)?, "");
    Ok(())
}

/// INT-43: Revert of a merge undoes what the merge brought in, against its
/// first parent (`git revert -m 1`), and says that merging the branch again
/// will not bring those commits back.
#[test]
fn reverting_a_merge_undoes_what_it_brought_in() -> Result {
    let dir = merged_topic()?;
    let p = dir.path();
    let receipt = gg_git::mutation::revert(p, "HEAD")?;
    assert!(receipt.applied, "{receipt:?}");
    assert_eq!(subject(p)?, "Revert \"Merge branch 'topic'\"");
    assert!(
        !p.join("t").exists() && !p.join("t2").exists(),
        "topic's files are gone"
    );
    assert!(p.join("c").exists(), "main's own work stays");
    assert_eq!(status(p)?, "");
    assert!(
        receipt
            .warnings
            .iter()
            .any(|warning| warning.contains("again will not bring")),
        "{:?}",
        receipt.warnings
    );
    Ok(())
}
