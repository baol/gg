//! Cherry-pick, revert, and their continue/skip/abort recovery.
use crate::cli_args::usage_error;
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
    args: impl Iterator<Item = OsString>,
    operation: Operation,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut commits = Vec::new();
    let mut action = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--continue") if action.is_none() => action = Some("continue"),
            Some("--abort") if action.is_none() => action = Some("abort"),
            Some("--skip") if action.is_none() => action = Some("skip"),
            Some(value) if value.starts_with("--") => {
                return Err(usage_error(format!(
                    "Unknown {} option",
                    operation.as_str()
                )));
            }
            _ if commits.is_empty() || operation == Operation::CherryPick => {
                commits.push(
                    arg.into_string()
                        .map_err(|_| usage_error("Commit must be UTF-8"))?,
                );
            }
            _ => {
                return Err(usage_error(format!(
                    "Usage: git good --cli {} COMMIT [--repo PATH]\n       git good --cli {} --continue|--abort|--skip [--repo PATH]",
                    operation.as_str(),
                    operation.as_str()
                )));
            }
        }
    }
    let receipt = match action {
        Some("continue") => gg_git::mutation::continue_operation(&path, operation)?,
        Some("abort") => gg_git::mutation::abort_operation(&path, operation)?,
        Some("skip") => gg_git::mutation::skip_operation(&path, operation)?,
        _ => {
            if commits.is_empty() {
                return Err(usage_error(format!(
                    "Usage: git good --cli {} COMMIT [--repo PATH]",
                    operation.as_str()
                )));
            }
            let named: Vec<&str> = commits.iter().map(String::as_str).collect();
            match operation {
                Operation::CherryPick => gg_git::mutation::cherry_pick_many(&path, &named)?,
                Operation::Revert => gg_git::mutation::revert(&path, named[0])?,
                Operation::Merge => {
                    return Err(usage_error(
                        "Use git good --cli merge for merge operations.",
                    ));
                }
                Operation::Rebase => {
                    return Err(usage_error(
                        "Use git good --cli rebase for rebase operations.",
                    ));
                }
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
    crate::cli_args::emit(&receipt)?;
    Ok(())
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
