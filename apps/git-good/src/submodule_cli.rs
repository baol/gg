//! Inspect or update a gitlink submodule as JSON.
use std::{ffi::OsString, path::Path, path::PathBuf};

pub fn status(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut recorded = None;
    let mut before = None;
    let mut submodule = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--recorded") => {
                recorded = Some(
                    args.next()
                        .ok_or("--recorded needs a commit")?
                        .into_string()
                        .map_err(|_| "Recorded commit must be UTF-8")?,
                );
            }
            Some("--before") => {
                before = Some(
                    args.next()
                        .ok_or("--before needs a commit")?
                        .into_string()
                        .map_err(|_| "Before commit must be UTF-8")?,
                );
            }
            Some(value) if value.starts_with("--") => return Err(usage().into()),
            _ if submodule.is_none() => submodule = Some(arg),
            _ => return Err(usage().into()),
        }
    }
    let submodule = submodule.ok_or_else(|| usage().to_string())?;
    let bytes = path_bytes(&submodule)?;
    let details = gg_git::submodule::read(&path, &bytes, recorded.as_deref(), before.as_deref())?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": "submodule status",
            "repository": display_repository(&path),
            "submodule": details,
        }),
    )?;
    println!();
    Ok(())
}

pub fn update(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut submodule = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some(value) if value.starts_with("--") => return Err(usage().into()),
            _ if submodule.is_none() => submodule = Some(arg),
            _ => return Err(usage().into()),
        }
    }
    let submodule = submodule.ok_or_else(|| usage().to_string())?;
    let bytes = path_bytes(&submodule)?;
    let (details, receipt) = gg_git::submodule::update(&path, &bytes)?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": "submodule update",
            "repository": display_repository(&path),
            "submodule": details,
            "receipt": receipt,
        }),
    )?;
    println!();
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
            .ok_or("Submodule path must be Unicode on this platform")?
            .replace('\\', "/")
            .into_bytes())
    }
}

fn usage() -> &'static str {
    "Usage: git good submodule <status|update> PATH [--repo PATH]\ngit good submodule status PATH [--recorded COMMIT] [--before COMMIT] [--repo PATH]\ngit good submodule update PATH [--repo PATH]"
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
