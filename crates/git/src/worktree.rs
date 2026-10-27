//! Worktree inventory and explicit add/remove through Git.
use gg_engine::worktree::{AddRequest, Receipt, Snapshot, Worktree};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn command(path: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(path)
        .stdin(Stdio::null())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    command
}

fn output(path: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    command(path)
        .args(args)
        .output()
        .map_err(|error| format!("Could not run Git: {error}. Install Git and retry."))
}

fn detail(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

/// List every registered worktree, newest Git order (main first).
pub fn list(path: &Path) -> Result<Snapshot, String> {
    let output = output(path, &["worktree", "list", "--porcelain", "-z"])?;
    if !output.status.success() {
        return Err(format!(
            "List worktrees failed ({}).{}",
            output.status,
            match detail(&output) {
                detail if detail.is_empty() => String::new(),
                detail => format!("\n{detail}"),
            }
        ));
    }
    let current = std::fs::canonicalize(path).ok();
    let mut worktrees = Vec::new();
    let mut fields: Vec<Vec<u8>> = Vec::new();
    for line in output.stdout.split(|byte| *byte == 0) {
        if line.is_empty() {
            if !fields.is_empty() {
                worktrees.push(parse(&fields, worktrees.is_empty(), current.as_deref())?);
                fields.clear();
            }
        } else {
            fields.push(line.to_vec());
        }
    }
    if !fields.is_empty() {
        worktrees.push(parse(&fields, worktrees.is_empty(), current.as_deref())?);
    }
    for worktree in &mut worktrees {
        if worktree.bare {
            worktree.dirty = None;
            continue;
        }
        let worktree_path = path_from_bytes(&worktree.path_bytes);
        let status = command(&worktree_path)
            .args(["status", "--porcelain", "-z"])
            .output();
        worktree.dirty = status
            .ok()
            .and_then(|output| output.status.success().then_some(!output.stdout.is_empty()));
    }
    for worktree in &mut worktrees {
        worktree.current = current.as_ref().is_some_and(|current| {
            std::fs::canonicalize(path_from_bytes(&worktree.path_bytes))
                .is_ok_and(|candidate| candidate == *current)
        });
    }
    Ok(Snapshot { worktrees })
}

fn parse(fields: &[Vec<u8>], main: bool, current: Option<&Path>) -> Result<Worktree, String> {
    let mut path_bytes = None;
    let mut head = None;
    let mut branch = None;
    let mut detached = false;
    let mut bare = false;
    let mut locked = None;
    let mut prunable = None;
    for field in fields {
        let Some(space) = field.iter().position(|byte| *byte == b' ') else {
            match field.as_slice() {
                b"detached" => detached = true,
                b"bare" => bare = true,
                b"locked" => locked = Some(String::new()),
                b"prunable" => prunable = Some(String::new()),
                _ => {}
            }
            continue;
        };
        let (key, value) = (&field[..space], &field[space + 1..]);
        match key {
            b"worktree" => path_bytes = Some(value.to_vec()),
            b"HEAD" => head = Some(String::from_utf8_lossy(value).into_owned()),
            b"branch" => {
                branch = Some(
                    String::from_utf8_lossy(value)
                        .strip_prefix("refs/heads/")
                        .unwrap_or_default()
                        .to_string(),
                );
            }
            b"locked" => locked = Some(String::from_utf8_lossy(value).into_owned()),
            b"prunable" => prunable = Some(String::from_utf8_lossy(value).into_owned()),
            _ => {}
        }
    }
    let path_bytes = path_bytes.ok_or("Worktree listing had no path.")?;
    let worktree_path = path_from_bytes(&path_bytes);
    let current = current.is_some_and(|current| {
        std::fs::canonicalize(&worktree_path).is_ok_and(|candidate| candidate == current)
    });
    Ok(Worktree {
        path: gg_engine::review::display_path(&path_bytes),
        path_bytes,
        head,
        branch,
        detached,
        bare,
        locked,
        prunable,
        current,
        main,
        dirty: None,
    })
}

fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(String::from_utf8_lossy(bytes).replace('/', std::path::MAIN_SEPARATOR_STR))
    }
}

fn valid_name(value: &str) -> bool {
    !value.is_empty() && !value.starts_with('-') && !value.contains('\0')
}

