//! Rebase the current branch onto a local, remote-tracking, or tag reference.
use gg_engine::rebase::Receipt;
use std::{
    path::Path,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicBool, Ordering},
};

pub fn rebase(path: &Path, reference: &str, autostash: bool) -> Result<Receipt, String> {
    let reference = reference.trim();
    if reference.is_empty() || reference.starts_with('-') || reference.contains('\0') {
        return Err("Rebase reference must not start with '-'.".into());
    }
    if let Some(state) = crate::mutation::state(path)? {
        return Err(format!(
            "{} is already in progress. Continue, skip, or abort it first.",
            state.operation.as_str()
        ));
    }
    if !autostash {
        ensure_clean(path)?;
    }
    let upstream = resolve(path, reference)?;
    let previous = head(path)?;
    let backup = crate::backup::create(path, "rebase", &previous)?;
    let mut args = vec!["rebase".to_string()];
    args.push(if autostash {
        "--autostash".into()
    } else {
        "--no-autostash".into()
    });
    args.push(reference.to_string());
    let output = run(path, &args)?;
    let conflicts = unmerged(path)?;
    let state = crate::mutation::state(path)?;
    if !output.status.success() && state.is_none() && conflicts.is_empty() {
        return Err(failure("Rebase", &output));
    }
    let in_progress = state.is_some() || !conflicts.is_empty();
    let mut warnings = Vec::new();
    if !conflicts.is_empty() {
        warnings.push(
            "Resolve the listed conflicts, stage each path, then continue or abort the rebase."
                .into(),
        );
    }
    if autostash && !output.status.success() {
        warnings.push(
            "Working changes were autostashed. Git reapplies them when the rebase continues."
                .into(),
        );
    }
    Ok(Receipt {
        reference: reference.to_string(),
        commit: Some(upstream),
        previous: Some(previous),
        backup: Some(backup),
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
        return Err(format!("Rebase reference {reference} was not found."));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn head(path: &Path) -> Result<String, String> {
    let output = run(path, &["rev-parse".into(), "HEAD".into()])?;
    if !output.status.success() {
        return Err(failure("Read HEAD for the rebase", &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn ensure_clean(path: &Path) -> Result<(), String> {
    let session = crate::status::Session::open(path)?;
    let stop = AtomicBool::new(false);
    let mut dirty = None;
    session.scan(&stop, |entry| {
        if entry.area == gg_engine::status::Area::Untracked {
            return Ok(());
        }
        dirty = Some(entry.path);
        stop.store(true, Ordering::Relaxed);
        Ok(())
    })?;
    if let Some(path) = dirty {
        return Err(format!(
            "Working tree is not clean ({path}). Commit, stash, or discard changes, or enable autostash."
        ));
    }
    Ok(())
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
        return Err(failure("Read rebase conflicts", &output));
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
        .map_err(|error| format!("Could not run Git rebase. Install Git and retry: {error}"))
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
