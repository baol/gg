use std::{error::Error, path::Path, process::Command};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

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
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// CHG-77: each pattern Ignore writes makes Git ignore what it says, and
/// nothing else; the file's other lines are kept, and a pattern already
/// there is not written twice.
#[test]
fn ignore_patterns_match_what_git_ignores() -> Result {
    use gg_git::ignore::{Scope, add, pattern};
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-q", "-b", "main"])?;
    std::fs::write(p.join(".gitignore"), b"/target")?; // no final newline
    std::fs::create_dir_all(p.join("logs"))?;
    for file in [
        "build.log",
        "logs/a.log",
        "logs/b.txt",
        "odd [1].txt",
        "other.txt",
    ] {
        std::fs::write(p.join(file), b"x\n")?;
    }
    let ignored = |file: &str| -> Result<bool> {
        Ok(!git(p, &["check-ignore", "--no-index", file])?
            .trim()
            .is_empty())
    };

    let file = pattern(b"odd [1].txt", Scope::File)?.ok_or("no pattern")?;
    let receipt = add(p, &file)?;
    assert!(receipt.added);
    assert!(ignored("odd [1].txt")? && !ignored("other.txt")?);

    let extension = pattern(b"logs/a.log", Scope::Extension)?.ok_or("no pattern")?;
    add(p, &extension)?;
    assert!(ignored("build.log")? && ignored("logs/a.log")?);
    assert!(!ignored("logs/b.txt")?);

    let folder = pattern(b"logs/b.txt", Scope::Folder)?.ok_or("no pattern")?;
    add(p, &folder)?;
    assert!(ignored("logs/b.txt")?);

    let again = add(p, &folder)?;
    assert!(!again.added, "already there");
    assert_eq!(
        std::fs::read_to_string(p.join(".gitignore"))?,
        "/target\n/odd \\[1].txt\n*.log\n/logs/\n"
    );
    assert_eq!(
        receipt.file.canonicalize()?,
        p.join(".gitignore").canonicalize()?
    );
    assert!(add(p, "two\nlines").is_err());
    Ok(())
}

/// The root `.gitignore` is the one written, from a subfolder too.
#[test]
fn ignore_writes_the_root_gitignore_from_a_subfolder() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-q", "-b", "main"])?;
    std::fs::create_dir_all(p.join("src"))?;
    let receipt = gg_git::ignore::add(&p.join("src"), "/src/gen.rs")?;
    assert!(receipt.added);
    assert_eq!(
        std::fs::read_to_string(p.join(".gitignore"))?,
        "/src/gen.rs\n"
    );
    assert!(!p.join("src/.gitignore").exists());
    Ok(())
}
