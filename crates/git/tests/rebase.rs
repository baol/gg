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
        return Err(format!("git {args:?}: {}", String::from_utf8_lossy(&output.stderr)).into());
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

/// NET-39: Pro Git 3.6, "Rebase When You Rebase". After the upstream
/// rewrote a commit the branch was built on, `pull` replays only the
/// branch's own commit, as `git pull --rebase` does; a plain rebase onto
/// the same upstream would replay the old commit too and stop on it.
#[test]
fn pull_leaves_out_commits_the_upstream_rewrote() -> Result {
    let dir = tempfile::tempdir()?;
    let root = dir.path();
    git(root, &["init", "-q", "--bare", "-b", "main", "origin.git"])?;
    git(root, &["init", "-q", "-b", "main", "theirs"])?;
    let theirs = root.join("theirs");
    let origin = root.join("origin.git").display().to_string();
    git(&theirs, &["remote", "add", "origin", &origin])?;
    for name in ["a", "b", "c"] {
        std::fs::write(theirs.join(name), format!("{name} v1\n"))?;
        git(&theirs, &["add", name])?;
        git(&theirs, &["commit", "-qm", &format!("add {name}")])?;
    }
    git(&theirs, &["push", "-q", "-u", "origin", "main"])?;
    git(root, &["clone", "-q", &origin, "mine"])?;
    let mine = root.join("mine");
    // Git Good's own Git process commits as the repository's user.
    git(&mine, &["config", "user.name", "Test"])?;
    git(&mine, &["config", "user.email", "test@example.org"])?;
    std::fs::write(mine.join("d"), b"mine\n")?;
    git(&mine, &["add", "d"])?;
    git(&mine, &["commit", "-qm", "my work"])?;
    git(&theirs, &["reset", "-q", "--hard", "HEAD~1"])?;
    std::fs::write(theirs.join("c"), b"c v2 rewritten\n")?;
    git(&theirs, &["add", "c"])?;
    git(&theirs, &["commit", "-qm", "add c (rewritten)"])?;
    git(&theirs, &["push", "-q", "-f", "origin", "main"])?;
    git(&mine, &["fetch", "-q", "origin"])?;

    let receipt = gg_git::rebase::pull(&mine, "refs/remotes/origin/main", false)?;
    if !receipt.applied {
        // What Git did, for a failure on another Git version.
        let show = |args: &[&str]| {
            git(&mine, args)
                .map(|out| String::from_utf8_lossy(&out).into_owned())
                .unwrap_or_else(|error| error.to_string())
        };
        let plan = std::fs::read_to_string(mine.join(".git/rebase-merge/git-rebase-todo"))
            .unwrap_or_default();
        let done = std::fs::read_to_string(mine.join(".git/rebase-merge/done")).unwrap_or_default();
        panic!(
            "{receipt:?}\n--- git version\n{}--- status\n{}--- reflog origin/main\n{}--- todo\n{plan}--- done\n{done}--- fork point\n{}",
            show(&["version"]),
            show(&["status"]),
            show(&["reflog", "show", "refs/remotes/origin/main"]),
            show(&[
                "merge-base",
                "--fork-point",
                "refs/remotes/origin/main",
                "main"
            ]),
        );
    }
    assert!(receipt.conflicts.is_empty());
    assert_eq!(
        String::from_utf8(git(&mine, &["log", "--format=%s"])?)?,
        "my work\nadd c (rewritten)\nadd b\nadd a\n"
    );
    Ok(())
}

/// RWR-80: Pro Git 3.6, "More Interesting Rebases". `client` branched from
/// `server`, which branched from `master`; rebasing `client` onto `master`
/// leaving out `server` replays only `client`'s own commits.
#[test]
fn rebase_onto_leaves_out_the_commits_of_another_branch() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let commit = |name: &str| -> Result {
        std::fs::write(p.join(name), format!("{name}\n"))?;
        git(p, &["add", name])?;
        git(p, &["commit", "-qm", name])?;
        Ok(())
    };
    git(p, &["branch", "-qm", "master"])?;
    commit("C2")?;
    git(p, &["switch", "-qc", "server"])?;
    commit("C3")?;
    git(p, &["switch", "-qc", "client"])?;
    commit("C8")?;
    commit("C9")?;
    git(p, &["switch", "-q", "server"])?;
    commit("C4")?;
    commit("C10")?;
    git(p, &["switch", "-q", "master"])?;
    commit("C5")?;
    commit("C6")?;
    git(p, &["switch", "-q", "client"])?;
    let previous = head(p)?;

    let receipt = gg_git::rebase::rebase_onto(p, "master", "server", false)?;
    assert!(receipt.applied, "{receipt:?}");
    assert_eq!(receipt.reference, "master");
    assert_eq!(receipt.previous.as_deref(), Some(previous.as_str()));
    assert!(receipt.backup.is_some());
    assert_eq!(
        String::from_utf8(git(p, &["log", "--format=%s"])?)?,
        "C9\nC8\nC6\nC5\nC2\nbase\n",
        "C3 stays on server"
    );
    assert!(!p.join("C3").exists());

    let error = gg_git::rebase::rebase_onto(p, "master", "--exec=x", false).err();
    assert_eq!(
        error.as_deref(),
        Some("Leave out commits from must not start with '-'.")
    );
    let error = gg_git::rebase::rebase_onto(p, "master", "nosuch", false).err();
    assert_eq!(
        error.as_deref(),
        Some("Rebase reference nosuch was not found.")
    );
    Ok(())
}
