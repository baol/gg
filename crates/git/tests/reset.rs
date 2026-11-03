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
    init(dir.path())?;
    Ok(dir)
}

/// A repository at `path` with two commits that change `a`.
fn init(path: &Path) -> Result {
    git(path, &["init", "-b", "main"])?;
    git(path, &["config", "user.name", "Test"])?;
    git(path, &["config", "user.email", "test@example.org"])?;
    git(path, &["config", "core.autocrlf", "false"])?;
    std::fs::write(path.join("a"), b"one\n")?;
    git(path, &["add", "a"])?;
    git(path, &["commit", "-qm", "one"])?;
    std::fs::write(path.join("a"), b"two\n")?;
    git(path, &["commit", "-qam", "two"])?;
    Ok(())
}

fn head(path: &Path) -> Result<String> {
    Ok(String::from_utf8(git(path, &["rev-parse", "HEAD"])?)?
        .trim()
        .to_string())
}

#[test]
fn soft_reset_keeps_index_and_worktree() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let first = String::from_utf8(git(p, &["rev-parse", "HEAD^"])?)?
        .trim()
        .to_string();
    let receipt = gg_git::reset::reset(p, "HEAD^", gg_engine::reset::Mode::Soft)?;
    assert!(receipt.applied);
    assert_eq!(head(p)?, first);
    assert_eq!(std::fs::read(p.join("a"))?, b"two\n");
    assert_eq!(git(p, &["diff", "--cached", "--name-only"])?, b"a\n");
    Ok(())
}

#[test]
fn mixed_reset_clears_the_index_but_keeps_worktree() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let first = String::from_utf8(git(p, &["rev-parse", "HEAD^"])?)?
        .trim()
        .to_string();
    let receipt = gg_git::reset::reset(p, "HEAD^", gg_engine::reset::Mode::Mixed)?;
    assert!(receipt.applied);
    assert_eq!(head(p)?, first);
    assert_eq!(std::fs::read(p.join("a"))?, b"two\n");
    assert_eq!(git(p, &["diff", "--cached", "--name-only"])?, b"");
    assert_eq!(git(p, &["diff", "--name-only"])?, b"a\n");
    Ok(())
}

#[test]
fn hard_reset_discards_index_and_worktree() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let first = String::from_utf8(git(p, &["rev-parse", "HEAD^"])?)?
        .trim()
        .to_string();
    let receipt = gg_git::reset::reset(p, "HEAD^", gg_engine::reset::Mode::Hard)?;
    assert!(receipt.applied);
    assert_eq!(head(p)?, first);
    assert_eq!(std::fs::read(p.join("a"))?, b"one\n");
    assert_eq!(git(p, &["status", "--porcelain"])?, b"");
    Ok(())
}

