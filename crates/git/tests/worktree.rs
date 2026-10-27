use gg_engine::worktree::AddRequest;
use std::{error::Error, path::Path, process::Command};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

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

#[test]
fn lists_main_and_linked_worktrees_with_branch_state() -> Result {
    let dir = repo()?;
    let linked = tempfile::tempdir()?;
    let destination = linked.path().join("linked");
    let receipt = gg_git::worktree::add(
        dir.path(),
        &AddRequest {
            destination: destination.clone(),
            branch: None,
            create: Some("feature".into()),
            at: None,
        },
    )?;
    assert_eq!(receipt.branch.as_deref(), Some("feature"));
    let snapshot = gg_git::worktree::list(dir.path())?;
    assert_eq!(snapshot.worktrees.len(), 2);
    assert!(snapshot.worktrees[0].main);
    assert!(snapshot.worktrees[0].current);
    assert_eq!(snapshot.worktrees[0].branch.as_deref(), Some("main"));
    let feature = snapshot
        .worktrees
        .iter()
        .find(|worktree| worktree.branch.as_deref() == Some("feature"))
        .ok_or("missing feature worktree")?;
    assert!(!feature.current);
    assert!(!feature.main);
    assert_eq!(feature.dirty, Some(false));
    assert!(feature.can_remove().is_ok());
    Ok(())
}

#[test]
fn add_rejects_checked_out_branches_and_nonempty_destinations() -> Result {
    let dir = repo()?;
    let linked = tempfile::tempdir()?;
    let destination = linked.path().join("linked");
    gg_git::worktree::add(
        dir.path(),
        &AddRequest {
            destination: destination.clone(),
            branch: None,
            create: Some("feature".into()),
            at: None,
        },
    )?;
    let result = gg_git::worktree::add(
        dir.path(),
        &AddRequest {
            destination: linked.path().join("second"),
            branch: Some("feature".into()),
            create: None,
            at: None,
        },
    );
    assert!(result.is_err(), "checked-out branch must be rejected");
    if let Err(error) = result {
        assert!(error.contains("already checked out"), "{error}");
    }
    let result = gg_git::worktree::add(
        dir.path(),
        &AddRequest {
            destination: destination.clone(),
            branch: None,
            create: Some("other".into()),
            at: None,
        },
    );
    assert!(result.is_err(), "nonempty destination must be rejected");
    if let Err(error) = result {
        assert!(error.contains("not empty"), "{error}");
    }
    Ok(())
}

#[test]
fn remove_blocks_main_current_and_dirty_worktrees_then_removes_a_clean_one() -> Result {
    let dir = repo()?;
    let linked = tempfile::tempdir()?;
    let destination = linked.path().join("linked");
    gg_git::worktree::add(
        dir.path(),
        &AddRequest {
            destination: destination.clone(),
            branch: None,
            create: Some("feature".into()),
            at: None,
        },
    )?;
    let main = gg_git::worktree::remove(dir.path(), dir.path());
    assert!(main.is_err(), "main worktree must not be removable");
    if let Err(error) = main {
        assert!(error.contains("main worktree"), "{error}");
    }

    std::fs::write(destination.join("a"), b"dirty\n")?;
    let dirty = gg_git::worktree::remove(dir.path(), &destination);
    assert!(dirty.is_err(), "dirty worktree must not be removable");
    if let Err(error) = dirty {
        assert!(error.contains("local changes"), "{error}");
    }
    std::fs::write(destination.join("a"), b"base\n")?;

    let current = gg_git::worktree::remove(dir.path(), dir.path());
    assert!(current.is_err(), "current worktree must not be removable");
    if let Err(error) = current {
        assert!(error.contains("main worktree"), "{error}");
    }

    let removed = gg_git::worktree::remove(dir.path(), &destination)?;
    assert_eq!(removed.branch.as_deref(), Some("feature"));
    assert_eq!(gg_git::worktree::list(dir.path())?.worktrees.len(), 1);
    Ok(())
}

#[test]
fn add_existing_branch_and_list_current_worktree() -> Result {
    let dir = repo()?;
    git(dir.path(), &["branch", "existing"])?;
    let linked = tempfile::tempdir()?;
    let destination = linked.path().join("linked");
    let receipt = gg_git::worktree::add(
        dir.path(),
        &AddRequest {
            destination: destination.clone(),
            branch: Some("existing".into()),
            create: None,
            at: None,
        },
    )?;
    assert_eq!(receipt.branch.as_deref(), Some("existing"));
    let snapshot = gg_git::worktree::list(&destination)?;
    let current = snapshot
        .worktrees
        .iter()
        .find(|worktree| worktree.current)
        .ok_or("missing current worktree")?;
    assert_eq!(current.branch.as_deref(), Some("existing"));
    Ok(())
}
