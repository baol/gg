use gg_engine::status::StatusIdentity;
use std::{error::Error, path::Path, process::Command};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn git(path: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.org")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.org")
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(output.stdout)
}

fn repo() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init", "-b", "main"])?;
    git(dir.path(), &["config", "user.name", "Test"])?;
    git(dir.path(), &["config", "user.email", "test@example.org"])?;
    Ok(dir)
}

fn identity(path: &Path) -> Result<StatusIdentity> {
    Ok(gg_git::status::Session::open(path)?.identity)
}

fn commit(path: &Path, message: &str) -> std::result::Result<gg_git::commit::Receipt, String> {
    let repository = identity(path).map_err(|e| e.to_string())?;
    gg_git::commit::create(&repository, message)
}

fn rejected(path: &Path, message: &str) -> Result<String> {
    match commit(path, message) {
        Err(error) => Ok(error),
        Ok(_) => Err("commit unexpectedly succeeded".into()),
    }
}

fn hook(path: &Path, name: &str, body: &str) -> Result {
    let hooks = path.join(".git/hooks");
    std::fs::create_dir_all(&hooks)?;
    let file = hooks.join(name);
    std::fs::write(&file, format!("#!/bin/sh\n{body}\n"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

#[test]
fn creates_root_and_normal_commits_without_touching_worktree() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"root\n")?;
    git(p, &["add", "a"])?;
    let root = commit(p, "root commit")?;
    assert!(root.parents.is_empty());
    assert_eq!(git(p, &["rev-list", "--count", "HEAD"])?, b"1\n");
    assert_eq!(
        git(p, &["show", "-s", "--format=%s", "HEAD"])?,
        b"root commit\n"
    );
    assert_eq!(std::fs::read(p.join("a"))?, b"root\n");

    std::fs::write(p.join("a"), b"staged\n")?;
    git(p, &["add", "a"])?;
    std::fs::write(p.join("a"), b"working\n")?;
    let second = commit(p, "second subject\n\nBody text.")?;
    assert_eq!(second.parents, vec![root.id.clone()]);
    assert_eq!(
        git(p, &["show", "-s", "--format=%B", "HEAD"])?,
        b"second subject\n\nBody text.\n\n"
    );
    assert_eq!(
        git(p, &["reflog", "-1", "--format=%gs"])?,
        b"commit: second subject\n"
    );
    assert_eq!(
        git(p, &["show", ":a"])?,
        b"staged\n",
        "committing must use the index, not the working file"
    );
    assert_eq!(std::fs::read(p.join("a"))?, b"working\n");
    assert_eq!(git(p, &["status", "--porcelain=v1"])?, b" M a\n");
    Ok(())
}

#[test]
fn rejects_empty_messages_no_staged_changes_and_stale_identities() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    commit(p, "base")?;

    assert!(rejected(p, "   \n")?.contains("empty"));
    assert!(rejected(p, "nothing staged")?.contains("No staged changes"));

    let stale = identity(p)?;
    std::fs::write(p.join("a"), b"next\n")?;
    git(p, &["add", "a"])?;
    let Err(error) = gg_git::commit::create(&stale, "stale") else {
        return Err("stale commit unexpectedly succeeded".into());
    };
    assert!(error.contains("changed since"), "{error}");
    assert_eq!(git(p, &["rev-list", "--count", "HEAD"])?, b"1\n");
    Ok(())
}

