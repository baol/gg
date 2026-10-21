use gg_engine::status::{Area, Kind, StatusEntry};
use std::{
    error::Error,
    path::Path,
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
fn git(path: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.org")
        .env("GIT_COMMITTER_EMAIL", "test@example.org")
        .output()?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned().into());
    }
    Ok(out.stdout)
}
fn scan(path: &Path) -> Result<Vec<StatusEntry>> {
    let session = gg_git::status::Session::open(path)?;
    let mut rows = vec![];
    session.scan(&AtomicBool::new(false), |entry| {
        rows.push(entry);
        Ok(())
    })?;
    Ok(rows)
}
fn oracle(path: &Path) -> Result {
    let raw = git(
        path,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--no-renames",
        ],
    )?;
    let mut expected = vec![];
    for record in raw.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let bytes = &record[3..];
        if record[..2] == *b"??" {
            expected.push(("untracked".to_string(), 'A', bytes.to_vec()));
            continue;
        }
        if record[..2].contains(&b'U') || record[..2] == *b"AA" || record[..2] == *b"DD" {
            expected.push(("conflict".into(), 'U', bytes.to_vec()));
            continue;
        }
        for (i, area) in ["staged", "unstaged"].iter().enumerate() {
            if record[i] != b' ' {
                expected.push((area.to_string(), record[i] as char, bytes.to_vec()));
            }
        }
    }
    let mut actual: Vec<_> = scan(path)?
        .into_iter()
        .map(|entry| {
            (
                entry.area.label().to_lowercase(),
                match entry.kind {
                    Kind::Added | Kind::IntentToAdd => 'A',
                    Kind::Deleted => 'D',
                    Kind::Modified => 'M',
                    Kind::TypeChanged => 'T',
                    Kind::Conflicted => 'U',
                },
                entry.path_bytes,
            )
        })
        .collect();
    expected.sort();
    actual.sort();
    assert_eq!(actual, expected);
    Ok(())
}
fn snapshot(path: &Path) -> Result<Vec<(std::path::PathBuf, Vec<u8>)>> {
    let mut out = vec![];
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            out.extend(snapshot(&entry.path())?);
        } else if entry.file_type()?.is_symlink() {
            out.push((
                entry.path(),
                std::fs::read_link(entry.path())?
                    .to_string_lossy()
                    .as_bytes()
                    .to_vec(),
            ));
        } else {
            out.push((entry.path(), std::fs::read(entry.path())?));
        }
    }
    out.sort();
    Ok(out)
}
#[test]
fn status_matches_git_and_preserves_repository() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    for name in [
        "both",
        "staged-delete",
        "unstaged-delete",
        "mode",
        "type",
        "unchanged",
    ] {
        std::fs::write(p.join(name), b"old\n")?;
    }
    std::fs::write(p.join(".gitignore"), b"ignored\n")?;
    git(p, &["add", "."])?;
    git(p, &["commit", "-qm", "base"])?;
    std::fs::write(p.join("both"), b"index version\n")?;
    git(p, &["rm", "staged-delete"])?;
    git(p, &["add", "both"])?;
    std::fs::write(p.join("both"), b"working version\n")?;
    std::fs::remove_file(p.join("unstaged-delete"))?;
    std::fs::write(p.join("new"), b"staged\n")?;
    git(p, &["add", "new"])?;
    std::fs::create_dir(p.join("nested"))?;
    std::fs::write(p.join("nested/café.txt"), b"untracked")?;
    std::fs::write(p.join("ignored"), b"hidden")?;
    std::fs::write(p.join("intent"), b"intent")?;
    git(p, &["add", "-N", "intent"])?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        std::fs::set_permissions(p.join("mode"), std::fs::Permissions::from_mode(0o755))?;
        std::fs::remove_file(p.join("type"))?;
        symlink("both", p.join("type"))?;
        std::fs::write(p.join("line\nbreak"), b"untracked")?;
    }
    let before = snapshot(p)?;
    oracle(p)?;
    oracle(&p.join("nested"))?;
    assert_eq!(snapshot(p)?, before);
    assert!(scan(p)?.iter().any(|e| e.kind == Kind::IntentToAdd));
    Ok(())
}
#[test]
fn unborn_clean_bare_and_linked_worktree() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    assert!(scan(p)?.is_empty());
    assert!(gg_git::status::Session::open(p)?.identity.head.is_none());
    std::fs::write(p.join("a"), b"first")?;
    git(p, &["add", "a"])?;
    oracle(p)?;
    git(p, &["commit", "-qm", "first"])?;
    assert!(scan(p)?.is_empty());
    let linked = tempfile::tempdir()?;
    git(
        p,
        &[
            "worktree",
            "add",
            "-b",
            "linked",
            &linked.path().to_string_lossy(),
        ],
    )?;
    std::fs::write(linked.path().join("a"), b"changed")?;
    oracle(linked.path())?;
    let bare = tempfile::tempdir()?;
    git(bare.path(), &["init", "--bare"])?;
    assert!(
        gg_git::status::Session::open(bare.path())
            .err()
            .is_some_and(|e| e.contains("working tree"))
    );
    Ok(())
}
#[test]
fn conflicts_are_explicit_and_scan_cancels() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "."])?;
    git(p, &["commit", "-qm", "base"])?;
    git(p, &["checkout", "-b", "other"])?;
    std::fs::write(p.join("a"), b"theirs\n")?;
    git(p, &["commit", "-qam", "theirs"])?;
    git(p, &["checkout", "main"])?;
    std::fs::write(p.join("a"), b"ours\n")?;
    git(p, &["commit", "-qam", "ours"])?;
    assert!(git(p, &["merge", "other"]).is_err());
    oracle(p)?;
    assert_eq!(
        scan(p)?.iter().filter(|e| e.area == Area::Conflict).count(),
        1
    );
    for i in 0..500 {
        std::fs::write(p.join(format!("untracked-{i}")), b"x")?;
    }
    let stop = AtomicBool::new(false);
    let mut count = 0;
    gg_git::status::Session::open(p)?.scan(&stop, |_| {
        count += 1;
        stop.store(true, Ordering::Relaxed);
        Ok(())
    })?;
    assert_eq!(count, 1);
    Ok(())
}

