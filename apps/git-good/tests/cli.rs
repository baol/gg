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
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(())
}
fn init(path: &Path, args: &[&str]) -> Result {
    let mut all = Vec::with_capacity(args.len() + 1);
    all.push("init");
    all.extend_from_slice(args);
    git(path, &all)?;
    git(path, &["config", "core.autocrlf", "false"])
}

fn git_output(path: &Path, args: &[&str]) -> Result<Vec<u8>> {
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
    Ok(output.stdout)
}
#[test]
fn git_dispatch_discovers_current_and_nested_repositories() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    let nested = repo.path().join("nested");
    std::fs::create_dir(&nested)?;
    let binary = Path::new(env!("CARGO_BIN_EXE_git-good"));
    let bin_dir = binary.parent().ok_or("binary directory missing")?;
    let path = std::env::join_paths(std::iter::once(bin_dir.to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))?;
    for cwd in [repo.path(), nested.as_path()] {
        let output = Command::new("git")
            .args(["good", "history"])
            .current_dir(cwd)
            .env("PATH", &path)
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(json["head"], "main");
        assert_eq!(json["commits"], serde_json::json!([]));
        let resolved = json["path"].as_str().ok_or("missing path")?;
        assert_eq!(
            std::fs::canonicalize(resolved)?,
            std::fs::canonicalize(repo.path())?
        );
    }
    Ok(())
}
#[test]
fn explicit_path_overrides_cwd_and_failure_uses_stderr() -> Result {
    let repo = tempfile::tempdir()?;
    let other = tempfile::tempdir()?;
    init(repo.path(), &["-b", "explicit"])?;
    let binary = env!("CARGO_BIN_EXE_git-good");
    let output = Command::new(binary)
        .arg("history")
        .arg(repo.path())
        .current_dir(other.path())
        .output()?;
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json["head"], "explicit");
    let error = Command::new(binary)
        .arg("history")
        .current_dir(other.path())
        .output()?;
    assert!(!error.status.success());
    assert!(error.stdout.is_empty());
    assert!(String::from_utf8_lossy(&error.stderr).contains("Could not read"));
    Ok(())
}

#[test]
fn json_includes_every_commit_beyond_the_former_cap() -> Result {
    use std::io::Write;
    use std::process::Stdio;
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    let mut child = Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .args(["fast-import", "--quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    {
        let mut input = child.stdin.take().ok_or("missing fast-import input")?;
        for i in 0..501 {
            let message = format!("Commit {i}");
            writeln!(
                input,
                "commit refs/heads/main\ncommitter Ada <ada@example.org> 1760000000 +0000\ndata {}\n{}\n",
                message.len(),
                message
            )?;
        }
    }
    let import = child.wait_with_output()?;
    assert!(
        import.status.success(),
        "{}",
        String::from_utf8_lossy(&import.stderr)
    );
    let output = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .arg("history")
        .current_dir(repo.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let history: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let commits = history["commits"].as_array().ok_or("missing commits")?;
    assert_eq!(commits.len(), 501);
    assert_eq!(
        commits[0]["message"].as_str().map(str::trim),
        Some("Commit 500")
    );
    assert_eq!(
        commits[500]["message"].as_str().map(str::trim),
        Some("Commit 0")
    );
    assert_eq!(history["truncated"], false);
    Ok(())
}

#[test]
fn browsing_preserves_repository_index_and_dirty_worktree() -> Result {
    fn snapshot(root: &Path) -> Result<std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>> {
        fn visit(
            root: &Path,
            dir: &Path,
            files: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
        ) -> Result {
            for entry in std::fs::read_dir(dir)? {
                let path = entry?.path();
                if path.is_dir() {
                    visit(root, &path, files)?;
                } else {
                    files.insert(path.strip_prefix(root)?.to_path_buf(), std::fs::read(path)?);
                }
            }
            Ok(())
        }
        let mut files = std::collections::BTreeMap::new();
        visit(root, root, &mut files)?;
        Ok(files)
    }
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    std::fs::write(repo.path().join("tracked"), b"committed\n")?;
    git(repo.path(), &["add", "tracked"])?;
    git(
        repo.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
            "commit",
            "-m",
            "baseline",
        ],
    )?;
    std::fs::write(repo.path().join("tracked"), b"staged\n")?;
    git(repo.path(), &["add", "tracked"])?;
    std::fs::write(repo.path().join("tracked"), b"unstaged\n")?;
    std::fs::write(repo.path().join("untracked"), [0, 255, 1])?;
    let before = snapshot(repo.path())?;
    let exports = tempfile::tempdir()?;
    for pass in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_git-good"))
            .arg("history")
            .current_dir(repo.path())
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let history: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(
            history["commits"]
                .as_array()
                .ok_or("missing commits")?
                .len(),
            1
        );
        assert_eq!(
            snapshot(repo.path())?,
            before,
            "browsing changed repository files"
        );
        for args in [
            vec!["show", "HEAD"],
            vec!["show", "HEAD", "--file", "tracked"],
            vec![
                "show", "HEAD", "--file", "tracked", "--export", "after", "--output",
            ],
        ] {
            let exporting = args.last() == Some(&"--output");
            let destination = exports.path().join(format!("saved-{pass}"));
            let mut command = Command::new(env!("CARGO_BIN_EXE_git-good"));
            command.args(args).current_dir(repo.path());
            if exporting {
                command.arg(&destination);
            }
            let output = command.output()?;
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.stderr.is_empty());
            let _: serde_json::Value = serde_json::from_slice(&output.stdout)?;
            if exporting {
                assert_eq!(std::fs::read(destination)?, b"committed\n");
            }
            assert_eq!(
                snapshot(repo.path())?,
                before,
                "review/export changed repository files"
            );
        }
    }
    Ok(())
}

#[test]
fn version_matches_package_without_a_repository() -> Result {
    let directory = tempfile::tempdir()?;
    let output = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .arg("--version")
        .current_dir(directory.path())
        .output()?;
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout)?,
        format!("git-good {}\n", env!("CARGO_PKG_VERSION"))
    );
    Ok(())
}