/// Add one linked worktree from an existing or new branch.
pub fn add(repo: &Path, request: &AddRequest) -> Result<Receipt, String> {
    if request.destination.as_os_str().is_empty() {
        return Err("Choose a destination for the worktree.".into());
    }
    let destination = if request.destination.is_absolute() {
        request.destination.clone()
    } else {
        repo.join(&request.destination)
    };
    if request.branch.is_some() == request.create.is_some() {
        return Err("Choose either an existing branch or a new branch.".into());
    }
    if let Some(branch) = &request.branch
        && !valid_name(branch)
    {
        return Err("Worktree branch name is invalid.".into());
    }
    if let Some(branch) = &request.create
        && !valid_name(branch)
    {
        return Err("New branch name is invalid.".into());
    }
    if let Ok(metadata) = std::fs::symlink_metadata(&destination) {
        if !metadata.is_dir() {
            return Err("Worktree destination is not a directory.".into());
        }
        if std::fs::read_dir(&destination)
            .map_err(|error| error.to_string())?
            .next()
            .is_some()
        {
            return Err("Worktree destination is not empty.".into());
        }
    } else if destination
        .parent()
        .is_some_and(|parent| !parent.as_os_str().is_empty() && !parent.exists())
    {
        return Err("Worktree destination parent does not exist.".into());
    }
    let snapshot = list(repo)?;
    if let Some(branch) = &request.branch
        && let Some(existing) = snapshot
            .worktrees
            .iter()
            .find(|worktree| worktree.branch.as_deref() == Some(branch))
    {
        return Err(format!(
            "Branch {branch} is already checked out at {}.",
            existing.path
        ));
    }
    if let Some(branch) = &request.create
        && let Some(existing) = snapshot
            .worktrees
            .iter()
            .find(|worktree| worktree.branch.as_deref() == Some(branch))
    {
        return Err(format!(
            "Branch {branch} already exists at {}.",
            existing.path
        ));
    }
    if let Some(branch) = &request.create {
        let show = output(
            repo,
            &[
                "show-ref",
                "--verify",
                "--quiet",
                &format!("refs/heads/{branch}"),
            ],
        )?;
        if show.status.success() {
            return Err(format!("Branch {branch} already exists."));
        }
    }
    let mut add = command(repo);
    add.arg("worktree").arg("add");
    if let Some(branch) = &request.create {
        add.arg("-b").arg(branch);
    }
    add.arg("--").arg(&destination);
    if let Some(branch) = &request.branch {
        add.arg(branch);
    } else if let Some(at) = &request.at {
        add.arg(at);
    }
    let result = add
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("Could not add a worktree using Git: {error}"))?;
    if !result.status.success() {
        return Err(format!(
            "Add worktree failed ({}).{}",
            result.status,
            match detail(&result) {
                detail if detail.is_empty() => String::new(),
                detail => format!("\n{detail}"),
            }
        ));
    }
    let snapshot = list(repo)?;
    let created = snapshot
        .worktrees
        .iter()
        .find(|worktree| {
            std::fs::canonicalize(path_from_bytes(&worktree.path_bytes))
                .ok()
                .zip(std::fs::canonicalize(&destination).ok())
                .is_some_and(|(left, right)| left == right)
        })
        .or_else(|| snapshot.worktrees.last());
    created
        .map(Receipt::from_worktree)
        .ok_or_else(|| "Worktree was added but could not be listed.".into())
}

/// Remove one clean linked worktree without force or file deletion.
pub fn remove(repo: &Path, destination: &Path) -> Result<Receipt, String> {
    let snapshot = list(repo)?;
    let worktree = snapshot
        .worktrees
        .iter()
        .find(|worktree| {
            std::fs::canonicalize(path_from_bytes(&worktree.path_bytes))
                .ok()
                .zip(std::fs::canonicalize(destination).ok())
                .is_some_and(|(left, right)| left == right)
                || path_from_bytes(&worktree.path_bytes) == destination
        })
        .ok_or("Worktree is not registered.")?;
    worktree.can_remove()?;
    let result = command(repo)
        .args(["worktree", "remove", "--"])
        .arg(destination)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("Could not remove the worktree using Git: {error}"))?;
    if !result.status.success() {
        return Err(format!(
            "Remove worktree failed ({}).{}",
            result.status,
            match detail(&result) {
                detail if detail.is_empty() => String::new(),
                detail => format!("\n{detail}"),
            }
        ));
    }
    Ok(Receipt::from_worktree(worktree))
}