fn request(path: &Path, name: &[u8], area: Area) -> Result<gg_engine::status::WorkingRequest> {
    let session = gg_git::status::Session::open(path)?;
    let mut entry = None;
    session.scan(&AtomicBool::new(false), |row| {
        if row.path_bytes == name && row.area == area {
            entry = Some(row);
        }
        Ok(())
    })?;
    Ok(gg_engine::status::WorkingRequest {
        repository: session.identity,
        entry: entry.ok_or("status entry missing")?,
    })
}
#[test]
fn selected_working_comparisons_match_git_and_preserve_inputs() -> Result {
    use gg_engine::diff::RowKind;
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    std::fs::write(p.join("a"), b"base\ncommon\n")?;
    git(p, &["add", "a"])?;
    git(p, &["commit", "-qm", "base"])?;
    std::fs::write(p.join("a"), b"staged\ncommon\n")?;
    git(p, &["add", "a"])?;
    std::fs::write(p.join("a"), b"working\ncommon\nlast")?;
    std::fs::write(p.join("new"), b"new file\n")?;
    let before = snapshot(p)?;
    for (area, old, new) in [
        (
            Area::Staged,
            b"base\ncommon\n".as_slice(),
            b"staged\ncommon\n".as_slice(),
        ),
        (
            Area::Unstaged,
            b"staged\ncommon\n",
            b"working\ncommon\nlast",
        ),
        (Area::Untracked, b"", b"new file\n"),
    ] {
        let name = if area == Area::Untracked {
            b"new".as_slice()
        } else {
            b"a"
        };
        let req = request(p, name, area)?;
        let diff = gg_git::working_diff::read(&req, || false)?;
        let actual_old: Vec<_> = diff
            .rows
            .iter()
            .filter(|r| matches!(r.kind, RowKind::Context | RowKind::Deleted))
            .flat_map(|r| r.bytes.clone())
            .collect();
        let actual_new: Vec<_> = diff
            .rows
            .iter()
            .filter(|r| matches!(r.kind, RowKind::Context | RowKind::Added))
            .flat_map(|r| r.bytes.clone())
            .collect();
        assert_eq!(actual_old, old);
        assert_eq!(actual_new, new);
        if area != Area::Untracked {
            let args = if area == Area::Staged {
                vec!["diff", "--cached", "--numstat", "--", "a"]
            } else {
                vec!["diff", "--numstat", "--", "a"]
            };
            let stat = String::from_utf8(git(p, &args)?)?;
            let counts: Vec<_> = stat
                .split_whitespace()
                .take(2)
                .map(str::parse::<usize>)
                .collect::<std::result::Result<_, _>>()?;
            assert_eq!(
                counts,
                vec![
                    diff.rows
                        .iter()
                        .filter(|r| r.kind == RowKind::Added)
                        .count(),
                    diff.rows
                        .iter()
                        .filter(|r| r.kind == RowKind::Deleted)
                        .count()
                ]
            );
        }
    }
    assert_eq!(snapshot(p)?, before);
    Ok(())
}
#[test]
fn working_diffs_reject_stale_index_and_cancelled_reads() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    std::fs::write(p.join("a"), b"staged\n")?;
    git(p, &["add", "a"])?;
    let req = request(p, b"a", Area::Staged)?;
    assert!(
        gg_git::working_diff::read(&req, || true)
            .err()
            .is_some_and(|e| e.contains("cancelled"))
    );
    std::fs::write(p.join("a"), b"new index\n")?;
    git(p, &["add", "a"])?;
    assert!(
        gg_git::working_diff::read(&req, || false)
            .err()
            .is_some_and(|e| e.contains("Refresh status"))
    );
    let req = request(p, b"a", Area::Staged)?;
    git(p, &["commit", "-qm", "commit"])?;
    assert!(gg_git::working_diff::read(&req, || false).is_err());
    Ok(())
}
#[test]
fn working_diff_normalizes_text_and_reports_binary_and_limits() -> Result {
    use gg_engine::diff::DiffKind;
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    std::fs::write(p.join(".gitattributes"), b"*.txt text eol=lf\n")?;
    std::fs::write(p.join("a.txt"), b"old\n")?;
    git(p, &["add", "."])?;
    git(p, &["commit", "-qm", "base"])?;
    std::fs::write(p.join("a.txt"), b"new\r\n")?;
    let diff = gg_git::working_diff::read(&request(p, b"a.txt", Area::Unstaged)?, || false)?;
    assert!(diff.rows.iter().any(|r| r.bytes == b"new\n"));
    for (name, data, kind) in [
        ("binary", vec![0, 1, 2], DiffKind::Binary),
        (
            "large",
            vec![b'a'; gg_engine::diff::MAX_BLOB_BYTES + 1],
            DiffKind::Limited,
        ),
    ] {
        std::fs::write(p.join(name), data)?;
        assert_eq!(
            gg_git::working_diff::read(&request(p, name.as_bytes(), Area::Untracked)?, || false)?
                .kind,
            kind
        );
    }
    Ok(())
}
#[cfg(unix)]
#[test]
fn working_diff_reads_symlink_targets_and_rejects_symlink_ancestors() -> Result {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    symlink("before", p.join("link"))?;
    git(p, &["add", "link"])?;
    git(p, &["commit", "-qm", "base"])?;
    std::fs::remove_file(p.join("link"))?;
    symlink("after", p.join("link"))?;
    let diff = gg_git::working_diff::read(&request(p, b"link", Area::Unstaged)?, || false)?;
    assert!(diff.rows.iter().any(|r| r.bytes == b"after"));
    std::fs::create_dir(p.join("nested"))?;
    std::fs::write(p.join("nested/file"), b"inside")?;
    let req = request(p, b"nested/file", Area::Untracked)?;
    std::fs::remove_file(p.join("nested/file"))?;
    std::fs::remove_dir(p.join("nested"))?;
    let outside = tempfile::tempdir()?;
    std::fs::write(outside.path().join("file"), b"outside")?;
    symlink(outside.path(), p.join("nested"))?;
    assert!(
        gg_git::working_diff::read(&req, || false)
            .err()
            .is_some_and(|e| e.contains("symlink"))
    );
    Ok(())
}