#[test]
fn help_lists_every_cli_command() -> Result {
    let directory = tempfile::tempdir()?;
    let output = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .arg("--help")
        .current_dir(directory.path())
        .output()?;
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let help = String::from_utf8(output.stdout)?;
    for usage in [
        "git-good history",
        "git-good refs",
        "git-good status",
        "git-good stage",
        "git-good unstage",
        "git-good commit",
        "git-good switch",
        "git-good fetch",
        "git-good push",
        "git-good clone",
        "git-good stash",
        "git-good cherry-pick",
        "git-good revert",
        "git-good show",
    ] {
        assert!(help.contains(usage), "help is missing {usage:?}:\n{help}");
    }
    Ok(())
}

#[test]
fn show_discovers_repo_streams_exact_files_and_rejects_invalid_parent() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    std::fs::create_dir(repo.path().join("nested"))?;
    std::fs::write(repo.path().join("nested/café.txt"), "hello")?;
    git(repo.path(), &["add", "."])?;
    git(
        repo.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
            "commit",
            "-m",
            "root",
        ],
    )?;
    let binary = env!("CARGO_BIN_EXE_git-good");
    let output = Command::new(binary)
        .args(["show", "HEAD"])
        .current_dir(repo.path().join("nested"))
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json["complete"], true);
    assert!(json["comparison"]["parent"].is_null());
    assert_eq!(json["files"][0]["path"], "nested/café.txt");
    assert_eq!(
        json["files"][0]["path_bytes"],
        serde_json::json!("nested/café.txt".as_bytes())
    );
    let other = tempfile::tempdir()?;
    let explicit = Command::new(binary)
        .args(["show", "HEAD", "--repo"])
        .arg(repo.path())
        .current_dir(other.path())
        .output()?;
    assert!(explicit.status.success());
    for args in [
        vec!["show", "HEAD", "--parent", "1"],
        vec!["show", "missing"],
        vec!["show"],
    ] {
        let error = Command::new(binary)
            .args(args)
            .current_dir(repo.path())
            .output()?;
        assert!(!error.status.success());
        assert!(error.stdout.is_empty());
        assert!(!error.stderr.is_empty());
    }
    Ok(())
}

#[test]
fn show_file_returns_diff_and_missing_path_uses_stderr() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    std::fs::write(repo.path().join("file"), "hello\nworld")?;
    git(repo.path(), &["add", "."])?;
    git(
        repo.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
            "commit",
            "-m",
            "root",
        ],
    )?;
    let output = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .args(["show", "HEAD", "--file", "file"])
        .current_dir(repo.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json["diff"]["kind"], "text");
    assert_eq!(json["diff"]["complete"], true);
    let rows = json["diff"]["rows"].as_array().ok_or("missing rows")?;
    assert!(rows.iter().any(|r| r["kind"] == "no_newline"));
    assert!(
        rows.iter()
            .any(|r| r["new_line"] == 2 && r["text"] == "world")
    );
    let error = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .args(["show", "HEAD", "--file", "missing"])
        .current_dir(repo.path())
        .output()?;
    assert!(!error.status.success());
    assert!(error.stdout.is_empty());
    assert!(!error.stderr.is_empty());
    Ok(())
}

#[cfg(unix)]
#[test]
fn show_file_uses_exact_non_utf8_path_bytes() -> Result {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    let name = OsString::from_vec(b"raw\xff\nname".to_vec());
    // Store raw Git paths directly: host filesystems need not support these bytes.
    use std::{io::Write, process::Stdio};
    let mut import = Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .args(["fast-import", "--quiet"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    import.stdin.take().ok_or("missing import input")?.write_all(
        b"blob\nmark :1\ndata 14\nexact content\n\ncommit refs/heads/main\ncommitter Test <test@example.org> 1700000000 +0000\ndata 8\nraw path\nM 100644 :1 \"raw\\377\\nname\"\n\n",
    )?;
    let imported = import.wait_with_output()?;
    assert!(
        imported.status.success(),
        "{}",
        String::from_utf8_lossy(&imported.stderr)
    );
    let output = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .args(["show", "HEAD", "--file"])
        .arg(name)
        .current_dir(repo.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        json["file"]["path_bytes"],
        serde_json::json!(b"raw\xff\nname")
    );
    assert_eq!(json["file"]["path"], "raw\\xff\\nname");
    assert_eq!(json["diff"]["kind"], "text");
    assert_eq!(
        json["diff"]["rows"][1]["bytes"],
        serde_json::json!(b"exact content\n")
    );
    Ok(())
}

#[test]
fn export_is_exact_beyond_diff_budget_and_preserves_existing_destinations() -> Result {
    let repo = tempfile::tempdir()?;
    let destination = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    let bytes: Vec<_> = (0..(3 * 1024 * 1024)).map(|n| (n % 251) as u8).collect();
    std::fs::write(repo.path().join("large.bin"), &bytes)?;
    git(repo.path(), &["add", "."])?;
    git(
        repo.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
            "commit",
            "-m",
            "large",
        ],
    )?;
    git(repo.path(), &["gc", "--prune=now"])?;
    let output = destination.path().join("export.bin");
    let run = |side: &str, path: &Path| {
        Command::new(env!("CARGO_BIN_EXE_git-good"))
            .args([
                "show",
                "HEAD",
                "--file",
                "large.bin",
                "--export",
                side,
                "--output",
            ])
            .arg(path)
            .current_dir(repo.path())
            .output()
    };
    let result = run("after", &output)?;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(std::fs::read(&output)?, bytes);
    let json: serde_json::Value = serde_json::from_slice(&result.stdout)?;
    assert_eq!(json["bytes"], bytes.len());
    std::fs::write(&output, b"keep")?;
    assert!(!run("after", &output)?.status.success());
    assert_eq!(std::fs::read(&output)?, b"keep");
    assert_eq!(std::fs::read_dir(destination.path())?.count(), 1);
    assert!(
        !run("before", &destination.path().join("absent"))?
            .status
            .success()
    );
    assert!(!destination.path().join("absent").exists());
    assert!(
        !run("after", &repo.path().join(".git/export"))?
            .status
            .success()
    );
    assert!(!repo.path().join(".git/export").exists());
    assert_eq!(std::fs::read(repo.path().join("large.bin"))?, bytes);
    Ok(())
}

#[test]
fn status_cli_streams_whole_worktree_and_reports_errors() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    std::fs::create_dir(repo.path().join("nested"))?;
    std::fs::write(repo.path().join("new.txt"), b"new")?;
    let binary = env!("CARGO_BIN_EXE_git-good");
    for explicit in [false, true] {
        let mut command = Command::new(binary);
        command
            .arg("status")
            .current_dir(repo.path().join("nested"));
        if explicit {
            command.arg(repo.path());
        }
        let out = command.output()?;
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stderr.is_empty());
        let json: serde_json::Value = serde_json::from_slice(&out.stdout)?;
        assert_eq!(json["complete"], true);
        assert_eq!(json["entries"][0]["area"], "untracked");
        assert_eq!(json["entries"][0]["path"], "new.txt");
        assert_eq!(json["repository"]["head"], serde_json::Value::Null);
    }
    let missing = repo.path().join("missing");
    for args in [
        vec!["status".into(), missing.into_os_string()],
        vec![
            "status".into(),
            repo.path().as_os_str().into(),
            "extra".into(),
        ],
    ] {
        let out = Command::new(binary).args(args).output()?;
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
    Ok(())
}

