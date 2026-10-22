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
        entry: entry.ok_or_else(|| {
            std::io::Error::other(format!(
                "status entry missing for {} ({:?})",
                String::from_utf8_lossy(name),
                area
            ))
        })?,
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
        .args(["-c", "core.autocrlf=false"])
        .arg("-C")
        .arg(p)
        .args(["merge", "other"])
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

fn hunk_headers(request: &gg_engine::status::WorkingRequest) -> Result<Vec<String>> {
    use gg_engine::diff::RowKind;
    Ok(gg_git::working_diff::read(request, || false)?
        .rows
        .iter()
        .filter(|row| row.kind == RowKind::Hunk)
        .map(|row| row.text.clone())
        .collect())
}

fn hunk_fixture(path: &Path, hash: &str) -> Result {
    git(path, &["init", &format!("--object-format={hash}")])?;
    std::fs::write(
        path.join("a"),
        "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n",
    )?;
    std::fs::write(path.join("other"), "base\n")?;
    git(path, &["add", "."])?;
    git(path, &["commit", "-qm", "base"])?;
    std::fs::write(
        path.join("a"),
        "ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n",
    )?;
    Ok(())
}

fn add_patch(path: &Path, file: &str, answers: &str) -> Result {
    use std::{io::Write, process::Stdio};
    let mut child = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["add", "-p", "--", file])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_PAGER", "cat")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.org")
        .env("GIT_COMMITTER_EMAIL", "test@example.org")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("missing stdin")?
        .write_all(answers.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(())
}

#[test]
fn stage_hunks_match_git_add_patch_and_keep_remaining_edits() -> Result {
    for hash in ["sha1", "sha256"] {
        let ours = tempfile::tempdir()?;
        let reference = tempfile::tempdir()?;
        for path in [ours.path(), reference.path()] {
            hunk_fixture(path, hash)?;
        }
        let req = request(ours.path(), b"a", Area::Unstaged)?;
        let headers = hunk_headers(&req)?;
        assert_eq!(headers.len(), 2);
        let index = std::fs::read(ours.path().join(".git/index"))?;
        gg_git::stage::hunks(&req, &headers[..1])?;
        add_patch(reference.path(), "a", "y\nn\n")?;
        assert_eq!(
            git(ours.path(), &["show", ":a"])?,
            git(reference.path(), &["show", ":a"])?
        );
        assert_ne!(std::fs::read(ours.path().join(".git/index"))?, index);
        assert_eq!(
            git(ours.path(), &["status", "--porcelain=v1", "-z", "--", "a"])?,
            b"MM a\0"
        );
        let debug = String::from_utf8(git(ours.path(), &["ls-files", "--debug", "--", "a"])?)?;
        assert!(debug.contains("size: 0"), "{debug}");
        assert_eq!(
            std::fs::read(ours.path().join("a"))?,
            b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n"
        );
        assert_eq!(git(ours.path(), &["show", ":other"])?, b"base\n");

        let req = request(ours.path(), b"a", Area::Unstaged)?;
        let remaining = hunk_headers(&req)?;
        assert_eq!(remaining.len(), 1);
        gg_git::stage::hunks(&req, &remaining)?;
        assert_eq!(
            git(ours.path(), &["show", ":a"])?,
            std::fs::read(ours.path().join("a"))?
        );
        assert_eq!(
            git(ours.path(), &["status", "--porcelain=v1", "-z", "--", "a"])?,
            b"M  a\0"
        );
        let debug = String::from_utf8(git(ours.path(), &["ls-files", "--debug", "--", "a"])?)?;
        assert!(!debug.contains("size: 0"), "{debug}");
        assert!(!ours.path().join(".git/index.lock").exists());
        oracle(ours.path())?;
    }
    Ok(())
}

#[test]
fn stage_hunks_select_multiple_and_preserve_other_entries() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    hunk_fixture(p, "sha1")?;
    std::fs::write(p.join("other"), "other working\n")?;
    git(p, &["add", "other"])?;
    std::fs::write(p.join("other"), "other working again\n")?;
    let req = request(p, b"a", Area::Unstaged)?;
    let headers = hunk_headers(&req)?;
    assert_eq!(headers.len(), 2);
    gg_git::stage::hunks(&req, &[headers[1].clone(), headers[0].clone()])?;
    assert_eq!(git(p, &["show", ":a"])?, std::fs::read(p.join("a"))?);
    assert_eq!(git(p, &["show", ":other"])?, b"other working\n");
    assert_eq!(
        git(p, &["status", "--porcelain=v1", "-z", "--", "other"])?,
        b"MM other\0"
    );
    assert_eq!(std::fs::read(p.join("other"))?, b"other working again\n");
    oracle(p)?;
    Ok(())
}

