//! Reset the current branch to a commit.
use gg_engine::reset::{Mode, Receipt};
use std::{
    collections::BTreeSet,
    path::Path,
    process::{Command, Output, Stdio},
};

/// Reset the current branch to `commit`. The tip it leaves is first saved as
/// a `reset` backup; when that cannot be written nothing is reset.
pub fn reset(path: &Path, commit: &str, mode: Mode) -> Result<Receipt, String> {
    reset_with(path, commit, mode, true)
}

/// [`reset`] for a caller that has already saved its own backup of the tip.
pub(crate) fn reset_after_backup(path: &Path, commit: &str, mode: Mode) -> Result<Receipt, String> {
    reset_with(path, commit, mode, false)
}

fn reset_with(path: &Path, commit: &str, mode: Mode, save: bool) -> Result<Receipt, String> {
    let commit = commit.trim();
    if commit.is_empty() || commit.starts_with('-') {
        return Err("Reset target must be a revision that does not start with '-'.".into());
    }
    let resolved = resolve(path, commit)?;
    if mode == Mode::Hard {
        let overwritten = overwritten_untracked(path, &resolved)?;
        if !overwritten.is_empty() {
            return Err(format!(
                "Hard reset would overwrite untracked files. Move or delete them, then retry:\n{}",
                overwritten.join("\n")
            ));
        }
    }
    // An unborn branch has no tip to save.
    let backup = match save.then(|| resolve(path, "HEAD")) {
        Some(Ok(previous)) => Some(crate::backup::create(path, "reset", &previous)?),
        _ => None,
    };
    let output = run(
        path,
        &[
            "reset".into(),
            format!("--{}", mode.as_str()),
            resolved.clone(),
        ],
    )?;
    if !output.status.success() {
        return Err(failure("Reset", &output));
    }
    Ok(Receipt {
        commit: resolved,
        mode,
        applied: true,
        backup,
    })
}

/// Untracked or ignored working paths that a hard reset to `commit` would
/// overwrite: files of the commit's tree that are not in the index but exist
/// on disk, and untracked files where the tree has a folder. Sorted,
/// repository-relative, with `/` separators.
pub fn overwritten_untracked(path: &Path, commit: &str) -> Result<Vec<String>, String> {
    let commit = commit.trim();
    if commit.is_empty() || commit.starts_with('-') {
        return Err("Reset target must be a revision that does not start with '-'.".into());
    }
    let resolved = resolve(path, commit)?;
    let tree = run(
        path,
        &[
            "ls-tree".into(),
            "-r".into(),
            "-z".into(),
            "--full-tree".into(),
            resolved,
        ],
    )?;
    if !tree.status.success() {
        return Err(failure("Reading the reset target", &tree));
    }
    let index = run(
        path,
        &["ls-files".into(), "-z".into(), "--full-name".into()],
    )?;
    if !index.status.success() {
        return Err(failure("Reading the index", &index));
    }
    let top = run(path, &["rev-parse".into(), "--show-toplevel".into()])?;
    if !top.status.success() {
        return Err(failure("Finding the working tree", &top));
    }
    // Paths stay the bytes Git printed until they are shown, so a name that
    // is not UTF-8, or a working tree whose name ends with a space, is still
    // the file on disk.
    let top = crate::raw_path::line(&top.stdout);
    let on_disk = |name: &[u8]| std::fs::symlink_metadata(top.join(crate::raw_path::path(name)));
    let indexed = nul_separated(&index.stdout).collect::<BTreeSet<_>>();
    let mut overwritten = BTreeSet::new();
    for entry in nul_separated(&tree.stdout) {
        // `<mode> <type> <object>\t<path>`; submodules (`commit`) are skipped.
        let Some(tab) = entry.iter().position(|byte| *byte == b'\t') else {
            continue;
        };
        let (meta, name) = (&entry[..tab], &entry[tab + 1..]);
        if meta.split(|byte| *byte == b' ').nth(1) != Some(&b"blob"[..]) || indexed.contains(name) {
            continue;
        }
        if on_disk(name).is_ok() {
            overwritten.insert(name);
        }
        let mut parent = name;
        while let Some(slash) = parent.iter().rposition(|byte| *byte == b'/') {
            parent = &parent[..slash];
            if !indexed.contains(parent) && on_disk(parent).is_ok_and(|meta| !meta.is_dir()) {
                overwritten.insert(parent);
            }
        }
    }
    Ok(overwritten
        .into_iter()
        .map(|name| String::from_utf8_lossy(name).into_owned())
        .collect())
}

fn nul_separated(bytes: &[u8]) -> impl Iterator<Item = &[u8]> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
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
    )?;
    if !output.status.success() {
        return Err(format!("Commit {commit} was not found."));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
        .map_err(|error| format!("Could not run Git reset. Install Git and retry: {error}"))
}

fn failure(label: &str, output: &Output) -> String {
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if detail.is_empty() {
        format!("{label} failed ({}).", output.status)
    } else {
        format!("{label} failed ({}).\n{detail}", output.status)
    }
}
