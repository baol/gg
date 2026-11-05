//! Create or delete one local tag.
use crate::cli_args::usage_error;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub fn run(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut tag = None;
    let mut delete = false;
    let mut create = false;
    let mut target = None;
    let mut message = None;
    let mut sign = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--delete") if !delete => delete = true,
            Some("--create") if !create => create = true,
            Some("--at") if target.is_none() => {
                target = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--at needs a reference"))?
                        .into_string()
                        .map_err(|_| usage_error("Tag target must be UTF-8"))?,
                );
            }
            Some("-m" | "--message") if message.is_none() => {
                message = Some(
                    args.value()
                        .ok_or_else(|| usage_error("Tag message is missing"))?
                        .into_string()
                        .map_err(|_| usage_error("Tag message must be UTF-8"))?,
                );
            }
            Some("--sign" | "-s") if sign.is_none() => sign = Some(true),
            Some("--no-sign") if sign.is_none() => sign = Some(false),
            Some(value) if value.starts_with("--") => {
                return Err(usage_error("Unknown tag option"));
            }
            _ if tag.is_none() => {
                tag = Some(
                    arg.into_string()
                        .map_err(|_| usage_error("Tag name must be UTF-8"))?,
                );
            }
            _ => return Err(usage_error(usage())),
        }
    }
    let tag = tag.ok_or_else(|| usage_error("Tag command requires a tag name"))?;
    if create && delete {
        return Err(usage_error(usage()));
    }
    if create && !delete {
        let target = target.ok_or_else(|| usage_error("Tag creation requires --at REF"))?;
        let message = message.ok_or_else(|| usage_error("Tag creation requires -m MESSAGE"))?;
        let signed = sign.unwrap_or_else(|| gg_git::signature::config(&path).tag_signing);
        let receipt = gg_git::tag::create(&path, &tag, &target, &message, signed)?;
        crate::cli_args::emit(&serde_json::json!({
            "operation": "tag-create",
            "repository": display_repository(&path),
            "tag": receipt.tag,
            "target": receipt.target,
            "id": receipt.id,
            "signed": receipt.signed,
            "complete": true,
        }))?;
        return Ok(());
    }
    if !delete {
        return Err(usage_error(usage()));
    }
    let receipt = gg_git::tag::delete(&path, &tag)?;
    crate::cli_args::emit(&serde_json::json!({
        "operation": "tag-delete",
        "repository": display_repository(&path),
        "tag": receipt.tag,
        "complete": true,
    }))?;
    Ok(())
}

fn usage() -> &'static str {
    "Usage: git good --cli tag --delete TAG | --create TAG --at REF -m MESSAGE [--sign|--no-sign] [--repo PATH]"
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
