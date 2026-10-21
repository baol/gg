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
#[test]
fn git_dispatch_discovers_current_and_nested_repositories() -> Result {
    let repo = tempfile::tempdir()?;
    git(repo.path(), &["init", "-b", "main"])?;
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
    git(repo.path(), &["init", "-b", "explicit"])?;
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
    git(repo.path(), &["init", "-b", "main"])?;
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
    git(repo.path(), &["init", "-b", "main"])?;
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
fn show_discovers_repo_streams_exact_files_and_rejects_invalid_parent() -> Result {
    let repo = tempfile::tempdir()?;
    git(repo.path(), &["init", "-b", "main"])?;
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
    git(repo.path(), &["init", "-b", "main"])?;
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
    git(repo.path(), &["init", "-b", "main"])?;
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
    git(repo.path(), &["init", "-b", "main"])?;
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
    git(repo.path(), &["init", "-b", "main"])?;
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
    git(p, &["init", "-b", "main"])?;
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
    git(p, &["init"])?;
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
