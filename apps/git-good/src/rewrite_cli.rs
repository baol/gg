//! Rewrite a linear commit range through an interactive rebase todo.
use crate::cli_args::usage_error;
use gg_engine::{
    mutation::Operation,
    rewrite::{Action, Step},
};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub fn run(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut from = None;
    let mut autostash = false;
    let mut steps = Vec::new();
    let mut action = None;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--from") if action.is_none() => {
                from = Some(
                    args.value()
                        .ok_or_else(|| usage_error("--from needs a commit"))?
                        .into_string()
                        .map_err(|_| usage_error("Commit must be UTF-8"))?,
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
                let commit = args
                    .value()
                    .ok_or_else(|| usage_error(format!("{flag} needs a commit")))?;
                steps.push(Step::new(kind, utf8(commit, "Commit")?));
            }
            Some("--reword") if action.is_none() => {
                let commit = args
                    .value()
                    .ok_or_else(|| usage_error("--reword needs a commit and a message"))?;
                let message = args
                    .value()
                    .ok_or_else(|| usage_error("--reword needs a commit and a message"))?;
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
                return Err(usage_error("Unknown rewrite option"));
            }
            _ => {
                return Err(usage_error(USAGE));
            }
        }
    }
    if let Some(action) = action {
        let receipt = match action {
            "continue" => gg_git::mutation::continue_operation(&path, Operation::Rebase)?,
            "skip" => gg_git::mutation::skip_operation(&path, Operation::Rebase)?,
            "abort" => gg_git::mutation::abort_operation(&path, Operation::Rebase)?,
            other => return Err(usage_error(format!("Unknown action --{other}"))),
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
    let from = from.ok_or_else(|| usage_error("Rewrite requires --from COMMIT"))?;
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

const USAGE: &str = "Usage: git good --cli rewrite --from COMMIT [--autostash] STEP... [--repo PATH]\n       STEP: --pick|--edit|--squash|--fixup|--drop COMMIT, or --reword COMMIT MESSAGE\n       git good --cli rewrite --continue|--skip|--abort [--repo PATH]";

fn utf8(value: OsString, what: &str) -> Result<String, Box<dyn std::error::Error>> {
    value
        .into_string()
        .map_err(|_| usage_error(format!("{what} must be UTF-8")))
}

fn write_receipt(receipt: serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
    crate::cli_args::emit(&receipt)?;
    Ok(())
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}
