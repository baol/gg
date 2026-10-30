//! Context and whitespace options match `git diff` on the same bytes.
use gg_engine::diff::{Options, RowKind, Whitespace, compare_with};
use std::{error::Error, path::Path, process::Command};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

/// `git diff --no-index` body after the `+++` line, with hunk headers
/// written in the reader's `-a,b +c,d` form.
fn oracle(dir: &Path, flags: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "core.autocrlf=false",
            "diff",
            "--no-index",
            "--no-color",
        ])
        .args(flags)
        .args(["old", "new"])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .output()?;
    // `--no-index` exits 1 when the files differ.
    if output.status.code().is_none_or(|code| code > 1) {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    let mut body = vec![];
    let mut started = false;
    for line in output.stdout.split_inclusive(|b| *b == b'\n') {
        if !started {
            started = line.starts_with(b"+++ ");
            continue;
        }
        if let Some(rest) = line.strip_prefix(b"@@ -") {
            let text = String::from_utf8_lossy(rest);
            let (ranges, _) = text.split_once(" @@").ok_or("hunk header")?;
            let (old, new) = ranges.split_once(" +").ok_or("hunk ranges")?;
            let full = |range: &str| {
                if range.contains(',') {
                    range.to_string()
                } else {
                    format!("{range},1")
                }
            };
            body.extend_from_slice(format!("@@ -{} +{} @@\n", full(old), full(new)).as_bytes());
        } else {
            body.extend_from_slice(line);
        }
    }
    Ok(body)
}

fn ours(old: &[u8], new: &[u8], options: Options) -> Result<Vec<u8>> {
    let diff = compare_with(old, new, options, || false)?;
    let mut body = vec![];
    for row in diff.rows {
        match row.kind {
            RowKind::Hunk => body.extend_from_slice(format!("{}\n", row.text).as_bytes()),
            RowKind::NoNewline => body.extend_from_slice(format!("{}\n", row.text).as_bytes()),
            kind => {
                body.push(match kind {
                    RowKind::Added => b'+',
                    RowKind::Deleted => b'-',
                    _ => b' ',
                });
                body.extend_from_slice(&row.bytes);
                if !row.bytes.ends_with(b"\n") {
                    body.push(b'\n');
                }
            }
        }
    }
    Ok(body)
}

fn check(old: &[u8], new: &[u8], options: Options, flags: &[&str]) -> Result {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("old"), old)?;
    std::fs::write(dir.path().join("new"), new)?;
    let expected = oracle(dir.path(), flags)?;
    let actual = ours(old, new, options)?;
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
        "options {options:?} against git diff {flags:?}"
    );
    Ok(())
}

#[test]
fn context_lines_match_git_unified() -> Result {
    let old: String = (1..=40).map(|n| format!("line {n}\n")).collect();
    let new = old
        .replace("line 6\n", "line six\n")
        .replace("line 14\n", "line fourteen\n")
        .replace("line 21\n", "")
        .replace("line 35\n", "line 35\nline 35b\n");
    for context in [3, 5, 10] {
        check(
            old.as_bytes(),
            new.as_bytes(),
            Options {
                context,
                whitespace: Whitespace::All,
            },
            &[&format!("-U{context}")],
        )?;
    }
    Ok(())
}

#[test]
fn whitespace_options_match_git() -> Result {
    let old = b"alpha\nbeta  \ngamma\tdelta\nepsilon\nzeta\r\neta\ntheta\niota\nkappa";
    let new = b"alpha\nbeta\ngamma  delta\nEPSILON\nzeta\neta \ntheta\niota\nkappa\n";
    for (whitespace, flags) in [
        (Whitespace::All, &["-U3"][..]),
        (
            Whitespace::IgnoreAtEol,
            &["-U3", "--ignore-space-at-eol"][..],
        ),
        (Whitespace::IgnoreAll, &["-U3", "-w"][..]),
    ] {
        check(
            old,
            new,
            Options {
                context: 3,
                whitespace,
            },
            flags,
        )?;
    }
    Ok(())
}

#[test]
fn whitespace_only_changes_leave_no_hunks_and_say_so() -> Result {
    let old = b"one\ntwo\tthree\n";
    let new = b"one  \ntwo three\n";
    let options = Options {
        context: 3,
        whitespace: Whitespace::IgnoreAll,
    };
    check(old, new, options, &["-w"])?;
    let diff = compare_with(old, new, options, || false)?;
    assert!(diff.rows.is_empty());
    assert!(diff.note.contains("whitespace"), "{}", diff.note);
    Ok(())
}
