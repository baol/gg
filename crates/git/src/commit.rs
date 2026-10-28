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
    pub amended: bool,
    pub signed: bool,
    pub previous: Option<String>,
    pub backup: Option<String>,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Create one commit from the current index after validating the observed HEAD.
pub fn create(repository: &StatusIdentity, message: &str) -> std::result::Result<Receipt, String> {
    create_inner(repository, message, false).map_err(|e| format!("Commit: {e}"))
}

/// Create one signed commit. A missing key or agent fails instead of falling back to unsigned.
pub fn create_signed(
    repository: &StatusIdentity,
    message: &str,
) -> std::result::Result<Receipt, String> {
    create_inner(repository, message, true).map_err(|e| format!("Commit: {e}"))
}

fn create_inner(repository: &StatusIdentity, message: &str, sign: bool) -> Result<Receipt> {
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
    let id = if sign {
        let author = author.trim().to_owned()?;
        let committer = committer.trim().to_owned()?;
        let id = signed_commit_id(&repo, tree, &parents, &message_path, &author, &committer)?;
        update_created_head(&repo, id, &parents, &message, &committer)?;
        id
    } else {
        repo.commit_as(committer, author, "HEAD", &message, tree, parents)?
            .detach()
    };
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
        amended: false,
        signed: sign,
        previous: None,
        backup: None,
    })
}

/// Rewrite HEAD with the current index and a new message.
pub fn amend(repository: &StatusIdentity, message: &str) -> std::result::Result<Receipt, String> {
    amend_inner(repository, message, false).map_err(|e| format!("Amend: {e}"))
}

/// Rewrite HEAD as a signed commit. A signing failure leaves HEAD and the index untouched.
pub fn amend_signed(
    repository: &StatusIdentity,
    message: &str,
) -> std::result::Result<Receipt, String> {
    amend_inner(repository, message, true).map_err(|e| format!("Amend: {e}"))
}

fn amend_inner(repository: &StatusIdentity, message: &str, sign: bool) -> Result<Receipt> {
    let message = normalize_message(message)?;
    crate::status::validate(repository)?;
    let repo = gix::discover(&repository.path)?;
    let worktree = repo
        .workdir()
        .ok_or("Amend requires a working tree")?
        .to_path_buf();
    let previous =
        head_id(&repo)?.ok_or("There is no commit to amend. Create the first commit instead.")?;
    let previous_id = gix::ObjectId::from_hex(previous.as_bytes())?;
    let head = repo.find_commit(previous_id)?;
    let author = head.author()?.to_owned()?;
    let parents: Vec<_> = head.parent_ids().map(|id| id.detach()).collect();
    let mut index = (**repo.index_or_empty()?).clone();
    check_index(&index)?;

    let message_path = repo.git_dir().join("COMMIT_EDITMSG");
    write_message(&message_path, &message)?;

    run_hook(&worktree, "pre-commit", &[])?;
    ensure_head(&repo, Some(&previous))?;
    run_hook(
        &worktree,
        "prepare-commit-msg",
        &[
            message_path.as_os_str(),
            OsStr::new("commit"),
            OsStr::new(&previous),
        ],
    )?;
    ensure_head(&repo, Some(&previous))?;
    run_hook(&worktree, "commit-msg", &[message_path.as_os_str()])?;
    let message = read_message(&message_path)?;

    let lock = patch::acquire(&repo)?;
    ensure_head(&repo, Some(&previous))?;
    index = (**repo.index_or_empty()?).clone();
    check_index(&index)?;
    let tree = tree_from_index(&repo, &index)?;
    let committer = repo
        .committer()
        .ok_or("Committer identity is not configured. Set user.name/user.email or GIT_COMMITTER_NAME/EMAIL.")?
        .map_err(|e| format!("Read committer identity: {e}"))?
        .to_owned()?;
    let (backup, id) = if sign {
        let id = signed_commit_id(&repo, tree, &parents, &message_path, &author, &committer)?;
        let backup = crate::backup::create(&repository.path, "amend", &previous)?;
        update_amended_head(&repo, id, previous_id, &message, &committer)?;
        (backup, id)
    } else {
        let backup = crate::backup::create(&repository.path, "amend", &previous)?;
        let id = write_amended_commit(
            &repo,
            committer.clone(),
            author,
            &message,
            tree,
            &parents,
            previous_id,
        )?;
        (backup, id)
    };
    drop(lock);

    let mut warnings = vec![];
    if let Err(e) = run_hook(&worktree, "post-commit", &[]) {
        warnings.push(format!(
            "The commit was amended, but its post-commit hook failed. Inspect the commit and fix the hook before retrying.\n{e}"
        ));
    }
    let rewritten = format!("{previous} {id}\n");
    if let Err(e) = run_hook_input(
        &worktree,
        "post-rewrite",
        &[OsStr::new("amend")],
        Some(rewritten.as_bytes()),
    ) {
        warnings.push(format!(
            "The commit was amended, but its post-rewrite hook failed. Inspect the commit and fix the hook before retrying.\n{e}"
        ));
    }
    Ok(Receipt {
        id: id.to_string(),
        tree: tree.to_string(),
        parents: parents.iter().map(ToString::to_string).collect(),
        warnings,
        amended: true,
        signed: sign,
        previous: Some(previous),
        backup: Some(backup),
    })
}

