//! New repositories: `git init -b <branch>`, optionally committing what the
//! folder already holds.
use std::{
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

/// What [`init`] made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    /// The new repository's top-level folder.
    pub path: PathBuf,
    pub branch: String,
    /// The `Initial commit` when the existing files were committed.
    pub head: Option<String>,
    /// Files in that commit; 0 without one.
    pub files: usize,
}

/// Files a folder holds, for the New repository dialog's preview.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Count {
    /// Every file below the folder.
    pub files: usize,
    /// Of those, the ones `.gitignore` excludes.
    pub ignored: usize,
}

/// The repository that already contains `path` (or its nearest existing
/// ancestor), as its top-level folder.
pub fn enclosing_repository(path: &Path) -> Option<PathBuf> {
    let existing = path.ancestors().find(|folder| folder.is_dir())?;
    let output = git(existing, &[], &["rev-parse", "--show-toplevel"]).ok()?;
    if !output.status.success() {
        // A bare repository or a `.git` folder has no top level.
        let inside = git(existing, &[], &["rev-parse", "--git-dir"]).ok()?;
        return inside.status.success().then(|| existing.to_path_buf());
    }
    let top = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!top.is_empty()).then(|| PathBuf::from(top))
}

/// Initialize `path` on `branch`, creating the folder when it is missing,
/// and with `commit_existing` add and commit everything not ignored as
/// `Initial commit`. Refuses, changing nothing, when `path` is already
/// inside a repository.
pub fn init(path: &Path, branch: &str, commit_existing: bool) -> Result<Receipt, String> {
    init_configured(path, branch, commit_existing, &[])
}

/// [`init`] with extra `-c key=value` settings on every Git call, such as
/// an identity for tests.
pub fn init_configured(
    path: &Path,
    branch: &str,
    commit_existing: bool,
    config: &[(&str, &str)],
) -> Result<Receipt, String> {
    let branch = branch.trim();
    if branch.is_empty() {
        return Err("Name the first branch, such as main.".into());
    }
    if !path.is_absolute() {
        return Err(format!(
            "Choose an absolute folder for the new repository, not {}.",
            path.display()
        ));
    }
    if let Some(repository) = enclosing_repository(path) {
        return Err(format!(
            "{} is inside the repository {}. Open it instead?",
            path.display(),
            repository.display()
        ));
    }
    let existing = path
        .ancestors()
        .find(|folder| folder.is_dir())
        .unwrap_or(path);
    let valid = git(existing, config, &["check-ref-format", "--branch", branch]);
    if !valid.is_ok_and(|output| output.status.success()) {
        return Err(format!("{branch} is not a valid branch name."));
    }
    std::fs::create_dir_all(path).map_err(|error| format!("Create {}: {error}", path.display()))?;
    let output = git(path, config, &["init", "-q", "-b", branch])?;
    if !output.status.success() {
        return Err(failure("Initialize repository", &output));
    }
    let top = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut receipt = Receipt {
        path: top,
        branch: branch.to_string(),
        head: None,
        files: 0,
    };
    if !commit_existing {
        return Ok(receipt);
    }
    let output = git(path, config, &["add", "-A"])?;
    if !output.status.success() {
        return Err(failure("Add the existing files", &output));
    }
    let staged = git(path, config, &["diff", "--cached", "--name-only", "-z"])?;
    receipt.files = staged
        .stdout
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .count();
    if receipt.files == 0 {
        return Ok(receipt);
    }
    let output = git(path, config, &["commit", "-q", "-m", "Initial commit"])?;
    if !output.status.success() {
        return Err(failure(
            "The repository was created, but committing the existing files",
            &output,
        ));
    }
    let head = git(path, config, &["rev-parse", "HEAD"])?;
    receipt.head = head
        .status
        .success()
        .then(|| String::from_utf8_lossy(&head.stdout).trim().to_string());
    Ok(receipt)
}

/// Count the files below `path` and how many of them `.gitignore` excludes,
/// without creating a repository there.
pub fn count_files(path: &Path) -> Result<Count, String> {
    let scratch = std::env::temp_dir().join(format!(
        "gg-count-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos())
    ));
    let scratch_arg = scratch.to_string_lossy().into_owned();
    let made = git(path, &[], &["init", "-q", "--bare", &scratch_arg]);
    let result = match made {
        Ok(output) if output.status.success() => {
            let list = |ignored: bool| -> Result<usize, String> {
                let mut args = vec![
                    format!("--git-dir={}", scratch.display()),
                    format!("--work-tree={}", path.display()),
                    "ls-files".into(),
                    "-z".into(),
                    "--others".into(),
                    "--exclude-standard".into(),
                ];
                if ignored {
                    args.push("--ignored".into());
                }
                let args = args.iter().map(String::as_str).collect::<Vec<_>>();
                let output = git(path, &[], &args)?;
                if !output.status.success() {
                    return Err(failure("Count files", &output));
                }
                Ok(output
                    .stdout
                    .split(|byte| *byte == 0)
                    .filter(|name| !name.is_empty())
                    .count())
            };
            let kept = list(false)?;
            let ignored = list(true)?;
            Ok(Count {
                files: kept + ignored,
                ignored,
            })
        }
        Ok(output) => Err(failure("Count files", &output)),
        Err(error) => Err(error),
    };
    let _ = std::fs::remove_dir_all(&scratch);
    result
}

fn git(path: &Path, config: &[(&str, &str)], args: &[&str]) -> Result<Output, String> {
    let mut command = Command::new("git");
    command.current_dir(path);
    for (key, value) in config {
        command.arg("-c").arg(format!("{key}={value}"));
    }
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_err(|error| format!("Could not run Git. Install Git and retry: {error}"))
}

fn failure(label: &str, output: &Output) -> String {
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if detail.is_empty() {
        format!("{label} failed ({}).", output.status)
    } else {
        format!("{label} failed ({}).\n{detail}", output.status)
    }
}
