//! Switch and create local branches after enforcing a clean worktree.
use gix::bstr::BStr;
use std::{
    path::Path,
    process::Stdio,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub branch: String,
    pub head: String,
    pub created: bool,
    pub warnings: Vec<String>,
    /// Stash entry created before switching, when the user asked to park changes.
    pub stash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteReceipt {
    pub branch: String,
    pub force: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateReceipt {
    pub branch: String,
    pub commit: String,
}

/// Create a local branch at a revision without switching to it.
pub fn create_at(path: &Path, branch: &str, start: &str) -> Result<CreateReceipt, String> {
    let branch = branch.trim();
    validate_name(branch)?;
    let start = start.trim();
    if start.is_empty() || start.starts_with('-') || start.contains('\0') {
        return Err("Branch start point is invalid.".into());
    }
    let repo = gix::discover(path).map_err(|e| format!("Read repository: {e}"))?;
    let full = format!("refs/heads/{branch}");
    if repo
        .try_find_reference(full.as_str())
        .map_err(|e| format!("Read branch {branch}: {e}"))?
        .is_some()
    {
        return Err(format!(
            "Branch {branch} already exists. Choose another name or switch to it."
        ));
    }
    let resolved = crate::process::git()
        .current_dir(path)
        .args(["rev-parse", "--verify", "--quiet"])
        .arg(format!("{start}^{{commit}}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_err(|e| format!("Could not resolve {start} using Git: {e}. Install Git and retry."))?;
    if !resolved.status.success() {
        return Err(format!("Revision {start} was not found."));
    }
    let commit = String::from_utf8_lossy(&resolved.stdout).trim().to_string();
    let output = crate::process::git()
        .current_dir(path)
        .args(["branch", "--"])
        .arg(branch)
        .arg(&commit)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_err(|e| {
            format!("Could not create branch {branch} using Git: {e}. Install Git and retry.")
        })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "Create branch {branch} failed ({}).{}",
            output.status,
            if detail.is_empty() {
                String::new()
            } else {
                format!("\n{detail}")
            }
        ));
    }
    Ok(CreateReceipt {
        branch: branch.to_string(),
        commit,
    })
}

pub fn switch(path: &Path, branch: &str) -> Result<Receipt, String> {
    run(path, branch, false)
}

/// Park tracked changes in a stash entry, then switch to another local branch.
///
/// The stash entry is kept for the user to restore or drop explicitly.
pub fn switch_with_stash(
    path: &Path,
    branch: &str,
    include_untracked: bool,
) -> Result<Receipt, String> {
    let branch = branch.trim();
    validate_name(branch)?;
    let repo = gix::discover(path).map_err(|e| format!("Read repository: {e}"))?;
    let full = format!("refs/heads/{branch}");
    if repo
        .try_find_reference(full.as_str())
        .map_err(|e| format!("Read branch {branch}: {e}"))?
        .is_none()
    {
        return Err(format!(
            "Local branch {branch} does not exist. Refresh refs or create it first."
        ));
    }
    let message = format!("Git Good: switch to {branch}");
    let stash = crate::stash::push(path, Some(&message), include_untracked, false)?;
    let reference = stash
        .reference
        .clone()
        .unwrap_or_else(|| "stash@{0}".into());
    match run(path, branch, false) {
        Ok(mut receipt) => {
            receipt.stash = Some(reference.clone());
            receipt.warnings.push(format!(
                "Stashed working changes as {reference}. Restore them from Stashes."
            ));
            Ok(receipt)
        }
        Err(error) => Err(format!(
            "Working changes were saved to {reference}, but the switch failed. {error}"
        )),
    }
}

pub fn delete(path: &Path, branch: &str, force: bool) -> Result<DeleteReceipt, String> {
    let branch = branch.trim();
    validate_name(branch)?;
    let session = crate::status::Session::open(path)?;
    crate::status::validate(&session.identity)?;
    if session.identity.head.is_none() {
        return Err("Repository has no commits yet. There are no branches to delete.".into());
    }
    let repo = gix::discover(path).map_err(|e| format!("Read repository: {e}"))?;
    let full = format!("refs/heads/{branch}");
    let head = repo
        .head()
        .map_err(|e| format!("Read HEAD: {e}"))?
        .referent_name()
        .map(ToString::to_string);
    if head.as_deref() == Some(full.as_str()) {
        return Err(format!("Cannot delete the current branch {branch}."));
    }
    if repo
        .try_find_reference(full.as_str())
        .map_err(|e| format!("Read branch {branch}: {e}"))?
        .is_none()
    {
        return Err(format!("Local branch {branch} does not exist."));
    }
    let mut command = crate::process::git();
    command
        .current_dir(&session.identity.path)
        .arg("branch")
        .arg(if force { "-D" } else { "-d" })
        .arg(branch)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    let output = command.output().map_err(|e| {
        format!("Could not run branch deletion using Git: {e}. Install Git and retry.")
    })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "Delete branch {branch} failed ({}).{}",
            output.status,
            if detail.is_empty() {
                String::new()
            } else {
                format!("\n{detail}")
            }
        ));
    }
    Ok(DeleteReceipt {
        branch: branch.to_string(),
        force,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveReceipt {
    /// The new branch, now checked out, at the old tip.
    pub branch: String,
    /// The branch the commits left, now at `base`.
    pub from_branch: String,
    pub base: String,
    pub tip: String,
    /// How many commits moved.
    pub commits: usize,
    /// The backup of `from_branch`'s old tip.
    pub backup: String,
}

/// Move the current branch's own commits, from `from` through HEAD, to a new
/// branch: the new branch starts at HEAD and is checked out, and the current
/// branch goes back to `from`'s parent. Only commits its upstream (or the
/// default branch) lacks can move, so nothing pushed is rewritten. The working
/// tree and index are untouched: HEAD's commit stays the same.
pub fn move_to_new(path: &Path, branch: &str, from: &str) -> Result<MoveReceipt, String> {
    let branch = branch.trim();
    validate_name(branch)?;
    let from = from.trim();
    if from.is_empty() || from.starts_with('-') || from.contains('\0') {
        return Err("The first commit to move is invalid.".into());
    }
    let current =
        crate::backup::current_branch(path).ok_or("Switch to a branch first: HEAD is detached.")?;
    if ref_exists(path, &format!("refs/heads/{branch}"))? {
        return Err(format!("A branch named {branch} already exists."));
    }
    let tip = git_out(
        path,
        &["rev-parse", "--verify", "HEAD^{commit}"],
        "Read HEAD",
    )?;
    let from = git_out(
        path,
        &["rev-parse", "--verify", &format!("{from}^{{commit}}")],
        "Read the first commit to move",
    )?;
    let (start, _) = crate::rewrite::branch_start(path)?.ok_or(format!(
        "Every commit on {current} is already on its upstream; there is nothing to move."
    ))?;
    let within = |older: &str, newer: &str| {
        crate::process::git()
            .current_dir(path)
            .args(["merge-base", "--is-ancestor", older, newer])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    };
    if !within(&from, &tip) || !within(&start, &from) {
        return Err(format!(
            "Only commits on {current} that are not pushed can move to a new branch."
        ));
    }
    let base = git_out(
        path,
        &["rev-parse", "--verify", &format!("{from}^")],
        "Read the commit before the first one to move",
    )
    .map_err(|_| "The first commit of the repository cannot move: nothing comes before it.")?;
    let commits = git_out(
        path,
        &["rev-list", "--count", &format!("{base}..{tip}")],
        "Count the commits to move",
    )?
    .parse()
    .unwrap_or(0);
    let backup = crate::backup::create(path, "move", &tip)?;
    git(
        path,
        &["switch", "-c", branch],
        "Create and switch to the new branch",
    )?;
    git(
        path,
        &["branch", "-f", "--", &current, &base],
        "Move the old branch back",
    )?;
    Ok(MoveReceipt {
        branch: branch.to_string(),
        from_branch: current,
        base,
        tip,
        commits,
        backup,
    })
}

pub fn create(path: &Path, branch: &str) -> Result<Receipt, String> {
    run(path, branch, true)
}

/// Create a local branch from a remote-tracking ref and configure its upstream.
pub fn track_remote(path: &Path, remote_branch: &str) -> Result<Receipt, String> {
    let (remote, branch) = parse_remote_branch(remote_branch)?;
    validate_name(&branch)?;
    let session = crate::status::Session::open(path)?;
    crate::status::validate(&session.identity)?;
    if session.identity.head.is_none() {
        return Err(
            "Repository has no commits yet. Fetch a remote branch before tracking it.".into(),
        );
    }
    ensure_clean(&session)?;
    let repo = gix::discover(path).map_err(|e| format!("Read repository: {e}"))?;
    let remote_exists = repo
        .remote_names()
        .iter()
        .any(|name| <gix::bstr::BString as AsRef<[u8]>>::as_ref(name) == remote.as_bytes());
    if !remote_exists {
        return Err(format!(
            "Remote {remote} is not configured. Add it or choose another remote branch."
        ));
    }
    let remote_full = format!("refs/remotes/{remote}/{branch}");
    if repo
        .try_find_reference(remote_full.as_str())
        .map_err(|e| format!("Read remote branch {remote}/{branch}: {e}"))?
        .is_none()
    {
        return Err(format!(
            "Remote branch {remote}/{branch} does not exist. Fetch it first."
        ));
    }
    let local_full = format!("refs/heads/{branch}");
    if repo
        .try_find_reference(local_full.as_str())
        .map_err(|e| format!("Read branch {branch}: {e}"))?
        .is_some()
    {
        return Err(format!(
            "Local branch {branch} already exists. Switch to it or choose another remote branch."
        ));
    }
    let mut command = crate::process::git();
    command
        .current_dir(&session.identity.path)
        .arg("switch")
        .arg("--track")
        .arg(format!("{remote}/{branch}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    let output = command.output().map_err(|e| {
        format!("Could not run remote branch checkout using Git: {e}. Install Git and retry.")
    })?;
    complete(path, output, &branch, true, "remote tracking checkout")
}

fn run(path: &Path, branch: &str, create: bool) -> Result<Receipt, String> {
    let branch = branch.trim();
    validate_name(branch)?;
    let session = crate::status::Session::open(path)?;
    crate::status::validate(&session.identity)?;
    if session.identity.head.is_none() {
        return Err(
            "Repository has no commits yet. Create the first commit before using branches.".into(),
        );
    }
    ensure_clean(&session)?;
    let repo = gix::discover(path).map_err(|e| format!("Read repository: {e}"))?;
    let full = format!("refs/heads/{branch}");
    let exists = repo
        .try_find_reference(full.as_str())
        .map_err(|e| format!("Read branch {branch}: {e}"))?
        .is_some();
    if create && exists {
        return Err(format!(
            "Branch {branch} already exists. Choose another name or switch to it."
        ));
    }
    if !create && !exists {
        return Err(format!(
            "Local branch {branch} does not exist. Refresh refs or create it first."
        ));
    }
    let mut command = crate::process::git();
    command
        .current_dir(&session.identity.path)
        .arg("switch")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    if create {
        command.arg("-c").arg(branch);
    } else {
        command.arg(branch);
    }
    let output = command.output().map_err(|e| {
        format!(
            "Could not run branch {operation} using Git: {e}. Install Git and retry.",
            operation = if create { "creation" } else { "switch" }
        )
    })?;
    complete(
        path,
        output,
        branch,
        create,
        if create { "creation" } else { "switch" },
    )
}

fn complete(
    path: &Path,
    output: std::process::Output,
    branch: &str,
    created: bool,
    operation: &str,
) -> Result<Receipt, String> {
    let detail = [output.stdout, output.stderr]
        .into_iter()
        .filter(|bytes| !bytes.is_empty())
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .collect::<Vec<_>>()
        .join("\n");
    let detail = detail.trim();
    let updated = crate::status::Session::open(path)?;
    let repo = gix::discover(path).map_err(|e| format!("Read repository: {e}"))?;
    let current_ref = repo
        .head()
        .map_err(|e| format!("Read HEAD: {e}"))?
        .referent_name()
        .map(ToString::to_string);
    let target_full = format!("refs/heads/{branch}");
    let switched = current_ref.as_deref() == Some(target_full.as_str());
    if !output.status.success() && !switched {
        return Err(format!(
            "Branch {operation} failed ({}). Review the current branch and hook output before retrying.{}{}",
            output.status,
            if detail.is_empty() { "" } else { "\n" },
            detail,
        ));
    }
    let head = updated
        .identity
        .head
        .ok_or("Branch switch succeeded but HEAD is unborn. Refresh status.")?;
    let warnings = if output.status.success() {
        vec![]
    } else {
        vec![format!(
            "Branch {operation} completed, but its post-checkout hook failed ({}). Review the hook output before continuing.{}{}",
            output.status,
            if detail.is_empty() { "" } else { "\n" },
            detail,
        )]
    };
    Ok(Receipt {
        branch: branch.to_string(),
        head,
        created,
        warnings,
        stash: None,
    })
}

fn parse_remote_branch(remote_branch: &str) -> Result<(String, String), String> {
    let remote_branch = remote_branch.trim();
    let short = remote_branch
        .strip_prefix("refs/remotes/")
        .unwrap_or(remote_branch);
    let (remote, branch) = short.split_once('/').ok_or_else(|| {
        "Remote branch must look like origin/main or refs/remotes/origin/main.".to_string()
    })?;
    if remote.is_empty() || branch.is_empty() {
        return Err("Remote branch must look like origin/main or refs/remotes/origin/main.".into());
    }
    Ok((remote.to_string(), branch.to_string()))
}

fn validate_name(branch: &str) -> Result<(), String> {
    if branch.is_empty() {
        return Err("Branch name is empty.".into());
    }
    if branch.starts_with('-') {
        return Err("Branch name cannot start with '-'.".into());
    }
    let full = format!("refs/heads/{branch}");
    gix::validate::reference::branch_name(BStr::new(&full))
        .map(|_| ())
        .map_err(|e| format!("Invalid branch name: {e}"))
}

fn ensure_clean(session: &crate::status::Session) -> Result<(), String> {
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
            "Working tree is not clean ({path}). Commit, stash, or discard changes before switching or creating a branch."
        ));
    }
    Ok(())
}