#[test]
fn rejects_option_shaped_and_missing_targets() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let error = match gg_git::reset::reset(p, "--hard", gg_engine::reset::Mode::Hard) {
        Ok(_) => return Err("option-shaped reset unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("must be a revision"), "{error}");
    let error = match gg_git::reset::reset(p, "missing", gg_engine::reset::Mode::Soft) {
        Ok(_) => return Err("missing reset unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("was not found"), "{error}");
    Ok(())
}

#[test]
fn hard_reset_refuses_to_overwrite_untracked_and_ignored_files() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("b"), b"tracked later\n")?;
    std::fs::write(p.join("c"), b"tracked later\n")?;
    git(p, &["add", "b", "c"])?;
    git(p, &["commit", "-qm", "three"])?;
    let target = head(p)?;
    git(p, &["rm", "-q", "--cached", "b", "c"])?;
    git(p, &["commit", "-qm", "four"])?;
    let before = head(p)?;
    std::fs::write(p.join("b"), b"my untracked notes\n")?;
    std::fs::write(p.join(".gitignore"), b"c\n")?;
    std::fs::write(p.join("c"), b"my ignored notes\n")?;
    std::fs::write(p.join("d"), b"untracked, not in the target\n")?;
    assert_eq!(
        gg_git::reset::overwritten_untracked(p, &target)?,
        vec!["b".to_string(), "c".to_string()]
    );
    let error = match gg_git::reset::reset(p, &target, gg_engine::reset::Mode::Hard) {
        Ok(_) => return Err("hard reset unexpectedly overwrote untracked files".into()),
        Err(error) => error,
    };
    assert!(error.contains("untracked"), "{error}");
    assert!(error.contains("\nb\n") && error.ends_with("\nc"), "{error}");
    assert_eq!(head(p)?, before, "nothing changed");
    assert_eq!(std::fs::read(p.join("b"))?, b"my untracked notes\n");
    assert_eq!(std::fs::read(p.join("c"))?, b"my ignored notes\n");
    let receipt = gg_git::reset::reset(p, &target, gg_engine::reset::Mode::Mixed)?;
    assert!(receipt.applied, "soft and mixed leave working files alone");
    assert_eq!(std::fs::read(p.join("b"))?, b"my untracked notes\n");
    Ok(())
}

/// Commit `name` in a third commit, then untrack it again, so a hard reset
/// back to that commit would write `name` from the tree.
#[cfg(not(windows))]
fn tracked_then_untracked(p: &Path, name: &std::ffi::OsStr, pattern: &str) -> Result<String> {
    std::fs::write(p.join(name), b"tracked later\n")?;
    git(p, &["add", "-A"])?;
    git(p, &["commit", "-qm", "three"])?;
    let target = head(p)?;
    git(p, &["rm", "-q", "--cached", "--", pattern])?;
    git(p, &["commit", "-qm", "four"])?;
    std::fs::write(p.join(name), b"my untracked notes\n")?;
    Ok(target)
}

// EXP-01: a name that is not UTF-8 is still found on disk.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn hard_reset_refuses_to_overwrite_an_untracked_file_whose_name_is_not_utf8() -> Result {
    use std::os::unix::ffi::OsStrExt;
    let dir = repo()?;
    let p = dir.path();
    let name = std::ffi::OsStr::from_bytes(b"raw\xff.txt");
    let target = tracked_then_untracked(p, name, "raw*.txt")?;
    let before = head(p)?;
    assert_eq!(
        gg_git::reset::overwritten_untracked(p, &target)?,
        vec!["raw\u{fffd}.txt".to_string()],
        "listed for display"
    );
    let error = match gg_git::reset::reset(p, &target, gg_engine::reset::Mode::Hard) {
        Ok(_) => return Err("hard reset unexpectedly overwrote a non-UTF-8 file".into()),
        Err(error) => error,
    };
    assert!(error.contains("untracked"), "{error}");
    assert_eq!(head(p)?, before, "nothing changed");
    assert_eq!(std::fs::read(p.join(name))?, b"my untracked notes\n");
    Ok(())
}

// EXP-02: the working tree's own trailing space is kept.
#[cfg(not(windows))]
#[test]
fn hard_reset_refuses_to_overwrite_untracked_files_in_a_folder_ending_with_a_space() -> Result {
    let parent = tempfile::tempdir()?;
    let p = parent.path().join("repo ");
    std::fs::create_dir(&p)?;
    init(&p)?;
    let target = tracked_then_untracked(&p, std::ffi::OsStr::new("b"), "b")?;
    let before = head(&p)?;
    assert_eq!(
        gg_git::reset::overwritten_untracked(&p, &target)?,
        vec!["b".to_string()]
    );
    let error = match gg_git::reset::reset(&p, &target, gg_engine::reset::Mode::Hard) {
        Ok(_) => return Err("hard reset unexpectedly overwrote an untracked file".into()),
        Err(error) => error,
    };
    assert!(error.ends_with("\nb"), "{error}");
    assert_eq!(head(&p)?, before, "nothing changed");
    assert_eq!(std::fs::read(p.join("b"))?, b"my untracked notes\n");
    Ok(())
}

// EXP-05: every mode saves the tip it moves away from first.
#[test]
fn every_reset_mode_saves_the_previous_tip_as_a_backup() -> Result {
    for mode in [
        gg_engine::reset::Mode::Soft,
        gg_engine::reset::Mode::Mixed,
        gg_engine::reset::Mode::Hard,
    ] {
        let dir = repo()?;
        let p = dir.path();
        let tip = head(p)?;
        let receipt = gg_git::reset::reset(p, "HEAD^", mode)?;
        let backup = receipt.backup.ok_or("the receipt names no backup")?;
        let backups = gg_git::backup::list(p)?;
        assert_eq!(backups.len(), 1, "{mode:?}");
        assert_eq!(backups[0].reference, backup);
        assert_eq!(backups[0].label, "reset");
        assert_eq!(backups[0].commit, tip, "{mode:?}");
        assert_eq!(backups[0].branch.as_deref(), Some("main"));
    }
    Ok(())
}

#[test]
fn reset_fails_without_moving_when_the_backup_cannot_be_written() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let before = head(p)?;
    // A file where the backup folder belongs makes `update-ref` fail.
    std::fs::write(p.join(".git/refs/gg"), b"in the way\n")?;
    let error = match gg_git::reset::reset(p, "HEAD^", gg_engine::reset::Mode::Hard) {
        Ok(_) => return Err("reset ran without a backup".into()),
        Err(error) => error,
    };
    assert!(error.contains("backup"), "{error}");
    assert_eq!(head(p)?, before, "nothing changed");
    assert_eq!(std::fs::read(p.join("a"))?, b"two\n");
    Ok(())
}
