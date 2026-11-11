use gg_git::identity::{self, Identity, Scope};
use std::{error::Error, path::Path, process::Command};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn git(path: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

#[test]
fn set_writes_name_and_email_that_git_then_reads() -> Result {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("gitconfig");
    assert_eq!(
        identity::read_scope(Scope::File(&file)),
        Identity::default()
    );
    identity::set(Scope::File(&file), " Ada Lovelace ", "ada@example.org")?;
    let read = identity::read_scope(Scope::File(&file));
    assert!(read.complete());
    assert_eq!(
        read.label().as_deref(),
        Some("Ada Lovelace <ada@example.org>")
    );
    // Git itself reads what was written.
    let config = file.to_string_lossy().into_owned();
    let name = Command::new("git")
        .args(["config", "--file", &config, "user.name"])
        .output()?;
    assert_eq!(String::from_utf8(name.stdout)?.trim(), "Ada Lovelace");
    Ok(())
}

#[test]
fn set_refuses_a_missing_or_multi_line_value_and_writes_nothing() -> Result {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("gitconfig");
    for (name, email) in [
        ("", "ada@example.org"),
        ("Ada", "  "),
        ("Ada\nLovelace", "ada@example.org"),
        ("Ada", "<ada@example.org>"),
    ] {
        assert!(
            identity::set(Scope::File(&file), name, email).is_err(),
            "{name:?} {email:?}"
        );
    }
    assert_eq!(
        identity::read_scope(Scope::File(&file)),
        Identity::default()
    );
    Ok(())
}

#[test]
fn read_in_a_repository_prefers_its_own_config() -> Result {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-q"])?;
    git(dir.path(), &["config", "user.name", "Local Name"])?;
    git(dir.path(), &["config", "user.email", "local@example.org"])?;
    let read = identity::read(Some(dir.path()));
    assert_eq!(read.name.as_deref(), Some("Local Name"));
    assert_eq!(read.email.as_deref(), Some("local@example.org"));
    Ok(())
}
