//! Streaming comparison JSON for terminal consumers.
use crate::cli_args::usage_error;
use gg_engine::review::{ReviewReader, ReviewRequest, ReviewStep};
use std::{ffi::OsString, io::Write, path::PathBuf};
pub fn run(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let revision = args
        .next()
        .ok_or_else(|| usage_error("Usage: git good --cli show <commit> [--repo PATH] [--parent N] [--file PATH] [--export before|after --output PATH]"))?
        .into_string()
        .map_err(|_| usage_error("Commit revision must be UTF-8"))?;
    let mut request = ReviewRequest {
        path: std::env::current_dir()?,
        revision,
        parent: None,
        against: None,
    };
    let mut file = None;
    let mut export_side = None;
    let mut output = None;
    while let Some(option) = args.next() {
        match args.option(&option) {
            Some("--export") => {
                export_side = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--export needs before or after"))?,
                );
            }
            Some("--output") => {
                output =
                    Some(PathBuf::from(args.value().ok_or_else(|| {
                        usage_error("--output needs a destination path")
                    })?));
            }
            Some("--repo") => {
                request.path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--file") => {
                file = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--file needs a repository-relative path"))?,
                )
            }
            Some("--parent") => {
                request.parent = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--parent needs a number"))?
                        .to_str()
                        .ok_or_else(|| usage_error("Invalid parent"))?
                        .parse()?,
                )
            }
            Some("--against") => {
                request.against = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--against needs a revision"))?
                        .into_string()
                        .map_err(|_| usage_error("Revision must be UTF-8"))?,
                )
            }
            _ => {
                return Err(usage_error(
                    "Usage: git good --cli show <commit> [--repo PATH] [--parent N] [--against REF] [--file PATH] [--export before|after --output PATH]",
                ));
            }
        }
    }
    if request.against.is_some() && request.parent.is_some() {
        return Err(usage_error("--against cannot be combined with --parent"));
    }
    if export_side.is_some() != output.is_some() || (export_side.is_some() && file.is_none()) {
        return Err(usage_error(
            "Export requires --file PATH --export before|after --output PATH",
        ));
    }
    if export_side
        .as_ref()
        .is_some_and(|s| s != "before" && s != "after")
    {
        return Err(usage_error("--export needs before or after"));
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
            .ok_or_else(|| usage_error("File path must be Unicode on this platform"))?
            .replace('\\', "/")
            .into_bytes();
        let change = loop {
            match reader.step()? {
                ReviewStep::Change(change) if change.path_bytes == bytes => break change,
                ReviewStep::Complete => {
                    return Err("File is absent from this comparison; use a repository-relative changed path".into());
                }
                _ => {}
            }
        };
        if let (Some(which), Some(destination)) = (export_side, output) {
            let side = if which == "before" {
                &change.old
            } else {
                &change.new
            };
            let side = side.as_ref().ok_or("Selected side is absent")?;
            let size =
                crate::export::save(&reader.comparison().path, side, &destination, || false)?;
            crate::cli_args::emit(&serde_json::json!({
                "comparison": reader.comparison(), "file": change, "side": which.to_string_lossy(),
                "output": destination.to_string_lossy(), "bytes": size
            }))?;
            return Ok(());
        }
        let diff = gg_git::diff::read(&reader.comparison().path, &change, || false)?;
        let mut out = std::io::BufWriter::new(std::io::stdout().lock());
        write!(out, "{{\"comparison\":")?;
        serde_json::to_writer(&mut out, reader.comparison())?;
        write!(out, ",\"file\":")?;
        serde_json::to_writer(&mut out, &change)?;
        write!(out, ",\"diff\":")?;
        serde_json::to_writer(&mut out, &diff)?;
        writeln!(out, ",\"status\":\"completed\"}}")?;
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
    let read = loop {
        match reader.step() {
            Ok(ReviewStep::Change(change)) => {
                if !first {
                    write!(out, ",")?;
                }
                serde_json::to_writer(&mut out, &change)?;
                first = false;
            }
            Ok(ReviewStep::Pending) => {}
            Ok(ReviewStep::Complete) => break Ok(()),
            Err(error) => break Err(error.into()),
        }
    };
    write!(out, "]")?;
    crate::cli_args::end_stream(&mut out, read)
}
