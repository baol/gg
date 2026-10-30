//! Cherry-pick, revert, and their continue/skip/abort recovery.
use gg_engine::mutation::{
    MutationReceipt, Operation, OperationState, RebaseProgress, RebaseStep, RebaseStop,
};
use std::{
    path::Path,
    process::{Command, Output, Stdio},
};

pub fn state(path: &Path) -> Result<Option<OperationState>, String> {
    let conflicts = unmerged(path)?;
    if rebase_in_progress(path)? {
        return Ok(Some(OperationState {
            operation: Operation::Rebase,
            commit: head(path, "REBASE_HEAD")?,
            conflicts,
        }));
    }
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

/// The steps and stop of an interactive rebase in progress, or `None` when no
/// interactive rebase is running.
pub fn rebase_progress(path: &Path) -> Result<Option<RebaseProgress>, String> {
    let Some(dir) = git_path(path, "rebase-merge")? else {
        return Ok(None);
    };
    if !dir.is_dir() {
        return Ok(None);
    }
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap_or_default();
    let done_lines = read("done");
    let done = commit_lines(&done_lines);
    let todo = commit_lines(&read("git-rebase-todo"));
    let subjects = subjects(
        path,
        done.iter()
            .chain(todo.iter())
            .map(|(_, commit)| commit.as_str()),
    )?;
    let step = |(action, commit): &(String, String)| RebaseStep {
        action: action.clone(),
        commit: subjects
            .iter()
            .find(|(id, _)| id.starts_with(commit.as_str()))
            .map_or_else(|| commit.clone(), |(id, _)| id.clone()),
        subject: subjects
            .iter()
            .find(|(id, _)| id.starts_with(commit.as_str()))
            .map(|(_, subject)| subject.clone())
            .unwrap_or_default(),
    };
    let done = done.iter().map(step).collect::<Vec<_>>();
    let todo = todo.iter().map(step).collect::<Vec<_>>();
    // gg's reopen script is the last line Git ran when it stopped there.
    let reopen_last = done_lines
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty() && !line.starts_with('#'))
        .is_some_and(|line| line.starts_with("exec") && line.contains("gg-reopen"));
    let reopen = read("gg-reopen");
    let mut reopen = reopen.lines();
    let reopen_base = reopen.next().map(str::trim).unwrap_or_default().to_string();
    let reopen_commit = reopen.next().map(str::trim).unwrap_or_default().to_string();
    let stop = if !unmerged(path)?.is_empty() {
        RebaseStop::Conflict
    } else if reopen_last
        && !reopen_commit.is_empty()
        && done.last().is_some_and(|step| {
            step.commit.starts_with(&reopen_commit) || reopen_commit.starts_with(&step.commit)
        })
    {
        let base = (!reopen_base.is_empty()).then_some(reopen_base);
        let message = log(path, &["-1", "--format=%B", &reopen_commit])?
            .trim_end()
            .to_string();
        let subject = message.lines().next().unwrap_or_default().to_string();
        let made = match &base {
            Some(base) => log(
                path,
                &["--reverse", "--format=%H%x1f%s", &format!("{base}..HEAD")],
            )?
            .lines()
            .filter_map(|line| line.split_once('\x1f'))
            .map(|(commit, subject)| RebaseStep {
                action: "commit".into(),
                commit: commit.to_string(),
                subject: subject.to_string(),
            })
            .collect(),
            None => Vec::new(),
        };
        RebaseStop::Reopened {
            base,
            commit: reopen_commit,
            subject,
            message,
            made,
        }
    } else {
        RebaseStop::Other
    };
    let branch = read("head-name")
        .trim()
        .strip_prefix("refs/heads/")
        .map(str::to_string);
    let status = run(
        path,
        &[
            "status".into(),
            "--porcelain".into(),
            "--untracked-files=no".into(),
        ],
        false,
    )?;
    let tracked_changes = String::from_utf8_lossy(&status.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .count();
    Ok(Some(RebaseProgress {
        branch,
        done,
        todo,
        stop,
        tracked_changes,
    }))
}

/// `(action, commit)` for each commit-bearing todo line, with abbreviations
/// expanded and options such as `fixup -C` skipped.
fn commit_lines(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let action = match words.next()? {
                "p" | "pick" => "pick",
                "r" | "reword" => "reword",
                "e" | "edit" => "edit",
                "s" | "squash" => "squash",
                "f" | "fixup" => "fixup",
                "d" | "drop" => "drop",
                _ => return None,
            };
            let commit = words.find(|word| !word.starts_with('-'))?;
            Some((action.to_string(), commit.to_string()))
        })
        .collect()
}

/// Full IDs and subjects for the given commits.
fn subjects<'a>(
    path: &Path,
    commits: impl Iterator<Item = &'a str>,
) -> Result<Vec<(String, String)>, String> {
    let mut args = vec![
        "--no-walk=unsorted".to_string(),
        "--format=%H%x1f%s".to_string(),
    ];
    args.extend(commits.map(str::to_string));
    if args.len() == 2 {
        return Ok(Vec::new());
    }
    Ok(
        log(path, &args.iter().map(String::as_str).collect::<Vec<_>>())?
            .lines()
            .filter_map(|line| line.split_once('\x1f'))
            .map(|(id, subject)| (id.to_string(), subject.to_string()))
            .collect(),
    )
}

fn log(path: &Path, args: &[&str]) -> Result<String, String> {
    let mut all = vec!["log".to_string()];
    all.extend(args.iter().map(|arg| arg.to_string()));
    let output = run(path, &all, false)?;
    if !output.status.success() {
        return Err(failure("Read rebase commits", &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Absolute path of `name` inside the Git directory.
fn git_path(path: &Path, name: &str) -> Result<Option<std::path::PathBuf>, String> {
    let output = run(
        path,
        &["rev-parse".into(), "--git-path".into(), name.into()],
        false,
    )?;
    if !output.status.success() {
        return Ok(None);
    }
    let found = std::path::PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    Ok(Some(if found.is_absolute() {
        found
    } else {
        path.join(found)
    }))
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
    let flag = match operation {
        Operation::CherryPick => "cherry-pick",
        Operation::Revert => "revert",
        Operation::Merge | Operation::Rebase => {
            return Err(format!(
                "Use the {} action for this operation.",
                label(operation)
            ));
        }
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
    if !output.status.success() {
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

fn rebase_in_progress(path: &Path) -> Result<bool, String> {
    let repo = gix::discover(path).map_err(|e| format!("Read repository: {e}"))?;
    Ok(repo.path().join("rebase-merge").is_dir() || repo.path().join("rebase-apply").is_dir())
}

fn label(operation: Operation) -> &'static str {
    match operation {
        Operation::CherryPick => "Cherry-pick",
        Operation::Revert => "Revert",
        Operation::Merge => "Merge",
        Operation::Rebase => "Rebase",
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
