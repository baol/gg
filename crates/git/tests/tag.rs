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
    std::fs::write(dir.path().join("a"), b"base\n")?;
    git(dir.path(), &["add", "a"])?;
    git(dir.path(), &["commit", "-qm", "base"])?;
    Ok(dir)
}

#[test]
fn deletes_a_tag_and_rejects_invalid_names() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["tag", "v1"])?;
    let receipt = gg_git::tag::delete(p, "v1")?;
    assert_eq!(receipt.tag, "v1");
    let error = match gg_git::tag::delete(p, "missing") {
        Ok(_) => return Err("missing tag unexpectedly deleted".into()),
        Err(error) => error,
    };
    assert!(error.contains("failed"), "{error}");
    let error = match gg_git::tag::delete(p, "--force") {
        Ok(_) => return Err("option-shaped tag unexpectedly deleted".into()),
        Err(error) => error,
    };
    assert!(error.contains("invalid"), "{error}");
    Ok(())
}
