//! Read unmerged stages and mark conflict sides resolved.
use gg_engine::conflict::{Content, Side};
use std::{
    path::Path,
    process::{Command, Output, Stdio},
};

const LIMIT: usize = 2 * 1024 * 1024;

pub fn unmerged(path: &Path) -> Result<Vec<String>, String> {
    let output = run(
        path,
        &[
            "diff".into(),
            "--name-only".into(),
            "-z".into(),
            "--diff-filter=U".into(),
        ],
    )?;
    if !output.status.success() {
        return Err(failure("List conflicts", &output));
    }
    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .map(|field| String::from_utf8_lossy(field).into_owned())
        .collect())
}

pub fn read(path: &Path, file: &str) -> Result<Content, String> {
    validate(file)?;
    let mut content = Content {
        path: file.to_string(),
        base: None,
        ours: None,
        theirs: None,
        binary: false,
        truncated: false,
    };
    for (stage, slot) in [(1, 0), (2, 1), (3, 2)] {
        if let Some((text, binary, truncated)) = stage_text(path, stage, file)? {
            content.binary |= binary;
            content.truncated |= truncated;
            match slot {
                0 => content.base = Some(text),
                1 => content.ours = Some(text),
                _ => content.theirs = Some(text),
            }
        }
    }
    if content.base.is_none() && content.ours.is_none() && content.theirs.is_none() {
        return Err(format!("No unmerged stages exist for {file}."));
    }
    Ok(content)
}

pub fn use_side(path: &Path, file: &str, side: Side) -> Result<(), String> {
    validate(file)?;
    let flag = match side {
        Side::Ours => "--ours",
        Side::Theirs => "--theirs",
        Side::Base => {
            return Err("Base content cannot be selected as the resolution side.".into());
        }
    };
    let output = run(
        path,
        &[
            "checkout".into(),
            flag.into(),
            "--".into(),
            file.to_string(),
        ],
    )?;
    if !output.status.success() {
        return Err(failure("Select conflict side", &output));
    }
    stage(path, file)
}

pub fn stage(path: &Path, file: &str) -> Result<(), String> {
    validate(file)?;
    let output = run(path, &["add".into(), "--".into(), file.to_string()])?;
    if !output.status.success() {
        return Err(failure("Stage resolved path", &output));
    }
    Ok(())
}

fn stage_text(path: &Path, stage: u8, file: &str) -> Result<Option<(String, bool, bool)>, String> {
    let output = run(path, &["show".into(), format!(":{stage}:{file}")])?;
    if !output.status.success() {
        return Ok(None);
    }
    let truncated = output.stdout.len() > LIMIT;
    let bytes = if truncated {
        &output.stdout[..LIMIT]
    } else {
        &output.stdout[..]
    };
    let binary = bytes.contains(&0) || std::str::from_utf8(bytes).is_err();
    let text = if binary {
        format!("[binary content, {} bytes]", output.stdout.len())
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    Ok(Some((text, binary, truncated)))
}

fn validate(file: &str) -> Result<(), String> {
    if file.is_empty() || file.contains('\0') {
        return Err("Conflict path is empty or invalid.".into());
    }
    Ok(())
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
        .map_err(|error| format!("Could not run Git conflict command: {error}"))
}

fn failure(label: &str, output: &Output) -> String {
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if detail.is_empty() {
        format!("{label} failed ({}).", output.status)
    } else {
        format!("{label} failed ({}).\n{detail}", output.status)
    }
}
