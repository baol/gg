//! List, add, and remove worktrees as JSON.
use crate::cli_args::usage_error;
use gg_engine::worktree::AddRequest;
use std::{ffi::OsString, path::Path, path::PathBuf};

pub fn list(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            _ => return Err(usage_error(usage())),
        }
    }
    let snapshot = gg_git::worktree::list(&path)?;
    crate::cli_args::emit(&serde_json::json!({
        "operation": "worktree list",
        "repository": display_repository(&path),
        "worktrees": snapshot.worktrees,
    }))?;
    Ok(())
}

pub fn add(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut destination = None;
    let mut branch = None;
    let mut create = None;
    let mut at = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--branch") => {
                branch = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--branch needs a name"))?
                        .into_string()
                        .map_err(|_| usage_error("Branch name must be UTF-8"))?,
                );
            }
            Some("--create") => {
                create = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--create needs a name"))?
                        .into_string()
                        .map_err(|_| usage_error("Branch name must be UTF-8"))?,
                );
            }
            Some("--at") => {
                at = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--at needs a revision"))?
                        .into_string()
                        .map_err(|_| usage_error("Revision must be UTF-8"))?,
                );
            }
            Some(value) if value.starts_with("--") => return Err(usage_error(usage())),
            _ if destination.is_none() => destination = Some(PathBuf::from(arg)),
            _ => return Err(usage_error(usage())),
        }
    }
    let destination = destination.ok_or_else(|| usage_error(usage()))?;
    let receipt = gg_git::worktree::add(
        &path,
        &AddRequest {
            destination,
            branch,
            create,
            at,
        },
    )?;
    crate::cli_args::emit(&serde_json::json!({
        "operation": "worktree add",
        "repository": display_repository(&path),
        "path": receipt.path,
        "branch": receipt.branch,
        "head": receipt.head,
    }))?;
    Ok(())
}

pub fn remove(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut destination = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some(value) if value.starts_with("--") => return Err(usage_error(usage())),
            _ if destination.is_none() => destination = Some(PathBuf::from(arg)),
            _ => return Err(usage_error(usage())),
        }
    }
    let destination = destination.ok_or_else(|| usage_error(usage()))?;
    let receipt = gg_git::worktree::remove(&path, &destination)?;
    crate::cli_args::emit(&serde_json::json!({
        "operation": "worktree remove",
        "repository": display_repository(&path),
        "path": receipt.path,
        "branch": receipt.branch,
        "head": receipt.head,
    }))?;
    Ok(())
}

fn usage() -> &'static str {
    "Usage: git good --cli worktree <list|add|remove> ...\ngit good --cli worktree list [--repo PATH]\ngit good --cli worktree add DEST --branch BRANCH [--repo PATH]\ngit good --cli worktree add DEST --create BRANCH [--at REF] [--repo PATH]\ngit good --cli worktree remove PATH [--repo PATH]"
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
