//! Reset the current branch to a commit.
use gg_engine::reset::{Mode, Receipt};
use std::{
    path::Path,
    process::{Command, Output, Stdio},
};

pub fn reset(path: &Path, commit: &str, mode: Mode) -> Result<Receipt, String> {
    let commit = commit.trim();
    if commit.is_empty() || commit.starts_with('-') {
        return Err("Reset target must be a revision that does not start with '-'.".into());
    }
    let resolved = resolve(path, commit)?;
    let output = run(
        path,
        &[
            "reset".into(),
            format!("--{}", mode.as_str()),
            resolved.clone(),
        ],
    )?;
    if !output.status.success() {
        return Err(failure("Reset", &output));
    }
    Ok(Receipt {
        commit: resolved,
        mode,
        applied: true,
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
    )?;
    if !output.status.success() {
        return Err(format!("Commit {commit} was not found."));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run(path: &Path, args: &[String]) -> Result<Output, String> {
    Command::new("git")
        .current_dir(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_err(|error| format!("Could not run Git reset. Install Git and retry: {error}"))
}

fn failure(label: &str, output: &Output) -> String {
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if detail.is_empty() {
        format!("{label} failed ({}).", output.status)
    } else {
        format!("{label} failed ({}).\n{detail}", output.status)
    }
}
