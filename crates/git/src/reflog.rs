//! Read reflog entries without changing repository state.
use gg_engine::reflog::{Entry, Snapshot};
use std::{
    path::Path,
    process::{Command, Stdio},
};

pub fn read(path: &Path, reference: &str) -> Result<Snapshot, String> {
    let reference = reference.trim();
    let reference = if reference.is_empty() {
        "HEAD"
    } else {
        reference
    };
    if reference.starts_with('-') || reference.contains('\0') {
        return Err("Reflog reference is invalid.".into());
    }
    let output = Command::new("git")
        .current_dir(path)
        .args(["reflog", "show", "--date=unix", "--format=%H%x00%gD%x00%gs"])
        .arg(reference)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_err(|e| format!("Could not read the reflog using Git: {e}. Install Git and retry."))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "Read reflog {reference} failed ({}).{}",
            output.status,
            if detail.is_empty() {
                String::new()
            } else {
                format!("\n{detail}")
            }
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();
    for (index, line) in stdout.lines().enumerate() {
        let mut fields = line.split('\0');
        let Some(id) = fields.next() else {
            continue;
        };
        let Some(dated_selector) = fields.next() else {
            continue;
        };
        let subject = fields.next().unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        let time = dated_selector
            .split_once('{')
            .and_then(|(_, rest)| rest.strip_suffix('}'))
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or_default();
        let (action, message) = match subject.split_once(':') {
            Some((action, message)) => (action.to_string(), message.trim().to_string()),
            None => (subject.to_string(), String::new()),
        };
        entries.push(Entry {
            selector: format!("{reference}@{{{index}}}"),
            id: id.to_string(),
            time,
            action,
            message,
        });
    }
    Ok(Snapshot {
        reference: reference.to_string(),
        entries,
    })
}
