use gg_engine::file_history::{HistoryKind, Predecessor};
use gg_git::file_history::Session;
use std::{error::Error, path::Path, process::Command};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn git(path: &Path, args: &[&str]) -> Result<String> {
    git_date(path, args, "2026-01-01T12:00:00+00:00")
}

fn git_date(path: &Path, args: &[&str], date: &str) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_AUTHOR_NAME", "Test Author")
        .env("GIT_AUTHOR_EMAIL", "test@example.org")
        .env("GIT_COMMITTER_NAME", "Test Author")
        .env("GIT_COMMITTER_EMAIL", "test@example.org")
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn init() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-b", "main"])?;
    git(dir.path(), &["config", "core.autocrlf", "false"])?;
    Ok(dir)
}

fn write(path: &Path, name: &str, content: &str) -> Result {
    std::fs::write(path.join(name), content)?;
    Ok(())
}

fn commit(path: &Path, message: &str, date: &str) -> Result {
    git_date(path, &["add", "-A"], date)?;
    git_date(path, &["commit", "-m", message], date)?;
    Ok(())
}

fn open(path: &Path, revision: &str, file: &[u8]) -> std::result::Result<Session, String> {
    Session::open(path, revision, file, &|| false)
}

fn collect(session: &mut Session) -> Result<Vec<String>> {
    while !session.complete() {
        if !session.next(&|| false)? {
            break;
        }
    }
    Ok(session
        .versions()
        .iter()
        .map(|version| version.path.clone())
        .collect())
}

#[test]
fn linear_history_skips_commits_that_carry_the_same_content() -> Result {
    let dir = init()?;
    write(dir.path(), "f.txt", "one\n")?;
    commit(dir.path(), "Add file", "2026-01-01T12:00:00+00:00")?;
    git_date(
        dir.path(),
        &["commit", "--allow-empty", "-m", "Unrelated"],
        "2026-01-02T12:00:00+00:00",
    )?;
    write(dir.path(), "f.txt", "two\n")?;
    commit(dir.path(), "Change file", "2026-01-03T12:00:00+00:00")?;

    let mut session = open(dir.path(), "HEAD", b"f.txt").map_err(|e| e.to_string())?;
    assert_eq!(session.versions().len(), 1);
    assert_eq!(session.versions()[0].subject, "Change file");
    assert_eq!(session.versions()[0].kind, Some(HistoryKind::Modified));
    assert!(matches!(session.versions()[0].previous, Predecessor::More));
    assert!(collect(&mut session)?.len() == 2);
    let versions = session.versions();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].subject, "Change file");
    assert_eq!(versions[1].subject, "Add file");
    assert_eq!(versions[1].kind, Some(HistoryKind::Added));
    assert!(matches!(versions[1].previous, Predecessor::Oldest));
    assert!(versions[1].old_path.is_none());
    Ok(())
}

#[test]
fn an_unchanged_commit_still_resolves_to_the_content_version() -> Result {
    let dir = init()?;
    write(dir.path(), "other.txt", "other\n")?;
    commit(dir.path(), "Root", "2026-01-01T12:00:00+00:00")?;
    write(dir.path(), "f.txt", "one\n")?;
    commit(dir.path(), "Add file", "2026-01-02T12:00:00+00:00")?;
    git_date(
        dir.path(),
        &["commit", "--allow-empty", "-m", "Unrelated"],
        "2026-01-03T12:00:00+00:00",
    )?;
    let unrelated = git(dir.path(), &["rev-parse", "HEAD"])?;
    let mut session = open(dir.path(), &unrelated, b"f.txt").map_err(|e| e.to_string())?;
    assert_eq!(session.versions()[0].subject, "Add file");
    assert!(collect(&mut session)?.len() == 2);
    assert_eq!(session.versions()[1].side, None);
    assert!(matches!(
        session.versions()[1].previous,
        Predecessor::Oldest
    ));
    Ok(())
}

