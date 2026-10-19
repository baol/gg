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
    for _ in 0..2 {
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
