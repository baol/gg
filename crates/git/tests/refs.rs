use gg_engine::refs::RefKind;
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

#[test]
fn reads_head_branches_remotes_and_annotated_tags() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    git(p, &["commit", "-qm", "base"])?;
    let head = String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?
        .trim()
        .to_string();
    git(p, &["branch", "feature"])?;
    git(p, &["tag", "light"])?;
    git(p, &["tag", "-a", "annotated", "-m", "tag"])?;
    git(p, &["update-ref", "refs/remotes/origin/main", &head])?;

    let snapshot = gg_git::refs::read(p)?;
    let head_ref = snapshot.head.ok_or("missing HEAD")?;
    assert_eq!(head_ref.name.as_deref(), Some("refs/heads/main"));
    assert!(!head_ref.detached);
    assert_eq!(head_ref.commit.as_deref(), Some(head.as_str()));
    let branch = snapshot
        .items
        .iter()
        .find(|item| item.kind == RefKind::Branch && item.name == "feature")
        .ok_or("missing feature branch")?;
    assert_eq!(branch.commit, head);
    let remote = snapshot
        .items
        .iter()
        .find(|item| item.kind == RefKind::Remote)
        .ok_or("missing remote branch")?;
    assert_eq!(remote.name, "origin/main");
    assert_eq!(remote.commit, head);
    for name in ["light", "annotated"] {
        let tag = snapshot
            .items
            .iter()
            .find(|item| item.kind == RefKind::Tag && item.name == name)
            .ok_or("missing tag")?;
        assert_eq!(tag.commit, head);
    }
    Ok(())
}

#[test]
fn unborn_and_detached_heads_are_explicit() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "unborn"])?;
    let snapshot = gg_git::refs::read(p)?;
    let head = snapshot.head.ok_or("missing HEAD")?;
    assert_eq!(head.name.as_deref(), Some("refs/heads/unborn"));
    assert!(head.commit.is_none());
    assert!(snapshot.items.is_empty());

    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    git(p, &["commit", "-qm", "base"])?;
    git(p, &["checkout", "-q", "--detach"])?;
    let snapshot = gg_git::refs::read(p)?;
    let head = snapshot.head.ok_or("missing detached HEAD")?;
    assert!(head.detached);
    assert!(head.name.is_none());
    assert!(head.commit.is_some());
    Ok(())
}

#[test]
fn duplicate_loose_and_packed_refs_are_reported_once() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    git(p, &["commit", "-qm", "base"])?;
    let old = String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?
        .trim()
        .to_string();
    std::fs::write(p.join("a"), b"newer\n")?;
    git(p, &["commit", "-qam", "newer"])?;
    let new = String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?
        .trim()
        .to_string();
    git(p, &["update-ref", "refs/remotes/origin/main", &old])?;
    git(p, &["pack-refs", "--all"])?;
    let loose = p.join(".git/refs/remotes/origin");
    std::fs::create_dir_all(&loose)?;
    std::fs::write(loose.join("main"), format!("{new}\n"))?;

    let snapshot = gg_git::refs::read(p)?;
    let remotes = snapshot
        .items
        .iter()
        .filter(|item| item.kind == RefKind::Remote && item.name == "origin/main")
        .collect::<Vec<_>>();
    assert_eq!(remotes.len(), 1);
    assert_eq!(remotes[0].commit, new);
    Ok(())
}