/// Rename local branch `old` to `new` (`git branch -m`), refusing an existing
/// target name.
pub fn rename(path: &Path, old: &str, new: &str) -> Result<(), String> {
    let old = old.trim();
    let new = new.trim();
    validate_name(old)?;
    validate_name(new)?;
    if old == new {
        return Err("The new branch name is the same as the current one.".into());
    }
    if !ref_exists(path, &format!("refs/heads/{old}"))? {
        return Err(format!("Branch {old} does not exist."));
    }
    if ref_exists(path, &format!("refs/heads/{new}"))? {
        return Err(format!("Branch {new} already exists. Choose another name."));
    }
    git(
        path,
        &["branch", "-m", "--", old, new],
        &format!("Rename branch {old}"),
    )
}

/// Make `upstream` (a remote-tracking branch such as `origin/main`, or a local
/// branch) the upstream of local `branch`.
pub fn set_upstream(path: &Path, branch: &str, upstream: &str) -> Result<(), String> {
    let branch = branch.trim();
    let upstream = upstream.trim();
    validate_name(branch)?;
    if upstream.is_empty() || upstream.starts_with('-') || upstream.contains('\0') {
        return Err("Upstream branch is empty or invalid.".into());
    }
    if !ref_exists(path, &format!("refs/heads/{branch}"))? {
        return Err(format!("Branch {branch} does not exist."));
    }
    if !ref_exists(path, &format!("refs/remotes/{upstream}"))?
        && !ref_exists(path, &format!("refs/heads/{upstream}"))?
    {
        return Err(format!(
            "Upstream {upstream} is not a remote-tracking or local branch. Fetch it first."
        ));
    }
    git(
        path,
        &[
            "branch",
            &format!("--set-upstream-to={upstream}"),
            "--",
            branch,
        ],
        &format!("Set the upstream of {branch}"),
    )
}

