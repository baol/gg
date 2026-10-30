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
    std::fs::write(dir.path().join("base"), b"base\n")?;
    git(dir.path(), &["add", "base"])?;
    git(dir.path(), &["commit", "-qm", "base"])?;
    Ok(dir)
}

fn head(path: &Path) -> Result<String> {
    Ok(String::from_utf8(git(path, &["rev-parse", "HEAD"])?)?
        .trim()
        .to_string())
}

fn commit(path: &Path, file: &str, contents: &str, message: &str) -> Result<String> {
    std::fs::write(path.join(file), contents.as_bytes())?;
    git(path, &["add", file])?;
    git(path, &["commit", "-qm", message])?;
    Ok(String::from_utf8(git(path, &["rev-parse", "HEAD"])?)?
        .trim()
        .to_string())
}

fn subjects(path: &Path) -> Result<Vec<String>> {
    Ok(
        String::from_utf8(git(path, &["log", "--reverse", "--format=%s"])?)?
            .lines()
            .map(str::to_string)
            .collect(),
    )
}

fn rewrite_commits() -> Result<(tempfile::TempDir, String, String, String)> {
    let dir = repo()?;
    let p = dir.path();
    let first = commit(p, "one", "one\n", "one")?;
    let second = commit(p, "two", "two\n", "two")?;
    let third = commit(p, "three", "three\n", "three")?;
    Ok((dir, first, second, third))
}

#[test]
fn rewrite_lists_a_linear_range_oldest_first() -> Result {
    let (dir, first, second, third) = rewrite_commits()?;
    let commits = gg_git::rewrite::commits(dir.path(), &first)?;
    assert_eq!(
        commits
            .iter()
            .map(|commit| commit.id.as_str())
            .collect::<Vec<_>>(),
        vec![first.as_str(), second.as_str(), third.as_str()]
    );
    assert_eq!(
        commits
            .iter()
            .map(|commit| commit.subject.as_str())
            .collect::<Vec<_>>(),
        vec!["one", "two", "three"]
    );
    assert!(commits.iter().all(|commit| commit.author == "Test"));
    assert!(commits.iter().all(|commit| !commit.date.is_empty()));
    assert_eq!(commits[1].message, "two");

    git(
        dir.path(),
        &[
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "four",
            "-m",
            "body\nline",
        ],
    )?;
    let (commits, base) = gg_git::rewrite::plan_range(dir.path(), &second)?;
    assert_eq!(base.as_deref(), Some(first.as_str()));
    assert_eq!(commits.len(), 3);
    assert_eq!(commits[2].subject, "four");
    assert_eq!(commits[2].message, "four\n\nbody\nline");
    Ok(())
}

#[test]
fn rewrite_reorders_commits_and_records_the_previous_tip() -> Result {
    let (dir, first, second, third) = rewrite_commits()?;
    let p = dir.path();
    let previous = head(p)?;
    let steps = vec![
        gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Pick, &second),
        gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Pick, &first),
        gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Pick, &third),
    ];
    let receipt = gg_git::rewrite::rewrite(p, &first, &steps, false)?;
    assert!(receipt.applied);
    assert!(!receipt.in_progress);
    assert_eq!(receipt.steps, 3);
    assert_eq!(receipt.previous.as_deref(), Some(previous.as_str()));
    let backup = receipt.backup.as_deref().ok_or("missing backup ref")?;
    assert!(backup.starts_with("refs/gg/backup/rewrite/"), "{backup}");
    assert_eq!(
        String::from_utf8(git(p, &["rev-parse", backup])?)?.trim(),
        previous
    );
    assert_eq!(subjects(p)?, vec!["base", "two", "one", "three"]);
    assert_eq!(std::fs::read(p.join("one"))?, b"one\n");
    assert_eq!(std::fs::read(p.join("two"))?, b"two\n");
    assert_eq!(std::fs::read(p.join("three"))?, b"three\n");
    assert_eq!(
        String::from_utf8(git(p, &["log", "-1", "--format=%P"])?)?
            .split_whitespace()
            .count(),
        1
    );
    Ok(())
}

