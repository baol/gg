//! `git good --cli ignore FILE [--extension|--folder] [--repo PATH]`: add the
//! pattern that ignores an untracked FILE (or its extension, or its
//! folder) to the repository's root `.gitignore`.
use crate::cli_args::usage_error;
use gg_git::ignore::Scope;
use std::{ffi::OsString, path::PathBuf};

const USAGE: &str = "Usage: git good --cli ignore FILE [--extension|--folder] [--repo PATH]";

pub fn run(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut file = None;
    let mut scope = Scope::File;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--extension") => scope = Scope::Extension,
            Some("--folder") => scope = Scope::Folder,
            Some(value) if value.starts_with("--") => return Err(usage_error(USAGE)),
            _ if file.is_none() => {
                file = Some(
                    arg.into_string()
                        .map_err(|_| usage_error("FILE must be UTF-8"))?,
                )
            }
            _ => return Err(usage_error(USAGE)),
        }
    }
    let file = file.ok_or_else(|| usage_error(USAGE))?;
    let relative = file.replace('\\', "/");
    let pattern = gg_git::ignore::pattern(relative.as_bytes(), scope)?.ok_or(match scope {
        Scope::Extension => "The file has no extension to ignore.",
        _ => "The file is at the repository root; ignore the file itself.",
    })?;
    let receipt = gg_git::ignore::add(&path, &pattern)?;
    crate::cli_args::emit(&serde_json::json!({
        "operation": "ignore",
        "pattern": receipt.pattern,
        "file": receipt.file,
        "added": receipt.added,
        "complete": true,
    }))?;
    Ok(())
}
