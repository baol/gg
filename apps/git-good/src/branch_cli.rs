//! Switch or create one local branch after clean-worktree validation.
use std::{ffi::OsString, path::PathBuf};

pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut create = false;
    let mut branch = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--create") if !create => create = true,
            Some(value) if value.starts_with("--") => {
                return Err("Unknown branch option".into());
            }
            _ if branch.is_none() => {
                branch = Some(arg.into_string().map_err(|_| "Branch name must be UTF-8")?);
            }
            _ => {
                return Err("Usage: git good switch [--create] BRANCH [--repo PATH]".into());
            }
        }
    }
    let branch = branch.ok_or("Branch switch requires a branch name")?;
    let receipt = if create {
        gg_git::branch::create(&path, &branch)?
    } else {
        gg_git::branch::switch(&path, &branch)?
    };
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": if receipt.created { "create" } else { "switch" },
            "branch": receipt.branch,
            "head": receipt.head,
            "warnings": receipt.warnings,
            "complete": true,
        }),
    )?;
    println!();
    Ok(())
}

pub fn manage(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut branch = None;
    let mut delete = false;
    let mut create = false;
    let mut force = false;
    let mut start = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--delete") if !delete => delete = true,
            Some("--create") if !create => create = true,
            Some("--force") if !force => force = true,
            Some("--at") if start.is_none() => {
                start = Some(
                    args.next()
                        .ok_or("--at needs a revision")?
                        .into_string()
                        .map_err(|_| "Branch start point must be UTF-8")?,
                );
            }
            Some(value) if value.starts_with("--") => {
                return Err("Unknown branch option".into());
            }
            _ if branch.is_none() => {
                branch = Some(arg.into_string().map_err(|_| "Branch name must be UTF-8")?);
            }
            _ => {
                return Err("Usage: git good branch --delete BRANCH [--force] [--repo PATH]\n       git good branch --create NAME [--at REF] [--repo PATH]".into());
            }
        }
    }
    let branch = branch.ok_or("Branch operation requires a branch name")?;
    if delete && create {
        return Err("Choose either --delete or --create".into());
    }
    if force && !delete {
        return Err("--force is only valid with --delete".into());
    }
    if start.is_some() && !create {
        return Err("--at is only valid with --create".into());
    }
    if delete {
        let receipt = gg_git::branch::delete(&path, &branch, force)?;
        serde_json::to_writer(
            std::io::stdout().lock(),
            &serde_json::json!({
                "operation": "branch-delete",
                "branch": receipt.branch,
                "force": receipt.force,
                "complete": true,
            }),
        )?;
    } else if create {
        let start = start.as_deref().unwrap_or("HEAD");
        let receipt = gg_git::branch::create_at(&path, &branch, start)?;
        serde_json::to_writer(
            std::io::stdout().lock(),
            &serde_json::json!({
                "operation": "branch-create",
                "branch": receipt.branch,
                "commit": receipt.commit,
                "complete": true,
            }),
        )?;
    } else {
        return Err(
            "Usage: git good branch --delete BRANCH [--force] [--repo PATH]\n       git good branch --create NAME [--at REF] [--repo PATH]"
                .into(),
        );
    }
    println!();
    Ok(())
}
