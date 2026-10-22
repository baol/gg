use std::{error::Error, path::Path, process::Command, sync::atomic::AtomicBool};
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
    git(dir.path(), &["config", "core.autocrlf", "false"])?;
    std::fs::write(dir.path().join("a"), b"base\n")?;
    git(dir.path(), &["add", "a"])?;
    git(dir.path(), &["commit", "-qm", "base"])?;
    Ok(dir)
}

fn bare() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "--bare", "-b", "main"])?;
    Ok(dir)
}

fn push_all(path: &Path, remote: &Path) -> Result {
    git(
        path,
        &[
            "push",
            "--quiet",
            remote.to_string_lossy().as_ref(),
            "HEAD:refs/heads/main",
        ],
    )?;
    Ok(())
}

fn head(path: &Path, reference: &str) -> Result<String> {
    Ok(String::from_utf8(git(path, &["rev-parse", reference])?)?
        .trim()
        .to_string())
}

fn no_cancel() -> AtomicBool {
    AtomicBool::new(false)
}

#[test]
fn clone_fetch_push_upstream_and_non_fast_forward() -> Result {
    let remote = bare()?;
    let source = repo()?;
    push_all(source.path(), remote.path())?;

    let destination = tempfile::tempdir()?;
    let target = destination.path().join("cloned");
    let cancel = no_cancel();
    let mut progress = Vec::new();
    let receipt = gg_git::sync::clone(
        remote.path().to_string_lossy().as_ref(),
        Some(&target),
        false,
        &cancel,
        &mut |update| progress.push(update),
    )?;
    assert_eq!(
        std::fs::canonicalize(&target)?,
        receipt.path,
        "clone receipt path"
    );
    assert_eq!(receipt.branch.as_deref(), Some("main"));
    assert_eq!(
        receipt.head.as_deref(),
        Some(head(source.path(), "HEAD")?.as_str())
    );
    assert_eq!(git(&target, &["show", "HEAD:a"])?, b"base\n");
    let working = std::fs::read(target.join("a"))?;
    assert!(!progress.is_empty(), "clone produced no progress");
    assert!(
        progress
            .iter()
            .all(|update| update.message.chars().count() <= 240)
    );

    // Advance the remote, then fetch into the clone without touching its local branch.
    std::fs::write(source.path().join("a"), b"remote update\n")?;
    git(source.path(), &["commit", "-qam", "remote update"])?;
    push_all(source.path(), remote.path())?;
    let remote_head = head(source.path(), "HEAD")?;
    let local_head = head(&target, "HEAD")?;
    let index = std::fs::read(target.join(".git/index"))?;
    let mut progress = Vec::new();
    let receipt = gg_git::sync::fetch(
        &target,
        Some("origin"),
        false,
        false,
        &cancel,
        &mut |update| progress.push(update),
    )?;
    assert_eq!(receipt.remote, "origin");
    assert!(!receipt.pruned);
    assert_eq!(head(&target, "refs/remotes/origin/main")?, remote_head);
    assert_eq!(head(&target, "HEAD")?, local_head);
    assert_eq!(std::fs::read(target.join("a"))?, working);
    assert_eq!(std::fs::read(target.join(".git/index"))?, index);
    assert!(!progress.is_empty(), "fetch produced no progress");

    // Fast-forward to the fetched remote, then push a new local commit and configure upstream.
    git(&target, &["merge", "--ff-only", "origin/main"])?;
    std::fs::write(target.join("a"), b"local update\n")?;
    git(&target, &["commit", "-qam", "local update"])?;
    let local_head = head(&target, "HEAD")?;
    let receipt = gg_git::sync::push(
        &target,
        Some("origin"),
        None,
        true,
        false,
        &cancel,
        &mut |_| {},
    )?;
    assert_eq!(receipt.remote, "origin");
    assert_eq!(receipt.branch, "main");
    assert_eq!(receipt.upstream.as_deref(), Some("origin/main"));
    assert!(receipt.set_upstream);
    assert_eq!(head(remote.path(), "refs/heads/main")?, local_head);
    assert_eq!(
        git(&target, &["config", "branch.main.remote"])?,
        b"origin\n"
    );
    assert_eq!(
        git(&target, &["config", "branch.main.merge"])?,
        b"refs/heads/main\n"
    );

    // A diverged branch is rejected without force and remains intact.
    let remote_path = remote.path().to_string_lossy().into_owned();
    git(source.path(), &["remote", "add", "origin", &remote_path])?;
    git(source.path(), &["fetch", "origin"])?;
    git(source.path(), &["reset", "--hard", "origin/main"])?;
    std::fs::write(source.path().join("a"), b"source wins\n")?;
    git(source.path(), &["commit", "-qam", "source wins"])?;
    push_all(source.path(), remote.path())?;
    let remote_head = head(remote.path(), "refs/heads/main")?;
    std::fs::write(target.join("b"), b"local only\n")?;
    git(&target, &["add", "b"])?;
    git(&target, &["commit", "-qm", "local only"])?;
    let local_head = head(&target, "HEAD")?;
    let error = match gg_git::sync::push(
        &target,
        Some("origin"),
        None,
        false,
        false,
        &cancel,
        &mut |_| {},
    ) {
        Ok(_) => return Err("diverged push unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("failed"), "{error}");
    assert_eq!(head(remote.path(), "refs/heads/main")?, remote_head);
    assert_eq!(head(&target, "HEAD")?, local_head);
    assert_eq!(std::fs::read(target.join("b"))?, b"local only\n");
    Ok(())
}

#[test]
fn fetch_reports_missing_and_ambiguous_default_remotes() -> Result {
    let dir = repo()?;
    let cancel = no_cancel();
    let error = match gg_git::sync::fetch(dir.path(), None, false, false, &cancel, &mut |_| {}) {
        Ok(_) => return Err("fetch without a remote unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("No remotes"), "{error}");
    git(
        dir.path(),
        &["remote", "add", "one", "https://example.org/one.git"],
    )?;
    git(
        dir.path(),
        &["remote", "add", "two", "https://example.org/two.git"],
    )?;
    let error = match gg_git::sync::fetch(dir.path(), None, false, false, &cancel, &mut |_| {}) {
        Ok(_) => return Err("ambiguous fetch unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("Multiple remotes"), "{error}");
    assert!(error.contains("one") && error.contains("two"), "{error}");
    Ok(())
}

#[test]
fn cancellation_reports_cancel_without_a_receipt() -> Result {
    let remote = bare()?;
    let source = repo()?;
    push_all(source.path(), remote.path())?;
    let target = tempfile::tempdir()?;
    let destination = target.path().join("cancelled");
    let cancel = AtomicBool::new(true);
    let error = match gg_git::sync::clone(
        remote.path().to_string_lossy().as_ref(),
        Some(&destination),
        false,
        &cancel,
        &mut |_| {},
    ) {
        Ok(_) => return Err("cancelled clone unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("cancelled"), "{error}");
    Ok(())
}

#[test]
fn scripts_cannot_inject_git_options_or_read_ambient_repository_state() -> Result {
    let remote = bare()?;
    let source = repo()?;
    push_all(source.path(), remote.path())?;
    let target = tempfile::tempdir()?;
    let cancel = no_cancel();
    let _error = match gg_git::sync::clone(
        "--upload-pack=echo should-not-run",
        Some(&target.path().join("bad")),
        false,
        &cancel,
        &mut |_| {},
    ) {
        Ok(_) => return Err("option-shaped URL unexpectedly cloned".into()),
        Err(error) => error,
    };
    assert!(!target.path().join("bad").exists());
    Ok(())
}
