//! Read reflog entries as JSON.
use crate::cli_args::usage_error;
use std::{ffi::OsString, path::Path, path::PathBuf};

pub fn run(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
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
            Some(value) if value.starts_with("--") => {
                return Err(usage_error("Unknown reflog option"));
            }
            _ if reference.is_none() => {
                reference = Some(
                    arg.into_string()
                        .map_err(|_| usage_error("Reflog reference must be UTF-8"))?,
                );
            }
            _ => {
                return Err(usage_error(
                    "Usage: git good --cli reflog [REF] [--repo PATH]",
                ));
            }
        }
    }
    let reference = reference.unwrap_or_else(|| "HEAD".into());
    let snapshot = gg_git::reflog::read(&path, &reference)?;
    crate::cli_args::emit(&serde_json::json!({
        "operation": "reflog",
        "repository": display_repository(&path),
        "reference": snapshot.reference,
        "entries": snapshot.entries,
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
