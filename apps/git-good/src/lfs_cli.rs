//! Inspect an LFS pointer and fetch its object explicitly.
use gg_engine::review::{ChangeKind, FileChange, FileSide};
use std::{ffi::OsString, path::Path, path::PathBuf, process::Command};

pub fn status(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let (path, change) = parse(args)?;
    let details = gg_git::lfs::read(&path, &change)?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": "lfs status",
            "repository": display_repository(&path),
            "lfs": details,
        }),
    )?;
    println!();
    Ok(())
}

pub fn fetch(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let (path, change) = parse(args)?;
    let (details, receipt) = gg_git::lfs::fetch(&path, &change)?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": "lfs fetch",
            "repository": display_repository(&path),
            "lfs": details,
            "receipt": receipt,
        }),
    )?;
    println!();
    Ok(())
}

fn parse(
    mut args: impl Iterator<Item = OsString>,
) -> Result<(PathBuf, FileChange), Box<dyn std::error::Error>> {
    let mut repo = std::env::current_dir()?;
    let mut revision = "HEAD".to_string();
    let mut before = None;
    let mut file = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => repo = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--revision") => {
                revision = args
                    .next()
                    .ok_or("--revision needs a commit")?
                    .into_string()
                    .map_err(|_| "Revision must be UTF-8")?;
            }
            Some("--before") => {
                before = Some(
                    args.next()
                        .ok_or("--before needs a commit")?
                        .into_string()
                        .map_err(|_| "Before revision must be UTF-8")?,
                );
            }
            Some(value) if value.starts_with("--") => return Err(usage().into()),
            _ if file.is_none() => file = Some(arg),
            _ => return Err(usage().into()),
        }
    }
    let file = file.ok_or_else(|| usage().to_string())?;
    let path_bytes = path_bytes(&file)?;
    let display = String::from_utf8_lossy(&path_bytes).into_owned();
    let after = side(&repo, &format!("{revision}:{display}"))?;
    let before_ref = before.unwrap_or_else(|| format!("{revision}^"));
    let before = side(&repo, &format!("{before_ref}:{display}"))?;
    Ok((
        repo,
        FileChange {
            path: display,
            path_bytes,
            kind: ChangeKind::Modified,
            old: before,
            new: after,
        },
    ))
}

fn side(repo: &Path, reference: &str) -> Result<Option<FileSide>, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", reference])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(FileSide {
        id: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        mode: 0o100644,
    }))
}

fn path_bytes(path: &std::ffi::OsStr) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok(path.as_bytes().to_vec())
    }
    #[cfg(not(unix))]
    {
        Ok(path
            .to_str()
            .ok_or("LFS path must be Unicode on this platform")?
            .replace('\\', "/")
            .into_bytes())
    }
}

fn usage() -> &'static str {
    "Usage: git good lfs <status|fetch> PATH [--revision REF] [--before REF] [--repo PATH]"
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