#[test]
fn working_bytes_changed_during_read_are_not_published() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    std::fs::write(p.join("a"), b"before\n")?;
    let req = request(p, b"a", Area::Untracked)?;
    let checks = std::cell::Cell::new(0);
    let result = gg_git::working_diff::read(&req, || {
        checks.set(checks.get() + 1);
        if checks.get() == 2 {
            return std::fs::write(p.join("a"), b"after\n").is_err();
        }
        false
    });
    assert!(
        result
            .err()
            .is_some_and(|error| error.contains("Working file changed"))
    );
    Ok(())
}

#[test]
fn working_diff_respects_emulated_symlinks() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    git(p, &["config", "core.symlinks", "false"])?;
    std::fs::write(p.join("link.txt"), b"old-target")?;
    let id = String::from_utf8(git(p, &["hash-object", "-w", "link.txt"])?)?;
    git(
        p,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "120000",
            id.trim(),
            "link.txt",
        ],
    )?;
    git(p, &["commit", "-qm", "base"])?;
    std::fs::write(p.join(".gitattributes"), b"*.txt text eol=lf\n")?;
    std::fs::write(p.join("link.txt"), b"new-target\r\n")?;
    let diff = gg_git::working_diff::read(&request(p, b"link.txt", Area::Unstaged)?, || false)?;
    assert!(diff.rows.iter().any(|r| r.bytes == b"new-target\r\n"));
    assert!(diff.note.contains("mode 120000"));
    assert!(!diff.note.contains("→"));
    Ok(())
}