fn write_amended_commit(
    repo: &gix::Repository,
    committer: gix::actor::Signature,
    author: gix::actor::Signature,
    message: &str,
    tree: gix::ObjectId,
    parents: &[gix::ObjectId],
    previous: gix::ObjectId,
) -> Result<gix::ObjectId> {
    let commit = gix::objs::Commit {
        message: message.into(),
        tree,
        author,
        committer: committer.clone(),
        encoding: None,
        parents: parents.to_vec().into(),
        extra_headers: Default::default(),
    };
    let id = repo.write_object(&commit)?.detach();
    update_amended_head(repo, id, previous, message, &committer)?;
    Ok(id)
}

fn update_amended_head(
    repo: &gix::Repository,
    id: gix::ObjectId,
    previous: gix::ObjectId,
    message: &str,
    committer: &gix::actor::Signature,
) -> Result<()> {
    let subject = message.lines().next().unwrap_or_default();
    let name: gix::refs::FullName = "HEAD"
        .try_into()
        .map_err(|error| format!("Resolve HEAD: {error}"))?;
    let edit = gix::refs::transaction::RefEdit::update(
        name,
        id,
        gix::refs::transaction::PreviousValue::MustExistAndMatch(gix::refs::Target::Object(
            previous,
        )),
        format!("commit (amend): {subject}"),
    )
    .with_deref(true);
    let mut time = gix::date::parse::TimeBuf::default();
    repo.edit_references_as(std::iter::once(edit), Some(committer.to_ref(&mut time)))?;
    Ok(())
}

fn update_created_head(
    repo: &gix::Repository,
    id: gix::ObjectId,
    parents: &[gix::ObjectId],
    message: &str,
    committer: &gix::actor::Signature,
) -> Result<()> {
    let expected = match parents.first() {
        Some(previous) => gix::refs::transaction::PreviousValue::MustExistAndMatch(
            gix::refs::Target::Object(*previous),
        ),
        None => gix::refs::transaction::PreviousValue::MustNotExist,
    };
    let name: gix::refs::FullName = "HEAD"
        .try_into()
        .map_err(|error| format!("Resolve HEAD: {error}"))?;
    let edit = gix::refs::transaction::RefEdit::update(
        name,
        id,
        expected,
        gix::reference::log::message("commit", message.into(), parents.len()),
    )
    .with_deref(true);
    let mut time = gix::date::parse::TimeBuf::default();
    repo.edit_references_as(std::iter::once(edit), Some(committer.to_ref(&mut time)))?;
    Ok(())
}

