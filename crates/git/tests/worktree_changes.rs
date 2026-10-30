use std::{error::Error, path::Path, process::Command};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn git(path: &Path, args: &[&str]) -> Result<String> {
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
    Ok(String::from_utf8(output.stdout)?)
}

fn init(path: &Path) -> Result {
    git(path, &["init", "-q", "-b", "main"])?;
    git(path, &["config", "user.name", "Test"])?;
    git(path, &["config", "user.email", "test@example.org"])?;
    Ok(())
}

fn repo() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    init(p)?;
    std::fs::write(p.join("kept"), b"kept\n")?;
    std::fs::write(p.join("gone"), b"gone\n")?;
    git(p, &["add", "."])?;
    git(p, &["commit", "-qm", "base"])?;
    Ok(dir)
}

fn status(path: &Path) -> Result<Vec<String>> {
    let mut lines = git(path, &["status", "--porcelain"])?
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    lines.sort();
    Ok(lines)
}

#[test]
fn stage_all_includes_modified_untracked_and_deleted_files() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("kept"), b"changed\n")?;
    std::fs::remove_file(p.join("gone"))?;
    std::fs::create_dir(p.join("sub"))?;
    std::fs::write(p.join("sub/new"), b"new\n")?;
    gg_git::worktree_changes::stage_all(&p.join("sub"))?;
    assert_eq!(status(p)?, ["A  sub/new", "D  gone", "M  kept"]);
    Ok(())
}

#[test]
fn unstage_all_keeps_working_files_on_born_and_unborn_branches() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("kept"), b"changed\n")?;
    std::fs::write(p.join("new"), b"new\n")?;
    git(p, &["add", "-A"])?;
    gg_git::worktree_changes::unstage_all(p)?;
    assert_eq!(status(p)?, [" M kept", "?? new"]);

    let unborn = tempfile::tempdir()?;
    let u = unborn.path();
    init(u)?;
    std::fs::write(u.join("first"), b"first\n")?;
    git(u, &["add", "first"])?;
    gg_git::worktree_changes::unstage_all(u)?;
    assert_eq!(status(u)?, ["?? first"]);
    assert_eq!(std::fs::read(u.join("first"))?, b"first\n");
    Ok(())
}

#[test]
fn discard_all_restores_tracked_files_and_keeps_untracked() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("kept"), b"changed\n")?;
    std::fs::remove_file(p.join("gone"))?;
    std::fs::write(p.join("staged"), b"staged\n")?;
    git(p, &["add", "staged"])?;
    std::fs::write(p.join("untracked"), b"untracked\n")?;
    gg_git::worktree_changes::discard_all(p)?;
    assert_eq!(std::fs::read(p.join("kept"))?, b"kept\n");
    assert_eq!(std::fs::read(p.join("gone"))?, b"gone\n");
    // A newly added file leaves the index; its bytes and untracked files stay.
    assert_eq!(status(p)?, ["?? staged", "?? untracked"]);
    assert_eq!(std::fs::read(p.join("staged"))?, b"staged\n");
    Ok(())
}

#[test]
fn discard_all_refuses_during_a_rebase() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let base = git(p, &["rev-parse", "HEAD"])?.trim().to_string();
    std::fs::write(p.join("kept"), b"one\n")?;
    git(p, &["commit", "-qam", "one"])?;
    git(p, &["switch", "-qc", "other", &base])?;
    std::fs::write(p.join("kept"), b"two\n")?;
    git(p, &["commit", "-qam", "two"])?;
    assert!(git(p, &["rebase", "main"]).is_err(), "the rebase conflicts");
    let error = match gg_git::worktree_changes::discard_all(p) {
        Ok(()) => return Err("discard ran during a rebase".into()),
        Err(error) => error,
    };
    assert!(error.contains("rebase is in progress"), "{error}");
    Ok(())
}

#[test]
fn rename_refuses_an_existing_name() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["branch", "feature"])?;
    git(p, &["branch", "taken"])?;
    gg_git::branch::rename(p, "feature", "renamed")?;
    let branches = git(p, &["branch", "--format=%(refname:short)"])?;
    assert!(branches.lines().any(|line| line == "renamed"), "{branches}");
    assert!(
        !branches.lines().any(|line| line == "feature"),
        "{branches}"
    );
    let error = match gg_git::branch::rename(p, "renamed", "taken") {
        Ok(()) => return Err("renamed onto an existing branch".into()),
        Err(error) => error,
    };
    assert!(error.contains("already exists"), "{error}");
    let error = match gg_git::branch::rename(p, "renamed", "-bad") {
        Ok(()) => return Err("accepted an option-like name".into()),
        Err(error) => error,
    };
    assert!(error.contains("cannot start with '-'"), "{error}");
    // Renaming the current branch keeps HEAD on it.
    gg_git::branch::rename(p, "main", "trunk")?;
    assert_eq!(git(p, &["branch", "--show-current"])?.trim(), "trunk");
    Ok(())
}

#[test]
fn set_and_unset_upstream() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let head = git(p, &["rev-parse", "HEAD"])?.trim().to_string();
    git(
        p,
        &[
            "config",
            "remote.origin.url",
            "https://example.org/repo.git",
        ],
    )?;
    git(
        p,
        &[
            "config",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/origin/*",
        ],
    )?;
    git(p, &["update-ref", "refs/remotes/origin/main", &head])?;
    gg_git::branch::set_upstream(p, "main", "origin/main")?;
    assert_eq!(
        git(p, &["rev-parse", "--abbrev-ref", "main@{upstream}"])?.trim(),
        "origin/main"
    );
    let error = match gg_git::branch::set_upstream(p, "main", "origin/missing") {
        Ok(()) => return Err("accepted a missing upstream".into()),
        Err(error) => error,
    };
    assert!(error.contains("Fetch it first"), "{error}");
    gg_git::branch::unset_upstream(p, "main")?;
    assert!(git(p, &["rev-parse", "--abbrev-ref", "main@{upstream}"]).is_err());
    Ok(())
}
