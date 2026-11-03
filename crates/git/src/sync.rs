//! Explicit clone, fetch, and push operations delegated to Git for transport and credentials.
use gg_engine::sync::{
    CloneReceipt, FetchReceipt, Operation, Progress, PushFailure, PushReceipt, UpstreamRelation,
};
use gix::remote::Direction;
use std::{
    io::{BufReader, Read},
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

const MAX_MESSAGE: usize = 240;
const MAX_DETAIL: usize = 8 * 1024;
const MAX_LINE: usize = 4 * 1024;

pub fn fetch(
    path: &Path,
    remote: Option<&str>,
    prune: bool,
    interactive: bool,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> Result<FetchReceipt, String> {
    let (repo, root) = open(path)?;
    let remote = resolve_remote(&repo, remote, Direction::Fetch)?;
    let mut args = vec!["fetch".to_string(), "--progress".to_string()];
    if prune {
        args.push("--prune".to_string());
    }
    args.push(remote.clone());
    let output = run_git(
        &root,
        &args,
        Operation::Fetch,
        interactive,
        cancel,
        progress,
    )?;
    finish(output, format!("Fetch from {remote}"))?;
    Ok(FetchReceipt {
        remote,
        pruned: prune,
    })
}

#[derive(Clone, Debug, Default)]
pub struct PushOptions {
    pub set_upstream: bool,
    /// The exact expected remote commit for `--force-with-lease`; `None` keeps the push non-force.
    pub force_with_lease: Option<String>,
}

fn push_target(
    repo: &gix::Repository,
    remote: Option<&str>,
    branch: Option<&str>,
) -> Result<(String, String, String), String> {
    let head = repo.head().map_err(|e| format!("Read HEAD: {e}"))?;
    if head.is_detached() {
        return Err("Push requires a local branch. Switch to a branch and retry.".into());
    }
    if head.is_unborn() {
        return Err("Push requires at least one local commit.".into());
    }
    let current = head
        .referent_name()
        .ok_or("Push requires a local branch. Switch to a branch and retry.")?
        .to_owned();
    let branch = branch
        .map(str::to_string)
        .unwrap_or_else(|| current.shorten().to_string());
    validate_branch(&branch)?;
    let full = format!("refs/heads/{branch}");
    let branch_ref = repo
        .try_find_reference(full.as_str())
        .map_err(|e| format!("Read branch {branch}: {e}"))?
        .ok_or_else(|| format!("Local branch {branch} does not exist. Refresh refs and retry."))?;
    let branch_full = branch_ref.name().to_owned();
    let configured_remote = repo
        .branch_remote_name(branch_full.shorten(), Direction::Fetch)
        .map(|name| name.as_bstr().to_string());
    let remote = resolve_remote(repo, remote, Direction::Push)?;
    let configured_branch = if configured_remote.as_deref() == Some(remote.as_str()) {
        repo.branch_remote_ref_name(branch_full.as_ref(), Direction::Fetch)
            .transpose()
            .map_err(|e| format!("Read upstream for {branch}: {e}"))?
            .map(|name| {
                name.as_bstr()
                    .to_string()
                    .strip_prefix("refs/heads/")
                    .unwrap_or_default()
                    .to_string()
            })
            .filter(|name| !name.is_empty())
    } else {
        None
    };
    let remote_branch = configured_branch.unwrap_or_else(|| branch.clone());
    Ok((branch, remote, remote_branch))
}

fn lease_id(repo: &gix::Repository, remote: &str, remote_branch: &str) -> Result<String, String> {
    let tracking = format!("refs/remotes/{remote}/{remote_branch}");
    repo.try_find_reference(tracking.as_str())
        .map_err(|e| format!("Read remote-tracking ref {tracking}: {e}"))?
        .and_then(|reference| reference.target().try_id().map(|id| id.to_string()))
        .ok_or_else(|| {
            format!(
                "Force with lease requires the remote-tracking ref {remote}/{remote_branch}. Fetch first, then retry."
            )
        })
}

/// Read the exact lease commit for a forced push, without changing anything.
pub fn push_lease(
    path: &Path,
    remote: Option<&str>,
    branch: Option<&str>,
) -> Result<String, String> {
    let (repo, _) = open(path)?;
    let (_, remote, remote_branch) = push_target(&repo, remote, branch)?;
    lease_id(&repo, &remote, &remote_branch)
}

pub fn push(
    path: &Path,
    remote: Option<&str>,
    branch: Option<&str>,
    options: PushOptions,
    interactive: bool,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> Result<PushReceipt, String> {
    let (repo, root) = open(path)?;
    let (branch, remote, remote_branch) = push_target(&repo, remote, branch)?;
    let lease = options.force_with_lease;
    if let Some(lease) = &lease {
        gix::ObjectId::from_hex(lease.as_bytes())
            .map_err(|_| "Force-with-lease expected commit is invalid.".to_string())?;
    }
    let mut args = vec!["push".to_string(), "--progress".to_string()];
    if options.set_upstream {
        args.push("--set-upstream".to_string());
    }
    if let Some(lease) = &lease {
        args.push(format!(
            "--force-with-lease=refs/heads/{remote_branch}:{lease}"
        ));
    }
    args.push(remote.clone());
    args.push(format!("refs/heads/{branch}:refs/heads/{remote_branch}"));
    let output = run_git(&root, &args, Operation::Push, interactive, cancel, progress)?;
    if lease.is_none()
        && !output.cancelled
        && !output.status.success()
        && non_fast_forward(&output.detail)
    {
        return Err(format!(
            "{REJECTED}{remote}/{remote_branch} has commits you don't have. Fetch to see them.\n{}",
            output.detail
        ));
    }
    finish(output, format!("Push {branch} to {remote}/{remote_branch}"))?;
    let snapshot = crate::refs::read(path)?;
    let full = format!("refs/heads/{branch}");
    let upstream = snapshot
        .tracking
        .iter()
        .find(|tracking| tracking.full_name == full)
        .map(|tracking| tracking.upstream.clone());
    Ok(PushReceipt {
        remote,
        branch,
        upstream,
        set_upstream: options.set_upstream,
        force_with_lease: lease.is_some(),
        lease,
    })
}

const REJECTED: &str = "Push rejected: ";

/// True when Git refused a push because the remote branch moved past the local one.
fn non_fast_forward(detail: &str) -> bool {
    let detail = detail.to_lowercase();
    detail.contains("[rejected]")
        && (detail.contains("non-fast-forward") || detail.contains("fetch first"))
}

/// Classify a [`push`] error so the caller offers the follow-up that can help.
pub fn push_failure(error: &str) -> PushFailure {
    if error.starts_with(REJECTED) {
        return PushFailure::Rejected;
    }
    let error = error.to_lowercase();
    let transient = needs_credentials(&error)
        || [
            "could not read from remote repository",
            "could not resolve host",
            "unable to access",
            "connection refused",
            "connection reset",
            "connection timed out",
            "timed out",
            "timeout",
            "network is unreachable",
            "the remote end hung up",
            "early eof",
            "rpc failed",
            "ssl",
            "tls",
        ]
        .iter()
        .any(|text| error.contains(text));
    if transient {
        PushFailure::Transient
    } else {
        PushFailure::Other
    }
}

/// How `branch` (a full ref name, or the current branch) stands against its
/// upstream's remote-tracking ref; `None` without an upstream ref. Reads local
/// refs only.
pub fn upstream_relation(
    path: &Path,
    branch: Option<&str>,
) -> Result<Option<UpstreamRelation>, String> {
    let snapshot = crate::refs::read(path)?;
    let tracking = match branch {
        Some(branch) => snapshot
            .tracking
            .iter()
            .find(|tracking| tracking.full_name == branch),
        None => snapshot.current_tracking(),
    };
    Ok(tracking.and_then(|tracking| {
        Some(UpstreamRelation::from_counts(
            tracking.ahead?,
            tracking.behind?,
        ))
    }))
}

/// Delete `branch` on `remote` with a lease: Git refuses when the remote
/// branch no longer points at `expected` (the last fetched commit).
pub fn delete_remote_branch(
    path: &Path,
    remote: &str,
    branch: &str,
    expected: &str,
    interactive: bool,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> Result<String, String> {
    let (_, root) = open(path)?;
    gix::ObjectId::from_hex(expected.as_bytes())
        .map_err(|_| "The remote branch's last known commit is invalid.".to_string())?;
    if remote.is_empty() || branch.is_empty() || remote.starts_with('-') || branch.starts_with('-')
    {
        return Err("Name a remote and a branch to delete.".into());
    }
    let args = vec![
        "push".to_string(),
        "--progress".to_string(),
        format!("--force-with-lease=refs/heads/{branch}:{expected}"),
        remote.to_string(),
        "--delete".to_string(),
        format!("refs/heads/{branch}"),
    ];
    let output = run_git(&root, &args, Operation::Push, interactive, cancel, progress)?;
    finish(output, format!("Delete {branch} on {remote}"))?;
    Ok(format!("Deleted {branch} on {remote}"))
}

pub fn clone(
    url: &str,
    destination: Option<&Path>,
    interactive: bool,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> Result<CloneReceipt, String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("Clone requires a repository URL.".into());
    }
    let cwd = std::env::current_dir().map_err(|e| format!("Read current directory: {e}"))?;
    let destination = match destination {
        Some(path) if !path.as_os_str().is_empty() => path.to_path_buf(),
        _ => default_destination(url)?,
    };
    let args = vec![
        "clone".to_string(),
        "--progress".to_string(),
        "--".to_string(),
        url.to_string(),
        destination.to_string_lossy().into_owned(),
    ];
    let output = run_git(&cwd, &args, Operation::Clone, interactive, cancel, progress)?;
    finish(output, format!("Clone {}", redact_url(url)))?;
    let destination = if destination.is_absolute() {
        destination
    } else {
        cwd.join(destination)
    };
    let destination = std::fs::canonicalize(&destination)
        .map_err(|e| format!("Clone completed but its destination is unreadable: {e}"))?;
    let snapshot = crate::refs::read(&destination)?;
    let head = snapshot.head.as_ref();
    Ok(CloneReceipt {
        path: destination,
        head: head.and_then(|head| head.commit.clone()),
        branch: head
            .and_then(|head| head.name.as_deref())
            .and_then(|name| name.strip_prefix("refs/heads/"))
            .map(str::to_string),
    })
}

fn open(path: &Path) -> Result<(gix::Repository, PathBuf), String> {
    let repo =
        gix::discover(path).map_err(|e| format!("Read repository {}: {e}", path.display()))?;
    let root = repo.workdir().unwrap_or_else(|| repo.path()).to_path_buf();
    Ok((repo, root))
}

fn resolve_remote(
    repo: &gix::Repository,
    requested: Option<&str>,
    direction: Direction,
) -> Result<String, String> {
    if let Some(name) = requested {
        let name = name.trim();
        if name.is_empty() {
            return Err("Remote name is empty.".into());
        }
        return Ok(name.to_string());
    }
    let names = repo
        .remote_names()
        .into_iter()
        .map(|name| String::from_utf8_lossy(&name).into_owned())
        .collect::<Vec<_>>();
    if let Some(default) = repo.remote_default_name(direction) {
        let default = String::from_utf8_lossy(&default).into_owned();
        if names.iter().any(|name| name == &default) {
            return Ok(default);
        }
    }
    match names.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err("No remotes are configured. Add a remote before fetching or pushing.".into()),
        _ => Err(format!(
            "Multiple remotes are configured ({}). Choose one explicitly.",
            names.join(", ")
        )),
    }
}

fn validate_branch(branch: &str) -> Result<(), String> {
    if branch.is_empty() {
        return Err("Branch name is empty.".into());
    }
    if branch.starts_with('-') {
        return Err("Branch name cannot start with '-'.".into());
    }
    let full = format!("refs/heads/{branch}");
    gix::validate::reference::branch_name(gix::bstr::BStr::new(&full))
        .map(|_| ())
        .map_err(|e| format!("Invalid branch name: {e}"))
}

fn default_destination(url: &str) -> Result<PathBuf, String> {
    let name = gg_engine::sync::repository_name(url)
        .ok_or("Could not derive a clone destination. Provide a path.")?;
    Ok(PathBuf::from(name))
}

struct GitOutput {
    status: ExitStatus,
    cancelled: bool,
    detail: String,
}

fn run_git(
    cwd: &Path,
    args: &[String],
    operation: Operation,
    interactive: bool,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> Result<GitOutput, String> {
    let mut command = crate::process::git();
    command
        .current_dir(cwd)
        .args(args)
        .stdin(if interactive {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    if !interactive {
        command
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "never");
    }
    let mut child = command.spawn().map_err(|e| {
        format!(
            "Could not run Git {}. Install Git and retry: {e}",
            operation.as_str()
        )
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        format!(
            "Read Git {} progress: stderr was unavailable",
            operation.as_str()
        )
    })?;
    let (sender, receiver) = mpsc::sync_channel(8);
    let tail = Arc::new(Mutex::new(String::new()));
    let reader_tail = Arc::clone(&tail);
    let reader = thread::Builder::new()
        .name("gg-git-progress".into())
        .spawn(move || read_progress(stderr, operation, sender, reader_tail))
        .map_err(|e| format!("Start Git progress reader: {e}"))?;

    // Wake for each progress update, when Git closes stderr (it is exiting),
    // or every 15 ms to check for cancellation, so the result is seen as
    // soon as Git exits rather than at the next poll (RULE-16).
    let mut cancelled = false;
    let mut reading = true;
    let status = loop {
        if reading {
            match receiver.recv_timeout(Duration::from_millis(15)) {
                Ok(update) => {
                    progress(update);
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => reading = false,
            }
        } else {
            thread::sleep(Duration::from_millis(1));
        }
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            let _ = child.kill();
        }
        // Once stderr is closed Git is exiting: check every millisecond.
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("Wait for Git {}: {e}", operation.as_str()));
            }
        }
    };
    let _ = reader.join();
    while let Ok(update) = receiver.try_recv() {
        progress(update);
    }
    let detail = tail
        .lock()
        .map(|detail| detail.trim().to_string())
        .unwrap_or_default();
    Ok(GitOutput {
        status,
        cancelled,
        detail,
    })
}

fn read_progress(
    stderr: impl Read,
    operation: Operation,
    sender: mpsc::SyncSender<Progress>,
    tail: Arc<Mutex<String>>,
) {
    let mut reader = BufReader::new(stderr);
    let mut chunk = [0_u8; 4096];
    let mut line = Vec::new();
    let mut last = String::new();
    while let Ok(read) = reader.read(&mut chunk) {
        if read == 0 {
            break;
        }
        for &byte in &chunk[..read] {
            if byte == b'\r' || byte == b'\n' {
                emit_line(&mut line, operation, &sender, &tail, &mut last);
            } else if line.len() < MAX_LINE {
                line.push(byte);
            }
        }
    }
    emit_line(&mut line, operation, &sender, &tail, &mut last);
}

fn emit_line(
    line: &mut Vec<u8>,
    operation: Operation,
    sender: &mpsc::SyncSender<Progress>,
    tail: &Arc<Mutex<String>>,
    last: &mut String,
) {
    if line.is_empty() {
        return;
    }
    let text = String::from_utf8_lossy(line).trim().to_string();
    line.clear();
    if text.is_empty() {
        return;
    }
    append_tail(tail, &text);
    let Some(progress) = parse_progress(operation, &text) else {
        return;
    };
    if progress.message == *last {
        return;
    }
    *last = progress.message.clone();
    let _ = sender.try_send(progress);
}

fn append_tail(tail: &Arc<Mutex<String>>, text: &str) {
    if let Ok(mut tail) = tail.lock() {
        tail.push_str(text);
        tail.push('\n');
        if tail.len() > MAX_DETAIL {
            let mut keep = tail.len() - MAX_DETAIL;
            while !tail.is_char_boundary(keep) {
                keep += 1;
            }
            tail.drain(..keep);
        }
    }
}

fn parse_progress(operation: Operation, text: &str) -> Option<Progress> {
    let percent = text.find('%').and_then(|index| {
        let digits = text[..index]
            .chars()
            .rev()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();
        digits.parse::<u8>().ok().filter(|percent| *percent <= 100)
    });
    let phase = text
        .strip_prefix("remote: ")
        .unwrap_or(text)
        .split(':')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    if phase.is_empty() && percent.is_none() {
        return None;
    }
    let message = truncate(text, MAX_MESSAGE);
    Some(Progress {
        operation,
        phase,
        percent,
        message,
    })
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut truncated = text
        .chars()
        .take(limit.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}

/// Ends the error of a cancelled network operation after its label (`Fetch
/// from origin`): Git was stopped mid-way, so part of it may have happened
/// and nothing was rolled back.
pub const CANCELLED: &str =
    " cancelled. Git may have finished part of it; check what changed before retrying.";

fn finish(output: GitOutput, label: String) -> Result<(), String> {
    if output.cancelled {
        return Err(format!("{label}{CANCELLED}"));
    }
    if output.status.success() {
        return Ok(());
    }
    let detail = if output.detail.is_empty() {
        String::new()
    } else {
        format!("\n{}", output.detail)
    };
    let hint = if needs_credentials(&output.detail) {
        "\nConfigure a Git credential helper or SSH agent, then retry."
    } else {
        ""
    };
    Err(format!("{label} failed ({}){detail}{hint}", output.status))
}

fn needs_credentials(detail: &str) -> bool {
    let detail = detail.to_lowercase();
    [
        "authentication failed",
        "could not read username",
        "could not read password",
        "permission denied",
        "terminal prompts disabled",
        "host key verification failed",
    ]
    .iter()
    .any(|text| detail.contains(text))
}

fn redact_url(url: &str) -> String {
    let Some(scheme) = url.find("://") else {
        return url.to_string();
    };
    let authority = &url[scheme + 3..];
    let Some(at) = authority.find('@') else {
        return url.to_string();
    };
    let userinfo = &authority[..at];
    if !userinfo.contains(':') {
        return url.to_string();
    }
    let Some(colon) = userinfo.find(':') else {
        return url.to_string();
    };
    format!(
        "{}://{}@{}",
        &url[..scheme],
        &userinfo[..colon],
        &authority[at + 1..]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_passwords_but_keeps_url_identity() {
        assert_eq!(
            redact_url("https://user:secret@example.org/gg.git"),
            "https://user@example.org/gg.git"
        );
        assert_eq!(
            redact_url("ssh://git@example.org/gg.git"),
            "ssh://git@example.org/gg.git"
        );
        assert_eq!(
            redact_url("git@example.org:gg.git"),
            "git@example.org:gg.git"
        );
    }

    #[test]
    fn recognizes_actionable_credential_failures() {
        assert!(needs_credentials(
            "fatal: could not read Username for 'https://example.org': terminal prompts disabled"
        ));
        assert!(needs_credentials(
            "git@example.org: Permission denied (publickey)."
        ));
        assert!(!needs_credentials("fatal: repository not found"));
    }

    #[test]
    fn classifies_push_failures_by_the_follow_up_that_helps() {
        assert!(non_fast_forward(
            " ! [rejected]        main -> main (non-fast-forward)\nerror: failed to push some refs"
        ));
        assert!(non_fast_forward(
            " ! [rejected]        main -> main (fetch first)"
        ));
        assert!(!non_fast_forward(
            " ! [remote rejected] main -> main (pre-receive hook declined)"
        ));
        assert_eq!(
            push_failure("Push rejected: origin/main has commits you don't have."),
            PushFailure::Rejected
        );
        for transient in [
            "fatal: unable to access 'https://example.org/gg.git/': Could not resolve host: example.org",
            "ssh: connect to host example.org port 22: Connection timed out",
            "fatal: Authentication failed for 'https://example.org/gg.git/'",
            "fatal: the remote end hung up unexpectedly",
        ] {
            assert_eq!(
                push_failure(transient),
                PushFailure::Transient,
                "{transient}"
            );
        }
        assert_eq!(
            push_failure(" ! [remote rejected] main -> main (pre-receive hook declined)"),
            PushFailure::Other
        );
    }

    #[test]
    fn parses_bounded_progress_phases_and_percentages() {
        let progress = parse_progress(
            Operation::Fetch,
            "remote: Counting objects: 100% (5/5), done.",
        );
        let progress = match progress {
            Some(progress) => progress,
            None => return,
        };
        assert_eq!(progress.phase, "Counting objects");
        assert_eq!(progress.percent, Some(100));
        let long = format!("Receiving objects: 42% {}", "x".repeat(10_000));
        let progress = match parse_progress(Operation::Clone, &long) {
            Some(progress) => progress,
            None => return,
        };
        assert_eq!(progress.percent, Some(42));
        assert_eq!(progress.message.chars().count(), MAX_MESSAGE);
    }
}
