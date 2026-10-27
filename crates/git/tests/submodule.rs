use std::{error::Error, path::Path, process::Command};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn git(path: &Path, args: &[&str]) -> Result {
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
    Ok(())
}

fn output(path: &Path, args: &[&str]) -> Result<String> {
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
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn fixture() -> Result<(tempfile::TempDir, tempfile::TempDir, String, String)> {
    let nested = tempfile::tempdir()?;
    git(nested.path(), &["init", "-q", "-b", "main"])?;
    git(nested.path(), &["config", "user.name", "Test"])?;
    git(nested.path(), &["config", "user.email", "test@example.org"])?;
    git(nested.path(), &["config", "core.autocrlf", "false"])?;
    std::fs::write(nested.path().join("lib.txt"), b"one\n")?;
    git(nested.path(), &["add", "lib.txt"])?;
    git(nested.path(), &["commit", "-qm", "nested one"])?;
    let first = output(nested.path(), &["rev-parse", "HEAD"])?;
    std::fs::write(nested.path().join("lib.txt"), b"two\n")?;
    git(nested.path(), &["commit", "-qam", "nested two"])?;
    let second = output(nested.path(), &["rev-parse", "HEAD"])?;

    let parent = tempfile::tempdir()?;
    git(parent.path(), &["init", "-q", "-b", "main"])?;
    git(parent.path(), &["config", "user.name", "Test"])?;
    git(parent.path(), &["config", "user.email", "test@example.org"])?;
    git(parent.path(), &["config", "core.autocrlf", "false"])?;
    std::fs::write(parent.path().join("a"), b"base\n")?;
    git(parent.path(), &["add", "a"])?;
    git(parent.path(), &["commit", "-qm", "base"])?;
    let add = Command::new("git")
        .arg("-C")
        .arg(parent.path())
        .args(["-c", "protocol.file.allow=always", "submodule", "add"])
        .arg(nested.path())
        .arg("sub")
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
    if !add.status.success() {
        return Err(String::from_utf8_lossy(&add.stderr).into_owned().into());
    }
    git(parent.path(), &["commit", "-qm", "add submodule"])?;
    Ok((parent, nested, first, second))
}

#[test]
fn details_name_recorded_checked_out_and_index_state() -> Result {
    let (parent, _nested, _first, second) = fixture()?;
    let index = output(parent.path(), &["rev-parse", "HEAD:sub"])?;
    let details = gg_git::submodule::read(parent.path(), b"sub", Some(&index), Some(&second))?;
    assert_eq!(details.path, "sub");
    assert_eq!(details.index.as_deref(), Some(index.as_str()));
    assert_eq!(details.recorded.as_deref(), Some(index.as_str()));
    assert_eq!(details.checked_out.as_deref(), Some(second.as_str()));
    assert!(details.initialized);
    assert_eq!(details.dirty, Some(false));
    assert_eq!(details.recorded_available, Some(true));
    assert!(details.url.is_some());
    assert_eq!(details.status.as_deref(), Some(" "));
    assert!(details.can_update().is_ok());
    Ok(())
}

#[test]
fn dirty_and_index_mismatch_block_update_and_update_restores_the_index_commit() -> Result {
    let (parent, _nested, first, second) = fixture()?;
    let index = output(parent.path(), &["rev-parse", "HEAD:sub"])?;
    git(&parent.path().join("sub"), &["checkout", "-q", &first])?;
    let details = gg_git::submodule::read(parent.path(), b"sub", Some(&index), Some(&second))?;
    assert_eq!(details.checked_out.as_deref(), Some(first.as_str()));
    assert_eq!(details.status.as_deref(), Some("+"));
    assert!(details.can_update().is_ok());
    let (updated, receipt) = gg_git::submodule::update(parent.path(), b"sub")?;
    assert_eq!(updated.checked_out.as_deref(), Some(second.as_str()));
    assert_eq!(receipt.target.as_deref(), Some(index.as_str()));
    assert_eq!(updated.status.as_deref(), Some(" "));

    let mismatch = gg_git::submodule::read(parent.path(), b"sub", Some(&first), Some(&second))?;
    assert!(mismatch.can_update().is_err());

    std::fs::write(parent.path().join("sub/lib.txt"), b"dirty\n")?;
    let dirty = gg_git::submodule::read(parent.path(), b"sub", Some(&index), Some(&second))?;
    assert_eq!(dirty.dirty, Some(true));
    assert!(dirty.can_update().is_err());
    Ok(())
}

#[test]
fn uninitialized_submodule_keeps_the_parent_index_identity() -> Result {
    let (parent, _nested, _first, _second) = fixture()?;
    let index = output(parent.path(), &["rev-parse", "HEAD:sub"])?;
    let clone = tempfile::tempdir()?;
    let destination = clone.path().join("clone");
    let output = Command::new("git")
        .args(["clone", "-q"])
        .arg(parent.path())
        .arg(&destination)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let details = gg_git::submodule::read(&destination, b"sub", None, None)?;
    assert!(!details.initialized);
    assert_eq!(details.index.as_deref(), Some(index.as_str()));
    assert_eq!(details.checked_out, None);
    assert_eq!(details.dirty, None);
    assert_eq!(details.recorded_available, None);
    assert!(details.url.is_some());
    assert!(details.can_update().is_ok());
    Ok(())
}
