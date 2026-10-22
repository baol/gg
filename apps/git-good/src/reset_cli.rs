//! Reset the current branch to a commit.
use gg_engine::reset::Mode;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut commit = None;
    let mut mode = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--soft") if mode.is_none() => mode = Some(Mode::Soft),
            Some("--mixed") if mode.is_none() => mode = Some(Mode::Mixed),
            Some("--hard") if mode.is_none() => mode = Some(Mode::Hard),
            Some(value) if value.starts_with("--") => {
                return Err("Unknown reset option".into());
            }
            _ if commit.is_none() => {
                commit = Some(arg.into_string().map_err(|_| "Commit must be UTF-8")?);
            }
            _ => {
                return Err(
                    "Usage: git good reset --soft|--mixed|--hard COMMIT [--repo PATH]".into(),
                );
            }
        }
    }
    let commit = commit.ok_or("Reset requires a commit")?;
    let mode = mode.ok_or("Reset requires --soft, --mixed, or --hard")?;
    let receipt = gg_git::reset::reset(&path, &commit, mode)?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": "reset",
            "repository": display_repository(&path),
            "commit": receipt.commit,
            "mode": receipt.mode,
            "applied": receipt.applied,
            "complete": true,
        }),
    )?;
    println!();
    Ok(())
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
