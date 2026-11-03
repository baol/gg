//! Whole-worktree index changes: stage all, unstage all, discard all.
use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::{Command, Output, Stdio},
};

/// Stage every change, including untracked and deleted files (`git add -A`).
pub fn stage_all(path: &Path) -> Result<(), String> {
    check(run(path, &["add", "-A", "--", ":/"])?, "Stage all changes")
}

/// Unstage every staged change, keeping the working files. On an unborn
/// branch every path is removed from the index instead.
pub fn unstage_all(path: &Path) -> Result<(), String> {
    let output = if born(path)? {
        run(path, &["reset", "-q", "--"])?
    } else {
        run(
            path,
            &["rm", "-r", "-q", "--cached", "--ignore-unmatch", "--", ":/"],
        )?
    };
    check(output, "Unstage all changes")
}

/// Restore every file HEAD tracks, in the index and the worktree, to HEAD.
/// Untracked files are kept, and so are files only added to the index: they
/// leave the index and stay on disk as untracked files. Refused while a merge,
/// rebase, cherry-pick, or revert is in progress, since that operation owns
/// the index.
pub fn discard_all(path: &Path) -> Result<(), String> {
    if let Some(state) = crate::mutation::state(path)? {
        return Err(format!(
            "A {} is in progress. Continue, skip, or abort it before discarding changes.",
            state.operation.as_str()
        ));
    }
    if !born(path)? {
        return Err("The repository has no commits yet, so there is nothing to restore.".into());
    }
    // Files new in the index would be deleted by the restore; untrack them
    // first so their bytes stay on disk.
    let added = run(
        path,
        &[
            "diff",
            "--cached",
            "--name-only",
            "-z",
            "--diff-filter=A",
            "--",
            ":/",
        ],
    )?;
    if !added.status.success() {
        return check(added, "List newly added files");
    }
    // Names stay the bytes Git printed, so one that is not UTF-8 still
    // matches its index entry.
    let added = added
        .stdout
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .map(|field| crate::raw_path::os_string(&[b":(top,literal)", field].concat()))
        .collect::<Vec<_>>();
    if !added.is_empty() {
        let root = run(path, &["rev-parse", "--show-toplevel"])?;
        let root = crate::raw_path::line(&root.stdout);
        let mut args = ["rm", "-q", "--cached", "--"].map(OsString::from).to_vec();
        args.extend(added);
        check(run(&root, &args)?, "Keep newly added files")?;
    }
    check(
        run(
            path,
            &[
                "restore",
                "--source=HEAD",
                "--staged",
                "--worktree",
                "--",
                ":/",
            ],
        )?,
        "Discard all changes",
    )
}

fn born(path: &Path) -> Result<bool, String> {
    Ok(run(path, &["rev-parse", "--verify", "--quiet", "HEAD"])?
        .status
        .success())
}

fn run(path: &Path, args: &[impl AsRef<OsStr>]) -> Result<Output, String> {
    Command::new("git")
        .current_dir(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_err(|e| format!("Could not run Git: {e}. Install Git and retry."))
}

fn check(output: Output, label: &str) -> Result<(), String> {
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
