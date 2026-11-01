use std::{error::Error, path::Path, process::Command};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

const IDENTITY: &[(&str, &str)] = &[
    ("user.name", "Test"),
    ("user.email", "test@example.org"),
    ("commit.gpgsign", "false"),
    ("core.autocrlf", "false"),
];

fn git(path: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

#[test]
fn init_creates_the_folder_on_the_named_branch_without_committing() -> Result {
    let dir = tempfile::tempdir()?;
    let target = dir.path().join("new/project");
    let receipt = gg_git::init::init_configured(&target, "trunk", false, IDENTITY)?;
    assert_eq!(receipt.path, std::fs::canonicalize(&target)?);
    assert_eq!(receipt.branch, "trunk");
    assert_eq!(receipt.head, None);
    assert_eq!(git(&target, &["symbolic-ref", "--short", "HEAD"])?, "trunk");
    assert!(git(&target, &["rev-parse", "--verify", "-q", "HEAD"]).is_err());
    Ok(())
}

#[test]
fn init_commits_the_existing_files_except_ignored_ones() -> Result {
    let dir = tempfile::tempdir()?;
    let target = dir.path();
    std::fs::write(target.join("a.txt"), b"a\n")?;
    std::fs::create_dir(target.join("notes"))?;
    std::fs::write(target.join("notes/b.md"), b"b\n")?;
    std::fs::write(target.join(".gitignore"), b"build/\n")?;
    std::fs::create_dir(target.join("build"))?;
    std::fs::write(target.join("build/out.o"), b"o")?;
    assert_eq!(
        gg_git::init::count_files(target)?,
        gg_git::init::Count {
            files: 4,
            ignored: 1
        }
    );
    let receipt = gg_git::init::init_configured(target, "main", true, IDENTITY)?;
    assert_eq!(receipt.files, 3);
    let head = receipt.head.ok_or("no commit")?;
    assert_eq!(git(target, &["rev-parse", "HEAD"])?, head);
    assert_eq!(
        git(target, &["log", "-1", "--format=%s"])?,
        "Initial commit"
    );
    assert_eq!(
        git(target, &["ls-tree", "-r", "--name-only", "HEAD"])?,
        ".gitignore\na.txt\nnotes/b.md"
    );
    assert_eq!(git(target, &["status", "--porcelain"])?, "");
    Ok(())
}

#[test]
fn init_with_commit_in_an_empty_folder_makes_no_commit() -> Result {
    let dir = tempfile::tempdir()?;
    let receipt = gg_git::init::init_configured(dir.path(), "main", true, IDENTITY)?;
    assert_eq!((receipt.head, receipt.files), (None, 0));
    Ok(())
}

#[test]
fn init_refuses_inside_an_existing_repository() -> Result {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-q", "-b", "main"])?;
    let nested = dir.path().join("docs");
    std::fs::create_dir(&nested)?;
    std::fs::write(nested.join("x"), b"x")?;
    let top = std::fs::canonicalize(dir.path())?;
    assert_eq!(
        gg_git::init::enclosing_repository(&nested),
        Some(top.clone())
    );
    assert_eq!(
        gg_git::init::enclosing_repository(&nested.join("not/yet")),
        Some(top.clone())
    );
    let error = gg_git::init::init_configured(&nested, "main", true, IDENTITY)
        .err()
        .ok_or("init inside a repository succeeded")?;
    assert!(
        error.contains("is inside the repository") && error.contains(&*top.to_string_lossy()),
        "{error}"
    );
    assert!(!nested.join(".git").exists(), "nothing changed");
    assert_eq!(git(dir.path(), &["status", "--porcelain"])?, "?? docs/");
    Ok(())
}

#[test]
fn init_rejects_a_bad_branch_and_a_relative_folder() -> Result {
    let dir = tempfile::tempdir()?;
    let target = dir.path().join("x");
    assert!(gg_git::init::init_configured(&target, "bad..name", false, IDENTITY).is_err());
    assert!(gg_git::init::init_configured(&target, " ", false, IDENTITY).is_err());
    assert!(!target.exists(), "a refused init creates nothing");
    assert!(gg_git::init::init(Path::new("relative"), "main", false).is_err());
    Ok(())
}
