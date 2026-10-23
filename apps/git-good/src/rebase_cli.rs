//! Rebase the current branch and recover from conflicts.
use gg_engine::mutation::Operation;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut reference = None;
    let mut autostash = false;
    let mut action = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--autostash") if action.is_none() => autostash = true,
            Some("--no-autostash") if action.is_none() => autostash = false,
            Some("--continue") if action.is_none() => action = Some("continue"),
            Some("--skip") if action.is_none() => action = Some("skip"),
            Some("--abort") if action.is_none() => action = Some("abort"),
            Some(value) if value.starts_with("--") => {
                return Err("Unknown rebase option".into());
            }
            _ if reference.is_none() => {
                reference = Some(arg.into_string().map_err(|_| "Reference must be UTF-8")?);
            }
            _ => {
                return Err(
                    "Usage: git good rebase REF [--autostash] [--repo PATH]\n       git good rebase --continue|--skip|--abort [--repo PATH]"
                        .into(),
                );
            }
        }
    }
    if let Some(action) = action {
        let receipt = match action {
            "continue" => gg_git::mutation::continue_operation(&path, Operation::Rebase)?,
            "skip" => gg_git::mutation::skip_operation(&path, Operation::Rebase)?,
            "abort" => gg_git::mutation::abort_operation(&path, Operation::Rebase)?,
            _ => unreachable!(),
        };
        write_receipt(serde_json::json!({
            "operation": "rebase",
            "repository": display_repository(&path),
            "action": action,
            "reference": serde_json::Value::Null,
            "commit": serde_json::Value::Null,
            "previous": serde_json::Value::Null,
            "backup": serde_json::Value::Null,
            "applied": receipt.applied,
            "in_progress": receipt.in_progress,
            "conflicts": receipt.conflicts,
            "warnings": receipt.warnings,
            "complete": true,
        }))?;
        return Ok(());
    }
    let reference = reference.ok_or("Rebase requires a reference")?;
    let receipt = gg_git::rebase::rebase(&path, &reference, autostash)?;
    write_receipt(serde_json::json!({
        "operation": "rebase",
        "repository": display_repository(&path),
        "reference": receipt.reference,
        "commit": receipt.commit,
        "previous": receipt.previous,
        "backup": receipt.backup,
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