#[test]
fn unstage_matches_head_and_preserves_working_edits_and_other_entries() -> Result {
    for hash in ["sha1", "sha256"] {
        let dir = tempfile::tempdir()?;
        let p = dir.path();
        git(p, &["init", &format!("--object-format={hash}")])?;
        for name in ["changed", "deleted", "other"] {
            std::fs::write(p.join(name), "base\n")?;
        }
        git(p, &["add", "."])?;
        git(p, &["commit", "-m", "base"])?;
        std::fs::write(p.join("changed"), "staged\n")?;
        std::fs::write(p.join("other"), "other staged\n")?;
        std::fs::write(p.join("new"), "new\n")?;
        std::fs::remove_file(p.join("deleted"))?;
        git(p, &["add", "-A"])?;
        // Populate the tree extension: subsequent write-tree must see the restored entries.
        git(p, &["write-tree"])?;
        std::fs::write(p.join("changed"), "working\n")?;
        for name in ["changed", "deleted", "new"] {
            gg_git::unstage::file(&request(p, name.as_bytes(), Area::Staged)?)?;
            assert!(git(p, &["diff", "--cached", "--", name])?.is_empty());
        }
        assert_eq!(std::fs::read(p.join("changed"))?, b"working\n");
        assert_eq!(std::fs::read(p.join("new"))?, b"new\n");
        assert!(!p.join("deleted").exists());
        assert_eq!(git(p, &["show", ":other"])?, b"other staged\n");
        let tree = String::from_utf8(git(p, &["write-tree"])?)?;
        assert_eq!(
            git(p, &["show", &format!("{}:changed", tree.trim())])?,
            b"base\n"
        );
        oracle(p)?;
        assert!(!p.join(".git/index.lock").exists());
    }
    Ok(())
}
#[test]
fn unstage_unborn_lock_and_stale_observations() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init"])?;
    std::fs::write(p.join("new"), "working\n")?;
    git(p, &["add", "new"])?;
    let req = request(p, b"new", Area::Staged)?;
    let index = std::fs::read(p.join(".git/index"))?;
    std::fs::write(p.join(".git/index.lock"), "external owner")?;
    assert!(gg_git::unstage::file(&req).is_err());
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);
    assert_eq!(std::fs::read(p.join(".git/index.lock"))?, b"external owner");
    std::fs::remove_file(p.join(".git/index.lock"))?;
    std::fs::write(p.join("other"), "other")?;
    git(p, &["add", "other"])?;
    let index = std::fs::read(p.join(".git/index"))?;
    assert!(
        gg_git::unstage::file(&req)
            .err()
            .is_some_and(|e| e.contains("Refresh status"))
    );
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);
    gg_git::unstage::file(&request(p, b"new", Area::Staged)?)?;
    assert_eq!(git(p, &["ls-files", "--", "new"])?, b"");
    assert_eq!(std::fs::read(p.join("new"))?, b"working\n");
    oracle(p)?;
    Ok(())
}

