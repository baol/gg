//! Stash listing and mutations with JSON receipts.
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let command = args
        .next()
        .ok_or("Usage: git good stash <list|push|apply|pop|drop> [OPTIONS]")?
        .into_string()
        .map_err(|_| "Stash command must be UTF-8")?;
    match command.as_str() {
        "list" => list(args),
        "push" => push(args),
        "apply" => restore(args, false),
        "pop" => restore(args, true),
        "drop" => drop(args),
        _ => Err("Usage: git good stash <list|push|apply|pop|drop> [OPTIONS]".into()),
    }
}

fn list(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            _ => return Err("Usage: git good stash list [--repo PATH]".into()),
        }
    }
    let stashes = gg_git::stash::list(&path)?;
    write_receipt(serde_json::json!({
        "operation": "stash-list",
        "repository": display_repository(&path),
        "stashes": stashes.entries,
        "complete": true,
    }))?;
    Ok(())
}

fn push(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut message = None;
    let mut include_untracked = false;
    let mut keep_index = false;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("-m" | "--message") if message.is_none() => {
                message = Some(
                    args.next()
                        .ok_or("Stash message is missing")?
                        .into_string()
                        .map_err(|_| "Stash message must be UTF-8")?,
                );
            }
            Some("--include-untracked") if !include_untracked => include_untracked = true,
            Some("--keep-index") if !keep_index => keep_index = true,
            _ => {
                return Err(
                    "Usage: git good stash push [-m MESSAGE] [--include-untracked] [--keep-index] [--repo PATH]"
                        .into(),
                );
            }
        }
    }
    let receipt = gg_git::stash::push(&path, message.as_deref(), include_untracked, keep_index)?;
    write_receipt(serde_json::json!({
        "operation": "stash-push",
        "repository": display_repository(&path),
        "reference": receipt.reference,
        "message": receipt.message,
        "created": receipt.created,
        "complete": true,
    }))?;
    Ok(())
}

fn restore(
    mut args: impl Iterator<Item = OsString>,
    remove: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut reference = None;
    let mut restore_index = false;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--index") if !restore_index => restore_index = true,
            Some(value) if value.starts_with("--") => {
                return Err("Unknown stash option.".into());
            }
            _ if reference.is_none() => {
                reference = Some(
                    arg.into_string()
                        .map_err(|_| "Stash reference must be UTF-8")?,
                );
            }
            _ => {
                return Err(
                    "Usage: git good stash apply|pop [STASH] [--index] [--repo PATH]".into(),
                );
            }
        }
    }
    let receipt = if remove {
        gg_git::stash::pop(&path, reference.as_deref(), restore_index)?
    } else {
        gg_git::stash::apply(&path, reference.as_deref(), restore_index)?
    };
    write_receipt(serde_json::json!({
        "operation": if remove { "stash-pop" } else { "stash-apply" },
        "repository": display_repository(&path),
        "reference": receipt.reference,
        "applied": receipt.applied,
        "dropped": receipt.dropped,
        "conflicts": receipt.conflicts,
        "warnings": receipt.warnings,
        "complete": true,
    }))?;
    Ok(())
}

fn drop(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut reference = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some(value) if value.starts_with("--") => return Err("Unknown stash option.".into()),
            _ if reference.is_none() => {
                reference = Some(
                    arg.into_string()
                        .map_err(|_| "Stash reference must be UTF-8")?,
                );
            }
            _ => return Err("Usage: git good stash drop [STASH] [--repo PATH]".into()),
        }
    }
    let receipt = gg_git::stash::drop(&path, reference.as_deref())?;
    write_receipt(serde_json::json!({
        "operation": "stash-drop",
        "repository": display_repository(&path),
        "reference": receipt.reference,
        "dropped": receipt.dropped,
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
