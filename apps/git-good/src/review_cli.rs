//! Streaming comparison JSON for terminal consumers.
use gg_engine::review::{ReviewReader, ReviewRequest, ReviewStep};
use std::{ffi::OsString, io::Write, path::PathBuf};
pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let revision = args
        .next()
        .ok_or("Usage: git good show <commit> [--repo PATH] [--parent N]")?
        .into_string()
        .map_err(|_| "Commit revision must be UTF-8")?;
    let mut request = ReviewRequest {
        path: std::env::current_dir()?,
        revision,
        parent: None,
    };
    while let Some(option) = args.next() {
        match option.to_str() {
            Some("--repo") => {
                request.path = PathBuf::from(args.next().ok_or("--repo needs a path")?)
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
            _ => return Err("Usage: git good show <commit> [--repo PATH] [--parent N]".into()),
        }
    }
    let mut reader = gg_git::review::ReviewSession::open(&request)?;
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
