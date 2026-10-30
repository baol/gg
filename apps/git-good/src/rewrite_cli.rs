//! Rewrite a linear commit range through an interactive rebase todo.
use gg_engine::{
    mutation::Operation,
    rewrite::{Action, Step},
};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;
    let mut from = None;
    let mut autostash = false;
    let mut steps = Vec::new();
    let mut action = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--from") if action.is_none() => {
                from = Some(
                    args.next()
                        .ok_or("--from needs a commit")?
                        .into_string()
                        .map_err(|_| "Commit must be UTF-8")?,
                );
            }
            Some(flag @ ("--pick" | "--edit" | "--squash" | "--fixup" | "--drop"))
                if action.is_none() =>
            {
                let kind = match flag {
                    "--pick" => Action::Pick,
                    "--edit" => Action::Edit,
                    "--squash" => Action::Squash,
                    "--fixup" => Action::Fixup,
                    _ => Action::Drop,
                };
                let commit = args.next().ok_or(format!("{flag} needs a commit"))?;
                steps.push(Step::new(kind, utf8(commit, "Commit")?));
            }
            Some("--reword") if action.is_none() => {
                let commit = args.next().ok_or("--reword needs a commit and a message")?;
                let message = args.next().ok_or("--reword needs a commit and a message")?;
                steps.push(Step::reword(
                    utf8(commit, "Commit")?,
                    utf8(message, "Message")?,
                ));
            }
            Some("--autostash") if action.is_none() => autostash = true,
            Some("--no-autostash") if action.is_none() => autostash = false,
            Some("--continue") if action.is_none() => action = Some("continue"),
            Some("--skip") if action.is_none() => action = Some("skip"),
            Some("--abort") if action.is_none() => action = Some("abort"),
            Some(value) if value.starts_with("--") => {
                return Err("Unknown rewrite option".into());
            }
            _ => {
                return Err(USAGE.into());
            }
        }
    }
    if let Some(action) = action {
        let receipt = match action {
            "continue" => gg_git::mutation::continue_operation(&path, Operation::Rebase)?,
            "skip" => gg_git::mutation::skip_operation(&path, Operation::Rebase)?,
            "abort" => gg_git::mutation::abort_operation(&path, Operation::Rebase)?,
            _ => unreachable!(),
        };
        write_receipt(serde_json::json!({
            "operation": "rewrite",
            "repository": display_repository(&path),
            "action": action,
            "from": serde_json::Value::Null,
            "previous": serde_json::Value::Null,
            "backup": serde_json::Value::Null,
            "applied": receipt.applied,
            "in_progress": receipt.in_progress,
            "conflicts": receipt.conflicts,
            "warnings": receipt.warnings,
            "steps": 0,
            "complete": true,
        }))?;
        return Ok(());
    }
    let from = from.ok_or("Rewrite requires --from COMMIT")?;
    let receipt = gg_git::rewrite::rewrite(&path, &from, &steps, autostash)?;
    write_receipt(serde_json::json!({
        "operation": "rewrite",
        "repository": display_repository(&path),
        "from": receipt.from,
        "previous": receipt.previous,
        "backup": receipt.backup,
        "applied": receipt.applied,
        "in_progress": receipt.in_progress,
        "conflicts": receipt.conflicts,
        "warnings": receipt.warnings,
        "steps": receipt.steps,
        "complete": true,
    }))?;
    Ok(())
}

const USAGE: &str = "Usage: git good rewrite --from COMMIT [--autostash] STEP... [--repo PATH]\n       STEP: --pick|--edit|--squash|--fixup|--drop COMMIT, or --reword COMMIT MESSAGE\n       git good rewrite --continue|--skip|--abort [--repo PATH]";

fn utf8(value: OsString, what: &str) -> Result<String, Box<dyn std::error::Error>> {
    value
        .into_string()
        .map_err(|_| format!("{what} must be UTF-8").into())
}

fn write_receipt(receipt: serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer(std::io::stdout().lock(), &receipt)?;
    println!();
    Ok(())
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
