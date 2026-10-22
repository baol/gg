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
    std::fs::write(dir.path().join("a"), b"one\n")?;
    git(dir.path(), &["add", "a"])?;
    git(dir.path(), &["commit", "-qm", "one"])?;
    std::fs::write(dir.path().join("a"), b"two\n")?;
    git(dir.path(), &["commit", "-qam", "two"])?;
    Ok(dir)
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
