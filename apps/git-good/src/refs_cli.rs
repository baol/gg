//! Print read-only branch, remote, tag, and HEAD labels as JSON.
use std::ffi::OsString;

pub fn run(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let path = crate::cli_args::repo_only(args, "Usage: git good --cli refs [--repo PATH]")?;
    let snapshot = gg_git::refs::read(&path)?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({"path": path, "refs": snapshot}),
    )?;
    println!();
    Ok(())
}
