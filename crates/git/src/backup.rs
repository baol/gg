//! Save, list, restore, and delete pre-rewrite tips under private references.
use gg_engine::recovery::{Backup, Restore};
use std::{
    path::Path,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

/// Save `target` as `refs/gg/backup/<label>/<unix-nanos>[/<branch>]`, the
/// branch being the one checked out (the branch a rewrite is about to move).
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
    let name = match current_branch(path) {
        Some(branch) => format!("refs/gg/backup/{label}/{nanos}/{branch}"),
        None => format!("refs/gg/backup/{label}/{nanos}"),
    };
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
        let mut parts = reference
            .strip_prefix("refs/gg/backup/")
            .unwrap_or_default()
            .splitn(3, '/');
        let label = parts.next().unwrap_or_default();
        let saved = parts.next().unwrap_or_default();
        let branch = parts.next().filter(|branch| !branch.is_empty());
        if reference.is_empty() || commit.is_empty() || label.is_empty() {
            continue;
        }
        backups.push(Backup {
            reference: reference.to_string(),
            label: label.to_string(),
            commit: commit.to_string(),
            time: time.to_string(),
            subject: subject.to_string(),
            branch: branch.map(str::to_string),
            saved: saved
                .parse::<u128>()
                .map(|nanos| (nanos / 1_000_000_000) as i64)
                .unwrap_or_default(),
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
        .strip_prefix("refs/gg/backup/")
        .and_then(|rest| rest.split('/').nth(1))
        .and_then(|value| value.parse().ok())
        .unwrap_or_default()
}

/// The checked-out branch's short name; `None` on a detached HEAD.
pub fn current_branch(path: &Path) -> Option<String> {
    let output = run(
        path,
        &[
            "symbolic-ref".into(),
            "--quiet".into(),
            "--short".into(),
            "HEAD".into(),
        ],
    )
    .ok()?;
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (output.status.success() && !name.is_empty()).then_some(name)
}

/// Tracked files with staged or unstaged changes (untracked files are not
/// counted: a hard reset keeps them unless it would overwrite one).
pub fn uncommitted(path: &Path) -> Result<usize, String> {
    let output = run(
        path,
        &[
            "--no-optional-locks".into(),
            "status".into(),
            "--porcelain=v1".into(),
            "-z".into(),
            "--untracked-files=no".into(),
        ],
    )?;
    if !output.status.success() {
        return Err(failure("Read the working tree status", &output));
    }
    let mut count = 0;
    let mut records = output.stdout.split(|byte| *byte == 0);
    while let Some(record) = records.next() {
        if record.len() < 3 {
            continue;
        }
        count += 1;
        // A rename or copy carries its source path as the next record.
        if matches!(record[0], b'R' | b'C') {
            records.next();
        }
    }
    Ok(count)
}

/// Why a restore cannot run with `count` changed files.
pub fn uncommitted_reason(count: usize) -> String {
    if count == 1 {
        "1 file has uncommitted changes. Commit or stash it first.".into()
    } else {
        format!("{count} files have uncommitted changes. Commit or stash them first.")
    }
}

/// The tip of local `branch` and how many commits differ between it and
/// `commit` (both sides of the symmetric difference).
pub fn branch_now(path: &Path, branch: &str, commit: &str) -> Option<(String, usize)> {
    if branch.starts_with('-') || commit.starts_with('-') {
        return None;
    }
    let tip = run(
        path,
        &[
            "rev-parse".into(),
            "--verify".into(),
            "--quiet".into(),
            format!("refs/heads/{branch}^{{commit}}"),
        ],
    )
    .ok()
    .filter(|output| output.status.success())?;
    let tip = String::from_utf8_lossy(&tip.stdout).trim().to_string();
    let counted = run(
        path,
        &[
            "rev-list".into(),
            "--count".into(),
            format!("{tip}...{commit}"),
        ],
    )
    .ok()
    .filter(|output| output.status.success())?;
    let count = String::from_utf8_lossy(&counted.stdout)
        .trim()
        .parse()
        .ok()?;
    Some((tip, count))
}

/// Delete one gg backup reference; other references are refused.
pub fn delete(path: &Path, reference: &str) -> Result<(), String> {
    let valid = reference
        .strip_prefix("refs/gg/backup/")
        .is_some_and(|rest| !rest.is_empty() && !rest.split('/').any(|part| part == ".."));
    if !valid {
        return Err(format!("{reference} is not a Git Good backup."));
    }
    let output = run(path, &["update-ref".into(), "-d".into(), reference.into()])?;
    if !output.status.success() {
        return Err(failure("Delete the backup", &output));
    }
    Ok(())
}

/// Restore checked-out `branch` to `commit`: a hard reset that needs a clean
/// tree and first saves the current tip as a `restore` backup, so it can be
/// undone.
pub fn restore(path: &Path, commit: &str, branch: &str) -> Result<Restore, String> {
    let commit = commit.trim();
    if commit.is_empty() || commit.starts_with('-') || commit.contains('\0') {
        return Err("Restore target is invalid.".into());
    }
    match current_branch(path) {
        Some(current) if current == branch => {}
        Some(_) => return Err(format!("Switch to {branch} before restoring it.")),
        None => return Err("Check out a branch before restoring.".into()),
    }
    let changed = uncommitted(path)?;
    if changed > 0 {
        return Err(uncommitted_reason(changed));
    }
    let head = run(
        path,
        &["rev-parse".into(), "--verify".into(), "HEAD".into()],
    )?;
    if !head.status.success() {
        return Err(failure("Read the current tip", &head));
    }
    let previous = String::from_utf8_lossy(&head.stdout).trim().to_string();
    let target = run(
        path,
        &[
            "rev-parse".into(),
            "--verify".into(),
            "--quiet".into(),
            format!("{commit}^{{commit}}"),
        ],
    )?;
    if !target.status.success() {
        return Err(format!("Restore target {commit} was not found."));
    }
    let target = String::from_utf8_lossy(&target.stdout).trim().to_string();
    let saved = create(path, "restore", &previous)?;
    let receipt = crate::reset::reset(path, &target, gg_engine::reset::Mode::Hard)?;
    Ok(Restore {
        branch: branch.to_string(),
        previous,
        commit: receipt.commit,
        saved,
    })
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
