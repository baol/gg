//! Read bounded line attribution for one committed path.
use gg_engine::{
    blame::{Blame, Line, MAX_BYTES, MAX_LINES},
    review::display_path,
};
use std::{
    path::Path,
    process::{Command, Stdio},
};

fn check(cancelled: &dyn Fn() -> bool) -> Result<(), String> {
    if cancelled() {
        Err("Attribution cancelled".into())
    } else {
        Ok(())
    }
}

/// Attribute one line range of `path_bytes` at `revision`.
///
/// The range is one-based and inclusive. Omitting it attributes the whole
/// file. Only regular blobs within the reader's text budget are accepted.
pub fn read(
    path: &Path,
    revision: &str,
    path_bytes: &[u8],
    lines: Option<(usize, usize)>,
    cancelled: &dyn Fn() -> bool,
) -> Result<Blame, String> {
    let revision = revision.trim();
    let revision = if revision.is_empty() {
        "HEAD"
    } else {
        revision
    };
    if revision.starts_with('-') || revision.contains('\0') {
        return Err("Blame revision is invalid.".into());
    }
    if path_bytes.is_empty() {
        return Err("Blame path is empty.".into());
    }
    let display = display_path(path_bytes);
    let repo = gix::discover(path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    check(cancelled)?;
    let commit = repo
        .rev_parse_single(revision)
        .map_err(|error| format!("Could not resolve {revision}: {error}"))?
        .object()
        .map_err(|error| format!("Could not read {revision}: {error}"))?
        .peel_to_kind(gix::object::Kind::Commit)
        .map_err(|error| format!("{revision} is not a commit: {error}"))?
        .try_into_commit()
        .map_err(|error| format!("Could not read commit {revision}: {error}"))?
        .id;
    let tree = repo
        .find_commit(commit)
        .map_err(|error| format!("Could not read commit {commit}: {error}"))?
        .tree()
        .map_err(|error| format!("Could not read tree of {commit}: {error}"))?;
    let components: Vec<&[u8]> = path_bytes
        .split(|byte| *byte == b'/')
        .filter(|component| !component.is_empty())
        .collect();
    let entry = tree
        .lookup_entry(components)
        .map_err(|error| format!("Could not look up {display}: {error}"))?
        .filter(|entry| !entry.mode().is_tree())
        .ok_or_else(|| format!("{display} does not exist at {revision}."))?;
    let mode = entry.mode().value();
    if mode == 0o160000 {
        return Err(format!("Attribution is unavailable for gitlink {display}."));
    }
    if mode == 0o120000 {
        return Err(format!("Attribution is unavailable for symlink {display}."));
    }
    let blob = repo
        .find_blob(entry.object_id())
        .map_err(|error| format!("Could not read {display} at {revision}: {error}"))?;
    check(cancelled)?;
    if blob.data.len() > MAX_BYTES {
        return Err(format!("Attribution budget: {} bytes per file.", MAX_BYTES));
    }
    if blob.data.contains(&0) {
        return Err(format!(
            "Attribution is unavailable for binary content in {display}."
        ));
    }
    let line_count = blob.data.iter().filter(|byte| **byte == b'\n').count()
        + usize::from(!blob.data.is_empty() && !blob.data.ends_with(b"\n"));
    if line_count > MAX_LINES {
        return Err(format!("Attribution budget: {MAX_LINES} source lines."));
    }
    let requested = lines;
    let mut note = None;
    let (mut start, mut end) = lines.unwrap_or((1, line_count));
    if line_count == 0 {
        return Ok(Blame {
            revision: revision.to_string(),
            path: display,
            path_bytes: path_bytes.to_vec(),
            start_line: 1,
            end_line: 1,
            lines: Vec::new(),
            complete: true,
            note: Some("Empty file".into()),
        });
    }
    start = start.max(1);
    if start > line_count {
        return Err(format!(
            "Line range starts beyond {display}'s {line_count} lines at {revision}."
        ));
    }
    end = end.max(start).min(line_count);
    if requested.is_some_and(|(_, requested_end)| requested_end > line_count) {
        note = Some(format!("Requested range clamped to line {line_count}."));
    }
    let relative = gix::path::from_bstr(gix::bstr::BStr::new(path_bytes))
        .map_err(|error| format!("Could not use path {display}: {error}"))?;
    check(cancelled)?;
    let output = Command::new("git")
        .current_dir(path)
        .args(["blame", "--line-porcelain", "-L"])
        .arg(format!("{start},{end}"))
        .arg(commit.to_string())
        .arg("--")
        .arg(relative.as_ref())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_err(|error| {
            format!("Could not read attribution using Git: {error}. Install Git and retry.")
        })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "Blame {display} at {revision} failed ({}).{}",
            output.status,
            if detail.is_empty() {
                String::new()
            } else {
                format!("\n{detail}")
            }
        ));
    }
    check(cancelled)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut attributed = Vec::new();
    let mut current: Option<Line> = None;
    for line in stdout.lines() {
        if line.starts_with('\t') {
            if let Some(line) = current.take() {
                attributed.push(line);
            }
            continue;
        }
        if let Some((commit, origin_line, final_line)) = header(line) {
            current = Some(Line {
                line: final_line,
                origin_line,
                commit,
                author: String::new(),
                email: String::new(),
                time: 0,
                summary: String::new(),
            });
            continue;
        }
        let Some(entry) = current.as_mut() else {
            continue;
        };
        if let Some(value) = line.strip_prefix("author ") {
            entry.author = value.to_string();
        } else if let Some(value) = line.strip_prefix("author-mail ") {
            entry.email = value
                .strip_prefix('<')
                .and_then(|value| value.strip_suffix('>'))
                .unwrap_or(value)
                .to_string();
        } else if let Some(value) = line.strip_prefix("author-time ") {
            entry.time = value.parse().unwrap_or_default();
        } else if let Some(value) = line.strip_prefix("summary ") {
            entry.summary = value.to_string();
        }
    }
    attributed.sort_by_key(|line| line.line);
    attributed.retain(|line| line.line >= start && line.line <= end);
    if attributed.is_empty() {
        return Err(format!(
            "Attribution for {display} at {revision} returned no lines."
        ));
    }
    Ok(Blame {
        revision: revision.to_string(),
        path: display,
        path_bytes: path_bytes.to_vec(),
        start_line: start,
        end_line: end,
        lines: attributed,
        complete: true,
        note,
    })
}

