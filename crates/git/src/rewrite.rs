//! Rewrite a linear range through an interactive rebase todo: reorder, pick,
//! reword, edit (stop to reopen), squash, fixup, and drop.
use gg_engine::rewrite::{Action, RangeCommit, Receipt, Step};
use std::{
    collections::BTreeSet,
    path::Path,
    process::{Command, Output, Stdio},
};

struct Range {
    commits: Vec<RangeCommit>,
    root: bool,
    base: Option<String>,
}

/// List the commits from `from` through HEAD, oldest first.
pub fn commits(path: &Path, from: &str) -> Result<Vec<RangeCommit>, String> {
    Ok(range(path, from)?.commits)
}

/// The commits from `from` through HEAD, oldest first, and the commit they
/// are replayed onto (`None` for a root range).
pub fn plan_range(path: &Path, from: &str) -> Result<(Vec<RangeCommit>, Option<String>), String> {
    let range = range(path, from)?;
    Ok((range.commits, range.base))
}

/// The first commit of the current branch's own work, for an interactive rebase
/// that does not depend on a history selection.
///
/// Uses the commits the upstream lacks; when everything is pushed (or there is
/// no upstream), the commits the default branch lacks. Returns the oldest such
/// commit and the number of commits through HEAD, or `None` when the branch has
/// no commits of its own.
pub fn branch_start(path: &Path) -> Result<Option<(String, usize)>, String> {
    const BASES: [&str; 6] = [
        "@{upstream}",
        "refs/remotes/origin/HEAD",
        "refs/remotes/origin/main",
        "refs/remotes/origin/master",
        "refs/heads/main",
        "refs/heads/master",
    ];
    for base in BASES {
        let output = run(
            path,
            &[
                "rev-list".into(),
                "--reverse".into(),
                format!("{base}..HEAD"),
            ],
            None,
        )?;
        if !output.status.success() {
            continue;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let commits = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        if let Some(first) = commits.first() {
            return Ok(Some(((*first).to_string(), commits.len())));
        }
    }
    Ok(None)
}

pub fn rewrite(
    path: &Path,
    from: &str,
    steps: &[Step],
    autostash: bool,
) -> Result<Receipt, String> {
    if let Some(state) = crate::mutation::state(path)? {
        return Err(format!(
            "{} is already in progress. Continue, skip, or abort it first.",
            state.operation.as_str()
        ));
    }
    if !autostash {
        crate::rebase::ensure_clean(path)?;
    }
    let range = range(path, from)?;
    let steps = resolve_steps(path, &range, steps)?;
    let previous = head(path)?;
    let backup = crate::backup::create(path, "rewrite", &previous)?;
    let mut args = vec!["rebase".to_string(), "-i".to_string()];
    args.push(if autostash {
        "--autostash".into()
    } else {
        "--no-autostash".into()
    });
    args.push("--no-autosquash".into());
    args.push("--no-rebase-merges".into());
    args.push("--no-reschedule-failed-exec".into());
    if range.root {
        args.push("--root".into());
    } else {
        args.push(
            range
                .base
                .clone()
                .ok_or("Rewrite base is missing for a non-root range.")?,
        );
    }
    let editor = write_plan(path, &steps)?;
    let output = run(path, &args, Some(&editor))?;
    let conflicts = unmerged(path)?;
    let state = crate::mutation::state(path)?;
    if !output.status.success() && state.is_none() && conflicts.is_empty() {
        return Err(failure("Rewrite", &output));
    }
    let in_progress = state.is_some() || !conflicts.is_empty();
    let mut warnings = Vec::new();
    if !conflicts.is_empty() {
        warnings.push(
            "Resolve the listed conflicts, stage each path, then continue or abort the rewrite."
                .into(),
        );
    } else if in_progress && steps.iter().any(|step| step.action == Action::Edit) {
        warnings.push(
            "The rewrite stopped at a reopened commit. Its changes are staged on its parent; commit them in one or more parts, then continue, skip, or abort."
                .into(),
        );
    } else if in_progress {
        warnings.push(failure("The rewrite stopped", &output));
    }
    if autostash && !output.status.success() {
        warnings.push(
            "Working changes were autostashed. Git reapplies them when the rewrite continues."
                .into(),
        );
    }
    Ok(Receipt {
        from: range
            .commits
            .first()
            .map(|commit| commit.id.clone())
            .unwrap_or_default(),
        previous: Some(previous),
        backup: Some(backup),
        applied: output.status.success() && !in_progress,
        in_progress,
        conflicts,
        warnings,
        steps: steps.len(),
    })
}

fn range(path: &Path, from: &str) -> Result<Range, String> {
    let from = from.trim();
    if from.is_empty() || from.starts_with('-') || from.contains('\0') {
        return Err("Rewrite start commit must not start with '-'.".into());
    }
    let from_id = resolve(path, from)?;
    let head = head(path)?;
    let ancestor = run(
        path,
        &[
            "merge-base".into(),
            "--is-ancestor".into(),
            from_id.clone(),
            head.clone(),
        ],
        None,
    )?;
    if !ancestor.status.success() {
        if ancestor.status.code() == Some(1) {
            return Err(format!(
                "Rewrite start commit {from} is not an ancestor of HEAD."
            ));
        }
        return Err(failure("Check the rewrite range", &ancestor));
    }
    let parent = run(
        path,
        &[
            "rev-parse".into(),
            "--verify".into(),
            "--quiet".into(),
            format!("{from_id}^"),
        ],
        None,
    )?;
    let root = !parent.status.success();
    if root && raw_has_parent(path, &from_id)? {
        return Err(
            "Rewrite cannot start at a shallow boundary. Deepen the repository first.".into(),
        );
    }
    let base = if root {
        None
    } else {
        Some(String::from_utf8_lossy(&parent.stdout).trim().to_string())
    };
    let log_range = match &base {
        Some(base) => format!("{base}..{head}"),
        None => head.clone(),
    };
    let output = run(
        path,
        &[
            "log".into(),
            "--reverse".into(),
            "--topo-order".into(),
            "--format=%H%x1f%P%x1f%s%x1f%an%x1f%ar%x1f%B%x1e".into(),
            log_range,
        ],
        None,
    )?;
    if !output.status.success() {
        return Err(failure("List rewrite commits", &output));
    }
    let mut commits = Vec::new();
    for record in String::from_utf8_lossy(&output.stdout).split('\x1e') {
        let record = record.trim_start_matches(['\r', '\n']);
        if record.is_empty() {
            continue;
        }
        let mut fields = record.splitn(6, '\x1f');
        let id = fields.next().unwrap_or_default().to_string();
        let parents = fields
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let subject = fields.next().unwrap_or_default().to_string();
        let author = fields.next().unwrap_or_default().to_string();
        let date = fields.next().unwrap_or_default().to_string();
        let message = fields
            .next()
            .unwrap_or_default()
            .replace("\r\n", "\n")
            .trim_end()
            .to_string();
        if id.is_empty() {
            return Err("Git returned an incomplete rewrite range.".into());
        }
        if parents.len() > 1 {
            return Err(
                "Interactive rebase requires a linear range without merge commits. Select a start commit after the last merge."
                    .into(),
            );
        }
        commits.push((
            id,
            parents,
            RangeCommit {
                id: String::new(),
                subject,
                author,
                date,
                message,
            },
        ));
    }
    if commits.is_empty() {
        return Err("The rewrite range is empty.".into());
    }
    if commits[0].0 != from_id {
        return Err(format!(
            "Rewrite start commit {from} is not the first commit in the range."
        ));
    }
    if commits.last().map(|commit| commit.0.as_str()) != Some(head.as_str()) {
        return Err("The rewrite range does not end at HEAD.".into());
    }
    for (index, (id, parents, _)) in commits.iter().enumerate() {
        if index == 0 {
            let expected = base.iter().cloned().collect::<Vec<_>>();
            if *parents != expected {
                return Err(format!("Rewrite range is not linear at {id}."));
            }
        } else if parents.as_slice() != [commits[index - 1].0.clone()] {
            return Err(format!("Rewrite range is not linear at {id}."));
        }
    }
    Ok(Range {
        commits: commits
            .into_iter()
            .map(|(id, _, commit)| RangeCommit { id, ..commit })
            .collect(),
        root,
        base,
    })
}

fn resolve_steps(path: &Path, range: &Range, steps: &[Step]) -> Result<Vec<Step>, String> {
    if steps.is_empty() {
        return Err("Rewrite requires at least one step.".into());
    }
    let allowed = range
        .commits
        .iter()
        .map(|commit| commit.id.clone())
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut resolved = Vec::with_capacity(steps.len());
    let mut kept = false;
    for step in steps {
        let id = resolve(path, &step.commit)?;
        if !allowed.contains(&id) {
            return Err(format!(
                "Rewrite step {} is outside the selected range.",
                step.commit
            ));
        }
        if !seen.insert(id.clone()) {
            return Err(format!(
                "Rewrite commit {} appears more than once.",
                step.commit
            ));
        }
        let message = match (step.action, &step.message) {
            (Action::Reword, Some(message)) if !message.trim().is_empty() => Some(message.clone()),
            (Action::Reword, _) => {
                return Err(format!("Reword {} needs a non-empty message.", step.commit));
            }
            (_, Some(_)) => {
                return Err(format!(
                    "Only reword steps carry a message; {} is {}.",
                    step.commit,
                    step.action.as_str()
                ));
            }
            (_, None) => None,
        };
        match step.action {
            Action::Squash | Action::Fixup if !kept => {
                return Err(format!(
                    "{} {} needs a kept commit below it to combine into.",
                    capitalized(step.action.as_str()),
                    step.commit
                ));
            }
            Action::Drop => {}
            _ => kept = true,
        }
        resolved.push(Step {
            action: step.action,
            commit: id,
            message,
        });
    }
    if seen.len() != allowed.len() {
        let missing = allowed
            .difference(&seen)
            .next()
            .cloned()
            .unwrap_or_default();
        return Err(format!(
            "Every commit in the selected range must appear exactly once; {missing} is missing."
        ));
    }
    if !kept {
        return Err(
            "A rewrite cannot drop every commit. Use Reset to move the branch instead.".into(),
        );
    }
    Ok(resolved)
}

fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Write the todo and reword messages under the Git directory and return the
/// sequence editor that installs the todo.
///
/// A reword becomes `pick` followed by an `exec` that amends only the message,
/// so Git never needs an interactive editor.
fn write_plan(path: &Path, steps: &[Step]) -> Result<String, String> {
    let dir = git_dir(path)?.join("gg-rewrite");
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .map_err(|error| format!("Could not clear the previous rewrite plan: {error}"))?;
    }
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not write the rewrite plan: {error}"))?;
    let mut todo = String::new();
    for (index, step) in steps.iter().enumerate() {
        match (step.action, &step.message) {
            (Action::Reword, Some(message)) => {
                let file = dir.join(format!("message-{index}.txt"));
                std::fs::write(&file, message.as_bytes())
                    .map_err(|error| format!("Could not write the reword message: {error}"))?;
                todo.push_str(&format!("pick {}\n", step.commit));
                todo.push_str(&format!(
                    "exec git commit --amend --only --allow-empty --cleanup=whitespace -F {}\n",
                    shell_quote(&file.to_string_lossy())
                ));
            }
            (Action::Edit, _) => {
                todo.push_str(&format!("pick {}\n", step.commit));
                todo.push_str(&format!("exec {}\n", reopen_script(&step.commit)));
            }
            (action, _) => todo.push_str(&format!("{} {}\n", action.as_str(), step.commit)),
        }
    }
    let file = dir.join("todo");
    std::fs::write(&file, todo.as_bytes())
        .map_err(|error| format!("Could not write the rewrite plan: {error}"))?;
    Ok(format!("cp {}", shell_quote(&file.to_string_lossy())))
}