#[test]
fn pre_commit_hook_can_stage_content_and_failures_preserve_the_index() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    commit(p, "base")?;
    std::fs::write(p.join("a"), b"staged\n")?;
    git(p, &["add", "a"])?;
    hook(
        p,
        "pre-commit",
        "printf 'from hook\\n' > hook.txt\ngit add hook.txt",
    )?;
    let receipt = commit(p, "with hook")?;
    assert_eq!(
        git(p, &["show", &format!("{}:hook.txt", receipt.id)])?,
        b"from hook\n"
    );

    std::fs::write(p.join("a"), b"next\n")?;
    git(p, &["add", "a"])?;
    let before = std::fs::read(p.join(".git/index"))?;
    hook(p, "pre-commit", "echo blocked >&2\nexit 7")?;
    let error = rejected(p, "blocked")?;
    assert!(error.contains("pre-commit"), "{error}");
    assert!(error.contains("blocked"), "{error}");
    assert!(error.contains("preserved"), "{error}");
    assert_eq!(std::fs::read(p.join(".git/index"))?, before);
    assert_eq!(git(p, &["rev-list", "--count", "HEAD"])?, b"2\n");
    Ok(())
}

#[test]
fn prepare_and_commit_message_hooks_run_and_may_edit_the_message() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    commit(p, "base")?;
    std::fs::write(p.join("a"), b"next\n")?;
    git(p, &["add", "a"])?;
    hook(
        p,
        "prepare-commit-msg",
        "printf '%s\\n%s\\n' \"$1\" \"$2\" > prepare.args",
    )?;
    hook(
        p,
        "commit-msg",
        "printf 'Edited subject\\n\\nHook body.\\n' > \"$1\"",
    )?;
    commit(p, "original")?;
    let args = std::fs::read_to_string(p.join("prepare.args"))?;
    assert!(args.contains("COMMIT_EDITMSG"), "{args}");
    assert!(args.lines().nth(1) == Some("message"), "{args}");
    assert_eq!(
        git(p, &["show", "-s", "--format=%B", "HEAD"])?,
        b"Edited subject\n\nHook body.\n\n"
    );
    Ok(())
}

#[test]
fn post_commit_failure_is_a_warning_after_the_commit_exists() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    commit(p, "base")?;
    std::fs::write(p.join("a"), b"next\n")?;
    git(p, &["add", "a"])?;
    hook(p, "post-commit", "echo warning >&2\nexit 9")?;
    let receipt = commit(p, "created")?;
    assert!(!receipt.warnings.is_empty());
    assert!(receipt.warnings[0].contains("post-commit"));
    assert!(receipt.warnings[0].contains("warning"));
    assert_eq!(
        git(p, &["show", "-s", "--format=%s", "HEAD"])?,
        b"created\n"
    );
    Ok(())
}

#[test]
fn custom_hooks_path_is_honoured() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    commit(p, "base")?;
    std::fs::write(p.join("a"), b"next\n")?;
    git(p, &["add", "a"])?;
    let hooks = p.join("custom-hooks");
    std::fs::create_dir(&hooks)?;
    let hook_path = hooks.join("pre-commit");
    std::fs::write(&hook_path, "#!/bin/sh\nexit 6\n")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755))?;
    }
    git(p, &["config", "core.hooksPath", "custom-hooks"])?;
    let error = rejected(p, "blocked by custom path")?;
    assert!(error.contains("pre-commit"), "{error}");
    Ok(())
}

#[test]
fn rejects_conflicts_and_intent_to_add() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    commit(p, "base")?;

    std::fs::write(p.join("ita"), b"intent\n")?;
    git(p, &["add", "-N", "ita"])?;
    let error = rejected(p, "intent")?;
    assert!(error.contains("Intent-to-add"), "{error}");

    std::fs::write(p.join("c"), b"base\n")?;
    git(p, &["add", "c"])?;
    git(p, &["commit", "-qm", "conflict base"])?;
    git(p, &["checkout", "-qb", "other"])?;
    std::fs::write(p.join("c"), b"other\n")?;
    git(p, &["commit", "-qam", "other"])?;
    git(p, &["checkout", "-q", "main"])?;
    std::fs::write(p.join("c"), b"main\n")?;
    git(p, &["commit", "-qam", "main"])?;
    let _ = git(p, &["merge", "other"]);
    let error = rejected(p, "unmerged")?;
    assert!(error.contains("Unmerged"), "{error}");
    Ok(())
}