#[test]
fn stage_hunks_respect_attributes_crlf_and_final_newlines() -> Result {
    let ours = tempfile::tempdir()?;
    let reference = tempfile::tempdir()?;
    for p in [ours.path(), reference.path()] {
        git(p, &["init"])?;
        std::fs::write(p.join(".gitattributes"), b"*.txt text eol=lf\n")?;
        std::fs::write(
            p.join("a.txt"),
            b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix\r\nseven\r\neight\r\nnine\r\nten\r\n",
        )?;
        std::fs::write(p.join("nonewline"), b"a\nb")?;
        git(p, &["add", "."])?;
        git(p, &["commit", "-qm", "base"])?;
        // Change the sizes so a coarse index timestamp cannot make the edits look clean.
        std::fs::write(
            p.join("a.txt"),
            b"ONE!\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix\r\nseven\r\neight\r\nnine\r\nTEN!\r\n",
        )?;
        std::fs::write(p.join("nonewline"), b"a\nB!")?;
    }
    let req = request(ours.path(), b"a.txt", Area::Unstaged)?;
    let headers = hunk_headers(&req)?;
    assert_eq!(headers.len(), 2);
    gg_git::stage::hunks(&req, &headers[..1])?;
    add_patch(reference.path(), "a.txt", "y\nn\n")?;
    assert_eq!(
        git(ours.path(), &["show", ":a.txt"])?,
        git(reference.path(), &["show", ":a.txt"])?
    );
    assert_eq!(
        std::fs::read(ours.path().join("a.txt"))?,
        b"ONE!\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix\r\nseven\r\neight\r\nnine\r\nTEN!\r\n"
    );
    assert_eq!(
        git(
            ours.path(),
            &["status", "--porcelain=v1", "-z", "--", "a.txt"]
        )?,
        b"MM a.txt\0"
    );

    let req = request(ours.path(), b"nonewline", Area::Unstaged)?;
    let headers = hunk_headers(&req)?;
    assert_eq!(headers.len(), 1);
    gg_git::stage::hunks(&req, &headers)?;
    assert_eq!(git(ours.path(), &["show", ":nonewline"])?, b"a\nB!");
    assert_eq!(std::fs::read(ours.path().join("nonewline"))?, b"a\nB!");
    assert_eq!(
        git(
            ours.path(),
            &["status", "--porcelain=v1", "-z", "--", "nonewline"]
        )?,
        b"M  nonewline\0"
    );
    oracle(ours.path())?;
    Ok(())
}

