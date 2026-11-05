//! Switch or create one local branch after clean-worktree validation.
use crate::cli_args::usage_error;
use std::{ffi::OsString, path::PathBuf};

pub fn run(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut create = false;
    let mut stash = false;
    let mut include_untracked = false;
    let mut branch = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--create") if !create => create = true,
            Some("--stash") if !stash => stash = true,
            Some("--include-untracked") if !include_untracked => include_untracked = true,
            Some(value) if value.starts_with("--") => {
                return Err(usage_error("Unknown branch option"));
            }
            _ if branch.is_none() => {
                branch = Some(
                    arg.into_string()
                        .map_err(|_| usage_error("Branch name must be UTF-8"))?,
                );
            }
            _ => {
                return Err(usage_error(
                    "Usage: git good --cli switch [--create] [--stash [--include-untracked]] BRANCH [--repo PATH]",
                ));
            }
        }
    }
    let branch = branch.ok_or_else(|| usage_error("Branch switch requires a branch name"))?;
    if stash && create {
        return Err(usage_error(
            "--stash is only valid when switching to an existing branch",
        ));
    }
    if include_untracked && !stash {
        return Err(usage_error("--include-untracked requires --stash"));
    }
    let receipt = if create {
        gg_git::branch::create(&path, &branch)?
    } else if stash {
        gg_git::branch::switch_with_stash(&path, &branch, include_untracked)?
    } else {
        gg_git::branch::switch(&path, &branch)?
    };
    crate::cli_args::emit(&serde_json::json!({
        "operation": if receipt.created { "create" } else { "switch" },
        "branch": receipt.branch,
        "head": receipt.head,
        "warnings": receipt.warnings,
        "stash": receipt.stash,
        "complete": true,
    }))?;
    Ok(())
}

pub fn manage(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut branch = None;
    let mut delete = false;
    let mut create = false;
    let mut force = false;
    let mut start = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--delete") if !delete => delete = true,
            Some("--create") if !create => create = true,
            Some("--force") if !force => force = true,
            Some("--at") if start.is_none() => {
                start = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--at needs a revision"))?
                        .into_string()
                        .map_err(|_| usage_error("Branch start point must be UTF-8"))?,
                );
            }
            Some(value) if value.starts_with("--") => {
                return Err(usage_error("Unknown branch option"));
            }
            _ if branch.is_none() => {
                branch = Some(
                    arg.into_string()
                        .map_err(|_| usage_error("Branch name must be UTF-8"))?,
                );
            }
            _ => {
                return Err(usage_error(
                    "Usage: git good --cli branch --delete BRANCH [--force] [--repo PATH]\n       git good --cli branch --create NAME [--at REF] [--repo PATH]",
                ));
            }
        }
    }
    let branch = branch.ok_or_else(|| usage_error("Branch operation requires a branch name"))?;
    if delete && create {
        return Err(usage_error("Choose either --delete or --create"));
    }
    if force && !delete {
        return Err(usage_error("--force is only valid with --delete"));
    }
    if start.is_some() && !create {
        return Err(usage_error("--at is only valid with --create"));
    }
    if delete {
        let receipt = gg_git::branch::delete(&path, &branch, force)?;
        crate::cli_args::emit(&serde_json::json!({
            "operation": "branch-delete",
            "branch": receipt.branch,
            "force": receipt.force,
            "complete": true,
        }))?;
    } else if create {
        let start = start.as_deref().unwrap_or("HEAD");
        let receipt = gg_git::branch::create_at(&path, &branch, start)?;
        crate::cli_args::emit(&serde_json::json!({
            "operation": "branch-create",
            "branch": receipt.branch,
            "commit": receipt.commit,
            "complete": true,
        }))?;
    } else {
        return Err(usage_error(
            "Usage: git good --cli branch --delete BRANCH [--force] [--repo PATH]\n       git good --cli branch --create NAME [--at REF] [--repo PATH]",
        ));
    }
    Ok(())
}