#[test]
fn commits_on_detached_head() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    let base = commit(p, "base")?;
    git(p, &["checkout", "-q", "--detach"])?;
    std::fs::write(p.join("a"), b"detached\n")?;
    git(p, &["add", "a"])?;
    let detached = commit(p, "detached")?;
    assert_eq!(detached.parents, vec![base.id]);
    assert!(
        !Command::new("git")
            .arg("-C")
            .arg(p)
            .args(["symbolic-ref", "-q", "HEAD"])
            .status()?
            .success()
    );
    assert_eq!(
        git(p, &["show", "-s", "--format=%s", "HEAD"])?,
        b"detached\n"
    );
    Ok(())
}

#[test]
fn hook_can_empty_the_message_and_lock_conflicts_abort_cleanly() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    commit(p, "base")?;
    std::fs::write(p.join("a"), b"next\n")?;
    git(p, &["add", "a"])?;
    hook(p, "commit-msg", ": > \"$1\"")?;
    let error = rejected(p, "will be emptied")?;
    assert!(error.contains("empty"), "{error}");
    assert_eq!(git(p, &["rev-list", "--count", "HEAD"])?, b"1\n");

    std::fs::remove_file(p.join(".git/hooks/commit-msg"))?;
    std::fs::write(p.join(".git/index.lock"), b"owner")?;
    let error = rejected(p, "locked")?;
    assert!(error.to_lowercase().contains("lock"), "{error}");
    assert_eq!(git(p, &["rev-list", "--count", "HEAD"])?, b"1\n");
    std::fs::remove_file(p.join(".git/index.lock"))?;
    Ok(())
}

#[test]
fn hooks_run_in_order_and_linked_worktrees_commit_independently() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    commit(p, "base")?;
    std::fs::write(p.join("a"), b"next\n")?;
    git(p, &["add", "a"])?;
    hook(p, "pre-commit", "echo pre >> order.log")?;
    hook(p, "prepare-commit-msg", "echo prepare >> order.log")?;
    hook(p, "commit-msg", "echo commit-msg >> order.log")?;
    hook(p, "post-commit", "echo post >> order.log")?;
    commit(p, "ordered")?;
    assert_eq!(
        std::fs::read_to_string(p.join("order.log"))?,
        "pre\nprepare\ncommit-msg\npost\n"
    );

    let linked = tempfile::tempdir()?;
    let linked_path = linked.path().join("linked");
    git(
        p,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "linked",
            &linked_path.to_string_lossy(),
        ],
    )?;
    std::fs::write(linked_path.join("a"), b"linked\n")?;
    git(&linked_path, &["add", "a"])?;
    let receipt = commit(&linked_path, "from linked worktree")?;
    assert_eq!(
        git(&linked_path, &["show", "-s", "--format=%s", "HEAD"])?,
        b"from linked worktree\n"
    );
    assert_eq!(git(p, &["rev-list", "--count", "main"])?, b"2\n");
    assert_eq!(git(p, &["rev-list", "--count", "linked"])?, b"3\n");
    assert_eq!(receipt.parents.len(), 1);
    Ok(())
}

