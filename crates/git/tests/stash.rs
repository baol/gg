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

fn status(path: &Path) -> Result<String> {
    Ok(String::from_utf8(git(path, &["status", "--porcelain"])?)?)
}

#[test]
fn push_list_apply_pop_and_drop_match_git() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"modified\n")?;

    let receipt = gg_git::stash::push(p, Some("work in progress"), false, false)?;
    assert!(receipt.created);
    assert_eq!(receipt.reference.as_deref(), Some("stash@{0}"));
    assert!(
        receipt
            .message
            .as_deref()
            .is_some_and(|message| message.contains("work in progress"))
    );
    assert_eq!(std::fs::read(p.join("a"))?, b"base\n");
    assert_eq!(status(p)?, "");

    let list = gg_git::stash::list(p)?;
    assert_eq!(list.entries.len(), 1);
    assert_eq!(list.entries[0].index, 0);
    assert_eq!(list.entries[0].reference, "stash@{0}");
    assert_eq!(list.entries[0].message, "On main: work in progress");
    assert_eq!(
        git(p, &["stash", "list", "--format=%H"])?,
        format!("{}\n", list.entries[0].commit).as_bytes()
    );

    let applied = gg_git::stash::apply(p, None, false)?;
    assert!(applied.applied);
    assert!(!applied.dropped);
    assert!(applied.conflicts.is_empty());
    assert_eq!(std::fs::read(p.join("a"))?, b"modified\n");
    assert_eq!(gg_git::stash::list(p)?.entries.len(), 1);

    std::fs::write(p.join("a"), b"base\n")?;
    let popped = gg_git::stash::pop(p, None, false)?;
    assert!(popped.applied);
    assert!(popped.dropped);
    assert_eq!(std::fs::read(p.join("a"))?, b"modified\n");
    assert!(gg_git::stash::list(p)?.entries.is_empty());

    std::fs::write(p.join("a"), b"again\n")?;
    gg_git::stash::push(p, Some("again"), false, false)?;
    let dropped = gg_git::stash::drop(p, None)?;
    assert!(dropped.dropped);
    assert!(gg_git::stash::list(p)?.entries.is_empty());
    Ok(())
}

#[test]
fn push_supports_message_untracked_and_keep_index() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"staged\n")?;
    git(p, &["add", "a"])?;
    std::fs::write(p.join("u"), b"untracked\n")?;

    let receipt = gg_git::stash::push(p, Some("keep"), true, true)?;
    assert!(receipt.created);
    assert!(!p.join("u").exists(), "untracked file was not stashed");
    let porcelain = status(p)?;
    assert!(porcelain.contains("M  a"), "{porcelain:?}");
    assert_eq!(std::fs::read(p.join("a"))?, b"staged\n");

    let applied = gg_git::stash::apply(p, Some("stash@{0}"), true)?;
    assert!(applied.applied);
    assert!(p.join("u").exists());
    assert_eq!(std::fs::read(p.join("u"))?, b"untracked\n");
    assert_eq!(std::fs::read(p.join("a"))?, b"staged\n");
    Ok(())
}

#[test]
fn apply_conflicts_keep_the_stash_and_report_paths() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "other"])?;
    std::fs::write(p.join("a"), b"other\n")?;
    git(p, &["commit", "-qam", "other"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("a"), b"main\n")?;
    gg_git::stash::push(p, Some("conflict"), false, false)?;
    git(p, &["switch", "-q", "other"])?;

    let receipt = gg_git::stash::apply(p, None, false)?;
    assert!(receipt.applied);
    assert!(!receipt.dropped);
    assert_eq!(receipt.conflicts, vec!["a".to_string()]);
    assert_eq!(gg_git::stash::list(p)?.entries.len(), 1);
    assert!(status(p)?.contains("UU a"));
    Ok(())
}

#[test]
fn pop_conflicts_do_not_drop_the_stash() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "other"])?;
    std::fs::write(p.join("a"), b"other\n")?;
    git(p, &["commit", "-qam", "other"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("a"), b"main\n")?;
    gg_git::stash::push(p, Some("conflict"), false, false)?;
    git(p, &["switch", "-q", "other"])?;

    let receipt = gg_git::stash::pop(p, None, false)?;
    assert!(receipt.applied);
    assert!(!receipt.dropped);
    assert_eq!(receipt.conflicts, vec!["a".to_string()]);
    assert_eq!(gg_git::stash::list(p)?.entries.len(), 1);
    Ok(())
}

