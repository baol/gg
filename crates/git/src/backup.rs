//! Save the pre-rewrite tip under a private reference.
use std::{
    path::Path,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn create(path: &Path, label: &str, target: &str) -> Result<String, String> {
    let target = target.trim();
    if target.is_empty() || target.starts_with('-') || target.contains('\0') {
        return Err("Backup target is invalid.".into());
    }
    let label = sanitize(label);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Read the system clock for the backup: {e}"))?
        .as_nanos();
    let name = format!("refs/gg/backup/{label}/{nanos}");
    let resolved = run(
        path,
        &[
            "rev-parse".into(),
            "--verify".into(),
            "--quiet".into(),
            format!("{target}^{{commit}}"),
        ],
    )?;
    if !resolved.status.success() {
        return Err(format!("Backup target {target} was not found."));
    }
    let resolved = String::from_utf8_lossy(&resolved.stdout).trim().to_string();
    let output = run(path, &["update-ref".into(), name.clone(), resolved])?;
    if !output.status.success() {
        return Err(failure("Create backup reference", &output));
    }
    Ok(name)
}

fn sanitize(label: &str) -> String {
    let mut clean = String::new();
    for character in label.trim().chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
            clean.push(character);
        } else if !clean.ends_with('-') {
            clean.push('-');
        }
    }
    let clean = clean.trim_matches('-');
    if clean.is_empty() {
        "rewrite".into()
    } else {
        clean.to_string()
    }
}

fn run(path: &Path, args: &[String]) -> Result<std::process::Output, String> {
    Command::new("git")
        .current_dir(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_err(|e| format!("Could not create a backup using Git: {e}. Install Git and retry."))
}

fn failure(label: &str, output: &std::process::Output) -> String {
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if detail.is_empty() {
        format!("{label} failed ({}).", output.status)
    } else {
        format!("{label} failed ({}).\n{detail}", output.status)
    }
}
