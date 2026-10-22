//! Cherry-pick, revert, and their continue/skip/abort recovery.
use gg_engine::mutation::{MutationReceipt, Operation, OperationState};
use std::{
    path::Path,
    process::{Command, Output, Stdio},
};

pub fn state(path: &Path) -> Result<Option<OperationState>, String> {
    let conflicts = unmerged(path)?;
    if let Some(commit) = head(path, "CHERRY_PICK_HEAD")? {
        return Ok(Some(OperationState {
            operation: Operation::CherryPick,
            commit: Some(commit),
            conflicts,
        }));
    }
    if let Some(commit) = head(path, "REVERT_HEAD")? {
        return Ok(Some(OperationState {
            operation: Operation::Revert,
            commit: Some(commit),
            conflicts,
        }));
    }
    if let Some(commit) = head(path, "MERGE_HEAD")? {
        return Ok(Some(OperationState {
            operation: Operation::Merge,
            commit: Some(commit),
            conflicts,
        }));
    }
    Ok(None)
}

pub fn cherry_pick(path: &Path, commit: &str) -> Result<MutationReceipt, String> {
    start(path, Operation::CherryPick, commit)
}

pub fn revert(path: &Path, commit: &str) -> Result<MutationReceipt, String> {
    start(path, Operation::Revert, commit)
}

pub fn continue_operation(path: &Path, operation: Operation) -> Result<MutationReceipt, String> {
    finish(path, operation, "continue", "Continue")
}

pub fn abort_operation(path: &Path, operation: Operation) -> Result<MutationReceipt, String> {
    finish(path, operation, "abort", "Abort")
}

pub fn skip_operation(path: &Path, operation: Operation) -> Result<MutationReceipt, String> {
    if operation == Operation::Merge {
        return Err(
            "Merge does not support skip. Resolve conflicts and continue, or abort.".into(),
        );
    }
    finish(path, operation, "skip", "Skip")
}

fn start(path: &Path, operation: Operation, commit: &str) -> Result<MutationReceipt, String> {
    let commit = commit.trim();
    if commit.is_empty() || commit.starts_with('-') {
        return Err("Commit must be a revision that does not start with '-'.".into());
    }
    let resolved = resolve(path, commit)?;
    let flag = if operation == Operation::CherryPick {
        "cherry-pick"
    } else {
        "revert"
    };
    let output = run(
        path,
        &[flag.into(), "--no-edit".into(), resolved.clone()],
        false,
    )?;
    let conflicts = unmerged(path)?;
    let state = state(path)?;
    if !output.status.success() && conflicts.is_empty() && state.is_none() {
        return Err(failure(
            &format!("{} {resolved}", label(operation)),
            &output,
        ));
    }
    let in_progress = state.is_some() || !conflicts.is_empty();
    let mut warnings = Vec::new();
    if !conflicts.is_empty() {
        warnings.push("Resolve the listed conflicts, then continue or abort the operation.".into());
    }
    Ok(MutationReceipt {
        operation,
        commit: resolved,
        applied: output.status.success() && !in_progress,
        in_progress,
        conflicts,
        warnings,
    })
}

fn finish(
    path: &Path,
    operation: Operation,
    command: &str,
    label: &str,
) -> Result<MutationReceipt, String> {
    let output = run(
        path,
        &[operation.as_str().into(), format!("--{command}")],
        true,
    )?;
    let conflicts = unmerged(path)?;
    let state = state(path)?;
    if !output.status.success() && conflicts.is_empty() && state.is_none() && command != "abort" {
        return Err(failure(&format!("{label} {}", operation.as_str()), &output));
    }
    if command == "abort" && !output.status.success() && state.is_some() {
        return Err(failure(&format!("{label} {}", operation.as_str()), &output));
    }
    let in_progress = state.is_some() || !conflicts.is_empty();
    let mut warnings = Vec::new();
    if !conflicts.is_empty() {
        warnings.push("Resolve the listed conflicts, then continue or abort the operation.".into());
    }
    Ok(MutationReceipt {
        operation,
        commit: String::new(),
        applied: output.status.success() && !in_progress,
        in_progress,
        conflicts,
        warnings,
    })
}

fn resolve(path: &Path, commit: &str) -> Result<String, String> {
    let output = run(
        path,
        &[
            "rev-parse".into(),
            "--verify".into(),
            "--quiet".into(),
            format!("{commit}^{{commit}}"),
        ],
        false,
    )?;
    if !output.status.success() {
        return Err(format!("Commit {commit} was not found."));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn head(path: &Path, name: &str) -> Result<Option<String>, String> {
    let output = run(
        path,
        &[
            "rev-parse".into(),
            "--verify".into(),
            "--quiet".into(),
            name.into(),
        ],
        false,
    )?;
    if output.status.success() {
        Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ))
    } else {
        Ok(None)
    }
}

fn unmerged(path: &Path) -> Result<Vec<String>, String> {
    let output = run(
        path,
        &[
            "diff".into(),
            "--name-only".into(),
            "-z".into(),
            "--diff-filter=U".into(),
        ],
        false,
    )?;
    if !output.status.success() {
        return Err(failure("Read conflicts", &output));
    }
    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .map(|field| String::from_utf8_lossy(field).into_owned())
        .collect())
}

fn label(operation: Operation) -> &'static str {
    match operation {
        Operation::CherryPick => "Cherry-pick",
        Operation::Revert => "Revert",
        Operation::Merge => "Merge",
    }
}

fn run(path: &Path, args: &[String], editor: bool) -> Result<Output, String> {
    let mut command = Command::new("git");
    command
        .current_dir(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    if editor {
        command
            .env("GIT_EDITOR", "true")
            .env("GIT_SEQUENCE_EDITOR", "true");
    }
    command
        .output()
        .map_err(|error| format!("Could not run Git. Install Git and retry: {error}"))
}

fn combined(output: &Output) -> String {
    let mut detail = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        if !detail.is_empty() && !detail.ends_with('\n') {
            detail.push('\n');
        }
        detail.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    detail.trim().to_string()
}

fn failure(label: &str, output: &Output) -> String {
    let detail = combined(output);
    if detail.is_empty() {
        format!("{label} failed ({}).", output.status)
    } else {
        format!("{label} failed ({}).\n{detail}", output.status)
    }
}
