//! List and resolve unmerged paths.
use gg_engine::conflict::Side;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub fn list(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            _ => return Err("Usage: git good conflicts [--repo PATH]".into()),
        }
    }
    let paths = gg_git::conflict::unmerged(&path)?;
    write_receipt(serde_json::json!({
        "operation": "conflicts",
        "repository": display_repository(&path),
        "paths": paths,
        "complete": true,
    }))?;
    Ok(())
}

pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut file = None;
    let mut side = None;
    let mut stage = false;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--file") => {
                file = Some(
                    args.next()
                        .ok_or("--file needs a path")?
                        .into_string()
                        .map_err(|_| "Conflict path must be UTF-8")?,
                );
            }
            Some("--use") => {
                side = Some(
                    args.next()
                        .ok_or("--use needs ours or theirs")?
                        .into_string()
                        .map_err(|_| "Side must be UTF-8")?,
                );
            }
            Some("--stage") => stage = true,
            Some(value) if value.starts_with("--") => return Err("Unknown conflict option".into()),
            _ => {
                return Err(
                    "Usage: git good conflict --file PATH [--use ours|theirs|--stage] [--repo PATH]"
                        .into(),
                );
            }
        }
    }
    let file = file.ok_or("Conflict resolution requires --file PATH")?;
    if let Some(side) = side {
        let side = match side.as_str() {
            "ours" => Side::Ours,
            "theirs" => Side::Theirs,
            _ => return Err("--use accepts ours or theirs".into()),
        };
        gg_git::conflict::use_side(&path, &file, side)?;
        write_receipt(serde_json::json!({
            "operation": "conflict-resolve",
            "repository": display_repository(&path),
            "path": file,
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
            "path": file,
            "complete": true,
        }))?;
        return Ok(());
    }
    let content = gg_git::conflict::read(&path, &file)?;
    write_receipt(serde_json::json!({
        "operation": "conflict",
        "repository": display_repository(&path),
        "path": content.path,
        "base": content.base,
        "ours": content.ours,
        "theirs": content.theirs,
        "binary": content.binary,
        "truncated": content.truncated,
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
