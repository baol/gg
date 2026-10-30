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

#[test]
fn lists_tracked_files_and_finds_the_last_change() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-q", "-b", "main"])?;
    std::fs::create_dir(p.join("src"))?;
    std::fs::write(p.join("src/a b.rs"), b"one\n")?;
    std::fs::write(p.join("README"), b"readme\n")?;
    git(p, &["add", "."])?;
    git(p, &["commit", "-qm", "first"])?;
    std::fs::write(p.join("src/a b.rs"), b"two\n")?;
    git(p, &["commit", "-qam", "change a b"])?;
    std::fs::write(p.join("README"), b"again\n")?;
    git(p, &["commit", "-qam", "readme"])?;

    assert_eq!(
        gg_git::files::tracked(p, 10)?,
        vec!["README".to_string(), "src/a b.rs".to_string()]
    );
    assert_eq!(
        gg_git::files::tracked(p, 1)?.len(),
        1,
        "the list is bounded"
    );
    let (_, subject) = gg_git::files::last_change(p, "src/a b.rs")?.ok_or("no change")?;
    assert_eq!(subject, "change a b");
    assert_eq!(gg_git::files::last_change(p, "missing")?, None);
    Ok(())
}
