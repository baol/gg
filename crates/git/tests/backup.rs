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
    Ok(dir)
}

#[test]
fn backup_refs_point_at_the_previous_tip_and_stay_out_of_refs() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let head = String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?
        .trim()
        .to_string();
    let name = gg_git::backup::create(p, "amend", &head)?;
    assert!(name.starts_with("refs/gg/backup/amend/"));
    assert_eq!(
        String::from_utf8(git(p, &["rev-parse", &name])?)?.trim(),
        head
    );
    let refs = gg_git::refs::read(p)?;
    assert!(
        refs.items
            .iter()
            .all(|item| item.full_name != name && !item.full_name.starts_with("refs/gg/"))
    );

    let error = match gg_git::backup::create(p, "amend", "missing") {
        Ok(_) => return Err("missing backup target unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("not found"), "{error}");
    let error = match gg_git::backup::create(p, "amend", "--force") {
        Ok(_) => return Err("option-shaped backup target unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("invalid"), "{error}");
    Ok(())
}

#[test]
fn backup_refs_work_in_sha256_repositories() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main", "--object-format=sha256"])?;
    git(p, &["config", "user.name", "Test"])?;
    git(p, &["config", "user.email", "test@example.org"])?;
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    git(p, &["commit", "-qm", "base"])?;
    let head = String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?
        .trim()
        .to_string();
    assert_eq!(head.len(), 64);
    let name = gg_git::backup::create(p, "amend", &head)?;
    assert_eq!(
        String::from_utf8(git(p, &["rev-parse", &name])?)?.trim(),
        head
    );
    Ok(())
}