/// Ask Git to create a commit object and sign it, without updating any reference.
fn signed_commit_id(
    repo: &gix::Repository,
    tree: gix::ObjectId,
    parents: &[gix::ObjectId],
    message_path: &Path,
    author: &gix::actor::Signature,
    committer: &gix::actor::Signature,
) -> Result<gix::ObjectId> {
    let worktree = repo
        .workdir()
        .ok_or("Signing requires a working tree")?
        .to_path_buf();
    let mut command = Command::new("git");
    command
        .current_dir(&worktree)
        .arg("commit-tree")
        .arg("-S")
        .arg(tree.to_string());
    for parent in parents {
        command.arg("-p").arg(parent.to_string());
    }
    command
        .arg("-F")
        .arg(message_path)
        .stdin(Stdio::null())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    identity_env(&mut command, "AUTHOR", author);
    identity_env(&mut command, "COMMITTER", committer);
    let output = command.output().map_err(|error| {
        format!("Could not run Git to sign the commit: {error}. The commit was not created.")
    })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        let detail = detail.trim();
        return Err(format!(
            "Signing failed ({}). The commit was not created; your message, index, and working tree are unchanged.{}",
            output.status,
            if detail.is_empty() {
                String::new()
            } else {
                format!("\n{detail}")
            }
        )
        .into());
    }
    let id = String::from_utf8_lossy(&output.stdout);
    let id = id.trim();
    if id.is_empty() {
        return Err("Signing returned no commit ID. The commit was not created.".into());
    }
    Ok(gix::ObjectId::from_hex(id.as_bytes())?)
}

fn identity_env(command: &mut Command, role: &str, signature: &gix::actor::Signature) {
    let offset = signature.time.offset;
    let sign = if offset < 0 { '-' } else { '+' };
    let offset = offset.abs();
    let date = format!(
        "{} {sign}{:02}{:02}",
        signature.time.seconds,
        offset / 3600,
        (offset % 3600) / 60
    );
    command
        .env(format!("GIT_{role}_NAME"), signature.name.to_string())
        .env(format!("GIT_{role}_EMAIL"), signature.email.to_string())
        .env(format!("GIT_{role}_DATE"), date);
}

/// Read the current HEAD message without changing repository state.
pub fn head_message(path: &Path) -> std::result::Result<String, String> {
    let repo = gix::discover(path).map_err(|e| format!("Read repository: {e}"))?;
    let commit = repo
        .head_commit()
        .map_err(|e| format!("Read HEAD commit: {e}"))?;
    let decoded = commit
        .decode()
        .map_err(|e| format!("Read HEAD message: {e}"))?;
    Ok(decoded.message.to_str_lossy().trim_end().to_string())
}

/// Read the first parent of HEAD without changing repository state.
pub fn head_parent(path: &Path) -> std::result::Result<Option<String>, String> {
    let repo = gix::discover(path).map_err(|e| format!("Read repository: {e}"))?;
    let commit = repo
        .head_commit()
        .map_err(|e| format!("Read HEAD commit: {e}"))?;
    Ok(commit.parent_ids().next().map(|id| id.detach().to_string()))
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
    run_hook_input(worktree, name, args, None)
}

fn run_hook_input(
    worktree: &Path,
    name: &str,
    args: &[&OsStr],
    input: Option<&[u8]>,
) -> Result<()> {
    let mut command = Command::new("git");
    command
        .arg("hook")
        .arg("run")
        .arg("--ignore-missing")
        .current_dir(worktree)
        .stdin(Stdio::null())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut input_path = None;
    if let Some(input) = input {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!(
            "gg-{}-{}-post-rewrite.stdin",
            std::process::id(),
            nanos
        ));
        std::fs::write(&path, input)?;
        command.arg(format!("--to-stdin={}", path.display()));
        input_path = Some(path);
    }
    command.arg(name);
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
    });
    if let Some(path) = input_path {
        let _ = std::fs::remove_file(path);
    }
    let output = output?;
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
    let recovery = if name.starts_with("post-") {
        "The commit exists; fix the hook before retrying."
    } else {
        "Commit aborted; the index and working tree are preserved. Fix the hook and retry."
    };
    Err(format!(
        "The {name} hook failed ({}). {recovery}{detail}",
        output.status
    )
    .into())
}
