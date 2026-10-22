//! Cherry-pick, revert, and their continue/skip/abort recovery.
use gg_engine::mutation::Operation;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub fn cherry_pick(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    run(args, Operation::CherryPick)
}

pub fn revert(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    run(args, Operation::Revert)
}

fn run(
    mut args: impl Iterator<Item = OsString>,
    operation: Operation,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut commit = None;
    let mut action = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--continue") if action.is_none() => action = Some("continue"),
            Some("--abort") if action.is_none() => action = Some("abort"),
            Some("--skip") if action.is_none() => action = Some("skip"),
            Some(value) if value.starts_with("--") => {
                return Err(format!("Unknown {} option", operation.as_str()).into());
            }
            _ if commit.is_none() => {
                commit = Some(arg.into_string().map_err(|_| "Commit must be UTF-8")?);
            }
            _ => {
                return Err(format!(
                    "Usage: git good {} COMMIT [--repo PATH]\n       git good {} --continue|--abort|--skip [--repo PATH]",
                    operation.as_str(),
                    operation.as_str()
                )
                .into());
            }
        }
    }
    let receipt = match action {
        Some("continue") => gg_git::mutation::continue_operation(&path, operation)?,
        Some("abort") => gg_git::mutation::abort_operation(&path, operation)?,
        Some("skip") => gg_git::mutation::skip_operation(&path, operation)?,
        _ => {
            let commit = commit.ok_or_else(|| {
                format!(
                    "Usage: git good {} COMMIT [--repo PATH]",
                    operation.as_str()
                )
            })?;
            match operation {
                Operation::CherryPick => gg_git::mutation::cherry_pick(&path, &commit)?,
                Operation::Revert => gg_git::mutation::revert(&path, &commit)?,
            }
        }
    };
    write_receipt(serde_json::json!({
        "operation": operation.as_str(),
        "repository": display_repository(&path),
        "commit": receipt.commit,
        "applied": receipt.applied,
        "in_progress": receipt.in_progress,
        "conflicts": receipt.conflicts,
        "warnings": receipt.warnings,
        "complete": true,
    }))?;
    Ok(())
}

fn write_receipt(receipt: serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer(std::io::stdout().lock(), &receipt)?;
    println!();
    Ok(())
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