#[test]
fn unstage_rejects_changed_head_and_file_directory_collisions() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init"])?;
    std::fs::write(p.join("a"), "base")?;
    git(p, &["add", "."])?;
    git(p, &["commit", "-m", "base"])?;
    std::fs::write(p.join("a"), "staged")?;
    git(p, &["add", "a"])?;
    let req = request(p, b"a", Area::Staged)?;
    git(p, &["commit", "--allow-empty", "-m", "new head"])?;
    let index = std::fs::read(p.join(".git/index"))?;
    assert!(
        gg_git::unstage::file(&req)
            .err()
            .is_some_and(|e| e.contains("Refresh status"))
    );
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);
    std::fs::remove_file(p.join("a"))?;
    std::fs::create_dir(p.join("a"))?;
    std::fs::write(p.join("a/child"), "child")?;
    git(p, &["add", "-A"])?;
    let req = request(p, b"a", Area::Staged)?;
    let index = std::fs::read(p.join(".git/index"))?;
    assert!(
        gg_git::unstage::file(&req)
            .err()
            .is_some_and(|e| e.contains("file/directory"))
    );
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);
    assert_eq!(std::fs::read(p.join("a/child"))?, b"child");
    Ok(())
}

fn stage_all(path: &Path) -> Result<usize> {
    let mut staged = 0;
    loop {
        let session = gg_git::status::Session::open(path)?;
        let identity = session.identity.clone();
        let mut next = None;
        session.scan(&AtomicBool::new(false), |entry| {
            if next.is_none() && matches!(entry.area, Area::Unstaged | Area::Untracked) {
                next = Some(gg_engine::status::WorkingRequest {
                    repository: identity.clone(),
                    entry,
                });
            }
            Ok(())
        })?;
        let Some(req) = next else {
            return Ok(staged);
        };
        gg_git::stage::file(&req)?;
        staged += 1;
        assert!(staged < 100, "staging did not converge");
    }
}
fn ls_files(path: &Path) -> Result<Vec<u8>> {
    git(path, &["ls-files", "-s", "-z"])
}
fn stage_fixture(path: &Path, hash: &str) -> Result {
    git(path, &["init", &format!("--object-format={hash}")])?;
    for name in ["changed", "deleted", "typechange", "unrelated", "mode"] {
        std::fs::write(path.join(name), "base\n")?;
    }
    git(path, &["add", "."])?;
    git(path, &["commit", "-m", "base"])?;
    std::fs::write(path.join("changed"), "working\n")?;
    std::fs::remove_file(path.join("deleted"))?;
    std::fs::write(path.join("new"), "new\n")?;
    std::fs::write(path.join("ita"), "intent\n")?;
    git(path, &["add", "-N", "ita"])?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        std::fs::set_permissions(path.join("mode"), std::fs::Permissions::from_mode(0o755))?;
        std::fs::remove_file(path.join("typechange"))?;
        symlink("target", path.join("typechange"))?;
    }
    std::fs::write(path.join("unrelated"), "staged\n")?;
    git(path, &["add", "unrelated"])?;
    Ok(())
}

#[test]
fn stage_matches_git_add_and_preserves_working_files() -> Result {
    for hash in ["sha1", "sha256"] {
        let ours = tempfile::tempdir()?;
        let reference = tempfile::tempdir()?;
        for path in [ours.path(), reference.path()] {
            stage_fixture(path, hash)?;
        }
        let staged = stage_all(ours.path())?;
        assert!(staged >= 4);
        git(reference.path(), &["add", "-A"])?;
        assert_eq!(ls_files(ours.path())?, ls_files(reference.path())?);
        assert_eq!(std::fs::read(ours.path().join("changed"))?, b"working\n");
        assert_eq!(std::fs::read(ours.path().join("new"))?, b"new\n");
        assert!(!ours.path().join("deleted").exists());
        assert_eq!(
            git(ours.path(), &["show", ":unrelated"])?,
            git(reference.path(), &["show", ":unrelated"])?
        );
        assert!(!ours.path().join(".git/index.lock").exists());
        let porcelain = git(
            ours.path(),
            &[
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
                "--no-renames",
            ],
        )?;
        for record in porcelain.split(|b| *b == 0).filter(|r| !r.is_empty()) {
            assert_eq!(
                record[1],
                b' ',
                "unstaged: {}",
                String::from_utf8_lossy(record)
            );
        }
        oracle(ours.path())?;
    }
    Ok(())
}

