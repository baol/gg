//! Read bounded line attribution as JSON.
use crate::cli_args::usage_error;
use std::{ffi::OsString, path::Path, path::PathBuf};

pub fn run(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut revision = "HEAD".to_string();
    let mut lines = None;
    let mut file = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--revision") => {
                revision = args
                    .next()
                    .ok_or_else(|| usage_error("--revision needs a commit"))?
                    .into_string()
                    .map_err(|_| usage_error("Blame revision must be UTF-8"))?;
            }
            Some("--lines") => {
                let value = args
                    .next()
                    .ok_or_else(|| usage_error("--lines needs START,END"))?
                    .into_string()
                    .map_err(|_| usage_error("Line range must be UTF-8"))?;
                let (start, end) = value
                    .split_once(',')
                    .ok_or_else(|| usage_error("--lines needs START,END"))?;
                let start = start.parse::<usize>()?;
                let end = end.parse::<usize>()?;
                if start == 0 || end == 0 || start > end {
                    return Err(usage_error("--lines needs a positive START,END range"));
                }
                lines = Some((start, end));
            }
            Some(value) if value.starts_with("--") => {
                return Err(usage_error("Unknown blame option"));
            }
            _ if file.is_none() => file = Some(arg),
            _ => {
                return Err(usage_error(
                    "Usage: git good --cli blame PATH [--revision REF] [--lines START,END] [--repo PATH]",
                ));
            }
        }
    }
    let file = file.ok_or(
        "Usage: git good --cli blame PATH [--revision REF] [--lines START,END] [--repo PATH]",
    )?;
    let bytes = path_bytes(&file)?;
    let blame = gg_git::blame::read(&path, &revision, &bytes, lines, &|| false)?;
    crate::cli_args::emit(&serde_json::json!({
        "operation": "blame",
        "repository": display_repository(&path),
        "revision": blame.revision,
        "path": blame.path,
        "start_line": blame.start_line,
        "end_line": blame.end_line,
        "lines": blame.lines,
        "complete": blame.complete,
        "note": blame.note,
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
            .ok_or_else(|| usage_error("File path must be Unicode on this platform"))?
            .replace('\\', "/")
            .into_bytes())
    }
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
