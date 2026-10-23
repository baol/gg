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

fn repo() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-b", "main"])?;
    git(dir.path(), &["config", "user.name", "Test"])?;
    git(dir.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(dir.path().join("a"), b"base\n")?;
    git(dir.path(), &["add", "a"])?;
    git(dir.path(), &["commit", "-qm", "base"])?;
    std::fs::write(dir.path().join("a"), b"next\n")?;
    git(dir.path(), &["commit", "-qam", "next"])?;
    Ok(dir)
}

#[test]
fn reads_head_and_branch_reflogs_in_order() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let head = gg_git::reflog::read(p, "HEAD")?;
    assert_eq!(head.reference, "HEAD");
    assert!(head.entries.len() >= 2);
    assert_eq!(head.entries[0].selector, "HEAD@{0}");
    assert_eq!(head.entries[1].selector, "HEAD@{1}");
    assert_eq!(
        head.entries[0].id,
        String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?.trim()
    );
    assert_eq!(head.entries[0].action, "commit");
    assert_eq!(head.entries[0].message, "next");
    assert!(head.entries[0].time > 0);

    let branch = gg_git::reflog::read(p, "main")?;
    assert_eq!(branch.entries[0].selector, "main@{0}");
    assert!(branch.entries.iter().any(|entry| entry.message == "next"));
    Ok(())
}

#[test]
fn rejects_missing_and_option_shaped_reflogs() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let error = match gg_git::reflog::read(p, "missing") {
        Ok(_) => return Err("missing reflog unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("failed"), "{error}");
    let error = match gg_git::reflog::read(p, "--stat") {
        Ok(_) => return Err("option-shaped reflog unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("invalid"), "{error}");
    Ok(())
}