#[test]
fn selected_status_cli_requires_area_and_returns_working_diff() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    init(p, &["-b", "main"])?;
    std::fs::write(p.join("a"), b"staged\n")?;
    git(p, &["add", "a"])?;
    std::fs::write(p.join("a"), b"working\n")?;
    let binary = env!("CARGO_BIN_EXE_git-good");
    for area in ["staged", "unstaged"] {
        let output = Command::new(binary)
            .args(["status", "--file", "a", "--area", area])
            .current_dir(p)
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(json["comparison"]["entry"]["area"], area);
        assert_eq!(json["diff"]["kind"], "text");
        assert!(
            json["diff"]["rows"]
                .as_array()
                .is_some_and(|rows| !rows.is_empty())
        );
    }
    for args in [
        vec!["status", "--file", "a"],
        vec!["status", "--file", "missing", "--area", "unstaged"],
        vec!["status", "--area", "wrong"],
    ] {
        let output = Command::new(binary).args(args).current_dir(p).output()?;
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    Ok(())
}

#[test]
fn unstage_cli_receipt_and_errors_preserve_working_files() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    init(p, &[])?;
    std::fs::write(p.join("a"), "staged")?;
    git(p, &["add", "a"])?;
    std::fs::write(p.join("a"), "working")?;
    let binary = env!("CARGO_BIN_EXE_git-good");
    let out = Command::new(binary)
        .args(["unstage", "--file", "a", "--repo"])
        .arg(p)
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["complete"], true);
    assert_eq!(receipt["operation"], "unstage");
    assert_eq!(std::fs::read(p.join("a"))?, b"working");
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(p)
            .arg("ls-files")
            .output()?
            .stdout
            .is_empty()
    );
    for args in [
        vec!["unstage"],
        vec!["unstage", "--file", "missing"],
        vec!["unstage", "--file", "../a"],
    ] {
        let out = Command::new(binary).args(args).current_dir(p).output()?;
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
    Ok(())
}

#[test]
fn stage_cli_receipt_and_errors_preserve_working_files() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    init(p, &[])?;
    std::fs::write(p.join("a"), "working")?;
    let binary = env!("CARGO_BIN_EXE_git-good");
    let out = Command::new(binary)
        .args(["stage", "--file", "a", "--repo"])
        .arg(p)
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["complete"], true);
    assert_eq!(receipt["operation"], "stage");
    assert_eq!(std::fs::read(p.join("a"))?, b"working");
    assert!(
        !Command::new("git")
            .arg("-C")
            .arg(p)
            .args(["ls-files", "--", "a"])
            .output()?
            .stdout
            .is_empty()
    );
    for args in [
        vec!["stage"],
        vec!["stage", "--file", "missing"],
        vec!["stage", "--file", "../a"],
        vec!["stage", "--file", "a"],
    ] {
        let out = Command::new(binary).args(args).current_dir(p).output()?;
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
    assert_eq!(std::fs::read(p.join("a"))?, b"working");
    Ok(())
}

#[test]
fn stage_cli_selected_hunks_match_git_and_keep_remaining_edits() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    init(p, &["-b", "main"])?;
    std::fs::write(
        p.join("a"),
        b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n",
    )?;
    git(p, &["add", "a"])?;
    git(
        p,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
            "commit",
            "-qm",
            "base",
        ],
    )?;
    let working = b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\n";
    std::fs::write(p.join("a"), working)?;
    let binary = env!("CARGO_BIN_EXE_git-good");
    let header = |p: &Path| -> Result<String> {
        let status = Command::new(env!("CARGO_BIN_EXE_git-good"))
            .args(["status", "--file", "a", "--area", "unstaged"])
            .current_dir(p)
            .output()?;
        assert!(
            status.status.success(),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );
        let json: serde_json::Value = serde_json::from_slice(&status.stdout)?;
        json["diff"]["rows"]
            .as_array()
            .and_then(|rows| rows.iter().find(|row| row["kind"] == "hunk"))
            .and_then(|row| row["text"].as_str())
            .map(str::to_string)
            .ok_or_else(|| "missing hunk header".into())
    };
    let first = header(p)?;
    let out = Command::new(binary)
        .args(["stage", "--file", "a", "--hunk", &first])
        .current_dir(p)
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "stage");
    assert_eq!(receipt["hunks"], 1);
    assert_eq!(receipt["complete"], true);
    let staged = Command::new("git")
        .arg("-C")
        .arg(p)
        .args(["show", ":a"])
        .output()?;
    assert!(staged.status.success());
    assert_eq!(
        staged.stdout,
        b"ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\n"
    );
    assert_eq!(std::fs::read(p.join("a"))?, working);
    let status = Command::new("git")
        .arg("-C")
        .arg(p)
        .args(["status", "--porcelain=v1", "-z", "--", "a"])
        .output()?;
    assert_eq!(status.stdout, b"MM a\0");
    let index = std::fs::read(p.join(".git/index"))?;
    let bad = Command::new(binary)
        .args(["stage", "--file", "a", "--hunk", "@@ -99,1 +99,1 @@"])
        .current_dir(p)
        .output()?;
    assert!(!bad.status.success());
    assert!(bad.stdout.is_empty());
    assert!(!bad.stderr.is_empty());
    assert_eq!(std::fs::read(p.join(".git/index"))?, index);
    let missing = Command::new(binary)
        .args(["stage", "--file", "a", "--hunk"])
        .current_dir(p)
        .output()?;
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
    assert!(!missing.stderr.is_empty());

    let remaining = header(p)?;
    let out = Command::new(binary)
        .args(["stage", "--file", "a", "--hunk", &remaining])
        .current_dir(p)
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["hunks"], 1);
    let staged = Command::new("git")
        .arg("-C")
        .arg(p)
        .args(["show", ":a"])
        .output()?;
    assert_eq!(staged.stdout, working);
    assert_eq!(std::fs::read(p.join("a"))?, working);
    Ok(())
}

