use std::{error::Error, path::Path, process::Command};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn git(path: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .args(["-c", "core.autocrlf=false"])
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
    git(dir.path(), &["config", "core.autocrlf", "false"])?;
    std::fs::write(dir.path().join("a"), b"base\n")?;
    git(dir.path(), &["add", "a"])?;
    git(dir.path(), &["commit", "-qm", "base"])?;
    Ok(dir)
}

fn conflicting(dir: &Path) -> Result {
    git(dir, &["switch", "-qc", "feature"])?;
    std::fs::write(dir.join("a"), b"feature\n")?;
    git(dir, &["commit", "-qam", "feature"])?;
    git(dir, &["switch", "-q", "main"])?;
    std::fs::write(dir.join("a"), b"main\n")?;
    git(dir, &["commit", "-qam", "main"])?;
    let error = gg_git::merge::merge(dir, "feature", false);
    assert!(error.is_ok());
    Ok(())
}

#[test]
fn reads_and_selects_conflict_sides() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    assert_eq!(gg_git::conflict::unmerged(p)?, vec![b"a".to_vec()]);

    let content = gg_git::conflict::read(p, b"a")?;
    assert_eq!(content.base.as_deref(), Some("base\n"));
    assert_eq!(content.ours.as_deref(), Some("main\n"));
    assert_eq!(content.theirs.as_deref(), Some("feature\n"));
    assert!(!content.binary);
    assert!(!content.truncated);

    gg_git::conflict::use_side(p, b"a", gg_engine::conflict::Side::Theirs)?;
    assert_eq!(std::fs::read(p.join("a"))?, b"feature\n");
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    Ok(())
}

#[test]
fn stages_manually_resolved_content() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    std::fs::write(p.join("a"), b"resolved\n")?;
    gg_git::conflict::stage(p, b"a")?;
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    assert_eq!(std::fs::read(p.join("a"))?, b"resolved\n");
    Ok(())
}

#[test]
fn nested_stage_uses_the_repository_relative_conflict_path() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let nested = p.join("nested");
    std::fs::create_dir(&nested)?;
    std::fs::write(nested.join("a"), b"unrelated\n")?;
    std::fs::write(p.join("a"), b"resolved\n")?;

    gg_git::conflict::stage(&nested, b"a")?;

    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    assert_eq!(std::fs::read(p.join("a"))?, b"resolved\n");
    let status = String::from_utf8(git(p, &["status", "--porcelain"])?)?;
    assert!(status.contains("?? nested/"), "{status}");
    Ok(())
}

#[test]
fn add_add_conflict_has_no_base() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("new"), b"feature\n")?;
    git(p, &["add", "new"])?;
    git(p, &["commit", "-qm", "feature new"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("new"), b"main\n")?;
    git(p, &["add", "new"])?;
    git(p, &["commit", "-qm", "main new"])?;
    gg_git::merge::merge(p, "feature", false)?;

    let content = gg_git::conflict::read(p, b"new")?;
    assert!(content.base.is_none());
    assert_eq!(content.ours.as_deref(), Some("main\n"));
    assert_eq!(content.theirs.as_deref(), Some("feature\n"));
    Ok(())
}

#[test]
fn rejects_unknown_paths_and_base_selection() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let error = match gg_git::conflict::read(p, b"missing") {
        Ok(_) => return Err("missing conflict unexpectedly read".into()),
        Err(error) => error,
    };
    assert!(error.contains("No unmerged stages"), "{error}");
    let error = match gg_git::conflict::use_side(p, b"a", gg_engine::conflict::Side::Base) {
        Ok(_) => return Err("base side unexpectedly selected".into()),
        Err(error) => error,
    };
    assert!(error.contains("Base content"), "{error}");
    Ok(())
}