#[test]
fn stage_hunks_reject_unsupported_changes_and_selections() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    std::fs::write(p.join("text"), b"base\n")?;
    std::fs::write(p.join("binary"), b"base\n")?;
    std::fs::write(p.join("large"), b"base\n")?;
    std::fs::write(p.join("mode"), b"base\n")?;
    git(p, &["add", "."])?;
    git(p, &["commit", "-qm", "base"])?;
    std::fs::write(p.join("text"), b"changed\n")?;
    std::fs::write(p.join("binary"), [0, 1, 2])?;
    std::fs::write(
        p.join("large"),
        vec![b'a'; gg_engine::diff::MAX_BLOB_BYTES + 1],
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(p.join("mode"), std::fs::Permissions::from_mode(0o755))?;
    }
    let req = request(p, b"text", Area::Unstaged)?;
    let headers = hunk_headers(&req)?;
    assert_eq!(headers.len(), 1);
    let index = std::fs::read(p.join(".git/index"))?;
    assert!(
        gg_git::stage::hunks(&req, &[])
            .err()
            .is_some_and(|e| e.contains("at least one hunk"))
    );
    assert!(
        gg_git::stage::hunks(&req, &["@@ -99,1 +99,1 @@".into()])
            .err()
            .is_some_and(|e| e.contains("stale"))
    );
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);

    let binary = request(p, b"binary", Area::Unstaged)?;
    assert!(
        gg_git::stage::hunks(&binary, &["@@ -1 +1 @@".into()])
            .err()
            .is_some_and(|e| e.contains("text comparison"))
    );
    let large = request(p, b"large", Area::Unstaged)?;
    assert!(
        gg_git::stage::hunks(&large, &[headers[0].clone()])
            .err()
            .is_some_and(|e| e.contains("2 MiB"))
    );
    #[cfg(unix)]
    {
        let mode = request(p, b"mode", Area::Unstaged)?;
        assert!(
            gg_git::stage::hunks(&mode, &[headers[0].clone()])
                .err()
                .is_some_and(|e| e.contains("mode or type"))
        );
    }
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);

    std::fs::write(p.join("untracked"), b"new\n")?;
    let untracked = request(p, b"untracked", Area::Untracked)?;
    assert!(
        gg_git::stage::hunks(&untracked, &[headers[0].clone()])
            .err()
            .is_some_and(|e| e.contains("unstaged modification"))
    );
    std::fs::write(p.join("ita"), b"intent\n")?;
    git(p, &["add", "-N", "ita"])?;
    let ita = request(p, b"ita", Area::Unstaged)?;
    assert!(
        gg_git::stage::hunks(&ita, &[headers[0].clone()])
            .err()
            .is_some_and(|e| e.contains("unstaged modification"))
    );
    std::fs::remove_file(p.join("text"))?;
    let deleted = request(p, b"text", Area::Unstaged)?;
    assert!(
        gg_git::stage::hunks(&deleted, &[headers[0].clone()])
            .err()
            .is_some_and(|e| e.contains("unstaged modification"))
    );
    std::fs::write(p.join("text"), b"changed\n")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink("before", p.join("link"))?;
        git(p, &["add", "link"])?;
        git(p, &["commit", "-qm", "link"])?;
        std::fs::remove_file(p.join("link"))?;
        symlink("after", p.join("link"))?;
        let link = request(p, b"link", Area::Unstaged)?;
        assert!(
            gg_git::stage::hunks(&link, &["@@ -1 +1 @@".into()])
                .err()
                .is_some_and(|e| e.contains("regular text file"))
        );
    }
    oracle(p)?;
    Ok(())
}

#[test]
fn stage_selected_lines_keeps_unselected_sides_and_marks_partial_stat() -> Result {
    use gg_engine::diff::RowKind;
    use gg_git::patch::{Change, Edit};
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    hunk_fixture(p, "sha1")?;
    let req = request(p, b"a", Area::Unstaged)?;
    let diff = gg_git::working_diff::read(&req, || false)?;
    let hunk = diff
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Hunk)
        .ok_or("missing hunk")?;
    let added = diff
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Added && row.new_line == Some(1))
        .ok_or("missing added line")?;
    let edits = vec![Edit {
        header: hunk.text.clone(),
        changes: vec![Change::Added(added.new_line.ok_or("missing line")?)],
    }];
    gg_git::stage::changes(&req, &edits)?;
    assert_eq!(
        git(p, &["show", ":a"])?,
        b"one\nONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n"
    );
    assert_eq!(
        std::fs::read(p.join("a"))?,
        b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n"
    );
    assert_eq!(
        git(p, &["status", "--porcelain=v1", "-z", "--", "a"])?,
        b"MM a\0"
    );
    let debug = String::from_utf8(git(p, &["ls-files", "--debug", "--", "a"])?)?;
    assert!(debug.contains("size: 0"), "{debug}");

    let req = request(p, b"a", Area::Unstaged)?;
    let diff = gg_git::working_diff::read(&req, || false)?;
    let hunk = diff
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Hunk)
        .ok_or("missing hunk")?;
    let deleted = diff
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Deleted && row.old_line == Some(1))
        .ok_or("missing deleted line")?;
    gg_git::stage::changes(
        &req,
        &[Edit {
            header: hunk.text.clone(),
            changes: vec![Change::Deleted(deleted.old_line.ok_or("missing line")?)],
        }],
    )?;
    assert_eq!(
        git(p, &["show", ":a"])?,
        b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n"
    );
    assert_eq!(
        git(p, &["status", "--porcelain=v1", "-z", "--", "a"])?,
        b"MM a\0"
    );

    let req = request(p, b"a", Area::Unstaged)?;
    let headers = hunk_headers(&req)?;
    gg_git::stage::hunks(&req, &headers)?;
    assert_eq!(git(p, &["show", ":a"])?, std::fs::read(p.join("a"))?);
    assert_eq!(
        git(p, &["status", "--porcelain=v1", "-z", "--", "a"])?,
        b"M  a\0"
    );
    let debug = String::from_utf8(git(p, &["ls-files", "--debug", "--", "a"])?)?;
    assert!(!debug.contains("size: 0"), "{debug}");
    oracle(p)?;
    Ok(())
}