#[test]
fn commit_cli_commits_the_index_and_preserves_working_changes() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"base\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "base"])?;
    std::fs::write(repo.path().join("a"), b"staged\n")?;
    git(repo.path(), &["add", "a"])?;
    std::fs::write(repo.path().join("a"), b"working\n")?;
    let output = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .args(["commit", "-m", "from cli"])
        .current_dir(repo.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(receipt["operation"], "commit");
    let id = receipt["id"].as_str().ok_or("missing id")?;
    assert_eq!(
        id,
        String::from_utf8(git_output(repo.path(), &["rev-parse", "HEAD"])?)?.trim()
    );
    assert_eq!(
        git_output(repo.path(), &["show", "-s", "--format=%s", "HEAD"])?,
        b"from cli\n"
    );
    assert_eq!(git_output(repo.path(), &["show", ":a"])?, b"staged\n");
    assert_eq!(std::fs::read(repo.path().join("a"))?, b"working\n");
    assert_eq!(
        git_output(repo.path(), &["status", "--porcelain=v1"])?,
        b" M a\n"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn commit_cli_runs_hooks_and_reports_failures_with_recovery() -> Result {
    use std::os::unix::fs::PermissionsExt;
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"base\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "base"])?;
    std::fs::write(repo.path().join("a"), b"staged\n")?;
    git(repo.path(), &["add", "a"])?;
    let hooks = repo.path().join(".git/hooks");
    std::fs::create_dir_all(&hooks)?;
    let hook = hooks.join("pre-commit");
    std::fs::write(
        &hook,
        "#!/bin/sh\nprintf 'hooked\\n' > hook.txt\ngit add hook.txt\n",
    )?;
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
    let output = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .args(["commit", "-m", "with hook"])
        .current_dir(repo.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        git_output(repo.path(), &["show", "HEAD:hook.txt"])?,
        b"hooked\n"
    );

    std::fs::write(repo.path().join("a"), b"next\n")?;
    git(repo.path(), &["add", "a"])?;
    let index = std::fs::read(repo.path().join(".git/index"))?;
    std::fs::write(&hook, "#!/bin/sh\necho blocked >&2\nexit 5\n")?;
    let failed = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .args(["commit", "-m", "blocked"])
        .current_dir(repo.path())
        .output()?;
    assert!(!failed.status.success());
    assert!(failed.stdout.is_empty());
    let error = String::from_utf8_lossy(&failed.stderr);
    assert!(error.contains("pre-commit"), "{error}");
    assert!(error.contains("blocked"), "{error}");
    assert!(error.contains("preserved"), "{error}");
    assert_eq!(std::fs::read(repo.path().join(".git/index"))?, index);
    Ok(())
}

#[test]
fn refs_cli_streams_branches_and_tags_without_mutating() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"base\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "base"])?;
    git(repo.path(), &["branch", "feature"])?;
    git(repo.path(), &["tag", "v1"])?;
    git(
        repo.path(),
        &["remote", "add", "origin", "https://example.org/gg.git"],
    )?;
    git(
        repo.path(),
        &[
            "remote",
            "add",
            "secret",
            "https://user:password@example.org/secret.git",
        ],
    )?;
    let head = git_output(repo.path(), &["rev-parse", "HEAD"])?;
    git(
        repo.path(),
        &[
            "update-ref",
            "refs/remotes/origin/main",
            String::from_utf8_lossy(&head).trim(),
        ],
    )?;
    git(repo.path(), &["config", "branch.main.remote", "origin"])?;
    git(
        repo.path(),
        &["config", "branch.main.merge", "refs/heads/main"],
    )?;
    let before = git_output(repo.path(), &["rev-parse", "HEAD"])?;
    let output = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .arg("refs")
        .arg(repo.path())
        .current_dir(repo.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let snapshot = &json["refs"];
    assert_eq!(snapshot["head"]["name"], "refs/heads/main");
    let items = snapshot["items"].as_array().ok_or("missing refs")?;
    for name in ["main", "feature", "v1"] {
        assert!(
            items.iter().any(|item| item["name"] == name),
            "missing {name}"
        );
    }
    assert_eq!(
        snapshot["remotes"][0]["name"], "origin",
        "missing remote snapshot"
    );
    assert_eq!(
        snapshot["remotes"][0]["fetch_url"],
        "https://example.org/gg.git"
    );
    let secret = snapshot["remotes"]
        .as_array()
        .and_then(|remotes| remotes.iter().find(|remote| remote["name"] == "secret"))
        .ok_or("missing secret remote")?;
    assert_eq!(secret["fetch_url"], "https://user@example.org/secret.git");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("password"));
    assert_eq!(snapshot["tracking"][0]["upstream"], "origin/main");
    assert_eq!(snapshot["tracking"][0]["ahead"], 0);
    assert_eq!(snapshot["tracking"][0]["behind"], 0);
    assert_eq!(git_output(repo.path(), &["rev-parse", "HEAD"])?, before);
    Ok(())
}

#[test]
fn switch_cli_changes_branches_and_reports_dirty_worktrees() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"main\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "base"])?;
    git(repo.path(), &["switch", "-c", "feature"])?;
    std::fs::write(repo.path().join("a"), b"feature\n")?;
    git(repo.path(), &["commit", "-qam", "feature"])?;
    git(repo.path(), &["switch", "main"])?;

    let output = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .args(["switch", "feature"])
        .current_dir(repo.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(receipt["operation"], "switch");
    assert_eq!(receipt["branch"], "feature");
    assert_eq!(
        git_output(repo.path(), &["symbolic-ref", "--short", "HEAD"])?,
        b"feature\n"
    );

    std::fs::write(repo.path().join("a"), b"dirty\n")?;
    let dirty = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .args(["switch", "main"])
        .current_dir(repo.path())
        .output()?;
    assert!(!dirty.status.success());
    assert!(String::from_utf8_lossy(&dirty.stderr).contains("not clean"));
    assert_eq!(
        git_output(repo.path(), &["symbolic-ref", "--short", "HEAD"])?,
        b"feature\n"
    );
    std::fs::write(repo.path().join("a"), b"feature\n")?;

    let created = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .args(["switch", "--create", "next"])
        .current_dir(repo.path())
        .output()?;
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&created.stdout)?;
    assert_eq!(receipt["operation"], "create");
    assert_eq!(
        git_output(repo.path(), &["symbolic-ref", "--short", "HEAD"])?,
        b"next\n"
    );
    Ok(())
}