#[test]
fn previewed_sides_publish_only_after_validation() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let snapshot = gg_git::conflict::snapshot(p, b"a")?;
    assert!(snapshot.base.is_some());
    assert!(snapshot.ours.is_some());
    let ours = snapshot.ours.clone();
    assert!(ours.is_some());
    assert!(snapshot.theirs.is_some());
    assert_eq!(
        std::fs::read(p.join("a"))?,
        b"<<<<<<< HEAD\nmain\n=======\nfeature\n>>>>>>> feature\n"
    );

    gg_git::conflict::publish(p, b"a", &snapshot, gg_engine::conflict::Resolution::Ours)?;
    assert_eq!(std::fs::read(p.join("a"))?, b"main\n");
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    let staged = git(p, &["ls-files", "-s", "--", "a"])?;
    let staged = String::from_utf8_lossy(&staged);
    assert!(
        staged.contains(ours.as_deref().unwrap_or_default()),
        "staged output must be the selected side: {staged}"
    );
    Ok(())
}

#[test]
fn publish_rejects_changed_stages_or_working_bytes() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let stale = gg_git::conflict::snapshot(p, b"a")?;
    gg_git::conflict::use_side(p, b"a", gg_engine::conflict::Side::Theirs)?;
    assert!(
        gg_git::conflict::publish(p, b"a", &stale, gg_engine::conflict::Resolution::Ours,).is_err()
    );
    assert_eq!(std::fs::read(p.join("a"))?, b"feature\n");

    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let stale = gg_git::conflict::snapshot(p, b"a")?;
    std::fs::write(p.join("a"), b"external\n")?;
    assert!(
        gg_git::conflict::publish(p, b"a", &stale, gg_engine::conflict::Resolution::Working,)
            .is_err()
    );
    let fresh = gg_git::conflict::snapshot(p, b"a")?;
    gg_git::conflict::publish(p, b"a", &fresh, gg_engine::conflict::Resolution::Working)?;
    assert_eq!(std::fs::read(p.join("a"))?, b"external\n");
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    Ok(())
}

#[test]
fn base_resolution_writes_the_common_ancestor() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let snapshot = gg_git::conflict::snapshot(p, b"a")?;
    gg_git::conflict::publish(p, b"a", &snapshot, gg_engine::conflict::Resolution::Base)?;
    assert_eq!(std::fs::read(p.join("a"))?, b"base\n");
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    Ok(())
}

#[test]
fn snapshot_rejects_paths_outside_the_repository() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    assert!(gg_git::conflict::snapshot(p, b"../a").is_err());
    Ok(())
}

#[test]
fn oversized_working_files_validate_without_a_preview() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    std::fs::write(p.join("a"), vec![b'a'; 2 * 1024 * 1024 + 1])?;
    let snapshot = gg_git::conflict::snapshot(p, b"a")?;
    assert!(snapshot.working.is_none());
    assert!(snapshot.working_hash.is_some());
    assert!(
        snapshot
            .working_note
            .as_deref()
            .is_some_and(|note| note.contains("bytes"))
    );
    gg_git::conflict::publish(p, b"a", &snapshot, gg_engine::conflict::Resolution::Working)?;
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    Ok(())
}

#[test]
fn publishing_reports_a_partial_worktree_write() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let snapshot = gg_git::conflict::snapshot(p, b"a")?;
    std::fs::write(p.join(".git/index.lock"), b"locked")?;
    let error = match gg_git::conflict::publish(
        p,
        b"a",
        &snapshot,
        gg_engine::conflict::Resolution::Ours,
    ) {
        Err(error) => error,
        Ok(()) => return Err("locked index must fail".into()),
    };
    assert!(error.contains("written but staging failed"), "{error}");
    assert_eq!(std::fs::read(p.join("a"))?, b"main\n");
    std::fs::remove_file(p.join(".git/index.lock"))?;
    assert!(!gg_git::conflict::unmerged(p)?.is_empty());
    Ok(())
}

#[test]
fn non_regular_working_paths_keep_the_side_preview_explicit() -> Result {
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    std::fs::remove_file(p.join("a"))?;
    std::fs::create_dir(p.join("a"))?;
    let snapshot = gg_git::conflict::snapshot(p, b"a")?;
    assert!(snapshot.working.is_none());
    assert!(
        snapshot
            .working_note
            .as_deref()
            .is_some_and(|note| note.contains("not a regular file"))
    );
    let error = match gg_git::conflict::publish(
        p,
        b"a",
        &snapshot,
        gg_engine::conflict::Resolution::Ours,
    ) {
        Err(error) => error,
        Ok(()) => return Err("a directory cannot be replaced by rename".into()),
    };
    assert!(error.contains("Write working file"), "{error}");
    assert!(p.join("a").is_dir());
    Ok(())
}

