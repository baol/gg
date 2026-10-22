//! Merge a reference and recover from conflicts.
use gg_engine::mutation::Operation;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut reference = None;
    let mut no_ff = false;
    let mut action = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--no-ff") if !no_ff => no_ff = true,
            Some("--continue") if action.is_none() => action = Some("continue"),
            Some("--abort") if action.is_none() => action = Some("abort"),
            Some(value) if value.starts_with("--") => {
                return Err("Unknown merge option".into());
            }
            _ if reference.is_none() => {
                reference = Some(arg.into_string().map_err(|_| "Reference must be UTF-8")?);
            }
            _ => {
                return Err(
                    "Usage: git good merge REF [--no-ff] [--repo PATH]\n       git good merge --continue|--abort [--repo PATH]"
                        .into(),
                );
            }
        }
    }
    if let Some(action) = action {
        let receipt = match action {
            "continue" => gg_git::mutation::continue_operation(&path, Operation::Merge)?,
            "abort" => gg_git::mutation::abort_operation(&path, Operation::Merge)?,
            _ => unreachable!(),
        };
        write_receipt(serde_json::json!({
            "operation": "merge",
            "repository": display_repository(&path),
            "reference": receipt.commit,
            "applied": receipt.applied,
            "in_progress": receipt.in_progress,
            "conflicts": receipt.conflicts,
            "warnings": receipt.warnings,
            "complete": true,
        }))?;
        return Ok(());
    }
    let reference = reference.ok_or("Merge requires a reference")?;
    let receipt = gg_git::merge::merge(&path, &reference, no_ff)?;
    write_receipt(serde_json::json!({
        "operation": "merge",
        "repository": display_repository(&path),
        "reference": receipt.reference,
        "commit": receipt.commit,
        "kind": receipt.kind,
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