#[test]
fn renames_are_followed_and_name_the_old_path() -> Result {
    let dir = init()?;
    write(dir.path(), "old.txt", "one\ntwo\nthree\n")?;
    commit(dir.path(), "Add old", "2026-01-01T12:00:00+00:00")?;
    git_date(
        dir.path(),
        &["mv", "old.txt", "new.txt"],
        "2026-01-02T12:00:00+00:00",
    )?;
    git_date(
        dir.path(),
        &["commit", "-m", "Rename"],
        "2026-01-02T12:00:00+00:00",
    )?;
    write(dir.path(), "new.txt", "one\ntwo\nthree\nfour\n")?;
    commit(dir.path(), "Change new", "2026-01-03T12:00:00+00:00")?;

    let mut session = open(dir.path(), "HEAD", b"new.txt").map_err(|e| e.to_string())?;
    assert_eq!(collect(&mut session)?.len(), 3);
    let versions = session.versions();
    assert_eq!(versions[0].path, "new.txt");
    assert_eq!(versions[0].kind, Some(HistoryKind::Modified));
    assert_eq!(versions[1].path, "new.txt");
    assert_eq!(versions[1].kind, Some(HistoryKind::Renamed));
    assert_eq!(versions[1].old_path.as_deref(), Some("old.txt"));
    assert_eq!(versions[1].old_path_bytes.as_deref(), Some(&b"old.txt"[..]));
    assert_eq!(versions[2].path, "old.txt");
    assert_eq!(versions[2].kind, Some(HistoryKind::Added));
    Ok(())
}

#[test]
fn delete_and_readd_produce_an_explicit_absent_version() -> Result {
    let dir = init()?;
    write(dir.path(), "f.txt", "one\n")?;
    commit(dir.path(), "Add", "2026-01-01T12:00:00+00:00")?;
    std::fs::remove_file(dir.path().join("f.txt"))?;
    commit(dir.path(), "Delete", "2026-01-02T12:00:00+00:00")?;
    write(dir.path(), "f.txt", "two\n")?;
    commit(dir.path(), "Re-add", "2026-01-03T12:00:00+00:00")?;

    let mut session = open(dir.path(), "HEAD", b"f.txt").map_err(|e| e.to_string())?;
    assert_eq!(collect(&mut session)?.len(), 3);
    let versions = session.versions();
    assert!(versions[0].present());
    assert_eq!(versions[0].kind, Some(HistoryKind::Added));
    assert!(!versions[1].present());
    assert_eq!(versions[1].kind, Some(HistoryKind::Deleted));
    assert!(versions[2].present());
    assert_eq!(versions[2].kind, Some(HistoryKind::Added));
    Ok(())
}

#[test]
fn a_one_sided_merge_follows_the_parent_that_carries_the_content() -> Result {
    let dir = init()?;
    write(dir.path(), "f.txt", "base\n")?;
    write(dir.path(), "other.txt", "base\n")?;
    commit(dir.path(), "Base", "2026-01-01T12:00:00+00:00")?;
    git_date(
        dir.path(),
        &["checkout", "-b", "side"],
        "2026-01-02T12:00:00+00:00",
    )?;
    write(dir.path(), "f.txt", "side\n")?;
    commit(dir.path(), "Side change", "2026-01-02T12:00:00+00:00")?;
    git_date(
        dir.path(),
        &["checkout", "main"],
        "2026-01-03T12:00:00+00:00",
    )?;
    write(dir.path(), "other.txt", "main\n")?;
    commit(dir.path(), "Main change", "2026-01-03T12:00:00+00:00")?;
    git_date(
        dir.path(),
        &["merge", "--no-ff", "side", "-m", "Merge side"],
        "2026-01-04T12:00:00+00:00",
    )?;

    let mut session = open(dir.path(), "HEAD", b"f.txt").map_err(|e| e.to_string())?;
    assert_eq!(session.versions()[0].subject, "Side change");
    assert_eq!(session.versions()[0].kind, Some(HistoryKind::Modified));
    assert!(matches!(session.versions()[0].previous, Predecessor::More));
    assert_eq!(collect(&mut session)?.len(), 2);
    assert_eq!(session.versions()[1].subject, "Base");
    Ok(())
}

