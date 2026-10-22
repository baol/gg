//! Switch and create local branches after enforcing a clean worktree.
use gix::bstr::BStr;
use std::{
    path::Path,
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub branch: String,
    pub head: String,
    pub created: bool,
    pub warnings: Vec<String>,
}

pub fn switch(path: &Path, branch: &str) -> Result<Receipt, String> {
    run(path, branch, false)
}

pub fn create(path: &Path, branch: &str) -> Result<Receipt, String> {
    run(path, branch, true)
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
    let mut command = Command::new("git");
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
            operation = if create { "creation" } else { "switch" }
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
            operation = if create { "creation" } else { "switch" }
        )]
    };
    Ok(Receipt {
        branch: branch.to_string(),
        head,
        created: create,
        warnings,
    })
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
