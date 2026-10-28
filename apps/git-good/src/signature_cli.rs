//! Verify signatures and read signing configuration as JSON.
use std::{ffi::OsString, path::Path, path::PathBuf};

pub fn config(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            _ => return Err(usage().into()),
        }
    }
    let config = gg_git::signature::config(&path);
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": "signature config",
            "repository": display_repository(&path),
            "config": config,
        }),
    )?;
    println!();
    Ok(())
}

pub fn verify(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut reference = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some(value) if value.starts_with("--") => return Err(usage().into()),
            _ if reference.is_none() => {
                reference = Some(
                    arg.into_string()
                        .map_err(|_| "Signature reference must be UTF-8")?,
                );
            }
            _ => return Err(usage().into()),
        }
    }
    let reference = reference.ok_or_else(|| usage().to_string())?;
    let verification = gg_git::signature::verify(&path, &reference)?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": "signature verify",
            "repository": display_repository(&path),
            "verification": verification,
        }),
    )?;
    println!();
    Ok(())
}

fn usage() -> &'static str {
    "Usage: git good signature <config|verify> [REF] [--repo PATH]"
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
