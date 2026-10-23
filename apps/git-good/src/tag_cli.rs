//! Delete one local tag.
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut tag = None;
    let mut delete = false;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--delete") if !delete => delete = true,
            Some(value) if value.starts_with("--") => return Err("Unknown tag option".into()),
            _ if tag.is_none() => {
                tag = Some(arg.into_string().map_err(|_| "Tag name must be UTF-8")?);
            }
            _ => return Err("Usage: git good tag --delete TAG [--repo PATH]".into()),
        }
    }
    let tag = tag.ok_or("Tag deletion requires a tag name")?;
    if !delete {
        return Err("Usage: git good tag --delete TAG [--repo PATH]".into());
    }
    let receipt = gg_git::tag::delete(&path, &tag)?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": "tag-delete",
            "repository": display_repository(&path),
            "tag": receipt.tag,
            "complete": true,
        }),
    )?;
    println!();
    Ok(())
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
