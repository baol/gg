//! Reset the current branch to a commit.
use crate::cli_args::usage_error;
use gg_engine::reset::Mode;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub fn run(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut commit = None;
    let mut mode = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--soft") if mode.is_none() => mode = Some(Mode::Soft),
            Some("--mixed") if mode.is_none() => mode = Some(Mode::Mixed),
            Some("--hard") if mode.is_none() => mode = Some(Mode::Hard),
            Some(value) if value.starts_with("--") => {
                return Err(usage_error("Unknown reset option"));
            }
            _ if commit.is_none() => {
                commit = Some(
                    arg.into_string()
                        .map_err(|_| usage_error("Commit must be UTF-8"))?,
                );
            }
            _ => {
                return Err(usage_error(
                    "Usage: git good --cli reset --soft|--mixed|--hard COMMIT [--repo PATH]",
                ));
            }
        }
    }
    let commit = commit.ok_or_else(|| usage_error("Reset requires a commit"))?;
    let mode = mode.ok_or_else(|| usage_error("Reset requires --soft, --mixed, or --hard"))?;
    let receipt = gg_git::reset::reset(&path, &commit, mode)?;
    crate::cli_args::emit(&serde_json::json!({
        "operation": "reset",
        "repository": display_repository(&path),
        "commit": receipt.commit,
        "mode": receipt.mode,
        "applied": receipt.applied,
        "backup": receipt.backup,
        "complete": true,
    }))?;
    Ok(())
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
