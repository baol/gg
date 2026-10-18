//! Streaming comparison JSON for terminal consumers.
use gg_engine::review::{ReviewReader, ReviewRequest, ReviewStep};
use std::{ffi::OsString, io::Write, path::PathBuf};
pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let revision = args
        .next()
        .ok_or("Usage: git good show <commit> [--repo PATH] [--parent N] [--file PATH]")?
        .into_string()
        .map_err(|_| "Commit revision must be UTF-8")?;
    let mut request = ReviewRequest {
        path: std::env::current_dir()?,
        revision,
        parent: None,
    };
    let mut file = None;
    while let Some(option) = args.next() {
        match option.to_str() {
            Some("--repo") => {
                request.path = PathBuf::from(args.next().ok_or("--repo needs a path")?)
            }
            Some("--file") => {
                file = Some(
                    args.next()
                        .ok_or("--file needs a repository-relative path")?,
                )
            }
            Some("--parent") => {
                request.parent = Some(
                    args.next()
                        .ok_or("--parent needs a number")?
                        .to_str()
                        .ok_or("Invalid parent")?
                        .parse()?,
                )
            }
            _ => {
                return Err(
                    "Usage: git good show <commit> [--repo PATH] [--parent N] [--file PATH]".into(),
                );
            }
        }
    }
    let mut reader = gg_git::review::ReviewSession::open(&request)?;
    if let Some(file) = file {
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
        let change = loop {
            match reader.step()? {
                ReviewStep::Change(change) if change.path_bytes == bytes => break change,
                ReviewStep::Complete => return Err(
                    "File is absent from this comparison; use a repository-relative changed path"
                        .into(),
                ),
                _ => {}
            }
        };
        let diff = gg_git::diff::read(&reader.comparison().path, &change, || false)?;
        let mut out = std::io::BufWriter::new(std::io::stdout().lock());
        write!(out, "{{\"comparison\":")?;
        serde_json::to_writer(&mut out, reader.comparison())?;
        write!(out, ",\"file\":")?;
        serde_json::to_writer(&mut out, &change)?;
        write!(out, ",\"diff\":")?;
        serde_json::to_writer(&mut out, &diff)?;
        writeln!(out, "}}")?;
        out.flush()?;
        return Ok(());
    }
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    write!(
        out,
        "{{\"comparison\":{},\"files\":[",
        serde_json::to_string(reader.comparison())?
    )?;
    let mut first = true;
    loop {
        match reader.step()? {
            ReviewStep::Change(change) => {
                if !first {
                    write!(out, ",")?;
                }
                serde_json::to_writer(&mut out, &change)?;
                first = false;
            }
            ReviewStep::Pending => {}
            ReviewStep::Complete => break,
        }
    }
    writeln!(out, "],\"complete\":true}}")?;
    out.flush()?;
    Ok(())
}