#[test]
fn stage_selected_lines_can_replace_one_line_without_touching_another_hunk() -> Result {
    use gg_engine::diff::RowKind;
    use gg_git::patch::{Change, Edit};
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    hunk_fixture(p, "sha1")?;
    let req = request(p, b"a", Area::Unstaged)?;
    let diff = gg_git::working_diff::read(&req, || false)?;
    let hunk = diff
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Hunk)
        .ok_or("missing hunk")?;
    let deleted = diff
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Deleted)
        .and_then(|row| row.old_line)
        .ok_or("missing deleted line")?;
    let added = diff
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Added)
        .and_then(|row| row.new_line)
        .ok_or("missing added line")?;
    gg_git::stage::changes(
        &req,
        &[Edit {
            header: hunk.text.clone(),
            changes: vec![Change::Deleted(deleted), Change::Added(added)],
        }],
    )?;
    assert_eq!(
        git(p, &["show", ":a"])?,
        b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n"
    );
    assert_eq!(
        git(p, &["status", "--porcelain=v1", "-z", "--", "a"])?,
        b"MM a\0"
    );
    oracle(p)?;
    Ok(())
}

#[test]
fn unstage_selected_lines_restores_old_sides_and_preserves_working_tree() -> Result {
    use gg_engine::diff::RowKind;
    use gg_git::patch::{Change, Edit};
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    hunk_fixture(p, "sha1")?;
    git(p, &["add", "a"])?;
    let req = request(p, b"a", Area::Staged)?;
    let diff = gg_git::working_diff::read(&req, || false)?;
    let hunk = diff
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Hunk)
        .ok_or("missing hunk")?;
    let deleted = diff
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Deleted)
        .and_then(|row| row.old_line)
        .ok_or("missing deleted line")?;
    let working = std::fs::read(p.join("a"))?;
    gg_git::unstage::changes(
        &req,
        &[Edit {
            header: hunk.text.clone(),
            changes: vec![Change::Deleted(deleted)],
        }],
    )?;
    assert_eq!(
        git(p, &["show", ":a"])?,
        b"one\nONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n"
    );
    assert_eq!(std::fs::read(p.join("a"))?, working);
    assert_eq!(
        git(p, &["status", "--porcelain=v1", "-z", "--", "a"])?,
        b"MM a\0"
    );

    let req = request(p, b"a", Area::Staged)?;
    let diff = gg_git::working_diff::read(&req, || false)?;
    let hunk = diff
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Hunk)
        .ok_or("missing hunk")?;
    let start = diff
        .rows
        .iter()
        .position(|row| row.kind == RowKind::Hunk)
        .ok_or("missing hunk")?;
    let end = diff.rows[start + 1..]
        .iter()
        .position(|row| row.kind == RowKind::Hunk)
        .map_or(diff.rows.len(), |offset| start + 1 + offset);
    let mut changes = vec![];
    for row in &diff.rows[start + 1..end] {
        match row.kind {
            RowKind::Added => changes.push(Change::Added(row.new_line.ok_or("missing line")?)),
            RowKind::Deleted => changes.push(Change::Deleted(row.old_line.ok_or("missing line")?)),
            _ => {}
        }
    }
    gg_git::unstage::changes(
        &req,
        &[Edit {
            header: hunk.text.clone(),
            changes,
        }],
    )?;
    assert_eq!(
        git(p, &["show", ":a"])?,
        b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n"
    );
    assert_eq!(std::fs::read(p.join("a"))?, working);
    assert_eq!(
        git(p, &["status", "--porcelain=v1", "-z", "--", "a"])?,
        b"MM a\0"
    );

    let req = request(p, b"a", Area::Staged)?;
    let headers = hunk_headers(&req)?;
    gg_git::unstage::hunks(&req, &headers)?;
    assert_eq!(git(p, &["show", ":a"])?, git(p, &["show", "HEAD:a"])?);
    assert_eq!(std::fs::read(p.join("a"))?, working);
    assert_eq!(
        git(p, &["status", "--porcelain=v1", "-z", "--", "a"])?,
        b" M a\0"
    );
    oracle(p)?;
    Ok(())
}