/// Remove the upstream of local `branch`.
pub fn unset_upstream(path: &Path, branch: &str) -> Result<(), String> {
    let branch = branch.trim();
    validate_name(branch)?;
    if !ref_exists(path, &format!("refs/heads/{branch}"))? {
        return Err(format!("Branch {branch} does not exist."));
    }
    git(
        path,
        &["branch", "--unset-upstream", "--", branch],
        &format!("Remove the upstream of {branch}"),
    )
}

fn ref_exists(path: &Path, full: &str) -> Result<bool, String> {
    crate::process::git()
        .current_dir(path)
        .args(["show-ref", "--verify", "--quiet", full])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .status()
        .map(|status| status.success())
        .map_err(|e| format!("Could not run Git: {e}. Install Git and retry."))
}

/// [`git`], returning what Git printed, trimmed.
fn git_out(path: &Path, args: &[&str], label: &str) -> Result<String, String> {
    let output = crate::process::git()
        .current_dir(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_err(|e| format!("Could not run Git: {e}. Install Git and retry."))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(format!("{label} failed ({}). {detail}", output.status))
}

fn git(path: &Path, args: &[&str], label: &str) -> Result<(), String> {
    let output = crate::process::git()
        .current_dir(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_err(|e| format!("Could not run Git: {e}. Install Git and retry."))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if detail.is_empty() {
        format!("{label} failed ({}).", output.status)
    } else {
        format!("{label} failed ({}).\n{detail}", output.status)
    })
}