/// File under the rebase state that names a reopened commit: the commit its
/// changes were staged on (empty for a root commit) and the original commit.
pub const REOPEN_FILE: &str = "rebase-merge/gg-reopen";

/// Shell run after picking a reopened commit: record it, then move HEAD to
/// its parent so the commit's changes are staged. A root commit stays at HEAD.
/// The script always fails so Git stops here, even when nothing is staged;
/// `--no-reschedule-failed-exec` keeps Continue from running it again.
fn reopen_script(commit: &str) -> String {
    format!(
        "f=\"$(git rev-parse --git-path {REOPEN_FILE})\"; if p=\"$(git rev-parse -q --verify HEAD~1)\"; then printf '%s\\n%s\\n' \"$p\" {commit} > \"$f\" && git reset -q --soft HEAD~1; else printf '\\n%s\\n' {commit} > \"$f\"; fi; exit 1"
    )
}

/// Quote `value` as one POSIX shell word.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn git_dir(path: &Path) -> Result<std::path::PathBuf, String> {
    let output = run(
        path,
        &["rev-parse".into(), "--absolute-git-dir".into()],
        None,
    )?;
    if !output.status.success() {
        return Err(failure("Find the Git directory", &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().into())
}

fn raw_has_parent(path: &Path, commit: &str) -> Result<bool, String> {
    let output = run(
        path,
        &["cat-file".into(), "commit".into(), commit.to_string()],
        None,
    )?;
    if !output.status.success() {
        return Err(failure("Inspect the rewrite start commit", &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .take_while(|line| !line.is_empty())
        .any(|line| line.starts_with("parent ")))
}

fn resolve(path: &Path, value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.starts_with('-') || value.contains('\0') {
        return Err("Rewrite commit must not start with '-'.".into());
    }
    let output = run(
        path,
        &[
            "rev-parse".into(),
            "--verify".into(),
            "--quiet".into(),
            format!("{value}^{{commit}}"),
        ],
        None,
    )?;
    if !output.status.success() {
        return Err(format!("Commit {value} was not found."));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn head(path: &Path) -> Result<String, String> {
    let output = run(path, &["rev-parse".into(), "HEAD".into()], None)?;
    if !output.status.success() {
        return Err(failure("Read HEAD for the rewrite", &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn unmerged(path: &Path) -> Result<Vec<String>, String> {
    let output = run(
        path,
        &[
            "diff".into(),
            "--name-only".into(),
            "-z".into(),
            "--diff-filter=U".into(),
        ],
        None,
    )?;
    if !output.status.success() {
        return Err(failure("Read rewrite conflicts", &output));
    }
    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .map(|field| String::from_utf8_lossy(field).into_owned())
        .collect())
}

fn run(path: &Path, args: &[String], sequence_editor: Option<&str>) -> Result<Output, String> {
    let mut command = Command::new("git");
    command
        .current_dir(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_EDITOR", "true")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    if let Some(editor) = sequence_editor {
        command.env("GIT_SEQUENCE_EDITOR", editor);
    } else {
        command.env("GIT_SEQUENCE_EDITOR", "true");
    }
    command
        .output()
        .map_err(|error| format!("Could not run Git rewrite. Install Git and retry: {error}"))
}

fn combined(output: &Output) -> String {
    let mut detail = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        if !detail.is_empty() && !detail.ends_with('\n') {
            detail.push('\n');
        }
        detail.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    detail.trim().to_string()
}

fn failure(label: &str, output: &Output) -> String {
    let detail = combined(output);
    if detail.is_empty() {
        format!("{label} failed ({}).", output.status)
    } else {
        format!("{label} failed ({}).\n{detail}", output.status)
    }
}
