//! Tracked paths for search and the latest commit that changed a path.
use std::{
    path::Path,
    process::{Command, Output, Stdio},
};

/// Tracked paths at HEAD's index, at most `limit` of them.
pub fn tracked(path: &Path, limit: usize) -> Result<Vec<String>, String> {
    let output = run(path, &["ls-files", "-z"])?;
    if !output.status.success() {
        return Err(failure("List tracked files", &output));
    }
    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .take(limit)
        .map(|field| String::from_utf8_lossy(field).into_owned())
        .collect())
}

/// The newest commit reachable from HEAD that changed `file`, with its subject.
pub fn last_change(path: &Path, file: &str) -> Result<Option<(String, String)>, String> {
    if file.is_empty() || file.contains('\0') {
        return Err("The file path is empty or invalid.".into());
    }
    let spec = format!(":(literal){file}");
    let output = run(path, &["log", "-1", "--format=%H%x1f%s", "--", &spec])?;
    if !output.status.success() {
        return Err(failure("Find the file's latest commit", &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .split_once('\x1f')
        .map(|(id, subject)| (id.to_string(), subject.to_string())))
}

fn run(path: &Path, args: &[&str]) -> Result<Output, String> {
    Command::new("git")
        .current_dir(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_err(|error| format!("Could not run Git. Install Git and retry: {error}"))
}

fn failure(label: &str, output: &Output) -> String {
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if detail.is_empty() {
        format!("{label} failed ({}).", output.status)
    } else {
        format!("{label} failed ({}).\n{detail}", output.status)
    }
}