#[test]
fn unstage_selected_added_and_deleted_file_lines_restores_index_shape() -> Result {
    use gg_engine::diff::RowKind;
    use gg_git::patch::{Change, Edit};
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    std::fs::write(p.join("base"), b"base\n")?;
    git(p, &["add", "base"])?;
    git(p, &["commit", "-qm", "base"])?;
    std::fs::write(p.join("new"), b"a\nb\nc\n")?;
    git(p, &["add", "new"])?;
    let req = request(p, b"new", Area::Staged)?;
    let diff = gg_git::working_diff::read(&req, || false)?;
    let hunk = diff
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Hunk)
        .ok_or("missing hunk")?;
    let added = diff
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Added)
        .and_then(|row| row.new_line)
        .ok_or("missing added line")?;
    gg_git::unstage::changes(
        &req,
        &[Edit {
            header: hunk.text.clone(),
            changes: vec![Change::Added(added)],
        }],
    )?;
    assert_eq!(git(p, &["show", ":new"])?, b"b\nc\n");
    assert_eq!(std::fs::read(p.join("new"))?, b"a\nb\nc\n");
    assert_eq!(
        git(p, &["status", "--porcelain=v1", "-z", "--", "new"])?,
        b"AM new\0"
    );
    let req = request(p, b"new", Area::Staged)?;
    let headers = hunk_headers(&req)?;
    gg_git::unstage::hunks(&req, &headers)?;
    assert!(git(p, &["show", ":new"]).is_err());
    assert_eq!(
        git(p, &["status", "--porcelain=v1", "-z", "--", "new"])?,
        b"?? new\0"
    );

    std::fs::write(p.join("gone"), b"one\ntwo\nthree\n")?;
    git(p, &["add", "gone"])?;
    git(p, &["commit", "-qm", "gone"])?;
    git(p, &["rm", "gone"])?;
    let req = request(p, b"gone", Area::Staged)?;
    let diff = gg_git::working_diff::read(&req, || false)?;
    let hunk = diff
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Hunk)
        .ok_or("missing hunk")?;
    let deleted = diff
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Deleted && row.old_line == Some(2))
        .and_then(|row| row.old_line)
        .ok_or("missing deleted line")?;
    gg_git::unstage::changes(
        &req,
        &[Edit {
            header: hunk.text.clone(),
            changes: vec![Change::Deleted(deleted)],
        }],
    )?;
    assert_eq!(git(p, &["show", ":gone"])?, b"two\n");
    assert!(!p.join("gone").exists());
    assert_eq!(
        git(p, &["status", "--porcelain=v1", "-z", "--", "gone"])?,
        b"MD gone\0"
    );
    let req = request(p, b"gone", Area::Staged)?;
    let headers = hunk_headers(&req)?;
    gg_git::unstage::hunks(&req, &headers)?;
    assert_eq!(git(p, &["show", ":gone"])?, b"one\ntwo\nthree\n");
    assert_eq!(
        git(p, &["status", "--porcelain=v1", "-z", "--", "gone"])?,
        b" D gone\0"
    );
    oracle(p)?;
    Ok(())
}