#[test]
fn oversized_sides_publish_and_changed_bytes_are_detected() -> Result {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    git(p, &["init", "-b", "main"])?;
    git(p, &["config", "user.name", "Test"])?;
    git(p, &["config", "user.email", "test@example.org"])?;
    std::fs::write(p.join("a"), b"base\n")?;
    git(p, &["add", "a"])?;
    git(p, &["commit", "-qm", "base"])?;
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("a"), vec![b'f'; 2 * 1024 * 1024 + 1])?;
    git(p, &["commit", "-qam", "feature"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("a"), vec![b'm'; 2 * 1024 * 1024 + 1])?;
    git(p, &["commit", "-qam", "main"])?;
    let _ = gg_git::merge::merge(p, "feature", false);
    assert!(!gg_git::conflict::unmerged(p)?.is_empty());

    let snapshot = gg_git::conflict::snapshot(p, b"a")?;
    gg_git::conflict::publish(p, b"a", &snapshot, gg_engine::conflict::Resolution::Ours)?;
    let resolved = std::fs::read(p.join("a"))?;
    assert_eq!(resolved.len(), 2 * 1024 * 1024 + 1);
    assert!(resolved.iter().all(|byte| *byte == b'm'));
    assert!(gg_git::conflict::unmerged(p)?.is_empty());

    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    let mut oversized = vec![b'a'; 2 * 1024 * 1024 + 1];
    std::fs::write(p.join("a"), &oversized)?;
    let snapshot = gg_git::conflict::snapshot(p, b"a")?;
    oversized[100] = b'b';
    std::fs::write(p.join("a"), &oversized)?;
    assert!(
        gg_git::conflict::publish(p, b"a", &snapshot, gg_engine::conflict::Resolution::Working,)
            .is_err(),
        "a changed byte in a large file must invalidate the preview"
    );
    Ok(())
}

/// Two conflicting hunks separated by an unchanged line.
fn two_block_conflict(p: &Path) -> Result {
    std::fs::write(p.join("b"), b"top\nx\nmiddle\ny\nend\n")?;
    git(p, &["add", "b"])?;
    git(p, &["commit", "-qm", "add b"])?;
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("b"), b"top\nx feature\nmiddle\ny feature\nend\n")?;
    git(p, &["commit", "-qam", "feature b"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("b"), b"top\nx main\nmiddle\ny main\nend\n")?;
    git(p, &["commit", "-qam", "main b"])?;
    gg_git::merge::merge(p, "feature", false)?;
    Ok(())
}

#[test]
fn blocks_split_the_conflict_and_compose_a_staged_resolution() -> Result {
    use gg_engine::conflict::{BlockChoice, Segment, block_count, compose};
    let dir = repo()?;
    let p = dir.path();
    // The user's conflict style does not change the blocks.
    git(p, &["config", "merge.conflictStyle", "merge"])?;
    two_block_conflict(p)?;
    let segments = gg_git::conflict::blocks(p, b"b")?;
    assert_eq!(block_count(&segments), 2);
    assert_eq!(
        segments[1],
        Segment::Block {
            ours: "x main\n".into(),
            base: "x\n".into(),
            theirs: "x feature\n".into(),
        }
    );
    let snapshot = gg_git::conflict::snapshot(p, b"b")?;
    let text = compose(
        &segments,
        &[Some(BlockChoice::Theirs), Some(BlockChoice::OursThenTheirs)],
    );
    assert_eq!(text, "top\nx feature\nmiddle\ny main\ny feature\nend\n");
    gg_git::conflict::publish_text(p, b"b", &snapshot, &text)?;
    assert_eq!(std::fs::read(p.join("b"))?, text.as_bytes());
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    assert_eq!(
        String::from_utf8(git(p, &["show", ":b"])?)?,
        text,
        "the composed text is staged"
    );
    Ok(())
}