fn header(line: &str) -> Option<(String, usize, usize)> {
    let mut fields = line.split_whitespace();
    let commit = fields.next()?;
    if commit.len() < 40 || !commit.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let origin_line = fields.next()?.parse().ok()?;
    let final_line = fields.next()?.parse().ok()?;
    Some((commit.to_string(), origin_line, final_line))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{error::Error, fs, path::Path, process::Command};
    type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

    fn git(path: &Path, args: &[&str]) -> Result {
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
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }

    fn repo() -> Result<tempfile::TempDir> {
        let dir = tempfile::tempdir()?;
        git(dir.path(), &["init", "-b", "main"])?;
        git(dir.path(), &["config", "user.name", "Test"])?;
        git(dir.path(), &["config", "user.email", "test@example.org"])?;
        git(dir.path(), &["config", "core.autocrlf", "false"])?;
        fs::write(dir.path().join("a"), b"one\ntwo\nthree\n")?;
        git(dir.path(), &["add", "a"])?;
        git(dir.path(), &["commit", "-qm", "base"])?;
        fs::write(dir.path().join("a"), b"one\nTWO\nthree\nfour\n")?;
        git(dir.path(), &["commit", "-qam", "change"])?;
        Ok(dir)
    }

    #[test]
    fn attributes_a_bounded_range_and_preserves_origin_lines() -> Result {
        let dir = repo()?;
        let blame = read(dir.path(), "HEAD", b"a", None, &|| false)?;
        assert_eq!(blame.lines.len(), 4);
        assert_eq!(blame.lines[0].line, 1);
        assert_eq!(blame.lines[0].summary, "base");
        assert_eq!(blame.lines[1].summary, "change");
        assert_eq!(blame.lines[2].summary, "base");
        assert_eq!(blame.lines[3].summary, "change");
        assert_eq!(blame.lines[0].author, "Test");
        assert_eq!(blame.lines[0].email, "test@example.org");
        assert!(blame.lines[0].time > 0);
        assert_eq!(blame.path, "a");

        let one = read(dir.path(), "HEAD", b"a", Some((2, 3)), &|| false)?;
        assert_eq!(one.start_line, 2);
        assert_eq!(one.end_line, 3);
        assert_eq!(
            one.lines.iter().map(|line| line.line).collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert_eq!(one.lines[0].summary, "change");
        assert_eq!(one.lines[1].summary, "base");
        Ok(())
    }

    #[test]
    fn missing_paths_ranges_and_cancellation_are_explicit() -> Result {
        let dir = repo()?;
        assert!(read(dir.path(), "HEAD", b"missing", None, &|| false).is_err());
        assert!(read(dir.path(), "missing", b"a", None, &|| false).is_err());
        assert!(read(dir.path(), "HEAD", b"a", Some((9, 10)), &|| false).is_err());
        assert!(read(dir.path(), "HEAD", b"a", None, &|| true).is_err());
        Ok(())
    }

    #[test]
    fn binary_content_is_refused() -> Result {
        let dir = repo()?;
        fs::write(dir.path().join("binary"), [0, 1, 2, 3])?;
        git(dir.path(), &["add", "binary"])?;
        git(dir.path(), &["commit", "-qm", "binary"])?;
        let result = read(dir.path(), "HEAD", b"binary", None, &|| false);
        assert!(result.is_err(), "binary content must be refused");
        if let Err(error) = result {
            assert!(error.contains("binary"), "{error}");
        }
        Ok(())
    }
}
