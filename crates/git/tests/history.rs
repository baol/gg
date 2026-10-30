use gg_engine::{HistoryRequest, HistorySource};
use std::{collections::HashSet, error::Error, path::Path, process::Command};

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
fn history_includes_commits_from_all_local_remote_and_tag_refs() -> Result {
    let dir = init()?;
    commit(dir.path(), "Root")?;
    git(dir.path(), &["checkout", "-b", "side"])?;
    commit(dir.path(), "Side")?;
    let side = git(dir.path(), &["rev-parse", "HEAD"])?;
    git(dir.path(), &["checkout", "main"])?;
    commit(dir.path(), "Main")?;

    git(dir.path(), &["checkout", "-b", "remote-only"])?;
    commit(dir.path(), "Remote only")?;
    let remote_only = git(dir.path(), &["rev-parse", "HEAD"])?;
    git(dir.path(), &["checkout", "main"])?;
    git(dir.path(), &["branch", "-D", "remote-only"])?;
    git(
        dir.path(),
        &[
            "update-ref",
            "refs/remotes/origin/remote-only",
            &remote_only,
        ],
    )?;

    git(dir.path(), &["checkout", "-b", "tag-only"])?;
    commit(dir.path(), "Tag only")?;
    let tag_only = git(dir.path(), &["rev-parse", "HEAD"])?;
    git(dir.path(), &["tag", "tag-only", &tag_only])?;
    git(dir.path(), &["checkout", "main"])?;
    git(dir.path(), &["branch", "-D", "tag-only"])?;
    git(dir.path(), &["tag", "side-tag", &side])?;

    let history = read(dir.path())?;
    let mut actual = history
        .commits
        .iter()
        .map(|commit| commit.id.as_str())
        .collect::<Vec<_>>();
    actual.sort_unstable();
    let oracle = git(dir.path(), &["rev-list", "--all"])?;
    let mut expected = oracle.lines().collect::<Vec<_>>();
    expected.sort_unstable();
    assert_eq!(actual, expected);
    let mut seen = HashSet::new();
    for history_commit in &history.commits {
        for parent in &history_commit.parents {
            assert!(
                !seen.contains(parent),
                "parent {parent} appeared before child {}",
                history_commit.id
            );
        }
        seen.insert(history_commit.id.clone());
    }
    for subject in ["Side", "Main", "Remote only", "Tag only"] {
        assert!(
            history
                .commits
                .iter()
                .any(|commit| commit.subject() == subject),
            "missing {subject}"
        );
    }
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

fn review(
    path: &Path,
    revision: &str,
    parent: Option<usize>,
) -> Result<Vec<gg_engine::review::FileChange>> {
    use gg_engine::review::{ReviewReader, ReviewRequest, ReviewStep};
    let mut reader = gg_git::review::ReviewSession::open(&ReviewRequest {
        path: path.into(),
        revision: revision.into(),
        parent,
        against: None,
    })?;
    let mut changes = vec![];
    loop {
        match reader.step()? {
            ReviewStep::Change(change) => changes.push(change),
            ReviewStep::Pending => {}
            ReviewStep::Complete => return Ok(changes),
        }
    }
}

#[test]
fn changed_files_match_git_for_root_nested_and_type_changes() -> Result {
    use gg_engine::review::ChangeKind;
    let dir = init()?;
    std::fs::create_dir(dir.path().join("nested"))?;
    for (path, bytes) in [
        ("a", "before"),
        ("deleted", "remove"),
        ("nested/file", "nested"),
        ("replace", "file"),
        ("replace.txt", "before"),
    ] {
        std::fs::write(dir.path().join(path), bytes)?;
    }
    git(dir.path(), &["add", "."])?;
    commit(dir.path(), "root")?;
    let root = review(dir.path(), "HEAD", None)?;
    assert_eq!(root.len(), 5);
    assert!(
        root.iter()
            .all(|c| c.kind == ChangeKind::Added && c.old.is_none())
    );
    std::fs::write(dir.path().join("a"), "after")?;
    std::fs::write(dir.path().join("replace.txt"), "after")?;
    std::fs::remove_file(dir.path().join("deleted"))?;
    std::fs::write(dir.path().join("new"), [0, 255, 1])?;
    std::fs::remove_file(dir.path().join("replace"))?;
    std::fs::create_dir(dir.path().join("replace"))?;
    std::fs::write(dir.path().join("replace/child"), "child")?;
    git(dir.path(), &["add", "-A"])?;
    commit(dir.path(), "changes")?;
    let changes = review(dir.path(), "HEAD", None)?;
    let output = Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args([
            "diff-tree",
            "-r",
            "--no-commit-id",
            "--raw",
            "-z",
            "--no-renames",
            "HEAD^",
            "HEAD",
        ])
        .output()?;
    assert!(output.status.success());
    let fields: Vec<_> = output
        .stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(fields.len(), changes.len() * 2);
    for (change, pair) in changes.iter().zip(fields.as_chunks::<2>().0) {
        assert_eq!(change.path_bytes, pair[1]);
        let raw = std::str::from_utf8(pair[0])?;
        let values: Vec<_> = raw.trim_start_matches(':').split_whitespace().collect();
        assert_eq!(
            change.old.as_ref().map(|s| s.mode).unwrap_or(0),
            u16::from_str_radix(values[0], 8)?
        );
        assert_eq!(
            change.new.as_ref().map(|s| s.mode).unwrap_or(0),
            u16::from_str_radix(values[1], 8)?
        );
        if let Some(side) = &change.old {
            assert_eq!(side.id, values[2]);
        }
        if let Some(side) = &change.new {
            assert_eq!(side.id, values[3]);
        }
        let status = match change.kind {
            ChangeKind::Added => "A",
            ChangeKind::Deleted => "D",
            ChangeKind::Modified => "M",
            ChangeKind::TypeChanged => "T",
        };
        assert_eq!(status, values[4]);
    }
    commit(dir.path(), "empty")?;
    assert!(review(dir.path(), "HEAD", None)?.is_empty());
    assert!(review(dir.path(), "HEAD", Some(0)).is_err());
    assert!(review(dir.path(), "HEAD", Some(2)).is_err());
    assert!(review(dir.path(), "missing", None).is_err());
    Ok(())
}

#[test]
fn comparison_is_pinned_and_reads_trees_without_blobs() -> Result {
    use gg_engine::review::{ReviewReader, ReviewRequest, ReviewStep};
    let dir = init()?;
    std::fs::write(dir.path().join("file"), "content")?;
    git(dir.path(), &["add", "."])?;
    commit(dir.path(), "root")?;
    let id = git(dir.path(), &["rev-parse", "HEAD"])?;
    let blob = git(dir.path(), &["rev-parse", "HEAD:file"])?;
    let mut reader = gg_git::review::ReviewSession::open(&ReviewRequest {
        path: dir.path().into(),
        revision: "HEAD".into(),
        parent: None,
        against: None,
    })?;
    commit(dir.path(), "later")?;
    std::fs::remove_file(
        dir.path()
            .join(".git/objects")
            .join(&blob[..2])
            .join(&blob[2..]),
    )?;
    assert_eq!(reader.comparison().commit, id);
    assert!(reader.comparison().parent.is_none());
    assert_eq!(reader.comparison().author, "Test Author");
    assert_eq!(reader.comparison().date, "2026-01-01");
    let mut count = 0;
    loop {
        match reader.step()? {
            ReviewStep::Change(c) => {
                count += 1;
                assert_eq!(c.new.as_ref().map(|s| s.id.as_str()), Some(blob.as_str()));
            }
            ReviewStep::Pending => {}
            ReviewStep::Complete => break,
        }
    }
    assert_eq!(count, 1);
    Ok(())
}

#[test]
fn review_merge_defaults_to_first_parent_and_can_select_second() -> Result {
    let dir = init()?;
    commit(dir.path(), "base")?;
    git(dir.path(), &["checkout", "-b", "side"])?;
    std::fs::write(dir.path().join("side.txt"), "side")?;
    git(dir.path(), &["add", "."])?;
    commit(dir.path(), "side")?;
    git(dir.path(), &["checkout", "main"])?;
    std::fs::write(dir.path().join("main.txt"), "main")?;
    git(dir.path(), &["add", "."])?;
    commit(dir.path(), "main")?;
    git(dir.path(), &["merge", "--no-ff", "side", "-m", "merge"])?;
    use gg_engine::review::{ReviewReader, ReviewRequest};
    let request = ReviewRequest {
        path: dir.path().into(),
        revision: "HEAD".into(),
        parent: None,
        against: None,
    };
    let session = gg_git::review::ReviewSession::open(&request)?;
    let expected: Vec<String> = git(dir.path(), &["show", "-s", "--format=%P", "HEAD"])?
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    assert_eq!(session.comparison().parents, expected);
    assert_eq!(session.comparison().parent.as_ref(), expected.first());
    let first = review(dir.path(), "HEAD", None)?;
    let second = review(dir.path(), "HEAD", Some(2))?;
    assert_eq!(
        first.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(),
        vec!["side.txt"]
    );
    assert_eq!(
        second.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(),
        vec!["main.txt"]
    );
    let missing = &expected[1];
    std::fs::remove_file(
        dir.path()
            .join(".git/objects")
            .join(&missing[..2])
            .join(&missing[2..]),
    )?;
    let mut request = request;
    request.parent = Some(2);
    assert!(gg_git::review::ReviewSession::open(&request).is_err());
    request.parent = Some(1);
    assert_eq!(
        gg_git::review::ReviewSession::open(&request)?
            .comparison()
            .parents,
        expected
    );
    Ok(())
}

#[test]
fn selected_file_diff_matches_git_and_preserves_original_bytes() -> Result {
    use gg_engine::diff::RowKind;
    let dir = init()?;
    let old = b"one\r\nold\nkeep\nlast";
    let new = b"one\r\nnew\nkeep\nlast\n";
    std::fs::write(dir.path().join("file"), old)?;
    git(dir.path(), &["add", "."])?;
    commit(dir.path(), "before")?;
    std::fs::write(dir.path().join("file"), new)?;
    git(dir.path(), &["add", "."])?;
    commit(dir.path(), "after")?;
    let changes = review(dir.path(), "HEAD", None)?;
    let diff = gg_git::diff::read(dir.path(), &changes[0], || false)?;
    assert!(diff.complete);
    let oracle = Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args([
            "diff",
            "--no-ext-diff",
            "--no-color",
            "--no-indent-heuristic",
            "--unified=3",
            "HEAD^",
            "HEAD",
            "--",
            "file",
        ])
        .output()?;
    assert!(oracle.status.success());
    let mut actual = Vec::new();
    for row in &diff.rows {
        match row.kind {
            RowKind::Hunk => {
                actual.extend(row.text.as_bytes());
                actual.push(b'\n');
            }
            RowKind::NoNewline => {
                actual.extend(row.text.as_bytes());
                actual.push(b'\n');
            }
            _ => {
                actual.push(match row.kind {
                    RowKind::Added => b'+',
                    RowKind::Deleted => b'-',
                    _ => b' ',
                });
                actual.extend(&row.bytes);
                if !row.bytes.ends_with(b"\n") {
                    actual.push(b'\n');
                }
            }
        }
    }
    let start = oracle
        .stdout
        .windows(3)
        .position(|w| w == b"@@ ")
        .ok_or("missing Git hunk")?;
    assert_eq!(actual, &oracle.stdout[start..]);
    assert!(gg_git::diff::read(dir.path(), &changes[0], || true).is_err());
    Ok(())
}

#[test]
fn selected_file_binary_and_large_object_return_explicit_summaries() -> Result {
    use gg_engine::diff::{DiffKind, MAX_BLOB_BYTES};
    let dir = init()?;
    std::fs::write(dir.path().join("binary"), [0, 255, 1])?;
    std::fs::write(dir.path().join("large"), vec![b'a'; MAX_BLOB_BYTES + 1])?;
    git(dir.path(), &["add", "."])?;
    commit(dir.path(), "files")?;
    let changes = review(dir.path(), "HEAD", None)?;
    let binary = gg_git::diff::read(dir.path(), &changes[0], || false)?;
    assert_eq!(binary.kind, DiffKind::Binary);
    assert!(binary.complete);
    assert_eq!(binary.new_size, 3);
    let large = gg_git::diff::read(dir.path(), &changes[1], || false)?;
    assert_eq!(large.kind, DiffKind::Limited);
    assert!(!large.complete);
    assert!(large.rows.is_empty());
    assert_eq!(large.new_size, (MAX_BLOB_BYTES + 1) as u64);
    Ok(())
}
