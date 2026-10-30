//! Save and list pre-rewrite tips under private references.
use gg_engine::recovery::Backup;
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

/// Delete gg's backup references older than `age`, judged by the creation
/// time in their names; returns how many were deleted. Other references are
/// never touched.
pub fn prune(path: &Path, age: std::time::Duration) -> Result<usize, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Read the system clock: {e}"))?
        .as_nanos();
    let cutoff = now.saturating_sub(age.as_nanos());
    let mut removed = 0;
    for backup in list(path)? {
        let created = backup_nanos(&backup.reference);
        if created == 0 || created >= cutoff {
            continue;
        }
        let output = run(
            path,
            &["update-ref".into(), "-d".into(), backup.reference.clone()],
        )?;
        if !output.status.success() {
            return Err(failure("Delete an old backup", &output));
        }
        removed += 1;
    }
    Ok(removed)
}

pub fn list(path: &Path) -> Result<Vec<Backup>, String> {
    let output = run(
        path,
        &[
            "for-each-ref".into(),
            "--format=%(refname)%00%(objectname)%00%(committerdate:iso8601)%00%(subject)".into(),
            "refs/gg/backup".into(),
        ],
    )?;
    if !output.status.success() {
        return Err(failure("List backups", &output));
    }
    let mut backups = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let mut fields = line.splitn(4, '\0');
        let reference = fields.next().unwrap_or_default();
        let commit = fields.next().unwrap_or_default();
        let time = fields.next().unwrap_or_default();
        let subject = fields.next().unwrap_or_default();
        let label = reference
            .strip_prefix("refs/gg/backup/")
            .and_then(|rest| rest.split('/').next())
            .unwrap_or_default();
        if reference.is_empty() || commit.is_empty() || label.is_empty() {
            continue;
        }
        backups.push(Backup {
            reference: reference.to_string(),
            label: label.to_string(),
            commit: commit.to_string(),
            time: time.to_string(),
            subject: subject.to_string(),
        });
    }
    backups.sort_by(|left, right| {
        let left = backup_nanos(&left.reference);
        let right = backup_nanos(&right.reference);
        right.cmp(&left)
    });
    Ok(backups)
}

fn backup_nanos(reference: &str) -> u128 {
    reference
        .rsplit('/')
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or_default()
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
