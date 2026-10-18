use gg_engine::{HistoryRequest, HistorySource};
use std::{error::Error, path::Path, process::Command};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn git(path: &Path, args: &[&str]) -> Result<String> {
    git_date(path, args, "2026-01-01T12:00:00+00:00")
}
fn git_date(path: &Path, args: &[&str], date: &str) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_AUTHOR_NAME", "Test Author")
        .env("GIT_AUTHOR_EMAIL", "test@example.org")
        .env("GIT_COMMITTER_NAME", "Test Author")
        .env("GIT_COMMITTER_EMAIL", "test@example.org")
        .env("GIT_AUTHOR_DATE", "2026-01-01T12:00:00+00:00")
        .env("GIT_COMMITTER_DATE", date)
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}
fn init() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-b", "main"])?;
    Ok(dir)
}
fn read(path: &Path) -> Result<gg_engine::History> {
    gg_git::Git
        .history(&HistoryRequest { path: path.into() })
        .map_err(Into::into)
}
fn commit(path: &Path, message: &str) -> Result {
    git(path, &["commit", "--allow-empty", "-m", message])?;
    Ok(())
}
#[test]
fn empty_nested_and_invalid_paths() -> Result {
    let dir = init()?;
    let history = read(dir.path())?;
    assert!(history.commits.is_empty());
    assert_eq!(history.head, "main");
    let nested = dir.path().join("nested");
    std::fs::create_dir(&nested)?;
    assert!(read(&nested)?.commits.is_empty());
    assert!(read(&dir.path().join("missing")).is_err());
    let other = tempfile::tempdir()?;
    assert!(read(other.path()).is_err());
    Ok(())
}
#[test]
fn history_matches_git_after_packing_and_detaching() -> Result {
    let dir = init()?;
    commit(dir.path(), "First café\n\nA full message")?;
    commit(dir.path(), "Second")?;
    git(dir.path(), &["gc", "--prune=now"])?;
    let history = read(dir.path())?;
    let oracle = git(dir.path(), &["rev-list", "HEAD"])?;
    assert_eq!(
        history
            .commits
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        oracle.lines().collect::<Vec<_>>()
    );
    assert_eq!(history.commits.len(), 2);
    assert!(!history.truncated);
    assert_eq!(history.commits.first().map(|c| c.subject()), Some("Second"));
    assert!(
        history
            .commits
            .iter()
            .any(|c| c.message.contains("A full message"))
    );
    git(dir.path(), &["checkout", "--detach"])?;
    assert_eq!(read(dir.path())?.head, "Detached HEAD");
    Ok(())
}
#[test]
fn merge_bare_and_linked_worktree() -> Result {
    let dir = init()?;
    commit(dir.path(), "Root")?;
    git(dir.path(), &["checkout", "-b", "side"])?;
    commit(dir.path(), "Side")?;
    git(dir.path(), &["checkout", "main"])?;
    commit(dir.path(), "Main")?;
    git(dir.path(), &["merge", "--no-ff", "side", "-m", "Merge"])?;
    let history = read(dir.path())?;
    assert_eq!(history.commits.len(), 4);
    assert_eq!(history.commits.first().map(|c| c.parents.len()), Some(2));
    let extra = tempfile::tempdir()?;
    let worktree = extra.path().join("linked");
    git(
        dir.path(),
        &[
            "worktree",
            "add",
            "-b",
            "linked",
            &worktree.to_string_lossy(),
        ],
    )?;
    assert_eq!(read(&worktree)?.commits.len(), 4);
    let bare = extra.path().join("bare.git");
    git(
        dir.path(),
        &["clone", "--bare", ".", &bare.to_string_lossy()],
    )?;
    assert_eq!(read(&bare)?.commits.len(), 4);
    Ok(())
}
#[test]
fn history_crosses_the_old_limit() -> Result {
    let dir = init()?;
    // commit-tree builds a real linear history without worktree or hook overhead.
    let tree = git(dir.path(), &["mktree"])?;
    let mut parent: Option<String> = None;
    for i in 0..501 {
        let mut args = vec!["commit-tree", tree.as_str(), "-m", "Commit"];
        if let Some(id) = &parent {
            args.extend(["-p", id]);
        }
        parent = Some(git(dir.path(), &args)?);
        if i == 498 || i == 499 {
            if let Some(id) = &parent {
                git(dir.path(), &["update-ref", "refs/heads/main", id])?;
            }
            assert_eq!(read(dir.path())?.commits.len(), i + 1);
        }
    }
    if let Some(id) = parent {
        git(dir.path(), &["update-ref", "refs/heads/main", &id])?;
    }
    let history = read(dir.path())?;
    assert_eq!(history.commits.len(), 501);
    assert!(!history.truncated);
    Ok(())
}

#[test]
fn missing_commit_object_returns_error() -> Result {
    let dir = init()?;
    std::fs::write(
        dir.path().join(".git/refs/heads/main"),
        "1111111111111111111111111111111111111111\n",
    )?;
    assert!(read(dir.path()).is_err());
    Ok(())
}

#[test]
fn pinned_head_and_skewed_merge_order_are_stable() -> Result {
    let dir = init()?;
    let tree = git(dir.path(), &["mktree"])?;
    let root = git_date(
        dir.path(),
        &["commit-tree", &tree, "-m", "Root"],
        "2026-01-10T12:00:00+00:00",
    )?;
    let left = git_date(
        dir.path(),
        &["commit-tree", &tree, "-p", &root, "-m", "Left"],
        "2026-01-02T12:00:00+00:00",
    )?;
    let right = git_date(
        dir.path(),
        &["commit-tree", &tree, "-p", &root, "-m", "Right"],
        "2026-01-03T12:00:00+00:00",
    )?;
    let merge = git_date(
        dir.path(),
        &[
            "commit-tree",
            &tree,
            "-p",
            &left,
            "-p",
            &right,
            "-m",
            "Merge",
        ],
        "2026-01-01T12:00:00+00:00",
    )?;
    git(dir.path(), &["update-ref", "refs/heads/main", &merge])?;
    let mut session = gg_git::Session::open(&HistoryRequest {
        path: dir.path().into(),
    })?;
    commit(dir.path(), "New HEAD")?;
    let mut ids = Vec::new();
    while let Some(commit) = session.next_commit()? {
        if ids.is_empty() {
            assert_eq!(commit.parents, [left.clone(), right.clone()]);
        }
        ids.push(commit.id);
    }
    assert_eq!(ids, [merge, right, root, left]);
    assert_eq!(read(dir.path())?.commits.len(), 5);
    Ok(())
}

#[test]
fn shallow_history_stops_at_the_available_boundary() -> Result {
    let dir = init()?;
    commit(dir.path(), "Root")?;
    commit(dir.path(), "Middle")?;
    commit(dir.path(), "Tip")?;
    let extra = tempfile::tempdir()?;
    let clone = extra.path().join("shallow");
    git(
        extra.path(),
        &[
            "clone",
            "--depth=2",
            "--no-local",
            &dir.path().to_string_lossy(),
            &clone.to_string_lossy(),
        ],
    )?;
    let history = read(&clone)?;
    assert_eq!(history.commits.len(), 2);
    assert!(history.shallow);
    Ok(())
}
