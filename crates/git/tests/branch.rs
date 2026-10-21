use std::{error::Error, path::Path, process::Command};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn git(path: &Path, args: &[&str]) -> Result<Vec<u8>> {
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
    Ok(output.stdout)
}

fn rejected(result: std::result::Result<gg_git::branch::Receipt, String>) -> Result<String> {
    match result {
        Err(error) => Ok(error),
        Ok(_) => Err("branch operation unexpectedly succeeded".into()),
    }
}

fn repo() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-b", "main"])?;
    git(dir.path(), &["config", "user.name", "Test"])?;
    git(dir.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(dir.path().join("a"), b"main\n")?;
    git(dir.path(), &["add", "a"])?;
    git(dir.path(), &["commit", "-qm", "base"])?;
    Ok(dir)
}

#[test]
fn switches_branches_only_when_clean() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-c", "feature"])?;
    std::fs::write(p.join("a"), b"feature\n")?;
    git(p, &["commit", "-qam", "feature"])?;
    git(p, &["switch", "main"])?;

    let hooks = p.join(".git/hooks");
    std::fs::create_dir_all(&hooks)?;
    let hook = hooks.join("post-checkout");
    std::fs::write(
        &hook,
        "#!/bin/sh\nprintf 'checked out\n' > checkout-hook.txt\n",
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
    }
    let receipt = gg_git::branch::switch(p, "feature")?;
    assert_eq!(receipt.branch, "feature");
    assert!(!receipt.created);
    assert_eq!(git(p, &["symbolic-ref", "--short", "HEAD"])?, b"feature\n");
    assert_eq!(
        std::fs::read_to_string(p.join("checkout-hook.txt"))?,
        "checked out\n"
    );
    assert_eq!(std::fs::read(p.join("a"))?, b"feature\n");

    std::fs::write(p.join("a"), b"dirty\n")?;
    let error = rejected(gg_git::branch::switch(p, "main"))?;
    assert!(error.contains("not clean"), "{error}");
    assert_eq!(git(p, &["symbolic-ref", "--short", "HEAD"])?, b"feature\n");
    assert_eq!(std::fs::read(p.join("a"))?, b"dirty\n");
    std::fs::write(p.join("a"), b"feature\n")?;
    git(p, &["switch", "main"])?;
    Ok(())
}

#[test]
fn creates_and_switches_to_a_new_branch() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let parent = String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?
        .trim()
        .to_string();
    let receipt = gg_git::branch::create(p, "next")?;
    assert!(receipt.created);
    assert_eq!(receipt.branch, "next");
    assert_eq!(git(p, &["symbolic-ref", "--short", "HEAD"])?, b"next\n");
    assert_eq!(
        git(p, &["rev-parse", "HEAD"])?,
        format!("{parent}\n").as_bytes()
    );
    assert_eq!(std::fs::read(p.join("a"))?, b"main\n");
    let error = rejected(gg_git::branch::create(p, "next"))?;
    assert!(error.contains("already exists"), "{error}");
    let error = rejected(gg_git::branch::create(p, "bad..name"))?;
    assert!(error.contains("Invalid branch name"), "{error}");
    Ok(())
}

#[test]
fn rejects_dirty_missing_and_unborn_branches_and_switches_from_detached_head() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let head = git(p, &["rev-parse", "HEAD"])?;
    std::fs::write(p.join("new"), b"untracked\n")?;
    let error = rejected(gg_git::branch::create(p, "blocked"))?;
    assert!(error.contains("not clean"), "{error}");
    assert_eq!(git(p, &["rev-parse", "HEAD"])?, head);
    std::fs::remove_file(p.join("new"))?;

    std::fs::write(p.join("a"), b"staged\n")?;
    git(p, &["add", "a"])?;
    let error = rejected(gg_git::branch::switch(p, "feature"))?;
    assert!(error.contains("not clean"), "{error}");
    git(p, &["reset", "--hard", "HEAD"])?;

    git(p, &["branch", "feature"])?;
    git(p, &["checkout", "-q", "--detach"])?;
    let receipt = gg_git::branch::switch(p, "feature")?;
    assert_eq!(receipt.branch, "feature");
    assert_eq!(git(p, &["symbolic-ref", "--short", "HEAD"])?, b"feature\n");
    let error = rejected(gg_git::branch::switch(p, "missing"))?;
    assert!(error.contains("does not exist"), "{error}");

    let unborn = tempfile::tempdir()?;
    git(unborn.path(), &["init", "-b", "trunk"])?;
    let error = rejected(gg_git::branch::create(unborn.path(), "next"))?;
    assert!(error.contains("no commits yet"), "{error}");
    Ok(())
}

#[test]
fn post_checkout_failure_is_a_warning_after_the_switch() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["branch", "feature"])?;
    let hooks = p.join(".git/hooks");
    std::fs::create_dir_all(&hooks)?;
    let hook = hooks.join("post-checkout");
    std::fs::write(&hook, "#!/bin/sh\necho hook-warning >&2\nexit 3\n")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
    }
    let receipt = gg_git::branch::switch(p, "feature")?;
    assert_eq!(receipt.branch, "feature");
    assert!(!receipt.warnings.is_empty());
    assert!(receipt.warnings[0].contains("post-checkout"));
    assert!(receipt.warnings[0].contains("hook-warning"));
    assert_eq!(git(p, &["symbolic-ref", "--short", "HEAD"])?, b"feature\n");
    Ok(())
}
