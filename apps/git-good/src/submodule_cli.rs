//! Inspect or update a gitlink submodule as JSON.
use crate::cli_args::usage_error;
use std::{ffi::OsString, path::Path, path::PathBuf};

pub fn status(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut recorded = None;
    let mut before = None;
    let mut submodule = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--recorded") => {
                recorded = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--recorded needs a commit"))?
                        .into_string()
                        .map_err(|_| usage_error("Recorded commit must be UTF-8"))?,
                );
            }
            Some("--before") => {
                before = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--before needs a commit"))?
                        .into_string()
                        .map_err(|_| usage_error("Before commit must be UTF-8"))?,
                );
            }
            Some(value) if value.starts_with("--") => return Err(usage_error(usage())),
            _ if submodule.is_none() => submodule = Some(arg),
            _ => return Err(usage_error(usage())),
        }
    }
    let submodule = submodule.ok_or_else(|| usage_error(usage()))?;
    let bytes = path_bytes(&submodule)?;
    let details = gg_git::submodule::read(&path, &bytes, recorded.as_deref(), before.as_deref())?;
    crate::cli_args::emit(&serde_json::json!({
        "operation": "submodule status",
        "repository": display_repository(&path),
        "submodule": details,
    }))?;
    Ok(())
}

pub fn update(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut submodule = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some(value) if value.starts_with("--") => return Err(usage_error(usage())),
            _ if submodule.is_none() => submodule = Some(arg),
            _ => return Err(usage_error(usage())),
        }
    }
    let submodule = submodule.ok_or_else(|| usage_error(usage()))?;
    let bytes = path_bytes(&submodule)?;
    let (details, receipt) = gg_git::submodule::update(&path, &bytes)?;
    crate::cli_args::emit(&serde_json::json!({
        "operation": "submodule update",
        "repository": display_repository(&path),
        "submodule": details,
        "receipt": receipt,
    }))?;
    Ok(())
}

fn path_bytes(path: &std::ffi::OsStr) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok(path.as_bytes().to_vec())
    }
    #[cfg(not(unix))]
    {
        Ok(path
            .to_str()
            .ok_or_else(|| usage_error("Submodule path must be Unicode on this platform"))?
            .replace('\\', "/")
            .into_bytes())
    }
}

fn usage() -> &'static str {
    "Usage: git good --cli submodule <status|update> PATH [--repo PATH]\ngit good --cli submodule status PATH [--recorded COMMIT] [--before COMMIT] [--repo PATH]\ngit good --cli submodule update PATH [--repo PATH]"
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
