//! Create commits from the index, running the configured Git hooks.
use crate::patch;
use gg_engine::status::StatusIdentity;
use gix::{
    bstr::{BString, ByteSlice},
    objs::{
        Tree,
        tree::{Entry, EntryMode},
    },
};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    path::Path,
    process::{Command, Stdio},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub id: String,
    pub tree: String,
    pub parents: Vec<String>,
    pub warnings: Vec<String>,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Create one commit from the current index after validating the observed HEAD.
pub fn create(repository: &StatusIdentity, message: &str) -> std::result::Result<Receipt, String> {
    create_inner(repository, message).map_err(|e| format!("Commit: {e}"))
}

fn create_inner(repository: &StatusIdentity, message: &str) -> Result<Receipt> {
    let message = normalize_message(message)?;
    crate::status::validate(repository)?;
    let repo = gix::discover(&repository.path)?;
    let worktree = repo
        .workdir()
        .ok_or("Commit requires a working tree")?
        .to_path_buf();
    let initial_head = head_id(&repo)?;
    let mut index = (**repo.index_or_empty()?).clone();
    check_index(&index)?;
    let initial_tree = tree_from_index(&repo, &index)?;
    if initial_tree == repo.head_tree_id_or_empty()?.detach() {
        return Err("No staged changes to commit. Stage a file or hunk first.".into());
    }
    let message_path = repo.git_dir().join("COMMIT_EDITMSG");
    write_message(&message_path, &message)?;

    run_hook(&worktree, "pre-commit", &[])?;
    ensure_head(&repo, initial_head.as_deref())?;
    run_hook(
        &worktree,
        "prepare-commit-msg",
        &[message_path.as_os_str(), OsStr::new("message")],
    )?;
    ensure_head(&repo, initial_head.as_deref())?;
    run_hook(&worktree, "commit-msg", &[message_path.as_os_str()])?;
    let message = read_message(&message_path)?;

    let lock = patch::acquire(&repo)?;
    ensure_head(&repo, initial_head.as_deref())?;
    index = (**repo.index_or_empty()?).clone();
    check_index(&index)?;
    let tree = tree_from_index(&repo, &index)?;
    let head = repo.head()?.id().map(|id| id.detach());
    if tree == repo.head_tree_id_or_empty()?.detach() {
        return Err(
            "No staged changes remain after the hooks. Review status and retry the commit.".into(),
        );
    }
    let author = repo
        .author()
        .ok_or(
            "Author identity is not configured. Set user.name/user.email or GIT_AUTHOR_NAME/EMAIL.",
        )?
        .map_err(|e| format!("Read author identity: {e}"))?;
    let committer = repo
        .committer()
        .ok_or("Committer identity is not configured. Set user.name/user.email or GIT_COMMITTER_NAME/EMAIL.")?
        .map_err(|e| format!("Read committer identity: {e}"))?;
    let parents = head.into_iter().collect::<Vec<_>>();
    let id = repo
        .commit_as(committer, author, "HEAD", &message, tree, parents)?
        .detach();
    drop(lock);

    let warnings = match run_hook(&worktree, "post-commit", &[]) {
        Ok(()) => vec![],
        Err(e) => vec![format!(
            "The commit was created, but its post-commit hook failed. Inspect the commit and fix the hook before retrying.\n{e}"
        )],
    };
    Ok(Receipt {
        id: id.to_string(),
        tree: tree.to_string(),
        parents: head.iter().map(ToString::to_string).collect(),
        warnings,
    })
}

fn head_id(repo: &gix::Repository) -> Result<Option<String>> {
    Ok(repo.head()?.id().map(|id| id.detach().to_string()))
}

fn ensure_head(repo: &gix::Repository, expected: Option<&str>) -> Result<()> {
    if head_id(repo)?.as_deref() != expected {
        return Err(
            "HEAD changed while the commit hooks were running. Review status and retry.".into(),
        );
    }
    Ok(())
}

