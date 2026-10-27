//! List, add, and remove worktrees as JSON.
use gg_engine::worktree::AddRequest;
use std::{ffi::OsString, path::Path, path::PathBuf};

pub fn list(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            _ => return Err(usage().into()),
        }
    }
    let snapshot = gg_git::worktree::list(&path)?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": "worktree list",
            "repository": display_repository(&path),
            "worktrees": snapshot.worktrees,
        }),
    )?;
    println!();
    Ok(())
}

pub fn add(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut destination = None;
    let mut branch = None;
    let mut create = None;
    let mut at = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--branch") => {
                branch = Some(
                    args.next()
                        .ok_or("--branch needs a name")?
                        .into_string()
                        .map_err(|_| "Branch name must be UTF-8")?,
                );
            }
            Some("--create") => {
                create = Some(
                    args.next()
                        .ok_or("--create needs a name")?
                        .into_string()
                        .map_err(|_| "Branch name must be UTF-8")?,
                );
            }
            Some("--at") => {
                at = Some(
                    args.next()
                        .ok_or("--at needs a revision")?
                        .into_string()
                        .map_err(|_| "Revision must be UTF-8")?,
                );
            }
            Some(value) if value.starts_with("--") => return Err(usage().into()),
            _ if destination.is_none() => destination = Some(PathBuf::from(arg)),
            _ => return Err(usage().into()),
        }
    }
    let destination = destination.ok_or_else(|| usage().to_string())?;
    let receipt = gg_git::worktree::add(
        &path,
        &AddRequest {
            destination,
            branch,
            create,
            at,
        },
    )?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": "worktree add",
            "repository": display_repository(&path),
            "path": receipt.path,
            "branch": receipt.branch,
            "head": receipt.head,
        }),
    )?;
    println!();
    Ok(())
}

pub fn remove(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut destination = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some(value) if value.starts_with("--") => return Err(usage().into()),
            _ if destination.is_none() => destination = Some(PathBuf::from(arg)),
            _ => return Err(usage().into()),
        }
    }
    let destination = destination.ok_or_else(|| usage().to_string())?;
    let receipt = gg_git::worktree::remove(&path, &destination)?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": "worktree remove",
            "repository": display_repository(&path),
            "path": receipt.path,
            "branch": receipt.branch,
            "head": receipt.head,
        }),
    )?;
    println!();
    Ok(())
}

fn usage() -> &'static str {
    "Usage: git good worktree <list|add|remove> ...\ngit good worktree list [--repo PATH]\ngit good worktree add DEST --branch BRANCH [--repo PATH]\ngit good worktree add DEST --create BRANCH [--at REF] [--repo PATH]\ngit good worktree remove PATH [--repo PATH]"
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
