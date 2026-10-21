//! Stream working status as JSON without mutating the repository.
use std::{ffi::OsString, io::Write, path::PathBuf, sync::atomic::AtomicBool};
pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = None;
    let mut file = None;
    let mut area = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--file") => file = Some(args.next().ok_or("--file needs a path")?),
            Some("--area") => {
                area = Some(
                    match args.next().and_then(|s| s.into_string().ok()).as_deref() {
                        Some("staged") => gg_engine::status::Area::Staged,
                        Some("unstaged") => gg_engine::status::Area::Unstaged,
                        Some("untracked") => gg_engine::status::Area::Untracked,
                        Some("conflict") => gg_engine::status::Area::Conflict,
                        _ => {
                            return Err(
                                "--area needs staged, unstaged, untracked, or conflict".into()
                            );
                        }
                    },
                )
            }
            Some(value) if value.starts_with("--") => return Err("Unknown status option".into()),
            _ if path.is_none() => path = Some(PathBuf::from(arg)),
            _ => return Err("Usage: git good status [PATH] [--file PATH --area AREA]".into()),
        }
    }
    if file.is_some() != area.is_some() {
        return Err("Selected-file status requires --file and --area".into());
    }
    let path = path.unwrap_or(std::env::current_dir()?);
    let session = gg_git::status::Session::open(&path)?;
    if let (Some(file), Some(area)) = (file, area) {
        #[cfg(unix)]
        let bytes = {
            use std::os::unix::ffi::OsStrExt;
            file.as_os_str().as_bytes().to_vec()
        };
        #[cfg(not(unix))]
        let bytes = file
            .to_str()
            .ok_or("File path must be Unicode on this platform")?
            .replace('\\', "/")
            .into_bytes();
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
        serde_json::to_writer(
            std::io::stdout().lock(),
            &serde_json::json!({"comparison":request,"diff":diff}),
        )?;
        println!();
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
    session.scan(&AtomicBool::new(false), |entry| {
        (|| -> Result<(), Box<dyn std::error::Error>> {
            if !first {
                write!(out, ",")?;
            }
            first = false;
            serde_json::to_writer(&mut out, &entry)?;
            out.flush()?;
            Ok(())
        })()
        .map_err(|e| e.to_string())
    })?;
    writeln!(out, "],\"complete\":true}}")?;
    out.flush()?;
    Ok(())
}

/// Unstage one observed path, using the same status identity as the UI.
pub fn unstage(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut file = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--file") if file.is_none() => {
                file = Some(args.next().ok_or("--file needs a path")?)
            }
            _ => return Err("Usage: git good unstage --file PATH [--repo PATH]".into()),
        }
    }
    let file = file.ok_or("Unstage requires --file PATH")?;
    #[cfg(unix)]
    let bytes = {
        use std::os::unix::ffi::OsStrExt;
        file.as_os_str().as_bytes().to_vec()
    };
    #[cfg(not(unix))]
    let bytes = file
        .to_str()
        .ok_or("File path must be Unicode on this platform")?
        .replace('\\', "/")
        .into_bytes();
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
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({"operation":"unstage", "path_bytes":request.entry.path_bytes, "complete":true}),
    )?;
    println!();
    Ok(())
}
