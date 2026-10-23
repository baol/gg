//! Create or amend a commit from the index through the shared commit engine.
use std::{ffi::OsString, path::PathBuf};

pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut message = None;
    let mut amend = false;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--amend") if !amend => amend = true,
            Some("-m" | "--message") if message.is_none() => {
                message = Some(
                    args.next()
                        .ok_or("Commit message is missing")?
                        .into_string()
                        .map_err(|_| "Commit message must be UTF-8")?,
                );
            }
            _ => {
                return Err("Usage: git good commit [--amend] -m MESSAGE [--repo PATH]".into());
            }
        }
    }
    let message = message.ok_or("Commit requires -m MESSAGE")?;
    let session = gg_git::status::Session::open(&path)?;
    let receipt = if amend {
        gg_git::commit::amend(&session.identity, &message)?
    } else {
        gg_git::commit::create(&session.identity, &message)?
    };
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "operation": if amend { "amend" } else { "commit" },
            "id": receipt.id,
            "tree": receipt.tree,
            "parents": receipt.parents,
            "previous": receipt.previous,
            "backup": receipt.backup,
            "warnings": receipt.warnings,
            "complete": true,
        }),
    )?;
    println!();
    Ok(())
}
