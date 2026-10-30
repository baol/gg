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
    let backups = gg_git::backup::list(p)?;
    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0].reference, name);
    assert_eq!(backups[0].label, "amend");
    assert_eq!(backups[0].commit, head);
    assert_eq!(backups[0].subject, "base");
    assert!(!backups[0].time.is_empty());
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

#[test]
fn prune_deletes_only_gg_backups_older_than_the_age() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let head = String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?
        .trim()
        .to_string();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
    let day: u128 = 86_400 * 1_000_000_000;
    let old = format!("refs/gg/backup/rewrite/{}", now.as_nanos() - 40 * day);
    let recent = format!("refs/gg/backup/amend/{}", now.as_nanos() - day);
    git(p, &["update-ref", &old, &head])?;
    git(p, &["update-ref", &recent, &head])?;
    git(p, &["update-ref", "refs/heads/keep-me", &head])?;
    let removed = gg_git::backup::prune(p, std::time::Duration::from_secs(30 * 86_400))?;
    assert_eq!(removed, 1);
    let refs = String::from_utf8(git(p, &["for-each-ref", "--format=%(refname)"])?)?;
    assert!(!refs.contains(&old), "{refs}");
    assert!(refs.contains(&recent), "{refs}");
    assert!(refs.contains("refs/heads/keep-me"), "{refs}");
    Ok(())
}