#[test]
fn rewrite_squashes_commits_and_keeps_the_combined_message() -> Result {
    let (dir, first, second, third) = rewrite_commits()?;
    let p = dir.path();
    let steps = vec![
        gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Pick, &first),
        gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Squash, &second),
        gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Pick, &third),
    ];
    let receipt = gg_git::rewrite::rewrite(p, &first, &steps, false)?;
    assert!(receipt.applied);
    assert_eq!(receipt.steps, 3);
    assert_eq!(subjects(p)?, vec!["base", "one", "three"]);
    let message = String::from_utf8(git(p, &["log", "-1", "--format=%B", "HEAD^"])?)?;
    assert!(message.contains("one"), "{message}");
    assert!(message.contains("two"), "{message}");
    assert_eq!(std::fs::read(p.join("one"))?, b"one\n");
    assert_eq!(std::fs::read(p.join("two"))?, b"two\n");
    assert_eq!(std::fs::read(p.join("three"))?, b"three\n");
    Ok(())
}

#[test]
fn rewrite_supports_a_root_range() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let root = head(p)?;
    let second = commit(p, "one", "one\n", "one")?;
    let steps = vec![
        gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Pick, &root),
        gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Squash, &second),
    ];
    let receipt = gg_git::rewrite::rewrite(p, &root, &steps, false)?;
    assert!(receipt.applied);
    assert_eq!(subjects(p)?, vec!["base"]);
    assert_eq!(
        String::from_utf8(git(p, &["rev-list", "--count", "HEAD"])?)?.trim(),
        "1"
    );
    assert_eq!(std::fs::read(p.join("one"))?, b"one\n");
    Ok(())
}