#[test]
fn rejects_clean_trees_and_invalid_references() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let error = match gg_git::stash::push(p, None, false, false) {
        Ok(_) => return Err("clean stash push unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("No local changes"), "{error}");

    std::fs::write(p.join("a"), b"modified\n")?;
    gg_git::stash::push(p, None, false, false)?;
    let error = match gg_git::stash::apply(p, Some("--force"), false) {
        Ok(_) => return Err("option-shaped stash unexpectedly applied".into()),
        Err(error) => error,
    };
    assert!(error.contains("must look like"), "{error}");
    let error = match gg_git::stash::drop(p, Some("stash@{99}")) {
        Ok(_) => return Err("missing stash unexpectedly dropped".into()),
        Err(error) => error,
    };
    assert!(error.contains("Drop stash failed"), "{error}");
    Ok(())
}

#[test]
fn lists_newest_first_and_preserves_unrelated_files() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("keep"), b"keep\n")?;
    git(p, &["add", "keep"])?;
    git(p, &["commit", "-qm", "keep"])?;
    std::fs::write(p.join("a"), b"first\n")?;
    gg_git::stash::push(p, Some("first"), false, false)?;
    std::fs::write(p.join("a"), b"second\n")?;
    gg_git::stash::push(p, Some("second"), false, false)?;

    let list = gg_git::stash::list(p)?;
    assert_eq!(list.entries.len(), 2);
    assert_eq!(list.entries[0].index, 0);
    assert_eq!(list.entries[1].index, 1);
    assert!(list.entries[0].message.contains("second"));
    assert!(list.entries[1].message.contains("first"));
    assert_eq!(std::fs::read(p.join("keep"))?, b"keep\n");
    Ok(())
}

#[test]
fn has_changes_sees_staged_unstaged_and_untracked_work() -> Result {
    let dir = repo()?;
    let path = dir.path();
    assert!(
        !gg_git::stash::has_changes(path)?,
        "a clean worktree has nothing to stash"
    );
    std::fs::write(path.join("new"), b"new\n")?;
    assert!(gg_git::stash::has_changes(path)?, "untracked files count");
    git(path, &["add", "new"])?;
    assert!(gg_git::stash::has_changes(path)?, "staged files count");
    git(path, &["commit", "-qm", "new"])?;
    git(path, &["mv", "new", "renamed"])?;
    std::fs::write(path.join("extra"), b"extra\n")?;
    assert_eq!(
        gg_git::stash::change_count(path)?,
        2,
        "a rename counts once, plus the untracked file"
    );
    Ok(())
}

#[test]
fn entries_name_their_branch_age_and_file_count() -> Result {
    let dir = repo()?;
    let p = dir.path();
    // Tracked and untracked changes on main.
    std::fs::write(p.join("a"), b"changed\n")?;
    std::fs::write(p.join("b"), b"new\n")?;
    git(p, &["add", "b"])?;
    std::fs::write(p.join("untracked"), b"u\n")?;
    git(p, &["stash", "push", "-q", "-u", "-m", "work on main"])?;
    // One tracked change on a detached HEAD.
    git(p, &["switch", "-q", "--detach"])?;
    std::fs::write(p.join("a"), b"detached\n")?;
    git(p, &["stash", "push", "-q"])?;

    let list = gg_git::stash::list_with_files(p)?;
    assert_eq!(list.entries.len(), 2);
    let detached = &list.entries[0];
    let main = &list.entries[1];
    assert_eq!(main.branch.as_deref(), Some("main"));
    assert_eq!(main.files, Some(3), "a, b, and the untracked file");
    assert!(!main.age.is_empty(), "{main:?}");
    assert_eq!(detached.branch, None);
    assert_eq!(detached.files, Some(1));

    // The plain list stays cheap and leaves the counts out.
    let plain = gg_git::stash::list(p)?;
    assert_eq!(plain.entries[1].branch.as_deref(), Some("main"));
    assert_eq!(plain.entries[1].files, None);
    Ok(())
}