#[test]
fn amend_rewrites_head_preserves_author_and_saves_a_backup() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    let base = commit(p, "base")?;
    std::fs::write(p.join("a"), b"next\n")?;
    git(p, &["add", "a"])?;
    let previous = commit(p, "subject")?;
    let old_author = git(p, &["show", "-s", "--format=%an <%ae>", "HEAD"])?;

    std::fs::write(p.join("a"), b"amended\n")?;
    git(p, &["add", "a"])?;
    git(p, &["config", "user.name", "Different"])?;
    let receipt = gg_git::commit::amend(&identity(p)?, "amended subject\n\nBody.")?;
    assert!(receipt.amended);
    assert_eq!(receipt.previous.as_deref(), Some(previous.id.as_str()));
    assert_eq!(receipt.parents, vec![base.id.clone()]);
    assert_ne!(receipt.id, previous.id);
    let backup = receipt.backup.ok_or("missing backup")?;
    assert_eq!(
        git(p, &["rev-parse", &backup])?,
        format!("{}\n", previous.id).as_bytes()
    );
    assert_eq!(git(p, &["rev-list", "--count", "HEAD"])?, b"2\n");
    assert_eq!(
        git(p, &["show", "-s", "--format=%B", "HEAD"])?,
        b"amended subject\n\nBody.\n\n"
    );
    assert_eq!(git(p, &["show", "HEAD:a"])?, b"amended\n");
    assert_eq!(
        git(p, &["show", "-s", "--format=%an <%ae>", "HEAD"])?,
        old_author
    );
    assert_eq!(
        git(p, &["reflog", "-1", "--format=%gs"])?,
        b"commit (amend): amended subject\n"
    );
    let refs = gg_git::refs::read(p)?;
    assert!(
        refs.items
            .iter()
            .all(|item| !item.full_name.starts_with("refs/gg/"))
    );

    let amended = receipt.id;
    let message_only = gg_git::commit::amend(&identity(p)?, "message only")?;
    assert_eq!(message_only.tree, receipt.tree);
    assert_eq!(message_only.previous.as_deref(), Some(amended.as_str()));
    assert_eq!(
        git(p, &["show", "-s", "--format=%s", "HEAD"])?,
        b"message only\n"
    );
    Ok(())
}

#[test]
fn amend_hooks_receive_the_old_commit_and_post_rewrite_warnings_are_kept() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    let base = commit(p, "base")?;
    hook(
        p,
        "prepare-commit-msg",
        "printf '%s\\n%s\\n' \"$1\" \"$2\" > amend.args\nprintf '%s\\n' \"$3\" >> amend.args",
    )?;
    hook(p, "commit-msg", "printf 'Hook amended subject\\n' > \"$1\"")?;
    hook(
        p,
        "post-rewrite",
        "cat > rewrite.log\necho rewrite-warning >&2\nexit 3",
    )?;
    let receipt = gg_git::commit::amend(&identity(p)?, "original")?;
    let args = std::fs::read_to_string(p.join("amend.args"))?;
    assert!(args.contains("COMMIT_EDITMSG"), "{args}");
    assert!(args.lines().nth(1) == Some("commit"), "{args}");
    assert!(
        args.lines().any(|line| line.contains(&base.id)),
        "prepare-commit-msg must receive the old commit: {args}"
    );
    assert_eq!(
        git(p, &["show", "-s", "--format=%s", "HEAD"])?,
        b"Hook amended subject\n"
    );
    let rewritten = std::fs::read_to_string(p.join("rewrite.log"))?;
    assert!(rewritten.contains(&base.id), "{rewritten}");
    assert!(rewritten.contains(&receipt.id), "{rewritten}");
    assert!(receipt.warnings.iter().any(|w| w.contains("post-rewrite")));
    assert!(
        receipt
            .warnings
            .iter()
            .any(|w| w.contains("rewrite-warning"))
    );
    Ok(())
}

#[test]
fn amend_rejects_unborn_head_and_empty_messages() -> Result {
    let dir = repo()?;
    let p = dir.path();
    let error = match gg_git::commit::amend(&identity(p)?, "first") {
        Ok(_) => return Err("amend unexpectedly succeeded without a commit".into()),
        Err(error) => error,
    };
    assert!(error.contains("no commit to amend"), "{error}");

    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    commit(p, "base")?;
    let error = match gg_git::commit::amend(&identity(p)?, "   ") {
        Ok(_) => return Err("empty amend unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(error.contains("empty"), "{error}");
    assert_eq!(git(p, &["rev-list", "--count", "HEAD"])?, b"1\n");
    Ok(())
}
