//! Stash listing and mutations delegated to Git.
use gg_engine::stash::{StashEntry, StashList, StashReceipt};
use std::{
    path::Path,
    process::{Command, Output, Stdio},
};

const LIST_FORMAT: &str = "%gd%x00%H%x00%gs%x00%cr";

/// Entries whose files [`list_with_files`] counts; older ones keep `None`.
pub const COUNTED_ENTRIES: usize = 50;

pub fn list(path: &Path) -> Result<StashList, String> {
    let output = run(
        path,
        &[
            "stash".into(),
            "list".into(),
            format!("--format={LIST_FORMAT}"),
        ],
    )?;
    if !output.status.success() {
        return Err(failure("List stashes", &output));
    }
    let mut entries = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split('\0');
        let reference = fields.next().unwrap_or_default().to_string();
        let commit = fields.next().unwrap_or_default().to_string();
        let message = fields.next().unwrap_or_default().to_string();
        let age = fields.next().unwrap_or_default().to_string();
        let index = reference
            .strip_prefix("stash@{")
            .and_then(|value| value.strip_suffix('}'))
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| format!("Unexpected stash reference {reference}"))?;
        entries.push(StashEntry {
            index,
            reference,
            commit,
            branch: gg_engine::stash::subject_branch(&message),
            message,
            age,
            files: None,
        });
    }
    Ok(StashList { entries })
}

/// List stashes and count the files of the newest [`COUNTED_ENTRIES`]
/// entries. Run it off the UI thread: each counted entry costs up to three
/// Git reads.
pub fn list_with_files(path: &Path) -> Result<StashList, String> {
    let mut list = list(path)?;
    for entry in list.entries.iter_mut().take(COUNTED_ENTRIES) {
        entry.files = file_count(path, &entry.commit).ok();
    }
    Ok(list)
}

/// Paths a stash entry changes: its working tree against the commit it was
/// made on, plus untracked files stored in its third parent.
pub fn file_count(path: &Path, commit: &str) -> Result<usize, String> {
    let names = |args: &[String]| -> Result<Vec<String>, String> {
        let output = run(path, args)?;
        if !output.status.success() {
            return Err(failure("Count stash files", &output));
        }
        Ok(output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|name| !name.is_empty())
            .map(|name| String::from_utf8_lossy(name).into_owned())
            .collect())
    };
    let mut files = names(&[
        "diff-tree".into(),
        "-r".into(),
        "-z".into(),
        "--name-only".into(),
        "--no-commit-id".into(),
        format!("{commit}^1"),
        commit.to_string(),
    ])?;
    let untracked = run(
        path,
        &[
            "rev-parse".into(),
            "--verify".into(),
            "--quiet".into(),
            format!("{commit}^3"),
        ],
    )?;
    if untracked.status.success() {
        files.extend(names(&[
            "ls-tree".into(),
            "-r".into(),
            "-z".into(),
            "--name-only".into(),
            format!("{commit}^3"),
        ])?);
    }
    files.sort();
    files.dedup();
    Ok(files.len())
}

/// Whether the worktree has anything to stash: staged, unstaged, or untracked changes.
pub fn has_changes(path: &Path) -> Result<bool, String> {
    Ok(change_count(path)? > 0)
}

/// Number of changed paths: staged, unstaged, untracked, or conflicted, each path once.
pub fn change_count(path: &Path) -> Result<usize, String> {
    let output = run(
        path,
        &[
            "status".into(),
            "--porcelain".into(),
            "-z".into(),
            "--untracked-files=normal".into(),
        ],
    )?;
    if !output.status.success() {
        return Err(failure("Read worktree status", &output));
    }
    let mut count = 0;
    let mut records = output.stdout.split(|byte| *byte == 0);
    while let Some(record) = records.next() {
        if record.len() < 3 {
            continue;
        }
        count += 1;
        // Renames and copies carry their original path in the next field.
        if matches!(record[0], b'R' | b'C') || matches!(record[1], b'R' | b'C') {
            records.next();
        }
    }
    Ok(count)
}

pub fn push(
    path: &Path,
    message: Option<&str>,
    include_untracked: bool,
    keep_index: bool,
) -> Result<StashReceipt, String> {
    let mut args = vec!["stash".to_string(), "push".to_string()];
    if let Some(message) = message {
        if message.contains('\0') {
            return Err("Stash message cannot contain NUL.".into());
        }
        args.push("-m".into());
        args.push(message.to_string());
    }
    if include_untracked {
        args.push("--include-untracked".into());
    }
    if keep_index {
        args.push("--keep-index".into());
    }
    let output = run(path, &args)?;
    let detail = combined(&output);
    if detail.contains("No local changes to save") {
        return Err("No local changes to stash.".into());
    }
    if !output.status.success() {
        return Err(failure("Stash changes", &output));
    }
    let entry = list(path)?
        .entries
        .into_iter()
        .next()
        .ok_or("Stash push succeeded but no stash entry was recorded.")?;
    Ok(StashReceipt {
        reference: Some(entry.reference),
        message: Some(entry.message),
        created: true,
        ..StashReceipt::default()
    })
}

pub fn apply(
    path: &Path,
    reference: Option<&str>,
    restore_index: bool,
) -> Result<StashReceipt, String> {
    restore(path, reference, restore_index, false)
}

pub fn pop(
    path: &Path,
    reference: Option<&str>,
    restore_index: bool,
) -> Result<StashReceipt, String> {
    restore(path, reference, restore_index, true)
}

pub fn drop(path: &Path, reference: Option<&str>) -> Result<StashReceipt, String> {
    let reference = normalize(reference)?;
    let output = run(path, &["stash".into(), "drop".into(), reference.clone()])?;
    if !output.status.success() {
        return Err(failure("Drop stash", &output));
    }
    Ok(StashReceipt {
        reference: Some(reference),
        dropped: true,
        ..StashReceipt::default()
    })
}

fn restore(
    path: &Path,
    reference: Option<&str>,
    restore_index: bool,
    remove: bool,
) -> Result<StashReceipt, String> {
    let reference = normalize(reference)?;
    let mut args = vec![
        "stash".to_string(),
        if remove { "pop".into() } else { "apply".into() },
    ];
    if restore_index {
        args.push("--index".into());
    }
    args.push(reference.clone());
    let output = run(path, &args)?;
    let conflicts = unmerged(path)?;
    if !output.status.success() && conflicts.is_empty() {
        return Err(failure(
            if remove { "Pop stash" } else { "Apply stash" },
            &output,
        ));
    }
    let mut warnings = Vec::new();
    if !conflicts.is_empty() {
        warnings
            .push("Resolve the listed conflicts, then stage each path before continuing.".into());
    }
    Ok(StashReceipt {
        reference: Some(reference),
        applied: true,
        dropped: remove && conflicts.is_empty(),
        conflicts,
        warnings,
        ..StashReceipt::default()
    })
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

fn normalize(reference: Option<&str>) -> Result<String, String> {
    let Some(reference) = reference else {
        return Ok("stash@{0}".into());
    };
    let reference = reference.trim();
    let index = reference
        .strip_prefix("stash@{")
        .and_then(|value| value.strip_suffix('}'))
        .or(Some(reference))
        .and_then(|value| value.parse::<u32>().ok());
    match index {
        Some(index) => Ok(format!("stash@{{{index}}}")),
        None => Err("Stash reference must look like stash@{0} or 0.".into()),
    }
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
        .map_err(|error| format!("Could not run Git stash. Install Git and retry: {error}"))
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
