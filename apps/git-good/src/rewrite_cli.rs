//! Reorder and squash a linear commit range through an interactive rebase todo.
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
            Some("--pick") if action.is_none() => {
                steps.push(step(
                    Action::Pick,
                    args.next().ok_or("--pick needs a commit")?,
                )?);
            }
            Some("--squash") if action.is_none() => {
                steps.push(step(
                    Action::Squash,
                    args.next().ok_or("--squash needs a commit")?,
                )?);
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
                return Err(
                    "Usage: git good rewrite --from COMMIT [--autostash] [--pick COMMIT]... [--squash COMMIT]... [--repo PATH]\n       git good rewrite --continue|--skip|--abort [--repo PATH]"
                        .into(),
                );
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

fn step(action: Action, commit: OsString) -> Result<Step, Box<dyn std::error::Error>> {
    Ok(Step {
        action,
        commit: commit.into_string().map_err(|_| "Commit must be UTF-8")?,
    })
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