fn check_index(index: &gix::index::State) -> Result<()> {
    if index.is_sparse() {
        return Err("Commit requires an expanded index".into());
    }
    for entry in index.entries() {
        if entry.stage() != gix::index::entry::Stage::Unconflicted {
            return Err("Unmerged paths must be resolved before committing.".into());
        }
        if entry
            .flags
            .contains(gix::index::entry::Flags::INTENT_TO_ADD)
        {
            return Err(
                "Intent-to-add paths have no staged content. Stage or remove them before committing."
                    .into(),
            );
        }
        if entry.mode.bits() == 0 {
            return Err(
                "The index contains an entry without a usable mode. Refresh status.".into(),
            );
        }
    }
    Ok(())
}

#[derive(Default)]
struct Directory {
    files: BTreeMap<BString, (u32, gix::ObjectId)>,
    directories: BTreeMap<BString, Directory>,
}

fn tree_from_index(repo: &gix::Repository, index: &gix::index::State) -> Result<gix::ObjectId> {
    let mut root = Directory::default();
    for entry in index.entries() {
        let path = entry.path(index);
        let mut parts = path.split(|byte| *byte == b'/').peekable();
        let mut directory = &mut root;
        while let Some(part) = parts.next() {
            if part.is_empty() {
                return Err(format!("Invalid index path: {}", path.as_bstr()).into());
            }
            let name = BString::from(part);
            if parts.peek().is_some() {
                directory = directory.directories.entry(name).or_default();
            } else {
                directory.files.insert(name, (entry.mode.bits(), entry.id));
            }
        }
    }
    write_directory(repo, &root)
}

fn write_directory(repo: &gix::Repository, directory: &Directory) -> Result<gix::ObjectId> {
    let mut entries = Vec::with_capacity(directory.files.len() + directory.directories.len());
    for (name, child) in &directory.directories {
        entries.push(Entry {
            mode: EntryMode::try_from(0o040000u32)
                .map_err(|mode| format!("Invalid mode {mode:o}"))?,
            filename: name.clone(),
            oid: write_directory(repo, child)?,
        });
    }
    for (name, (mode, id)) in &directory.files {
        entries.push(Entry {
            mode: EntryMode::try_from(*mode)
                .map_err(|mode| format!("Invalid index mode {mode:o}"))?,
            filename: name.clone(),
            oid: *id,
        });
    }
    entries.sort();
    Ok(repo.write_object(&Tree { entries })?.detach())
}

fn normalize_message(message: &str) -> Result<String> {
    let message = message.trim_end();
    if message.trim().is_empty() {
        return Err("Commit message is empty. Add a subject before committing.".into());
    }
    Ok(format!("{message}\n"))
}

fn write_message(path: &Path, message: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, message)?;
    Ok(())
}

fn read_message(path: &Path) -> Result<String> {
    let message = std::fs::read_to_string(path)
        .map_err(|e| format!("Read COMMIT_EDITMSG after the commit-msg hook: {e}"))?;
    if message.trim().is_empty() {
        return Err(
            "The commit-msg hook left an empty commit message. Restore a subject and retry.".into(),
        );
    }
    normalize_message(&message)
}

fn run_hook(worktree: &Path, name: &str, args: &[&OsStr]) -> Result<()> {
    let mut command = Command::new("git");
    command
        .arg("hook")
        .arg("run")
        .arg("--ignore-missing")
        .arg(name)
        .current_dir(worktree)
        .stdin(Stdio::null())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !args.is_empty() {
        command.arg("--");
        for arg in args {
            command.arg(arg);
        }
    }
    let output = command.output().map_err(|e| {
        format!(
            "Could not run the {name} hook using Git: {e}. Install Git on PATH and retry. The index and working tree are preserved."
        )
    })?;
    if output.status.success() {
        return Ok(());
    }
    let mut detail = String::new();
    if !output.stdout.is_empty() {
        detail.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        if !detail.is_empty() && !detail.ends_with('\n') {
            detail.push('\n');
        }
        detail.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    let detail = detail.trim();
    let detail = if detail.is_empty() {
        String::new()
    } else {
        format!("\n{detail}")
    };
    Err(format!(
        "The {name} hook failed ({}). Commit aborted; the index and working tree are preserved. Fix the hook and retry.{detail}",
        output.status
    )
    .into())
}
