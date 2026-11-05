//! Verify signatures and read signing configuration as JSON.
use crate::cli_args::usage_error;
use std::{ffi::OsString, path::Path, path::PathBuf};

pub fn config(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
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
            _ => return Err(usage_error(usage())),
        }
    }
    let config = gg_git::signature::config(&path);
    crate::cli_args::emit(&serde_json::json!({
        "operation": "signature config",
        "repository": display_repository(&path),
        "config": config,
    }))?;
    Ok(())
}

pub fn verify(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut reference = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some(value) if value.starts_with("--") => return Err(usage_error(usage())),
            _ if reference.is_none() => {
                reference = Some(
                    arg.into_string()
                        .map_err(|_| usage_error("Signature reference must be UTF-8"))?,
                );
            }
            _ => return Err(usage_error(usage())),
        }
    }
    let reference = reference.ok_or_else(|| usage_error(usage()))?;
    let verification = gg_git::signature::verify(&path, &reference)?;
    crate::cli_args::emit(&serde_json::json!({
        "operation": "signature verify",
        "repository": display_repository(&path),
        "verification": verification,
    }))?;
    Ok(())
}

fn usage() -> &'static str {
    "Usage: git good --cli signature <config|verify> [REF] [--repo PATH]"
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
