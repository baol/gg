//! List and resolve unmerged paths.
use crate::cli_args::usage_error;
use gg_engine::{conflict::Side, review::display_path};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

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
            _ => return Err(usage_error("Usage: git good --cli conflicts [--repo PATH]")),
        }
    }
    let paths = gg_git::conflict::unmerged(&path)?;
    let shown = paths
        .iter()
        .map(|file| display_path(file))
        .collect::<Vec<_>>();
    write_receipt(serde_json::json!({
        "operation": "conflicts",
        "repository": display_repository(&path),
        "paths": shown,
        "path_bytes": paths,
        "complete": true,
    }))?;
    Ok(())
}

pub fn run(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut file = None;
    let mut side = None;
    let mut stage = false;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--file") => {
                file = Some(path_bytes(
                    &args
                        .value()
                        .ok_or_else(|| usage_error("--file needs a path"))?,
                )?);
            }
            Some("--use") => {
                side = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--use needs ours, theirs, or deletion"))?
                        .into_string()
                        .map_err(|_| usage_error("Side must be UTF-8"))?,
                );
            }
            Some("--stage") => stage = true,
            Some(value) if value.starts_with("--") => {
                return Err(usage_error("Unknown conflict option"));
            }
            _ => {
                return Err(usage_error(
                    "Usage: git good --cli conflict --file PATH [--use ours|theirs|deletion|--stage] [--repo PATH]",
                ));
            }
        }
    }
    let file = file.ok_or_else(|| usage_error("Conflict resolution requires --file PATH"))?;
    if side.as_deref() == Some("deletion") {
        gg_git::conflict::keep_deletion(&path, &file)?;
        write_receipt(serde_json::json!({
            "operation": "conflict-resolve",
            "repository": display_repository(&path),
            "path": display_path(&file),
            "path_bytes": file,
            "side": "deletion",
            "complete": true,
        }))?;
        return Ok(());
    }
    if let Some(side) = side {
        let side = match side.as_str() {
            "ours" => Side::Ours,
            "theirs" => Side::Theirs,
            _ => return Err(usage_error("--use accepts ours, theirs, or deletion")),
        };
        gg_git::conflict::use_side(&path, &file, side)?;
        write_receipt(serde_json::json!({
            "operation": "conflict-resolve",
            "repository": display_repository(&path),
            "path": display_path(&file),
            "path_bytes": file,
            "side": side,
            "complete": true,
        }))?;
        return Ok(());
    }
    if stage {
        gg_git::conflict::stage(&path, &file)?;
        write_receipt(serde_json::json!({
            "operation": "conflict-stage",
            "repository": display_repository(&path),
            "path": display_path(&file),
            "path_bytes": file,
            "complete": true,
        }))?;
        return Ok(());
    }
    let content = gg_git::conflict::read(&path, &file)?;
    write_receipt(serde_json::json!({
        "operation": "conflict",
        "repository": display_repository(&path),
        "path": content.path,
        "path_bytes": content.path_bytes,
        "base": content.base,
        "ours": content.ours,
        "theirs": content.theirs,
        "binary": content.binary,
        "truncated": content.truncated,
        "complete": true,
    }))?;
    Ok(())
}

/// The exact bytes of a path argument: raw on Unix, Unicode elsewhere.
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
            .ok_or_else(|| usage_error("Conflict path must be Unicode on this platform"))?
            .replace('\\', "/")
            .into_bytes())
    }
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
