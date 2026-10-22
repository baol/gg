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
    std::fs::write(dir.path().join("a"), b"base\n")?;
    git(dir.path(), &["add", "a"])?;
    git(dir.path(), &["commit", "-qm", "base"])?;
    Ok(dir)
}

fn conflicting(dir: &Path) -> Result {
    git(dir, &["switch", "-qc", "feature"])?;
    std::fs::write(dir.join("a"), b"feature\n")?;
    git(dir, &["commit", "-qam", "feature"])?;
    git(dir, &["switch", "-q", "main"])?;
    std::fs::write(dir.join("a"), b"main\n")?;
    git(dir, &["commit", "-qam", "main"])?;
    let error = gg_git::merge::merge(dir, "feature", false);
    assert!(error.is_ok());
    Ok(())
}

#[test]
fn reads_and_selects_conflict_sides() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    assert_eq!(gg_git::conflict::unmerged(p)?, vec!["a".to_string()]);

    let content = gg_git::conflict::read(p, "a")?;
    assert_eq!(content.base.as_deref(), Some("base\n"));
    assert_eq!(content.ours.as_deref(), Some("main\n"));
    assert_eq!(content.theirs.as_deref(), Some("feature\n"));
    assert!(!content.binary);
    assert!(!content.truncated);

    gg_git::conflict::use_side(p, "a", gg_engine::conflict::Side::Theirs)?;
    assert_eq!(std::fs::read(p.join("a"))?, b"feature\n");
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    Ok(())
}

#[test]
fn stages_manually_resolved_content() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    std::fs::write(p.join("a"), b"resolved\n")?;
    gg_git::conflict::stage(p, "a")?;
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    assert_eq!(std::fs::read(p.join("a"))?, b"resolved\n");
    Ok(())
}

#[test]
fn add_add_conflict_has_no_base() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("new"), b"feature\n")?;
    git(p, &["add", "new"])?;
    git(p, &["commit", "-qm", "feature new"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("new"), b"main\n")?;
    git(p, &["add", "new"])?;
    git(p, &["commit", "-qm", "main new"])?;
    gg_git::merge::merge(p, "feature", false)?;

    let content = gg_git::conflict::read(p, "new")?;
    assert!(content.base.is_none());
    assert_eq!(content.ours.as_deref(), Some("main\n"));
    assert_eq!(content.theirs.as_deref(), Some("feature\n"));
    Ok(())
}

#[test]
fn rejects_unknown_paths_and_base_selection() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let error = match gg_git::conflict::read(p, "missing") {
        Ok(_) => return Err("missing conflict unexpectedly read".into()),
        Err(error) => error,
    };
    assert!(error.contains("No unmerged stages"), "{error}");
    let error = match gg_git::conflict::use_side(p, "a", gg_engine::conflict::Side::Base) {
        Ok(_) => return Err("base side unexpectedly selected".into()),
        Err(error) => error,
    };
    assert!(error.contains("Base content"), "{error}");
    Ok(())
}
