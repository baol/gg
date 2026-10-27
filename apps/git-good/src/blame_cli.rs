//! Read bounded line attribution as JSON.
use std::{ffi::OsString, path::Path, path::PathBuf};

pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut revision = "HEAD".to_string();
    let mut lines = None;
    let mut file = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--revision") => {
                revision = args
                    .next()
                    .ok_or("--revision needs a commit")?
                    .into_string()
                    .map_err(|_| "Blame revision must be UTF-8")?;
            }
            Some("--lines") => {
                let value = args
                    .next()
                    .ok_or("--lines needs START,END")?
                    .into_string()
                    .map_err(|_| "Line range must be UTF-8")?;
                let (start, end) = value.split_once(',').ok_or("--lines needs START,END")?;
                let start = start.parse::<usize>()?;
                let end = end.parse::<usize>()?;
                if start == 0 || end == 0 || start > end {
                    return Err("--lines needs a positive START,END range".into());
                }
                lines = Some((start, end));
            }
            Some(value) if value.starts_with("--") => {
                return Err("Unknown blame option".into());
            }
            _ if file.is_none() => file = Some(arg),
            _ => {
                return Err(
                    "Usage: git good blame PATH [--revision REF] [--lines START,END] [--repo PATH]"
                        .into(),
                );
            }
        }
    }
    let file = file
        .ok_or("Usage: git good blame PATH [--revision REF] [--lines START,END] [--repo PATH]")?;
    let bytes = path_bytes(&file)?;
    let blame = gg_git::blame::read(&path, &revision, &bytes, lines, &|| false)?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": "blame",
            "repository": display_repository(&path),
            "revision": blame.revision,
            "path": blame.path,
            "start_line": blame.start_line,
            "end_line": blame.end_line,
            "lines": blame.lines,
            "complete": blame.complete,
            "note": blame.note,
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
            .ok_or("File path must be Unicode on this platform")?
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
