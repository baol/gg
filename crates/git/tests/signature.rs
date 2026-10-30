use std::{error::Error, path::Path, process::Command};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

const NO_CONFIG: &str = if cfg!(windows) { "NUL" } else { "/dev/null" };
const ISOLATED: &str = "GG_SIGNATURE_TEST_ISOLATED";

/// Runs `test` in a child test process whose Git sees no global or system
/// configuration, including the `git` commands that `gg_git` spawns itself.
/// A developer's global `gpg.format`, `user.signingkey`, or `*.gpgsign` would
/// otherwise leak into the signing results under test. Returns `true` in the
/// parent once the child passed, and `false` inside the child, which then runs
/// the test body.
fn in_isolated_child(test: &str) -> Result<bool> {
    if std::env::var_os(ISOLATED).is_some() {
        return Ok(false);
    }
    let output = Command::new(std::env::current_exe()?)
        .args([test, "--exact", "--test-threads=1", "--nocapture"])
        .env(ISOLATED, "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", NO_CONFIG)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() || !stdout.contains("1 passed") {
        return Err(format!(
            "isolated run of {test} failed:\n{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(true)
}

fn git(path: &Path, args: &[&str]) -> Result {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", NO_CONFIG)
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

fn repo() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-q", "-b", "main"])?;
    git(dir.path(), &["config", "user.name", "Test"])?;
    git(dir.path(), &["config", "user.email", "test@example.org"])?;
    git(dir.path(), &["config", "core.autocrlf", "false"])?;
    std::fs::write(dir.path().join("a"), b"base\n")?;
    git(dir.path(), &["add", "a"])?;
    git(dir.path(), &["commit", "-qm", "base"])?;
    Ok(dir)
}

#[test]
fn unsigned_commit_and_tag_report_kind_without_claiming_validity() -> Result {
    if in_isolated_child("unsigned_commit_and_tag_report_kind_without_claiming_validity")? {
        return Ok(());
    }
    let dir = repo()?;
    let commit = gg_git::signature::verify(dir.path(), "HEAD")?;
    assert_eq!(commit.kind.as_deref(), Some("commit"));
    assert_eq!(commit.state, gg_engine::signature::State::Unsigned);
    assert!(!commit.signed());
    assert!(commit.signer.is_none() && commit.key.is_none() && commit.trust.is_none());

    git(dir.path(), &["tag", "-a", "v1", "-m", "release"])?;
    let tag = gg_git::signature::verify(dir.path(), "v1")?;
    assert_eq!(tag.kind.as_deref(), Some("tag"));
    assert_eq!(tag.state, gg_engine::signature::State::Unsigned);
    Ok(())
}

#[test]
fn config_separates_signing_key_format_and_tool_readiness() -> Result {
    if in_isolated_child("config_separates_signing_key_format_and_tool_readiness")? {
        return Ok(());
    }
    let dir = repo()?;
    let config = gg_git::signature::config(dir.path());
    assert_eq!(config.format, "openpgp");
    assert!(config.signing_key.is_none());
    assert!(!config.commit_signing && !config.tag_signing);
    assert!(!config.ready());
    assert_eq!(
        config.note.as_deref(),
        Some("No signing key is configured.")
    );

    git(
        dir.path(),
        &["config", "user.signingkey", "0123456789ABCDEF"],
    )?;
    git(dir.path(), &["config", "commit.gpgsign", "true"])?;
    let config = gg_git::signature::config(dir.path());
    assert_eq!(config.signing_key.as_deref(), Some("0123456789ABCDEF"));
    assert!(config.commit_signing);
    assert!(!config.tag_signing);
    Ok(())
}

#[test]
fn non_commit_objects_and_invalid_references_are_rejected() -> Result {
    if in_isolated_child("non_commit_objects_and_invalid_references_are_rejected")? {
        return Ok(());
    }
    let dir = repo()?;
    let blob = Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args(["rev-parse", "HEAD:a"])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .output()?;
    let blob = String::from_utf8_lossy(&blob.stdout).trim().to_string();
    let error = match gg_git::signature::verify(dir.path(), &blob) {
        Err(error) => error,
        Ok(_) => return Err("blob verification unexpectedly succeeded".into()),
    };
    assert!(error.contains("not a commit or tag"), "{error}");

    let error = match gg_git::signature::verify(dir.path(), "--raw") {
        Err(error) => error,
        Ok(_) => return Err("option-like reference unexpectedly succeeded".into()),
    };
    assert!(error.contains("invalid"), "{error}");
    Ok(())
}