#[test]
fn stage_hunks_reject_stale_inputs_and_locks_without_publishing() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    hunk_fixture(p, "sha1")?;
    let req = request(p, b"a", Area::Unstaged)?;
    let headers = hunk_headers(&req)?;
    let index = std::fs::read(p.join(".git/index"))?;
    std::fs::write(p.join(".git/index.lock"), b"external owner")?;
    assert!(gg_git::stage::hunks(&req, &headers[..1]).is_err());
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);
    assert_eq!(std::fs::read(p.join(".git/index.lock"))?, b"external owner");
    std::fs::remove_file(p.join(".git/index.lock"))?;

    std::fs::write(p.join("a"), b"rewritten\n")?;
    assert!(
        gg_git::stage::hunks(&req, &headers[..1])
            .err()
            .is_some_and(|e| e.contains("stale"))
    );
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);
    assert_eq!(std::fs::read(p.join("a"))?, b"rewritten\n");

    std::fs::write(
        p.join("a"),
        b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n",
    )?;
    let req = request(p, b"a", Area::Unstaged)?;
    let headers = hunk_headers(&req)?;
    std::fs::write(p.join("other"), b"new other\n")?;
    git(p, &["add", "other"])?;
    let index = std::fs::read(p.join(".git/index"))?;
    assert!(
        gg_git::stage::hunks(&req, &headers[..1])
            .err()
            .is_some_and(|e| e.contains("Refresh status"))
    );
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);

    let req = request(p, b"a", Area::Unstaged)?;
    let headers = hunk_headers(&req)?;
    git(p, &["commit", "--allow-empty", "-qm", "new head"])?;
    let index = std::fs::read(p.join(".git/index"))?;
    assert!(
        gg_git::stage::hunks(&req, &headers[..1])
            .err()
            .is_some_and(|e| e.contains("Refresh status"))
    );
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);
    assert!(!p.join(".git/index.lock").exists());
    Ok(())
}

#[test]
fn drop_selected_working_lines_restores_index_sides_and_keeps_other_hunks() -> Result {
    use gg_engine::diff::RowKind;
    use gg_git::patch::{Change, Edit};
    for hash in ["sha1", "sha256"] {
        let dir = tempfile::tempdir()?;
        let p = dir.path();
        hunk_fixture(p, hash)?;
        let index = std::fs::read(p.join(".git/index"))?;
        let base = b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n";

        let req = request(p, b"a", Area::Unstaged)?;
        let diff = gg_git::working_diff::read(&req, || false)?;
        let start = diff
            .rows
            .iter()
            .position(|row| row.kind == RowKind::Hunk)
            .ok_or("missing first hunk")?;
        let end = diff.rows[start + 1..]
            .iter()
            .position(|row| row.kind == RowKind::Hunk)
            .map_or(diff.rows.len(), |offset| start + 1 + offset);
        let hunk = &diff.rows[start];
        let changes = diff.rows[start + 1..end]
            .iter()
            .filter_map(|row| match row.kind {
                RowKind::Added => row.new_line.map(Change::Added),
                RowKind::Deleted => row.old_line.map(Change::Deleted),
                _ => None,
            })
            .collect::<Vec<_>>();
        gg_git::drop::changes(
            &req,
            &[Edit {
                header: hunk.text.clone(),
                changes,
            }],
        )?;
        assert_eq!(
            std::fs::read(p.join("a"))?,
            b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n"
        );
        assert_eq!(git(p, &["show", ":a"])?, base.as_slice());
        assert_eq!(std::fs::read(p.join(".git/index"))?, index);
        assert_eq!(
            git(p, &["status", "--porcelain=v1", "-z", "--", "a"])?,
            b" M a\0"
        );

        let req = request(p, b"a", Area::Unstaged)?;
        gg_git::drop::changes(
            &req,
            &[Edit {
                header: hunk_headers(&req)?[0].clone(),
                changes: vec![],
            }],
        )?;
        assert_eq!(std::fs::read(p.join("a"))?, base.as_slice());
        assert_eq!(std::fs::read(p.join(".git/index"))?, index);
        oracle(p)?;
    }
    Ok(())
}