#[test]
fn rewrite_validates_the_full_step_set() -> Result {
    let (dir, first, second, _) = rewrite_commits()?;
    let p = dir.path();
    let pick =
        |commit: &str| gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Pick, commit);
    let squash =
        |commit: &str| gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Squash, commit);

    let error = match gg_git::rewrite::rewrite(p, &first, &[], false) {
        Ok(_) => return Err("empty rewrite unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("at least one step"), "{error}");

    let error = match gg_git::rewrite::rewrite(p, &first, &[squash(&first), pick(&second)], false) {
        Ok(_) => return Err("squash-first rewrite unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("needs a kept commit below"), "{error}");

    let error = match gg_git::rewrite::rewrite(p, &first, &[pick(&first), pick(&first)], false) {
        Ok(_) => return Err("duplicate rewrite unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("more than once"), "{error}");

    let error = match gg_git::rewrite::rewrite(p, &first, &[pick(&first), pick(&second)], false) {
        Ok(_) => return Err("incomplete rewrite unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("is missing"), "{error}");

    let error = match gg_git::rewrite::rewrite(p, "--abort", &[pick(&first)], false) {
        Ok(_) => return Err("option-shaped rewrite unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("must not start"), "{error}");
    Ok(())
}

#[test]
fn rewrite_rejects_merge_commits() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let base = head(p)?;
    git(p, &["switch", "-qc", "side"])?;
    commit(p, "side", "side\n", "side")?;
    git(p, &["switch", "-q", "main"])?;
    commit(p, "main", "main\n", "main")?;
    git(p, &["merge", "-q", "--no-ff", "side", "-m", "merge side"])?;
    let error = match gg_git::rewrite::commits(p, &base) {
        Ok(_) => return Err("merge rewrite unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("without merge commits"), "{error}");
    Ok(())
}

#[test]
fn rewrite_conflict_can_abort_or_skip() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    git(p, &["commit", "-qm", "add a"])?;
    let first = commit(p, "a", "one\n", "one")?;
    let second = commit(p, "a", "two\n", "two")?;
    let previous = head(p)?;
    let reorder = vec![
        gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Pick, &second),
        gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Pick, &first),
    ];
    let receipt = gg_git::rewrite::rewrite(p, &first, &reorder, false)?;
    assert!(!receipt.applied);
    assert!(receipt.in_progress);
    assert_eq!(receipt.conflicts, vec!["a".to_string()]);
    assert_eq!(
        gg_git::mutation::state(p)?.map(|state| state.operation),
        Some(gg_engine::mutation::Operation::Rebase)
    );
    gg_git::mutation::abort_operation(p, gg_engine::mutation::Operation::Rebase)?;
    assert_eq!(head(p)?, previous);
    assert_eq!(std::fs::read(p.join("a"))?, b"two\n");

    gg_git::rewrite::rewrite(p, &first, &reorder, false)?;
    let skipped = gg_git::mutation::skip_operation(p, gg_engine::mutation::Operation::Rebase)?;
    assert!(skipped.applied);
    assert!(!skipped.in_progress);
    assert_eq!(subjects(p)?, vec!["base", "add a", "one"]);
    assert_eq!(std::fs::read(p.join("a"))?, b"one\n");
    Ok(())
}

#[test]
fn rewrite_requires_a_clean_worktree_unless_autostash_is_enabled() -> Result {
    let (dir, first, second, third) = rewrite_commits()?;
    let p = dir.path();
    git(p, &["config", "rebase.autoStash", "true"])?;
    let before = head(p)?;
    std::fs::write(p.join("one"), b"dirty\n")?;
    let steps = vec![
        gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Pick, &second),
        gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Pick, &first),
        gg_engine::rewrite::Step::new(gg_engine::rewrite::Action::Pick, &third),
    ];
    let error = match gg_git::rewrite::rewrite(p, &first, &steps, false) {
        Ok(_) => return Err("dirty rewrite unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("not clean"), "{error}");
    assert_eq!(head(p)?, before);
    let receipt = gg_git::rewrite::rewrite(p, &first, &steps, true)?;
    assert!(receipt.applied);
    assert_eq!(std::fs::read(p.join("one"))?, b"dirty\n");
    Ok(())
}

#[test]
fn rewrite_rejects_a_shallow_boundary() -> Result {
    let source = repo()?;
    let p = source.path();
    commit(p, "one", "one\n", "one")?;
    commit(p, "two", "two\n", "two")?;
    let extra = tempfile::tempdir()?;
    let clone = extra.path().join("shallow");
    git(
        extra.path(),
        &[
            "clone",
            "--depth=1",
            "--no-local",
            &source.path().to_string_lossy(),
            &clone.to_string_lossy(),
        ],
    )?;
    let head = head(&clone)?;
    let error = match gg_git::rewrite::commits(&clone, &head) {
        Ok(_) => return Err("shallow rewrite unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("shallow boundary"), "{error}");
    Ok(())
}

#[test]
fn rewrite_rejects_a_start_outside_head_history() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "other"])?;
    let other = commit(p, "other", "other\n", "other")?;
    git(p, &["switch", "-q", "main"])?;
    commit(p, "main", "main\n", "main")?;
    let error = match gg_git::rewrite::commits(p, &other) {
        Ok(_) => return Err("unrelated rewrite unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("not an ancestor"), "{error}");
    Ok(())
}

#[test]
fn branch_start_prefers_unpushed_commits_then_the_default_branch() -> Result {
    let dir = repo()?;
    let path = dir.path();
    assert_eq!(
        gg_git::rewrite::branch_start(path)?,
        None,
        "main has no own commits"
    );

    git(path, &["switch", "-q", "-c", "feature"])?;
    let first = commit(path, "a", "a\n", "first")?;
    let second = commit(path, "b", "b\n", "second")?;
    assert_eq!(
        gg_git::rewrite::branch_start(path)?,
        Some((first.clone(), 2)),
        "without an upstream the range starts where the branch left main"
    );

    git(path, &["update-ref", "refs/remotes/origin/feature", &first])?;
    git(
        path,
        &[
            "config",
            "remote.origin.url",
            "https://example.org/repo.git",
        ],
    )?;
    git(
        path,
        &[
            "config",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/origin/*",
        ],
    )?;
    git(path, &["config", "branch.feature.remote", "origin"])?;
    git(
        path,
        &["config", "branch.feature.merge", "refs/heads/feature"],
    )?;
    assert_eq!(
        gg_git::rewrite::branch_start(path)?,
        Some((second.clone(), 1)),
        "only commits the upstream lacks"
    );

    git(
        path,
        &["update-ref", "refs/remotes/origin/feature", &second],
    )?;
    assert_eq!(
        gg_git::rewrite::branch_start(path)?,
        Some((first, 2)),
        "fully pushed branches fall back to the default branch"
    );
    Ok(())
}

fn step(action: gg_engine::rewrite::Action, commit: &str) -> gg_engine::rewrite::Step {
    gg_engine::rewrite::Step::new(action, commit)
}

fn message(path: &Path, rev: &str) -> Result<String> {
    Ok(
        String::from_utf8(git(path, &["log", "-1", "--format=%B", rev])?)?
            .trim_end()
            .to_string(),
    )
}

#[test]
fn rewrite_rewords_a_commit_without_an_editor() -> Result {
    use gg_engine::rewrite::{Action, Step};
    let (dir, first, second, third) = rewrite_commits()?;
    let p = dir.path();
    let steps = vec![
        step(Action::Pick, &first),
        Step::reword(
            &second,
            "two, reworded\n\n# kept as typed\nBody's 'quoted' line",
        ),
        step(Action::Pick, &third),
    ];
    let receipt = gg_git::rewrite::rewrite(p, &first, &steps, false)?;
    assert!(receipt.applied, "{receipt:?}");
    assert_eq!(subjects(p)?, vec!["base", "one", "two, reworded", "three"]);
    assert_eq!(
        message(p, "HEAD^")?,
        "two, reworded\n\n# kept as typed\nBody's 'quoted' line"
    );
    assert_eq!(std::fs::read(p.join("two"))?, b"two\n");
    Ok(())
}

#[test]
fn rewrite_fixup_keeps_only_the_older_message() -> Result {
    use gg_engine::rewrite::Action;
    let (dir, first, second, third) = rewrite_commits()?;
    let p = dir.path();
    let steps = vec![
        step(Action::Pick, &first),
        step(Action::Fixup, &second),
        step(Action::Pick, &third),
    ];
    let receipt = gg_git::rewrite::rewrite(p, &first, &steps, false)?;
    assert!(receipt.applied, "{receipt:?}");
    assert_eq!(subjects(p)?, vec!["base", "one", "three"]);
    assert_eq!(message(p, "HEAD^")?, "one");
    assert_eq!(std::fs::read(p.join("two"))?, b"two\n");
    Ok(())
}

#[test]
fn rewrite_squash_and_fixup_skip_dropped_commits() -> Result {
    use gg_engine::rewrite::Action;
    let (dir, first, second, third) = rewrite_commits()?;
    let p = dir.path();
    let steps = vec![
        step(Action::Pick, &first),
        step(Action::Drop, &second),
        step(Action::Fixup, &third),
    ];
    let receipt = gg_git::rewrite::rewrite(p, &first, &steps, false)?;
    assert!(receipt.applied, "{receipt:?}");
    assert_eq!(subjects(p)?, vec!["base", "one"]);
    assert!(!p.join("two").exists());
    assert_eq!(std::fs::read(p.join("three"))?, b"three\n");
    Ok(())
}

#[test]
fn rewrite_drops_a_commit() -> Result {
    use gg_engine::rewrite::Action;
    let (dir, first, second, third) = rewrite_commits()?;
    let p = dir.path();
    let steps = vec![
        step(Action::Drop, &first),
        step(Action::Pick, &second),
        step(Action::Pick, &third),
    ];
    let receipt = gg_git::rewrite::rewrite(p, &first, &steps, false)?;
    assert!(receipt.applied, "{receipt:?}");
    assert_eq!(subjects(p)?, vec!["base", "two", "three"]);
    assert!(!p.join("one").exists());
    Ok(())
}

#[test]
fn rewrite_edit_stops_after_the_commit_and_continues() -> Result {
    use gg_engine::rewrite::Action;
    let (dir, first, second, third) = rewrite_commits()?;
    let p = dir.path();
    let steps = vec![
        step(Action::Pick, &first),
        step(Action::Edit, &second),
        step(Action::Pick, &third),
    ];
    let receipt = gg_git::rewrite::rewrite(p, &first, &steps, false)?;
    assert!(!receipt.applied);
    assert!(receipt.in_progress);
    assert!(receipt.conflicts.is_empty());
    assert!(
        receipt
            .warnings
            .iter()
            .any(|w| w.contains("reopened commit")),
        "{:?}",
        receipt.warnings
    );
    assert_eq!(
        gg_git::mutation::state(p)?.map(|state| state.operation),
        Some(gg_engine::mutation::Operation::Rebase)
    );
    assert_eq!(head(p)?, second, "stops with the commit itself at HEAD");

    // Split the reopened commit: undo it into the worktree and commit twice.
    git(p, &["reset", "-q", "HEAD^"])?;
    std::fs::write(p.join("extra"), b"extra\n")?;
    git(p, &["add", "two"])?;
    git(p, &["commit", "-qm", "two, part 1"])?;
    git(p, &["add", "extra"])?;
    git(p, &["commit", "-qm", "two, part 2"])?;
    let done = gg_git::mutation::continue_operation(p, gg_engine::mutation::Operation::Rebase)?;
    assert!(done.applied, "{done:?}");
    assert!(!done.in_progress);
    assert_eq!(
        subjects(p)?,
        vec!["base", "one", "two, part 1", "two, part 2", "three"]
    );
    Ok(())
}

#[test]
fn rewrite_validates_new_actions() -> Result {
    use gg_engine::rewrite::{Action, Step};
    let (dir, first, second, third) = rewrite_commits()?;
    let p = dir.path();
    let before = head(p)?;
    let expect_error = |steps: Vec<Step>, needle: &str| -> Result {
        match gg_git::rewrite::rewrite(p, &first, &steps, false) {
            Ok(receipt) => Err(format!("{needle}: unexpectedly succeeded {receipt:?}").into()),
            Err(error) if error.contains(needle) => Ok(()),
            Err(error) => Err(format!("expected {needle:?}, got {error:?}").into()),
        }
    };
    expect_error(
        vec![
            step(Action::Drop, &first),
            step(Action::Fixup, &second),
            step(Action::Pick, &third),
        ],
        "needs a kept commit below",
    )?;
    expect_error(
        vec![
            step(Action::Pick, &first),
            Step::reword(&second, "  \n"),
            step(Action::Pick, &third),
        ],
        "needs a non-empty message",
    )?;
    expect_error(
        vec![
            step(Action::Pick, &first),
            step(Action::Reword, &second),
            step(Action::Pick, &third),
        ],
        "needs a non-empty message",
    )?;
    let mut with_message = step(Action::Pick, &second);
    with_message.message = Some("new".into());
    expect_error(
        vec![
            step(Action::Pick, &first),
            with_message,
            step(Action::Pick, &third),
        ],
        "Only reword steps carry a message",
    )?;
    expect_error(
        vec![
            step(Action::Drop, &first),
            step(Action::Drop, &second),
            step(Action::Drop, &third),
        ],
        "Use Reset",
    )?;
    assert_eq!(head(p)?, before);
    assert_eq!(gg_git::mutation::state(p)?, None);
    Ok(())
}

#[test]
fn rewrite_handles_a_repository_path_with_quotes_and_spaces() -> Result {
    use gg_engine::rewrite::{Action, Step};
    let outer = tempfile::tempdir()?;
    let p = outer.path().join("it's a repo");
    std::fs::create_dir(&p)?;
    git(&p, &["init", "-q", "-b", "main"])?;
    git(&p, &["config", "user.name", "Test"])?;
    git(&p, &["config", "user.email", "test@example.org"])?;
    commit(&p, "base", "base\n", "base")?;
    let first = commit(&p, "one", "one\n", "one")?;
    let second = commit(&p, "two", "two\n", "two")?;
    let steps = vec![
        Step::reword(&first, "one, reworded"),
        step(Action::Drop, &second),
    ];
    let receipt = gg_git::rewrite::rewrite(&p, &first, &steps, false)?;
    assert!(receipt.applied, "{receipt:?}");
    assert_eq!(subjects(&p)?, vec!["base", "one, reworded"]);
    Ok(())
}
