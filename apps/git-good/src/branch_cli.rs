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