#[test]
fn drop_whole_working_changes_restores_index_and_removes_untracked() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    std::fs::write(p.join("regular"), b"base\n")?;
    std::fs::write(p.join("deleted"), b"deleted\n")?;
    std::fs::write(p.join("exec"), b"exec\n")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(p.join("exec"), std::fs::Permissions::from_mode(0o755))?;
        std::os::unix::fs::symlink("regular", p.join("link"))?;
    }
    git(p, &["add", "."])?;
    git(p, &["commit", "-qm", "base"])?;
    let index = std::fs::read(p.join(".git/index"))?;

    std::fs::write(p.join("regular"), b"working\n")?;
    std::fs::remove_file(p.join("deleted"))?;
    std::fs::write(p.join("exec"), b"changed\n")?;
    std::fs::write(p.join("untracked"), b"new\n")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        std::fs::set_permissions(p.join("exec"), std::fs::Permissions::from_mode(0o644))?;
        std::fs::remove_file(p.join("link"))?;
        symlink("untracked", p.join("link"))?;
    }

    let req = request(p, b"regular", Area::Unstaged)?;
    gg_git::drop::file(&req)?;
    assert_eq!(std::fs::read(p.join("regular"))?, b"base\n");
    let req = request(p, b"deleted", Area::Unstaged)?;
    gg_git::drop::file(&req)?;
    assert_eq!(std::fs::read(p.join("deleted"))?, b"deleted\n");
    let req = request(p, b"exec", Area::Unstaged)?;
    gg_git::drop::file(&req)?;
    assert_eq!(std::fs::read(p.join("exec"))?, b"exec\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(p.join("exec"))?.permissions().mode() & 0o777,
            0o755
        );
    }
    let req = request(p, b"untracked", Area::Untracked)?;
    gg_git::drop::file(&req)?;
    assert!(!p.join("untracked").exists());
    #[cfg(unix)]
    {
        let req = request(p, b"link", Area::Unstaged)?;
        gg_git::drop::file(&req)?;
        assert_eq!(
            std::fs::read_link(p.join("link"))?,
            std::path::Path::new("regular")
        );
    }
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);
    assert!(scan(p)?.is_empty());
    Ok(())
}

#[test]
fn drop_rejects_partial_mode_changes_and_staged_inputs() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    std::fs::write(p.join("a"), b"one\ntwo\n")?;
    git(p, &["add", "a"])?;
    git(p, &["commit", "-qm", "base"])?;
    std::fs::write(p.join("a"), b"ONE\ntwo\n")?;
    let req = request(p, b"a", Area::Unstaged)?;
    let headers = hunk_headers(&req)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(p.join("a"), std::fs::Permissions::from_mode(0o755))?;
        let Err(error) = gg_git::drop::changes(
            &req,
            &[gg_git::patch::Edit {
                header: headers[0].clone(),
                changes: vec![],
            }],
        ) else {
            return Err("mode change was not rejected".into());
        };
        assert!(error.contains("whole file"), "{error}");
        std::fs::set_permissions(p.join("a"), std::fs::Permissions::from_mode(0o644))?;
    }
    let stale = request(p, b"a", Area::Unstaged)?;
    git(p, &["add", "a"])?;
    let Err(error) = gg_git::drop::file(&stale) else {
        return Err("stale drop was not rejected".into());
    };
    assert!(error.contains("changed since"), "{error}");
    let req = request(p, b"a", Area::Staged)?;
    let Err(error) = gg_git::drop::file(&req) else {
        return Err("staged file was not rejected".into());
    };
    assert!(error.contains("Unstage"), "{error}");
    let Err(error) = gg_git::drop::changes(
        &req,
        &[gg_git::patch::Edit {
            header: headers[0].clone(),
            changes: vec![],
        }],
    ) else {
        return Err("staged line drop was not rejected".into());
    };
    assert!(error.contains("unstaged modification"), "{error}");
    Ok(())
}

#[test]
fn drop_restores_filtered_line_endings_and_missing_final_newline() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    std::fs::write(p.join(".gitattributes"), b"*.txt text eol=crlf\n")?;
    let original = b"one\r\ntwo\r\n";
    std::fs::write(p.join("a.txt"), original)?;
    git(p, &["add", "."])?;
    git(p, &["commit", "-qm", "base"])?;
    let index = std::fs::read(p.join(".git/index"))?;
    std::fs::write(p.join("a.txt"), b"ONE\r\ntwo")?;

    let req = request(p, b"a.txt", Area::Unstaged)?;
    let header = hunk_headers(&req)?
        .into_iter()
        .next()
        .ok_or("missing hunk")?;
    gg_git::drop::changes(
        &req,
        &[gg_git::patch::Edit {
            header,
            changes: vec![],
        }],
    )?;
    assert_eq!(std::fs::read(p.join("a.txt"))?, original);
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);
    assert!(scan(p)?.is_empty());
    Ok(())
}