#[test]
fn stage_matches_git_add_for_attributes() -> Result {
    let ours = tempfile::tempdir()?;
    let reference = tempfile::tempdir()?;
    for path in [ours.path(), reference.path()] {
        git(path, &["init"])?;
        std::fs::write(path.join(".gitattributes"), "*.txt text eol=lf\n")?;
        git(path, &["add", ".gitattributes"])?;
        git(path, &["commit", "-m", "attributes"])?;
        std::fs::write(path.join("crlf.txt"), b"a\r\nb\r\n")?;
    }
    stage_all(ours.path())?;
    git(reference.path(), &["add", "-A"])?;
    assert_eq!(git(ours.path(), &["show", ":crlf.txt"])?, b"a\nb\n");
    assert_eq!(ls_files(ours.path())?, ls_files(reference.path())?);
    assert_eq!(std::fs::read(ours.path().join("crlf.txt"))?, b"a\r\nb\r\n");
    Ok(())
}

#[cfg(unix)]
#[test]
fn stage_preserves_symlink_targets_and_emulated_symlinks() -> Result {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init"])?;
    symlink("target", p.join("link"))?;
    let req = request(p, b"link", Area::Untracked)?;
    gg_git::stage::file(&req)?;
    assert!(git(p, &["ls-files", "-s", "--", "link"])?.starts_with(b"120000 "));
    assert_eq!(git(p, &["cat-file", "blob", ":link"])?, b"target");

    git(p, &["commit", "-qm", "link"])?;
    git(p, &["config", "core.symlinks", "false"])?;
    std::fs::remove_file(p.join("link"))?;
    std::fs::write(p.join("link"), b"after")?;
    let req = request(p, b"link", Area::Unstaged)?;
    assert!(matches!(req.entry.kind, Kind::Modified | Kind::TypeChanged));
    gg_git::stage::file(&req)?;
    assert!(git(p, &["ls-files", "-s", "--", "link"])?.starts_with(b"120000 "));
    assert_eq!(git(p, &["cat-file", "blob", ":link"])?, b"after");
    Ok(())
}

#[test]
fn stage_unborn_lock_and_stale_observations() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init"])?;
    std::fs::write(p.join("new"), "working\n")?;
    let req = request(p, b"new", Area::Untracked)?;
    std::fs::write(p.join(".git/index.lock"), "external owner")?;
    let index = std::fs::read(p.join(".git/index")).unwrap_or_default();
    assert!(gg_git::stage::file(&req).is_err());
    assert_eq!(
        std::fs::read(p.join(".git/index")).unwrap_or_default(),
        index
    );
    assert_eq!(std::fs::read(p.join(".git/index.lock"))?, b"external owner");
    std::fs::remove_file(p.join(".git/index.lock"))?;
    gg_git::stage::file(&request(p, b"new", Area::Untracked)?)?;
    assert_eq!(git(p, &["show", ":new"])?, b"working\n");
    assert!(git(p, &["rev-parse", "--verify", "HEAD"]).is_err());
    assert_eq!(std::fs::read(p.join("new"))?, b"working\n");

    std::fs::write(p.join("new"), "edited\n")?;
    let req = request(p, b"new", Area::Unstaged)?;
    std::fs::write(p.join("other"), "other\n")?;
    git(p, &["add", "other"])?;
    let index = std::fs::read(p.join(".git/index"))?;
    assert!(
        gg_git::stage::file(&req)
            .err()
            .is_some_and(|e| e.contains("Refresh status"))
    );
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);
    gg_git::stage::file(&request(p, b"new", Area::Unstaged)?)?;

    git(p, &["add", "."])?;
    git(p, &["commit", "-qm", "base"])?;
    std::fs::write(p.join("new"), "next\n")?;
    let req = request(p, b"new", Area::Unstaged)?;
    git(p, &["commit", "--allow-empty", "-qm", "new head"])?;
    let index = std::fs::read(p.join(".git/index"))?;
    assert!(
        gg_git::stage::file(&req)
            .err()
            .is_some_and(|e| e.contains("Refresh status"))
    );
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);
    assert_eq!(std::fs::read(p.join("new"))?, b"next\n");
    Ok(())
}