#[test]
fn sync_cli_clones_fetches_pushes_and_reports_failures() -> Result {
    let remote = tempfile::tempdir()?;
    git(remote.path(), &["init", "--bare", "-b", "main"])?;
    let source = tempfile::tempdir()?;
    init(source.path(), &["-b", "main"])?;
    std::fs::write(source.path().join("a"), b"base\n")?;
    git(source.path(), &["add", "a"])?;
    git(
        source.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
            "commit",
            "-qm",
            "base",
        ],
    )?;
    let remote_path = remote.path().to_string_lossy().into_owned();
    git(
        source.path(),
        &["remote", "add", "origin", remote_path.as_str()],
    )?;
    git(source.path(), &["push", "-q", "origin", "main"])?;

    let destination = tempfile::tempdir()?;
    let target = destination.path().join("cloned");
    let binary = env!("CARGO_BIN_EXE_git-good");
    let out = Command::new(binary)
        .arg("clone")
        .arg(remote.path())
        .arg(&target)
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "clone");
    assert_eq!(receipt["complete"], true);
    assert_eq!(receipt["branch"], "main");
    assert_eq!(
        std::fs::canonicalize(receipt["path"].as_str().ok_or("missing clone path")?)?,
        std::fs::canonicalize(&target)?
    );
    assert_eq!(git_output(&target, &["show", "HEAD:a"])?, b"base\n");
    git(&target, &["config", "core.autocrlf", "false"])?;
    git(&target, &["reset", "--hard", "HEAD"])?;

    std::fs::write(source.path().join("a"), b"remote\n")?;
    git(
        source.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
            "commit",
            "-qam",
            "remote",
        ],
    )?;
    git(source.path(), &["push", "-q", "origin", "main"])?;
    let remote_head = git_output(source.path(), &["rev-parse", "HEAD"])?;
    let local_head = git_output(&target, &["rev-parse", "HEAD"])?;
    let out = Command::new(binary)
        .args(["fetch", "origin", "--prune", "--repo"])
        .arg(&target)
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "fetch");
    assert_eq!(receipt["remote"], "origin");
    assert_eq!(receipt["pruned"], true);
    assert_eq!(receipt["complete"], true);
    assert_eq!(
        git_output(&target, &["rev-parse", "refs/remotes/origin/main"])?,
        remote_head
    );
    assert_eq!(git_output(&target, &["rev-parse", "HEAD"])?, local_head);

    git(&target, &["merge", "--ff-only", "origin/main"])?;
    std::fs::write(target.join("a"), b"pushed\n")?;
    git(
        &target,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
            "commit",
            "-qam",
            "pushed",
        ],
    )?;
    let pushed_head = git_output(&target, &["rev-parse", "HEAD"])?;
    let out = Command::new(binary)
        .args(["push", "origin", "main", "--set-upstream", "--repo"])
        .arg(&target)
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "push");
    assert_eq!(receipt["remote"], "origin");
    assert_eq!(receipt["branch"], "main");
    assert_eq!(receipt["upstream"], "origin/main");
    assert_eq!(receipt["set_upstream"], true);
    assert_eq!(receipt["complete"], true);
    assert_eq!(
        git_output(remote.path(), &["rev-parse", "refs/heads/main"])?,
        pushed_head
    );

    git(source.path(), &["fetch", "origin"])?;
    git(source.path(), &["reset", "--hard", "FETCH_HEAD"])?;
    std::fs::write(source.path().join("a"), b"source ahead\n")?;
    git(
        source.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
            "commit",
            "-qam",
            "source ahead",
        ],
    )?;
    git(source.path(), &["push", "-q", "origin", "main"])?;
    std::fs::write(target.join("b"), b"local only\n")?;
    git(&target, &["add", "b"])?;
    git(
        &target,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
            "commit",
            "-qm",
            "local only",
        ],
    )?;
    let rejected = Command::new(binary)
        .args(["push", "origin", "main", "--repo"])
        .arg(&target)
        .output()?;
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
    let error = String::from_utf8_lossy(&rejected.stderr);
    assert!(error.contains("failed"), "{error}");
    assert_eq!(std::fs::read(target.join("b"))?, b"local only\n");

    for args in [vec!["clone"], vec!["fetch", "--repo"]] {
        let out = Command::new(binary)
            .args(args)
            .current_dir(&target)
            .output()?;
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
    Ok(())
}

