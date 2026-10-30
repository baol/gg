//! Merge a local, remote-tracking, or tag reference into the current branch.
use gg_engine::merge::{Kind, Receipt};
use std::{
    path::Path,
    process::{Command, Output, Stdio},
};

pub fn merge(path: &Path, reference: &str, no_ff: bool) -> Result<Receipt, String> {
    merge_with(path, reference, if no_ff { "--no-ff" } else { "" })
}

/// Move the current branch to `reference` only when that is a fast-forward;
/// otherwise refuse and change nothing (Pull with the fast-forward-only
/// strategy).
pub fn fast_forward(path: &Path, reference: &str) -> Result<Receipt, String> {
    merge_with(path, reference, "--ff-only")
}

fn merge_with(path: &Path, reference: &str, mode: &str) -> Result<Receipt, String> {
    let no_ff = mode == "--no-ff";
    let reference = reference.trim();
    if reference.is_empty() || reference.starts_with('-') {
        return Err("Merge reference must not start with '-'.".into());
    }
    let resolved = resolve(path, reference)?;
    let before = head(path)?;
    let mut args = vec!["merge".into(), "--no-edit".into()];
    if no_ff || mode == "--ff-only" {
        args.push(mode.into());
    }
    args.push(reference.to_string());
    let output = run(path, &args)?;
    let conflicts = unmerged(path)?;
    let state = crate::mutation::state(path)?;
    if !output.status.success() && conflicts.is_empty() && state.is_none() {
        if mode == "--ff-only" {
            return Err(format!(
                "Not a fast-forward: the branch and {reference} have diverged. Pull with rebase or merge instead."
            ));
        }
        return Err(failure("Merge", &output));
    }
    let in_progress = state.is_some() || !conflicts.is_empty();
    let after = head(path)?;
    let kind = if in_progress {
        None
    } else if after == before {
        Some(Kind::UpToDate)
    } else if parents(path)? > 1 {
        Some(Kind::MergeCommit)
    } else {
        Some(Kind::FastForward)
    };
    let mut warnings = Vec::new();
    if !conflicts.is_empty() {
        warnings.push("Resolve the listed conflicts, then continue or abort the merge.".into());
    }
    Ok(Receipt {
        reference: reference.to_string(),
        commit: Some(resolved),
        kind,
        applied: output.status.success() && !in_progress,
        in_progress,
        conflicts,
        warnings,
    })
}

fn resolve(path: &Path, reference: &str) -> Result<String, String> {
    let output = run(
        path,
        &[
            "rev-parse".into(),
            "--verify".into(),
            "--quiet".into(),
            format!("{reference}^{{commit}}"),
        ],
    )?;
    if !output.status.success() {
        return Err(format!("Merge reference {reference} was not found."));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn head(path: &Path) -> Result<String, String> {
    let output = run(path, &["rev-parse".into(), "HEAD".into()])?;
    if !output.status.success() {
        return Err(failure("Read HEAD", &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parents(path: &Path) -> Result<usize, String> {
    let output = run(path, &["log".into(), "-1".into(), "--format=%P".into()])?;
    if !output.status.success() {
        return Err(failure("Read merge parents", &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .count())
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

fn run(path: &Path, args: &[String]) -> Result<Output, String> {
    Command::new("git")
        .current_dir(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_EDITOR", "true")
        .env("GIT_SEQUENCE_EDITOR", "true")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_err(|error| format!("Could not run Git merge. Install Git and retry: {error}"))
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
