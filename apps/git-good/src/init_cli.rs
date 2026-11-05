//! `git good --cli init [--branch NAME] [--commit] [PATH]`.
use crate::cli_args::usage_error;
use std::{ffi::OsString, path::PathBuf};

pub fn run(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut branch = "main".to_string();
    let mut commit = false;
    let mut path = None;
    let mut args = crate::cli_args::Args::new(args);
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--branch") => {
                branch = args
                    .next()
                    .and_then(|value| value.into_string().ok())
                    .ok_or_else(|| usage_error("--branch requires a branch name"))?;
            }
            Some("--commit") => commit = true,
            Some(value) if value.starts_with("--") => {
                return Err(usage_error(
                    "Usage: git good --cli init [--branch NAME] [--commit] [PATH]",
                ));
            }
            _ if path.is_none() => path = Some(PathBuf::from(arg)),
            _ => {
                return Err(usage_error(
                    "Usage: git good --cli init [--branch NAME] [--commit] [PATH]",
                ));
            }
        }
    }
    let path = std::path::absolute(match path {
        Some(path) => path,
        None => std::env::current_dir()?,
    })?;
    let receipt = gg_git::init::init(&path, &branch, commit)?;
    crate::cli_args::emit(&serde_json::json!({
        "operation": "init",
        "path": receipt.path,
        "branch": receipt.branch,
        "head": receipt.head,
        "files": receipt.files,
        "complete": true,
    }))?;
    Ok(())
}
