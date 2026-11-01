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

fn rev(p: &Path, revision: &str) -> Result<String> {
    Ok(String::from_utf8(git(p, &["rev-parse", revision])?)?
        .trim()
        .to_string())
}

#[test]
fn backups_record_the_rewritten_branch_and_their_save_time() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature/rebase-ui"])?;
    let head = rev(p, "HEAD")?;
    let name = gg_git::backup::create(p, "rebase", &head)?;
    assert!(name.ends_with("/feature/rebase-ui"), "{name}");
    git(p, &["switch", "-q", "--detach"])?;
    let detached = gg_git::backup::create(p, "amend", &head)?;
    let backups = gg_git::backup::list(p)?;
    assert_eq!(backups.len(), 2);
    assert_eq!(backups[0].reference, detached, "newest first");
    assert_eq!(backups[0].branch, None);
    assert_eq!(backups[1].branch.as_deref(), Some("feature/rebase-ui"));
    assert_eq!(backups[1].label, "rebase");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    assert!((now - backups[1].saved).abs() < 60, "{}", backups[1].saved);
    Ok(())
}

#[test]
fn restore_saves_the_current_tip_then_hard_resets_a_clean_branch() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let base = rev(p, "HEAD")?;
    std::fs::write(p.join("a"), b"second\n")?;
    git(p, &["commit", "-qam", "second"])?;
    let tip = rev(p, "HEAD")?;
    assert_eq!(
        gg_git::backup::branch_now(p, "main", &base),
        Some((tip.clone(), 1))
    );

    // A dirty tree is refused and nothing moves.
    std::fs::write(p.join("a"), b"edit\n")?;
    assert_eq!(gg_git::backup::uncommitted(p)?, 1);
    let error = match gg_git::backup::restore(p, &base, "main") {
        Ok(_) => return Err("restore of a dirty tree unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert_eq!(
        error,
        "1 file has uncommitted changes. Commit or stash it first."
    );
    assert!(gg_git::backup::list(p)?.is_empty());
    git(p, &["checkout", "--", "a"])?;

    // Another branch is refused.
    assert!(gg_git::backup::restore(p, &base, "other").is_err());

    let receipt = gg_git::backup::restore(p, &base, "main")?;
    assert_eq!(receipt.previous, tip);
    assert_eq!(receipt.commit, base);
    assert_eq!(rev(p, "HEAD")?, base);
    assert_eq!(std::fs::read(p.join("a"))?, b"base\n");
    let backups = gg_git::backup::list(p)?;
    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0].reference, receipt.saved);
    assert_eq!(backups[0].label, "restore");
    assert_eq!(backups[0].commit, tip, "the left tip can be restored again");
    assert_eq!(backups[0].branch.as_deref(), Some("main"));
    Ok(())
}

#[test]
fn delete_removes_one_backup_and_refuses_other_references() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let head = rev(p, "HEAD")?;
    let first = gg_git::backup::create(p, "amend", &head)?;
    let second = gg_git::backup::create(p, "rebase", &head)?;
    gg_git::backup::delete(p, &first)?;
    let backups = gg_git::backup::list(p)?;
    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0].reference, second);
    assert!(gg_git::backup::delete(p, "refs/heads/main").is_err());
    assert!(gg_git::backup::delete(p, "refs/gg/backup/../../heads/main").is_err());
    assert_eq!(rev(p, "refs/heads/main")?, head);
    Ok(())
}
