//! Print read-only branch, remote, tag, and HEAD labels as JSON.
use std::{ffi::OsString, path::PathBuf};

pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let path = match args.next() {
        Some(path) => PathBuf::from(path),
        None => std::env::current_dir()?,
    };
    if args.next().is_some() {
        return Err("Usage: git good refs [PATH]".into());
    }
    let snapshot = gg_git::refs::read(&path)?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({"path": path, "refs": snapshot}),
    )?;
    println!();
    Ok(())
}
