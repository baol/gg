//! Inspect an LFS pointer from history or Local Changes and fetch its object explicitly.
use crate::cli_args::usage_error;
use gg_engine::{
    review::{ChangeKind, FileChange, FileSide},
    status::{Area, Kind, StatusEntry, WorkingRequest},
};
use std::{ffi::OsString, path::Path, path::PathBuf};

enum Target {
    Change(FileChange),
    Working(WorkingRequest),
}

impl Target {
    fn scope(&self) -> &'static str {
        match self {
            Self::Change(_) => "revision",
            Self::Working(_) => "working",
        }
    }
}

pub fn status(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let (path, target) = parse(args)?;
    let details = match &target {
        Target::Change(change) => gg_git::lfs::read(&path, change)?,
        Target::Working(working) => gg_git::lfs::read_working(&path, working)?,
    };
    crate::cli_args::emit(&serde_json::json!({
        "operation": "lfs status",
        "repository": display_repository(&path),
        "scope": target.scope(),
        "lfs": details,
    }))?;
    Ok(())
}

pub fn fetch(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let (path, target) = parse(args)?;
    let (details, receipt) = match &target {
        Target::Change(change) => gg_git::lfs::fetch(&path, change)?,
        Target::Working(working) => gg_git::lfs::fetch_working(&path, working)?,
    };
    crate::cli_args::emit(&serde_json::json!({
        "operation": "lfs fetch",
        "repository": display_repository(&path),
        "scope": target.scope(),
        "lfs": details,
        "receipt": receipt,
    }))?;
    Ok(())
}

fn parse(
    args: impl Iterator<Item = OsString>,
) -> Result<(PathBuf, Target), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut repo = std::env::current_dir()?;
    let mut revision = "HEAD".to_string();
    let mut revision_set = false;
    let mut before = None;
    let mut area = None;
    let mut file = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                repo = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--revision") => {
                revision = args
                    .next()
                    .ok_or_else(|| usage_error("--revision needs a commit"))?
                    .into_string()
                    .map_err(|_| usage_error("Revision must be UTF-8"))?;
                revision_set = true;
            }
            Some("--before") => {
                before = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--before needs a commit"))?
                        .into_string()
                        .map_err(|_| usage_error("Before revision must be UTF-8"))?,
                );
            }
            Some("--area") => {
                let value = args.next().ok_or_else(|| {
                    usage_error("--area needs staged|unstaged|untracked|conflict")
                })?;
                area = Some(match value.to_str() {
                    Some("staged") => Area::Staged,
                    Some("unstaged") => Area::Unstaged,
                    Some("untracked") => Area::Untracked,
                    Some("conflict") => Area::Conflict,
                    _ => return Err(usage_error(usage())),
                });
            }
            Some(value) if value.starts_with("--") => return Err(usage_error(usage())),
            _ if file.is_none() => file = Some(arg),
            _ => return Err(usage_error(usage())),
        }
    }
    let file = file.ok_or_else(|| usage_error(usage()))?;
    let path_bytes = path_bytes(&file)?;
    let display = String::from_utf8_lossy(&path_bytes).into_owned();
    if let Some(area) = area {
        if revision_set || before.is_some() {
            return Err(usage_error(
                "--area cannot be combined with --revision or --before",
            ));
        }
        let entry = StatusEntry::new(area, Kind::Modified, &path_bytes);
        let working = WorkingRequest {
            repository: gg_git::status::Session::open(&repo)?.identity,
            entry,
        };
        return Ok((repo, Target::Working(working)));
    }
    let after = side(&repo, &format!("{revision}:{display}"))?;
    let before_ref = before.unwrap_or_else(|| format!("{revision}^"));
    let before = side(&repo, &format!("{before_ref}:{display}"))?;
    Ok((
        repo,
        Target::Change(FileChange {
            path: display,
            path_bytes,
            kind: ChangeKind::Modified,
            old: before,
            new: after,
            old_path: None,
            old_path_bytes: None,
        }),
    ))
}

fn side(repo: &Path, reference: &str) -> Result<Option<FileSide>, Box<dyn std::error::Error>> {
    let output = gg_git::process::git()
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
            .ok_or_else(|| usage_error("LFS path must be Unicode on this platform"))?
            .replace('\\', "/")
            .into_bytes())
    }
}

fn usage() -> &'static str {
    "Usage: git good --cli lfs <status|fetch> PATH [--area staged|unstaged|untracked|conflict] [--revision REF] [--before REF] [--repo PATH]"
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
