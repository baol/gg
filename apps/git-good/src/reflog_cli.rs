//! Read reflog entries as JSON.
use std::{ffi::OsString, path::Path, path::PathBuf};

pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut reference = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some(value) if value.starts_with("--") => {
                return Err("Unknown reflog option".into());
            }
            _ if reference.is_none() => {
                reference = Some(
                    arg.into_string()
                        .map_err(|_| "Reflog reference must be UTF-8")?,
                );
            }
            _ => return Err("Usage: git good reflog [REF] [--repo PATH]".into()),
        }
    }
    let reference = reference.unwrap_or_else(|| "HEAD".into());
    let snapshot = gg_git::reflog::read(&path, &reference)?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": "reflog",
            "repository": display_repository(&path),
            "reference": snapshot.reference,
            "entries": snapshot.entries,
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