#[test]
fn a_two_sided_merge_offers_every_predecessor_and_follows_the_choice() -> Result {
    let dir = init()?;
    write(dir.path(), "f.txt", "base\n")?;
    commit(dir.path(), "Base", "2026-01-01T12:00:00+00:00")?;
    git_date(
        dir.path(),
        &["checkout", "-b", "side"],
        "2026-01-02T12:00:00+00:00",
    )?;
    write(dir.path(), "f.txt", "side\n")?;
    commit(dir.path(), "Side change", "2026-01-02T12:00:00+00:00")?;
    git_date(
        dir.path(),
        &["checkout", "main"],
        "2026-01-03T12:00:00+00:00",
    )?;
    write(dir.path(), "f.txt", "main\n")?;
    commit(dir.path(), "Main change", "2026-01-03T12:00:00+00:00")?;
    git_date(
        dir.path(),
        &["merge", "side", "-m", "Merge side"],
        "2026-01-04T12:00:00+00:00",
    )
    .ok();
    write(dir.path(), "f.txt", "merged\n")?;
    commit(dir.path(), "Merge resolution", "2026-01-04T12:00:00+00:00")?;

    let mut session = open(dir.path(), "HEAD", b"f.txt").map_err(|e| e.to_string())?;
    let Predecessor::Candidates { choices } = session.versions()[0].previous.clone() else {
        return Err(format!(
            "expected predecessor choices, got {:?}",
            session.versions()[0].previous
        )
        .into());
    };
    assert_eq!(choices.len(), 2);
    let subjects: Vec<_> = choices
        .iter()
        .map(|choice| choice.subject.as_str())
        .collect();
    assert!(subjects.contains(&"Main change"));
    assert!(subjects.contains(&"Side change"));
    // Newest predecessor first.
    assert_eq!(subjects, ["Main change", "Side change"]);
    assert!(session.versions()[0].kind.is_none());
    assert!(session.versions()[0].old_path.is_none());

    let chosen = choices
        .iter()
        .position(|choice| choice.subject == "Side change")
        .ok_or("the side predecessor is missing")?;
    assert!(session.choose(chosen)?);
    assert_eq!(
        session.versions()[0].kind,
        Some(HistoryKind::Modified),
        "choosing names the edge kind"
    );
    assert!(!session.complete());
    assert_eq!(collect(&mut session)?.len(), 3);
    assert_eq!(session.versions()[1].subject, "Side change");
    assert_eq!(session.versions()[2].subject, "Base");
    assert_eq!(
        session.versions()[1].side.as_ref().map(|s| s.mode),
        Some(0o100644)
    );
    Ok(())
}

#[test]
fn a_root_addition_is_the_only_version() -> Result {
    let dir = init()?;
    write(dir.path(), "f.txt", "one\n")?;
    commit(dir.path(), "Add", "2026-01-01T12:00:00+00:00")?;
    let mut session = open(dir.path(), "HEAD", b"f.txt").map_err(|e| e.to_string())?;
    assert_eq!(session.versions().len(), 1);
    assert_eq!(session.versions()[0].kind, Some(HistoryKind::Added));
    assert!(matches!(
        session.versions()[0].previous,
        Predecessor::Oldest
    ));
    assert!(session.complete());
    assert!(!session.next(&|| false)?);
    Ok(())
}

// macOS worktree APIs reject byte paths that are not valid UTF-8, so the
// fixture can only exist on filesystems that accept exact byte names.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn non_utf8_paths_keep_their_exact_bytes() -> Result {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let dir = init()?;
    let name = OsStr::from_bytes(b"a/\xff");
    std::fs::create_dir(dir.path().join("a"))?;
    std::fs::write(dir.path().join(name), "one\n")?;
    commit(dir.path(), "Add", "2026-01-01T12:00:00+00:00")?;
    std::fs::write(dir.path().join(name), "two\n")?;
    commit(dir.path(), "Change", "2026-01-02T12:00:00+00:00")?;

    let mut session = open(dir.path(), "HEAD", b"a/\xff").map_err(|e| e.to_string())?;
    assert_eq!(session.versions()[0].path_bytes, b"a/\xff");
    assert_eq!(session.versions()[0].path, "a/\\xff");
    assert_eq!(collect(&mut session)?.len(), 2);
    assert_eq!(session.versions()[1].path_bytes, b"a/\xff");
    Ok(())
}

#[test]
fn cancellation_and_a_failed_step_keep_the_loaded_versions() -> Result {
    let dir = init()?;
    write(dir.path(), "f.txt", "one\n")?;
    commit(dir.path(), "Add", "2026-01-01T12:00:00+00:00")?;
    write(dir.path(), "f.txt", "two\n")?;
    commit(dir.path(), "Change", "2026-01-02T12:00:00+00:00")?;

    assert!(Session::open(dir.path(), "HEAD", b"f.txt", &|| true).is_err());
    let mut session = open(dir.path(), "HEAD", b"f.txt").map_err(|e| e.to_string())?;
    assert!(session.next(&|| true).is_err());
    assert_eq!(session.versions().len(), 1);
    assert!(!session.complete());
    assert!(session.next(&|| false)?);
    assert_eq!(session.versions().len(), 2);
    Ok(())
}