#[test]
fn stage_rejects_conflicts_submodules_collisions_and_stale_paths() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    std::fs::write(p.join("conflict"), "base\n")?;
    git(p, &["add", "."])?;
    git(p, &["commit", "-qm", "base"])?;
    git(p, &["checkout", "-qb", "other"])?;
    std::fs::write(p.join("conflict"), "other\n")?;
    git(p, &["commit", "-qam", "other"])?;
    git(p, &["checkout", "-q", "main"])?;
    std::fs::write(p.join("conflict"), "main\n")?;
    git(p, &["commit", "-qam", "main"])?;
    let _ = Command::new("git")
        .arg("-C")
        .arg(p)
        .args(["merge", "other"])
        .output()?;
    let req = request(p, b"conflict", Area::Conflict)?;
    assert!(
        gg_git::stage::file(&req)
            .err()
            .is_some_and(|e| e.contains("conflict"))
    );
    git(p, &["merge", "--abort"])?;

    std::fs::write(p.join("staged"), "base\n")?;
    git(p, &["add", "staged"])?;
    git(p, &["commit", "-qm", "staged"])?;
    std::fs::write(p.join("staged"), "changed\n")?;
    git(p, &["add", "staged"])?;
    let req = request(p, b"staged", Area::Staged)?;
    assert!(
        gg_git::stage::file(&req)
            .err()
            .is_some_and(|e| e.contains("already staged"))
    );

    std::fs::write(p.join("missing"), "base\n")?;
    git(p, &["add", "missing"])?;
    git(p, &["commit", "-qm", "missing"])?;
    std::fs::write(p.join("missing"), "changed\n")?;
    let req = request(p, b"missing", Area::Unstaged)?;
    std::fs::remove_file(p.join("missing"))?;
    let index = std::fs::read(p.join(".git/index"))?;
    assert!(
        gg_git::stage::file(&req)
            .err()
            .is_some_and(|e| e.contains("absent"))
    );
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);

    std::fs::write(p.join("gone"), "base\n")?;
    git(p, &["add", "gone"])?;
    git(p, &["commit", "-qm", "gone"])?;
    std::fs::remove_file(p.join("gone"))?;
    let req = request(p, b"gone", Area::Unstaged)?;
    std::fs::write(p.join("gone"), "back\n")?;
    assert!(
        gg_git::stage::file(&req)
            .err()
            .is_some_and(|e| e.contains("reappeared"))
    );

    std::fs::create_dir_all(p.join("tree"))?;
    std::fs::write(p.join("tree/child"), "base\n")?;
    git(p, &["add", "tree"])?;
    git(p, &["commit", "-qm", "tree"])?;
    std::fs::remove_dir_all(p.join("tree"))?;
    std::fs::write(p.join("tree"), "file\n")?;
    let req = request(p, b"tree", Area::Untracked)?;
    assert!(
        gg_git::stage::file(&req)
            .err()
            .is_some_and(|e| e.contains("file/directory"))
    );

    std::fs::write(p.join("file"), "base\n")?;
    git(p, &["add", "file"])?;
    git(p, &["commit", "-qm", "file"])?;
    std::fs::remove_file(p.join("file"))?;
    std::fs::create_dir_all(p.join("file"))?;
    std::fs::write(p.join("file/child"), "child\n")?;
    let req = request(p, b"file/child", Area::Untracked)?;
    assert!(
        gg_git::stage::file(&req)
            .err()
            .is_some_and(|e| e.contains("file/directory"))
    );

    let head = String::from_utf8(git(p, &["rev-parse", "HEAD"])?)?
        .trim()
        .to_string();
    git(
        p,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{head},sub"),
        ],
    )?;
    let req = gg_engine::status::WorkingRequest {
        repository: gg_git::status::Session::open(p)?.identity,
        entry: StatusEntry::new(Area::Unstaged, Kind::Modified, b"sub"),
    };
    assert!(
        gg_git::stage::file(&req)
            .err()
            .is_some_and(|e| e.contains("Submodule"))
    );
    Ok(())
}