#[test]
fn composed_resolution_refuses_a_changed_working_file() -> Result {
    use gg_engine::conflict::{BlockChoice, compose};
    let dir = repo()?;
    let p = dir.path();
    two_block_conflict(p)?;
    let segments = gg_git::conflict::blocks(p, b"b")?;
    let snapshot = gg_git::conflict::snapshot(p, b"b")?;
    std::fs::write(p.join("b"), b"edited elsewhere\n")?;
    let text = compose(
        &segments,
        &[Some(BlockChoice::Ours), Some(BlockChoice::Ours)],
    );
    let error = match gg_git::conflict::publish_text(p, b"b", &snapshot, &text) {
        Ok(()) => return Err("a stale preview was published".into()),
        Err(error) => error,
    };
    assert!(error.contains("working file changed"), "{error}");
    assert_eq!(gg_git::conflict::unmerged(p)?, vec![b"b".to_vec()]);
    Ok(())
}

#[test]
fn blocks_name_why_a_file_needs_a_whole_side() -> Result {
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature"])?;
    git(p, &["rm", "-q", "a"])?;
    git(p, &["commit", "-qm", "delete a"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("a"), b"main\n")?;
    git(p, &["commit", "-qam", "main"])?;
    gg_git::merge::merge(p, "feature", false)?;
    let error = match gg_git::conflict::blocks(p, b"a") {
        Ok(_) => return Err("delete/modify produced blocks".into()),
        Err(error) => error,
    };
    assert!(error.contains("Keep deletion or a whole side"), "{error}");
    Ok(())
}

/// `feature` deletes `a`; `main` modifies it; merging `feature` stops on a
/// delete/modify conflict.
fn delete_modify_conflict(p: &Path) -> Result {
    git(p, &["switch", "-qc", "feature"])?;
    git(p, &["rm", "-q", "a"])?;
    git(p, &["commit", "-qm", "delete a"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("a"), b"main\n")?;
    git(p, &["commit", "-qam", "main"])?;
    gg_git::merge::merge(p, "feature", false)?;
    Ok(())
}

#[test]
fn keeping_a_deletion_stages_the_removal() -> Result {
    use gg_engine::conflict::{Resolution, Side};
    let dir = repo()?;
    let p = dir.path();
    delete_modify_conflict(p)?;
    let content = gg_git::conflict::read(p, b"a")?;
    assert_eq!(content.deleted_side(), Some(Side::Theirs));
    let snapshot = gg_git::conflict::snapshot(p, b"a")?;
    gg_git::conflict::publish(p, b"a", &snapshot, Resolution::Deletion)?;
    assert!(!p.join("a").exists(), "the working file is removed");
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    assert!(
        git(p, &["ls-files", "--", "a"])?.is_empty(),
        "the removal is staged"
    );

    // The modified side still keeps the file.
    let dir = repo()?;
    let p = dir.path();
    delete_modify_conflict(p)?;
    let snapshot = gg_git::conflict::snapshot(p, b"a")?;
    gg_git::conflict::publish(p, b"a", &snapshot, Resolution::Ours)?;
    assert_eq!(std::fs::read(p.join("a"))?, b"main\n");
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    Ok(())
}

#[test]
fn keeping_a_deletion_is_refused_without_a_deleted_side_or_after_an_edit() -> Result {
    use gg_engine::conflict::Resolution;
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    assert_eq!(gg_git::conflict::read(p, b"a")?.deleted_side(), None);
    let snapshot = gg_git::conflict::snapshot(p, b"a")?;
    let error = match gg_git::conflict::publish(p, b"a", &snapshot, Resolution::Deletion) {
        Ok(()) => return Err("a modify/modify conflict was deleted".into()),
        Err(error) => error,
    };
    assert!(error.contains("one side deleted"), "{error}");
    assert!(p.join("a").exists());
    match gg_git::conflict::keep_deletion(p, b"a") {
        Ok(()) => return Err("the CLI deleted a modify/modify conflict".into()),
        Err(error) => assert!(error.contains("one side deleted"), "{error}"),
    }

    let dir = repo()?;
    let p = dir.path();
    delete_modify_conflict(p)?;
    let stale = gg_git::conflict::snapshot(p, b"a")?;
    std::fs::write(p.join("a"), b"edited after preview\n")?;
    assert!(gg_git::conflict::publish(p, b"a", &stale, Resolution::Deletion).is_err());
    assert_eq!(std::fs::read(p.join("a"))?, b"edited after preview\n");
    assert_eq!(gg_git::conflict::unmerged(p)?, vec![b"a".to_vec()]);

    // The CLI keeps the deletion without a preview.
    gg_git::conflict::keep_deletion(p, b"a")?;
    assert!(!p.join("a").exists());
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    Ok(())
}

#[test]
fn use_side_points_the_deleted_side_to_keep_deletion() -> Result {
    let dir = repo()?;
    let p = dir.path();
    delete_modify_conflict(p)?;
    let error = match gg_git::conflict::use_side(p, b"a", gg_engine::conflict::Side::Theirs) {
        Ok(()) => return Err("the deleted side was checked out".into()),
        Err(error) => error,
    };
    assert!(error.contains("--use deletion"), "{error}");
    assert_eq!(gg_git::conflict::unmerged(p)?, vec![b"a".to_vec()]);
    Ok(())
}

#[test]
fn a_hand_edited_working_file_is_told_apart_from_what_git_wrote() -> Result {
    use gg_engine::conflict::edited_outside;
    for style in ["merge", "diff3", "zdiff3"] {
        let dir = repo()?;
        let p = dir.path();
        git(p, &["config", "merge.conflictStyle", style])?;
        two_block_conflict(p)?;
        let segments = gg_git::conflict::blocks(p, b"b")?;
        let written = String::from_utf8(std::fs::read(p.join("b"))?)?;
        assert!(
            !edited_outside(&segments, written.as_bytes()),
            "Git's own {style} output is not an edit: {written}"
        );
        // Resolving one block by hand leaves markers but changes a side.
        let first = written.find("<<<<<<<").ok_or("no marker")?;
        let end = written[first..].find(">>>>>>>").ok_or("no end marker")? + first;
        let end = written[end..]
            .find('\n')
            .map_or(written.len(), |n| end + n + 1);
        let partly = format!("{}x by hand\n{}", &written[..first], &written[end..]);
        assert!(edited_outside(&segments, partly.as_bytes()), "{style}");
        // A fully hand-resolved file has no markers.
        assert!(
            edited_outside(&segments, b"top\nx\nmiddle\ny\nend\n"),
            "{style}"
        );
    }
    Ok(())
}

fn short(p: &Path, rev: &str) -> Result<String> {
    Ok(
        String::from_utf8(git(p, &["rev-parse", "--short=8", rev])?)?
            .trim()
            .to_string(),
    )
}

#[test]
fn side_names_follow_the_operation() -> Result {
    use gg_engine::conflict::SideNames;
    // Merge: the current branch and the merged branch.
    let dir = repo()?;
    let p = dir.path();
    conflicting(p)?;
    assert_eq!(
        gg_git::conflict::side_names(p)?,
        Some(SideNames {
            ours: "main".into(),
            theirs: "feature".into(),
        })
    );

    // Rebase: Ours is the base being rebased onto, Theirs the replayed commit.
    let dir = repo()?;
    let p = dir.path();
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("a"), b"feature\n")?;
    git(p, &["commit", "-qam", "Replay this change"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("a"), b"main\n")?;
    git(p, &["commit", "-qam", "main"])?;
    git(p, &["switch", "-q", "feature"])?;
    let replayed = short(p, "feature")?;
    assert!(git(p, &["rebase", "main"]).is_err(), "the rebase stops");
    assert_eq!(
        gg_git::conflict::side_names(p)?,
        Some(SideNames {
            ours: "onto main".into(),
            theirs: format!("{replayed} Replay this change"),
        })
    );
    git(p, &["rebase", "--abort"])?;

    // Cherry-pick: HEAD's branch and the picked commit.
    git(p, &["switch", "-q", "main"])?;
    assert!(
        git(p, &["cherry-pick", "feature"]).is_err(),
        "the pick stops"
    );
    assert_eq!(
        gg_git::conflict::side_names(p)?,
        Some(SideNames {
            ours: "main".into(),
            theirs: format!("{replayed} Replay this change"),
        })
    );
    git(p, &["cherry-pick", "--abort"])?;

    // No operation, no names.
    assert_eq!(gg_git::conflict::side_names(p)?, None);
    Ok(())
}

#[test]
fn a_staged_resolution_reopens_as_a_conflict() -> Result {
    let dir = repo()?;
    let p = dir.path();
    // A clean change beside the conflict is listed as also changed.
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("a"), b"feature\n")?;
    std::fs::write(p.join("b"), b"new\n")?;
    git(p, &["add", "b"])?;
    git(p, &["commit", "-qam", "feature"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("a"), b"main\n")?;
    git(p, &["commit", "-qam", "main"])?;
    gg_git::merge::merge(p, "feature", false)?;
    assert_eq!(gg_git::conflict::also_changed(p)?, vec![b"b".to_vec()]);

    std::fs::write(p.join("a"), b"resolved\n")?;
    gg_git::conflict::stage(p, b"a")?;
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    assert_eq!(
        gg_git::conflict::also_changed(p)?,
        vec![b"a".to_vec(), b"b".to_vec()]
    );

    assert!(
        gg_git::conflict::reopen(p, b"a")?,
        "Git recreates the conflict"
    );
    assert_eq!(gg_git::conflict::unmerged(p)?, vec![b"a".to_vec()]);
    let working = std::fs::read_to_string(p.join("a"))?;
    assert!(working.contains("<<<<<<<"), "markers are back: {working}");

    // A path with no conflict to recreate is unstaged to a plain change.
    assert!(!gg_git::conflict::reopen(p, b"b")?);
    assert!(!gg_git::conflict::also_changed(p)?.contains(&b"b".to_vec()));
    assert!(p.join("b").exists(), "the working file stays");
    Ok(())
}

/// Conflicts on a file literally named `*.txt` and on `a.txt`, which that
/// name matches as a glob pathspec.
#[cfg(unix)]
fn glob_named_conflicts(p: &Path) -> Result {
    std::fs::write(p.join("*.txt"), b"star base\n")?;
    std::fs::write(p.join("a.txt"), b"a base\n")?;
    git(p, &["add", "--", ":(literal)*.txt", "a.txt"])?;
    git(p, &["commit", "-qm", "glob base"])?;
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(p.join("*.txt"), b"star feature\n")?;
    std::fs::write(p.join("a.txt"), b"a feature\n")?;
    git(p, &["commit", "-qam", "feature"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("*.txt"), b"star main\n")?;
    std::fs::write(p.join("a.txt"), b"a main\n")?;
    git(p, &["commit", "-qam", "main"])?;
    gg_git::merge::merge(p, "feature", false)?;
    assert_eq!(
        gg_git::conflict::unmerged(p)?,
        vec![b"*.txt".to_vec(), b"a.txt".to_vec()]
    );
    Ok(())
}

/// The conflict on `a.txt` is untouched: still unmerged, markers on disk.
#[cfg(unix)]
fn other_conflict_untouched(p: &Path) -> Result {
    assert_eq!(gg_git::conflict::unmerged(p)?, vec![b"a.txt".to_vec()]);
    let working = std::fs::read_to_string(p.join("a.txt"))?;
    assert!(
        working.contains("<<<<<<<"),
        "a.txt was overwritten: {working}"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn choosing_a_side_for_a_glob_named_file_leaves_other_conflicts() -> Result {
    use gg_engine::conflict::Side;
    let dir = repo()?;
    let p = dir.path();
    glob_named_conflicts(p)?;
    let content = gg_git::conflict::read(p, b"*.txt")?;
    assert_eq!(content.theirs.as_deref(), Some("star feature\n"));
    gg_git::conflict::use_side(p, b"*.txt", Side::Theirs)?;
    assert_eq!(std::fs::read(p.join("*.txt"))?, b"star feature\n");
    other_conflict_untouched(p)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn staging_and_reopening_a_glob_named_conflict_leaves_other_conflicts() -> Result {
    let dir = repo()?;
    let p = dir.path();
    glob_named_conflicts(p)?;
    std::fs::write(p.join("*.txt"), b"star resolved\n")?;
    gg_git::conflict::stage(p, b"*.txt")?;
    other_conflict_untouched(p)?;
    assert_eq!(gg_git::conflict::also_changed(p)?, vec![b"*.txt".to_vec()]);

    assert!(gg_git::conflict::reopen(p, b"*.txt")?);
    assert_eq!(
        gg_git::conflict::unmerged(p)?,
        vec![b"*.txt".to_vec(), b"a.txt".to_vec()]
    );
    let working = std::fs::read_to_string(p.join("a.txt"))?;
    assert!(
        working.contains("<<<<<<<"),
        "a.txt was rewritten: {working}"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn publishing_a_glob_named_resolution_leaves_other_conflicts() -> Result {
    use gg_engine::conflict::Resolution;
    let dir = repo()?;
    let p = dir.path();
    glob_named_conflicts(p)?;
    let snapshot = gg_git::conflict::snapshot(p, b"*.txt")?;
    gg_git::conflict::publish(p, b"*.txt", &snapshot, Resolution::Ours)?;
    assert_eq!(std::fs::read(p.join("*.txt"))?, b"star main\n");
    other_conflict_untouched(p)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn keeping_the_deletion_of_a_glob_named_file_leaves_other_conflicts() -> Result {
    let dir = repo()?;
    let p = dir.path();
    std::fs::write(p.join("*.txt"), b"base\n")?;
    std::fs::write(p.join("a.txt"), b"base\n")?;
    git(p, &["add", "--", ":(literal)*.txt", "a.txt"])?;
    git(p, &["commit", "-qm", "glob base"])?;
    git(p, &["switch", "-qc", "feature"])?;
    git(p, &["rm", "-q", "--", ":(literal)*.txt", "a.txt"])?;
    git(p, &["commit", "-qm", "delete"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(p.join("*.txt"), b"main\n")?;
    std::fs::write(p.join("a.txt"), b"main\n")?;
    git(p, &["commit", "-qam", "main"])?;
    gg_git::merge::merge(p, "feature", false)?;

    gg_git::conflict::keep_deletion(p, b"*.txt")?;
    assert!(!p.join("*.txt").exists());
    assert_eq!(gg_git::conflict::unmerged(p)?, vec![b"a.txt".to_vec()]);
    assert_eq!(std::fs::read(p.join("a.txt"))?, b"main\n");
    assert!(!git(p, &["ls-files", "--", "a.txt"])?.is_empty());
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn conflict_paths_keep_their_exact_bytes() -> Result {
    use gg_engine::conflict::Side;
    use std::os::unix::ffi::OsStrExt;
    let dir = repo()?;
    let p = dir.path();
    let name: &[u8] = b"raw\xff\nname";
    let file = p.join(std::ffi::OsStr::from_bytes(name));
    std::fs::write(&file, b"base\n")?;
    git(p, &["add", "-A"])?;
    git(p, &["commit", "-qm", "raw base"])?;
    git(p, &["switch", "-qc", "feature"])?;
    std::fs::write(&file, b"feature\n")?;
    git(p, &["commit", "-qam", "feature"])?;
    git(p, &["switch", "-q", "main"])?;
    std::fs::write(&file, b"main\n")?;
    git(p, &["commit", "-qam", "main"])?;
    gg_git::merge::merge(p, "feature", false)?;

    assert_eq!(gg_git::conflict::unmerged(p)?, vec![name.to_vec()]);
    let content = gg_git::conflict::read(p, name)?;
    assert_eq!(content.path, "raw\\xff\\nname");
    assert_eq!(content.path_bytes, name);
    assert_eq!(content.theirs.as_deref(), Some("feature\n"));
    let snapshot = gg_git::conflict::snapshot(p, name)?;
    assert!(
        snapshot
            .working
            .is_some_and(|bytes| bytes.starts_with(b"<<<<<<<"))
    );
    gg_git::conflict::use_side(p, name, Side::Theirs)?;
    assert_eq!(std::fs::read(&file)?, b"feature\n");
    assert!(gg_git::conflict::unmerged(p)?.is_empty());
    assert_eq!(gg_git::conflict::also_changed(p)?, vec![name.to_vec()]);
    Ok(())
}
