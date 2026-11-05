//! Stream working status as JSON without mutating the repository.
use crate::cli_args::usage_error;
use std::{ffi::OsString, io::Write, path::PathBuf, sync::atomic::AtomicBool};
pub fn run(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = None;
    let mut file = None;
    let mut area = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--file") => {
                file = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--file needs a path"))?,
                )
            }
            Some("--area") => {
                area = Some(
                    match args.value().and_then(|s| s.into_string().ok()).as_deref() {
                        Some("staged") => gg_engine::status::Area::Staged,
                        Some("unstaged") => gg_engine::status::Area::Unstaged,
                        Some("untracked") => gg_engine::status::Area::Untracked,
                        Some("conflict") => gg_engine::status::Area::Conflict,
                        _ => {
                            return Err(usage_error(
                                "--area needs staged, unstaged, untracked, or conflict",
                            ));
                        }
                    },
                )
            }
            Some("--repo") if path.is_none() => {
                path = Some(PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                ));
            }
            _ => {
                return Err(usage_error(
                    "Usage: git good --cli status [--repo PATH] [--file PATH --area AREA]",
                ));
            }
        }
    }
    if file.is_some() != area.is_some() {
        return Err(usage_error(
            "Selected-file status requires --file and --area",
        ));
    }
    let path = path.unwrap_or(std::env::current_dir()?);
    let session = gg_git::status::Session::open(&path)?;
    if let (Some(file), Some(area)) = (file, area) {
        let bytes = path_bytes(&file)?;
        let stop = AtomicBool::new(false);
        let mut selected = None;
        session.scan(&stop, |entry| {
            if entry.path_bytes == bytes && entry.area == area {
                selected = Some(entry);
                stop.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            Ok(())
        })?;
        let request = gg_engine::status::WorkingRequest {
            repository: session.identity,
            entry: selected.ok_or("Path is absent from the selected status area")?,
        };
        let diff = gg_git::working_diff::read(&request, || false)?;
        crate::cli_args::emit(&serde_json::json!({"comparison":request,"diff":diff}))?;
        return Ok(());
    }
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    write!(
        out,
        "{{\"repository\":{},\"entries\":[",
        serde_json::to_string(&session.identity)?
    )?;
    out.flush()?;
    let mut first = true;
    let mut write_error = None;
    let scanned = session.scan(&AtomicBool::new(false), |entry| {
        (|| -> Result<(), Box<dyn std::error::Error>> {
            if !first {
                write!(out, ",")?;
            }
            first = false;
            serde_json::to_writer(&mut out, &entry)?;
            out.flush()?;
            Ok(())
        })()
        .map_err(|error| {
            let text = error.to_string();
            write_error = Some(error);
            text
        })
    });
    if let Some(error) = write_error {
        return Err(error);
    }
    write!(out, "]")?;
    crate::cli_args::end_stream(&mut out, scanned.map_err(Into::into))
}

/// Unstage one observed path, using the same status identity as the UI.
pub fn unstage(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut file = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--file") if file.is_none() => {
                file = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--file needs a path"))?,
                )
            }
            _ => {
                return Err(usage_error(
                    "Usage: git good --cli unstage --file PATH [--repo PATH]",
                ));
            }
        }
    }
    let file = file.ok_or_else(|| usage_error("Unstage requires --file PATH"))?;
    let bytes = path_bytes(&file)?;
    let session = gg_git::status::Session::open(&path)?;
    let mut selected = None;
    session.scan(&AtomicBool::new(false), |entry| {
        if entry.area == gg_engine::status::Area::Staged && entry.path_bytes == bytes {
            selected = Some(entry);
        }
        Ok(())
    })?;
    let request = gg_engine::status::WorkingRequest {
        repository: session.identity,
        entry: selected.ok_or("Path is absent from staged status")?,
    };
    gg_git::unstage::file(&request)?;
    crate::cli_args::emit(
        &serde_json::json!({"operation":"unstage", "path_bytes":request.entry.path_bytes, "complete":true}),
    )?;
    Ok(())
}

/// Stage one observed working path, using the same status identity as the UI.
pub fn stage(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut file = None;
    let mut hunks = vec![];
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--file") if file.is_none() => {
                file = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--file needs a path"))?,
                )
            }
            Some("--hunk") => hunks.push(
                args.value()
                    .ok_or_else(|| usage_error("--hunk needs an @@ header"))?
                    .into_string()
                    .map_err(|_| usage_error("Hunk header must be Unicode"))?,
            ),
            _ => {
                return Err(usage_error(
                    "Usage: git good --cli stage --file PATH [--hunk HEADER]... [--repo PATH]",
                ));
            }
        }
    }
    let file = file.ok_or_else(|| usage_error("Stage requires --file PATH"))?;
    let bytes = path_bytes(&file)?;
    let session = gg_git::status::Session::open(&path)?;
    let mut selected = None;
    let mut already_staged = false;
    let mut conflicted = false;
    session.scan(&AtomicBool::new(false), |entry| {
        if entry.path_bytes == bytes {
            match entry.area {
                gg_engine::status::Area::Unstaged | gg_engine::status::Area::Untracked
                    if selected.is_none() =>
                {
                    selected = Some(entry)
                }
                gg_engine::status::Area::Staged => already_staged = true,
                gg_engine::status::Area::Conflict => conflicted = true,
                _ => {}
            }
        }
        Ok(())
    })?;
    let entry = selected.ok_or(if already_staged {
        "Path is already staged"
    } else if conflicted {
        "Resolve this path's conflict before staging"
    } else {
        "Path is absent from unstaged or untracked status"
    })?;
    let request = gg_engine::status::WorkingRequest {
        repository: session.identity,
        entry,
    };
    if hunks.is_empty() {
        gg_git::stage::file(&request)?;
    } else {
        gg_git::stage::hunks(&request, &hunks)?;
    }
    let mut receipt = serde_json::json!({"operation":"stage", "path_bytes":request.entry.path_bytes, "complete":true});
    if !hunks.is_empty() {
        receipt["hunks"] = hunks.len().into();
    }
    crate::cli_args::emit(&receipt)?;
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
            .ok_or_else(|| usage_error("File path must be Unicode on this platform"))?
            .replace('\\', "/")
            .into_bytes())
    }
}