#[test]
fn stash_cli_lists_pushes_applies_pops_and_drops() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"base\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "base"])?;

    let binary = env!("CARGO_BIN_EXE_git-good");
    std::fs::write(repo.path().join("a"), b"modified\n")?;
    let out = Command::new(binary)
        .args(["stash", "push", "-m", "work", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "stash-push");
    assert_eq!(receipt["created"], true);
    assert_eq!(receipt["reference"], "stash@{0}");
    assert_eq!(std::fs::read(repo.path().join("a"))?, b"base\n");

    let out = Command::new(binary)
        .args(["stash", "list", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "stash-list");
    assert_eq!(receipt["stashes"][0]["index"], 0);
    assert_eq!(receipt["stashes"][0]["reference"], "stash@{0}");
    assert!(
        receipt["stashes"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("work"))
    );

    let out = Command::new(binary)
        .args(["stash", "apply", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "stash-apply");
    assert_eq!(receipt["applied"], true);
    assert_eq!(receipt["dropped"], false);
    assert_eq!(receipt["conflicts"], serde_json::json!([]));
    assert_eq!(std::fs::read(repo.path().join("a"))?, b"modified\n");
    assert_eq!(
        git_output(repo.path(), &["stash", "list"])?
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .count(),
        1
    );

    git(repo.path(), &["checkout", "--", "a"])?;
    let out = Command::new(binary)
        .args(["stash", "pop", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "stash-pop");
    assert_eq!(receipt["applied"], true);
    assert_eq!(receipt["dropped"], true);
    assert_eq!(std::fs::read(repo.path().join("a"))?, b"modified\n");
    assert_eq!(git_output(repo.path(), &["stash", "list"])?, b"");

    std::fs::write(repo.path().join("a"), b"again\n")?;
    git_output(repo.path(), &["stash", "push", "-m", "again"])?;
    let out = Command::new(binary)
        .args(["stash", "drop", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "stash-drop");
    assert_eq!(receipt["dropped"], true);
    assert_eq!(git_output(repo.path(), &["stash", "list"])?, b"");

    let out = Command::new(binary)
        .args(["stash", "apply", "abc", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("must look like"));
    Ok(())
}

#[test]
fn cherry_pick_cli_applies_conflicts_and_recovers() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"base\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "base"])?;
    git(repo.path(), &["switch", "-qc", "other"])?;
    std::fs::write(repo.path().join("a"), b"other\n")?;
    git(repo.path(), &["commit", "-qam", "other"])?;
    git(repo.path(), &["switch", "-q", "main"])?;
    std::fs::write(repo.path().join("a"), b"main\n")?;
    git(repo.path(), &["commit", "-qam", "main"])?;
    let main_head = git_output(repo.path(), &["rev-parse", "HEAD"])?;

    let binary = env!("CARGO_BIN_EXE_git-good");
    let out = Command::new(binary)
        .args(["cherry-pick", "other", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "cherry-pick");
    assert_eq!(receipt["applied"], false);
    assert_eq!(receipt["in_progress"], true);
    assert_eq!(receipt["conflicts"], serde_json::json!(["a"]));

    let out = Command::new(binary)
        .args(["cherry-pick", "--abort", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["in_progress"], false);
    assert_eq!(git_output(repo.path(), &["rev-parse", "HEAD"])?, main_head);
    assert_eq!(std::fs::read(repo.path().join("a"))?, b"main\n");

    Command::new(binary)
        .args(["cherry-pick", "other", "--repo"])
        .arg(repo.path())
        .output()?;
    std::fs::write(repo.path().join("a"), b"resolved\n")?;
    git(repo.path(), &["add", "a"])?;
    let out = Command::new(binary)
        .args(["cherry-pick", "--continue", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["applied"], true);
    assert_eq!(receipt["in_progress"], false);
    assert_eq!(
        git_output(repo.path(), &["log", "-1", "--format=%s"])?,
        b"other\n"
    );

    let out = Command::new(binary)
        .args(["cherry-pick", "--bad", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("Unknown cherry-pick option"));
    Ok(())
}

#[test]
fn revert_cli_inverts_a_commit() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"base\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "base"])?;
    std::fs::write(repo.path().join("a"), b"changed\n")?;
    git(repo.path(), &["commit", "-qam", "changed"])?;

    let binary = env!("CARGO_BIN_EXE_git-good");
    let out = Command::new(binary)
        .args(["revert", "HEAD", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "revert");
    assert_eq!(receipt["applied"], true);
    assert_eq!(receipt["in_progress"], false);
    assert_eq!(std::fs::read(repo.path().join("a"))?, b"base\n");
    assert!(
        String::from_utf8(git_output(repo.path(), &["log", "-1", "--format=%s"])?)?
            .starts_with("Revert")
    );
    Ok(())
}

#[test]
fn merge_cli_fast_forwards_conflicts_and_recovers() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"base\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "base"])?;
    git(repo.path(), &["switch", "-qc", "feature"])?;
    std::fs::write(repo.path().join("b"), b"feature\n")?;
    git(repo.path(), &["add", "b"])?;
    git(repo.path(), &["commit", "-qm", "feature"])?;
    git(repo.path(), &["switch", "-q", "main"])?;

    let binary = env!("CARGO_BIN_EXE_git-good");
    let out = Command::new(binary)
        .args(["merge", "feature", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "merge");
    assert_eq!(receipt["kind"], "fast_forward");
    assert_eq!(receipt["applied"], true);
    assert_eq!(std::fs::read(repo.path().join("b"))?, b"feature\n");

    git(repo.path(), &["switch", "-qc", "conflict"])?;
    std::fs::write(repo.path().join("a"), b"feature\n")?;
    git(repo.path(), &["commit", "-qam", "feature change"])?;
    git(repo.path(), &["switch", "-q", "main"])?;
    std::fs::write(repo.path().join("a"), b"main\n")?;
    git(repo.path(), &["commit", "-qam", "main change"])?;
    let main_head = git_output(repo.path(), &["rev-parse", "HEAD"])?;

    let out = Command::new(binary)
        .args(["merge", "conflict", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["applied"], false);
    assert_eq!(receipt["in_progress"], true);
    assert_eq!(receipt["conflicts"], serde_json::json!(["a"]));

    let out = Command::new(binary)
        .args(["merge", "--abort", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    assert_eq!(git_output(repo.path(), &["rev-parse", "HEAD"])?, main_head);
    assert_eq!(std::fs::read(repo.path().join("a"))?, b"main\n");

    Command::new(binary)
        .args(["merge", "conflict", "--repo"])
        .arg(repo.path())
        .output()?;
    std::fs::write(repo.path().join("a"), b"resolved\n")?;
    git(repo.path(), &["add", "a"])?;
    let out = Command::new(binary)
        .args(["merge", "--continue", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["applied"], true);
    assert_eq!(receipt["in_progress"], false);
    assert_eq!(std::fs::read(repo.path().join("a"))?, b"resolved\n");

    let out = Command::new(binary)
        .args(["merge", "--skip", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("Unknown merge option"));
    Ok(())
}

#[test]
fn rebase_cli_rewrites_records_backup_and_recovers() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"base\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "base"])?;
    git(repo.path(), &["switch", "-qc", "feature"])?;
    std::fs::write(repo.path().join("b"), b"feature\n")?;
    git(repo.path(), &["add", "b"])?;
    git(repo.path(), &["commit", "-qm", "feature work"])?;
    let previous_bytes = git_output(repo.path(), &["rev-parse", "HEAD"])?;
    let previous = String::from_utf8(previous_bytes)?.trim().to_string();
    git(repo.path(), &["switch", "-q", "main"])?;
    std::fs::write(repo.path().join("c"), b"main\n")?;
    git(repo.path(), &["add", "c"])?;
    git(repo.path(), &["commit", "-qm", "main work"])?;
    let main_bytes = git_output(repo.path(), &["rev-parse", "HEAD"])?;
    let main = String::from_utf8(main_bytes.clone())?.trim().to_string();
    git(repo.path(), &["switch", "-q", "feature"])?;

    let binary = env!("CARGO_BIN_EXE_git-good");
    std::fs::write(repo.path().join("b"), b"dirty\n")?;
    let dirty = Command::new(binary)
        .args(["rebase", "main", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(!dirty.status.success());
    assert!(String::from_utf8_lossy(&dirty.stderr).contains("not clean"));

    let out = Command::new(binary)
        .args(["rebase", "main", "--autostash", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "rebase");
    assert_eq!(receipt["reference"], "main");
    assert_eq!(receipt["commit"], main);
    assert_eq!(receipt["previous"], previous);
    assert_eq!(receipt["applied"], true);
    assert_eq!(receipt["in_progress"], false);
    let backup = receipt["backup"].as_str().ok_or("missing backup")?;
    assert!(backup.starts_with("refs/gg/backup/rebase/"));
    assert_eq!(
        git_output(repo.path(), &["rev-parse", "HEAD^"])?,
        main_bytes,
        "rebase parent"
    );
    assert_eq!(std::fs::read(repo.path().join("b"))?, b"dirty\n");
    git(repo.path(), &["checkout", "--", "b"])?;

    git(repo.path(), &["switch", "-qc", "conflict"])?;
    std::fs::write(repo.path().join("a"), b"feature\n")?;
    git(repo.path(), &["commit", "-qam", "feature change"])?;
    let conflict_head = git_output(repo.path(), &["rev-parse", "HEAD"])?;
    git(repo.path(), &["switch", "-q", "main"])?;
    std::fs::write(repo.path().join("a"), b"main\n")?;
    git(repo.path(), &["commit", "-qam", "main change"])?;
    git(repo.path(), &["switch", "-q", "conflict"])?;

    let out = Command::new(binary)
        .args(["rebase", "main", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["applied"], false);
    assert_eq!(receipt["in_progress"], true);
    assert_eq!(receipt["conflicts"], serde_json::json!(["a"]));
    let backup = receipt["backup"].as_str().ok_or("missing backup")?;
    assert!(backup.starts_with("refs/gg/backup/rebase/"));

    let out = Command::new(binary)
        .args(["rebase", "--abort", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["in_progress"], false);
    assert_eq!(
        git_output(repo.path(), &["rev-parse", "HEAD"])?,
        conflict_head
    );
    assert_eq!(std::fs::read(repo.path().join("a"))?, b"feature\n");

    Command::new(binary)
        .args(["rebase", "main", "--repo"])
        .arg(repo.path())
        .output()?;
    std::fs::write(repo.path().join("a"), b"resolved\n")?;
    git(repo.path(), &["add", "a"])?;
    let out = Command::new(binary)
        .args(["rebase", "--continue", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["applied"], true);
    assert_eq!(receipt["in_progress"], false);
    assert_eq!(std::fs::read(repo.path().join("a"))?, b"resolved\n");

    let out = Command::new(binary)
        .args(["rebase", "--bad", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("Unknown rebase option"));
    Ok(())
}

#[test]
fn merge_cli_no_ff_creates_a_merge_commit() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"base\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "base"])?;
    git(repo.path(), &["switch", "-qc", "feature"])?;
    std::fs::write(repo.path().join("b"), b"feature\n")?;
    git(repo.path(), &["add", "b"])?;
    git(repo.path(), &["commit", "-qm", "feature"])?;
    git(repo.path(), &["switch", "-q", "main"])?;

    let out = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .args(["merge", "feature", "--no-ff", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["kind"], "merge_commit");
    assert_eq!(
        String::from_utf8(git_output(repo.path(), &["log", "-1", "--format=%P"])?)?
            .split_whitespace()
            .count(),
        2
    );
    Ok(())
}

#[test]
fn conflict_cli_lists_reads_and_resolves() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"base\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "base"])?;
    git(repo.path(), &["switch", "-qc", "feature"])?;
    std::fs::write(repo.path().join("a"), b"feature\n")?;
    git(repo.path(), &["commit", "-qam", "feature"])?;
    git(repo.path(), &["switch", "-q", "main"])?;
    std::fs::write(repo.path().join("a"), b"main\n")?;
    git(repo.path(), &["commit", "-qam", "main"])?;
    let _ = git_output(repo.path(), &["merge", "feature"]);

    let binary = env!("CARGO_BIN_EXE_git-good");
    let out = Command::new(binary)
        .args(["conflicts", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "conflicts");
    assert_eq!(receipt["paths"], serde_json::json!(["a"]));

    let out = Command::new(binary)
        .args(["conflict", "--file", "a", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["base"], "base\n");
    assert_eq!(receipt["ours"], "main\n");
    assert_eq!(receipt["theirs"], "feature\n");

    let out = Command::new(binary)
        .args(["conflict", "--file", "a", "--use", "theirs", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "conflict-resolve");
    assert_eq!(std::fs::read(repo.path().join("a"))?, b"feature\n");
    assert_eq!(
        git_output(repo.path(), &["diff", "--name-only", "--diff-filter=U"])?,
        b""
    );

    let out = Command::new(binary)
        .args(["conflict", "--file", "missing", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("No unmerged stages"));
    Ok(())
}

#[test]
fn reset_cli_applies_soft_mixed_and_hard() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"one\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "one"])?;
    std::fs::write(repo.path().join("a"), b"two\n")?;
    git(repo.path(), &["commit", "-qam", "two"])?;
    let first = git_output(repo.path(), &["rev-parse", "HEAD^"])?;

    let binary = env!("CARGO_BIN_EXE_git-good");
    let out = Command::new(binary)
        .args(["reset", "--soft", "HEAD^", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "reset");
    assert_eq!(receipt["mode"], "soft");
    assert_eq!(receipt["applied"], true);
    assert_eq!(git_output(repo.path(), &["rev-parse", "HEAD"])?, first);
    assert_eq!(std::fs::read(repo.path().join("a"))?, b"two\n");
    assert_eq!(
        git_output(repo.path(), &["diff", "--cached", "--name-only"])?,
        b"a\n"
    );

    let out = Command::new(binary)
        .args(["reset", "--hard", "HEAD", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    assert_eq!(git_output(repo.path(), &["status", "--porcelain"])?, b"");

    let out = Command::new(binary)
        .args(["reset", "missing", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("requires --soft"));
    Ok(())
}

#[test]
fn show_against_compares_with_an_arbitrary_base() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"one\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "one"])?;
    std::fs::write(repo.path().join("a"), b"two\n")?;
    git(repo.path(), &["commit", "-qam", "two"])?;

    let out = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .args(["show", "HEAD", "--against", "HEAD^", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert!(
        json["comparison"]["subject"]
            .as_str()
            .is_some_and(|subject| subject.starts_with("Compare "))
    );
    assert_eq!(json["files"][0]["path"], "a");

    let out = Command::new(env!("CARGO_BIN_EXE_git-good"))
        .args([
            "show",
            "HEAD",
            "--against",
            "HEAD^",
            "--parent",
            "1",
            "--repo",
        ])
        .arg(repo.path())
        .output()?;
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot be combined"));
    Ok(())
}

#[test]
fn branch_and_tag_delete_cli() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"base\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "base"])?;
    git(repo.path(), &["branch", "feature"])?;
    git(repo.path(), &["tag", "v1"])?;

    let binary = env!("CARGO_BIN_EXE_git-good");
    let out = Command::new(binary)
        .args(["branch", "--delete", "feature", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "branch-delete");
    assert_eq!(receipt["branch"], "feature");
    assert!(
        !git_output(repo.path(), &["branch", "--list", "feature"])?
            .windows(7)
            .any(|window| window == b"feature")
    );

    let out = Command::new(binary)
        .args(["branch", "--delete", "main", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("current branch"));

    let out = Command::new(binary)
        .args(["tag", "--delete", "v1", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "tag-delete");
    assert_eq!(receipt["tag"], "v1");

    let out = Command::new(binary)
        .args(["tag", "--delete", "missing", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(!out.status.success());
    Ok(())
}

#[test]
fn commit_amend_and_reflog_cli() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"base\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "base"])?;
    std::fs::write(repo.path().join("a"), b"amended\n")?;
    git(repo.path(), &["add", "a"])?;
    let previous = String::from_utf8(git_output(repo.path(), &["rev-parse", "HEAD"])?)?
        .trim()
        .to_string();

    let binary = env!("CARGO_BIN_EXE_git-good");
    let out = Command::new(binary)
        .args(["commit", "--amend", "-m", "amended subject", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "amend");
    assert_eq!(receipt["previous"], previous);
    let id = receipt["id"].as_str().ok_or("missing amended id")?;
    let backup = receipt["backup"].as_str().ok_or("missing backup")?;
    assert!(backup.starts_with("refs/gg/backup/amend/"));
    assert_eq!(
        String::from_utf8(git_output(repo.path(), &["rev-parse", backup])?)?.trim(),
        previous
    );
    assert_eq!(
        git_output(repo.path(), &["show", "-s", "--format=%s", id])?,
        b"amended subject\n"
    );

    let out = Command::new(binary)
        .args(["reflog", "HEAD", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(out.status.success());
    let reflog: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(reflog["operation"], "reflog");
    assert_eq!(reflog["reference"], "HEAD");
    assert_eq!(reflog["entries"][0]["id"], id);
    assert_eq!(reflog["entries"][0]["selector"], "HEAD@{0}");
    assert_eq!(reflog["entries"][0]["action"], "commit (amend)");

    let out = Command::new(binary)
        .args(["reflog", "missing", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(!out.status.success());
    Ok(())
}

#[test]
fn branch_create_at_cli_does_not_switch() -> Result {
    let repo = tempfile::tempdir()?;
    init(repo.path(), &["-b", "main"])?;
    git(repo.path(), &["config", "user.name", "Test"])?;
    git(repo.path(), &["config", "user.email", "test@example.org"])?;
    std::fs::write(repo.path().join("a"), b"base\n")?;
    git(repo.path(), &["add", "a"])?;
    git(repo.path(), &["commit", "-qm", "base"])?;
    let head = String::from_utf8(git_output(repo.path(), &["rev-parse", "HEAD"])?)?
        .trim()
        .to_string();

    let binary = env!("CARGO_BIN_EXE_git-good");
    let out = Command::new(binary)
        .args(["branch", "--create", "recovered", "--at", &head, "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(receipt["operation"], "branch-create");
    assert_eq!(receipt["branch"], "recovered");
    assert_eq!(receipt["commit"], head);
    assert_eq!(
        String::from_utf8(git_output(repo.path(), &["rev-parse", "recovered"])?)?.trim(),
        head
    );
    assert_eq!(
        String::from_utf8(git_output(
            repo.path(),
            &["symbolic-ref", "--short", "HEAD"]
        )?)?
        .trim(),
        "main"
    );

    let out = Command::new(binary)
        .args(["branch", "--create", "recovered", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(!out.status.success());
    let out = Command::new(binary)
        .args(["branch", "--create", "bad", "--at", "nope", "--repo"])
        .arg(repo.path())
        .output()?;
    assert!(!out.status.success());
    Ok(())
}
