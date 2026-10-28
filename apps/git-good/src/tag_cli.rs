//! Create or delete one local tag.
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut tag = None;
    let mut delete = false;
    let mut create = false;
    let mut target = None;
    let mut message = None;
    let mut sign = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--delete") if !delete => delete = true,
            Some("--create") if !create => create = true,
            Some("--at") if target.is_none() => {
                target = Some(
                    args.next()
                        .ok_or("--at needs a reference")?
                        .into_string()
                        .map_err(|_| "Tag target must be UTF-8")?,
                );
            }
            Some("-m" | "--message") if message.is_none() => {
                message = Some(
                    args.next()
                        .ok_or("Tag message is missing")?
                        .into_string()
                        .map_err(|_| "Tag message must be UTF-8")?,
                );
            }
            Some("--sign" | "-s") if sign.is_none() => sign = Some(true),
            Some("--no-sign") if sign.is_none() => sign = Some(false),
            Some(value) if value.starts_with("--") => return Err("Unknown tag option".into()),
            _ if tag.is_none() => {
                tag = Some(arg.into_string().map_err(|_| "Tag name must be UTF-8")?);
            }
            _ => return Err(usage().into()),
        }
    }
    let tag = tag.ok_or("Tag command requires a tag name")?;
    if create && delete {
        return Err(usage().into());
    }
    if create && !delete {
        let target = target.ok_or("Tag creation requires --at REF")?;
        let message = message.ok_or("Tag creation requires -m MESSAGE")?;
        let signed = sign.unwrap_or_else(|| gg_git::signature::config(&path).tag_signing);
        let receipt = gg_git::tag::create(&path, &tag, &target, &message, signed)?;
        serde_json::to_writer(
            std::io::stdout().lock(),
            &serde_json::json!({
                "operation": "tag-create",
                "repository": display_repository(&path),
                "tag": receipt.tag,
                "target": receipt.target,
                "id": receipt.id,
                "signed": receipt.signed,
                "complete": true,
            }),
        )?;
        println!();
        return Ok(());
    }
    if !delete {
        return Err(usage().into());
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

fn usage() -> &'static str {
    "Usage: git good tag --delete TAG | --create TAG --at REF -m MESSAGE [--sign|--no-sign] [--repo PATH]"
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
