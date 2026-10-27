//! Inspect a gitlink's nested state and update it explicitly to the parent index.
use gg_engine::submodule::{Details, Receipt};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn command(path: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(path)
        .stdin(Stdio::null())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    command
}

fn run(path: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    command(path)
        .args(args)
        .output()
        .map_err(|error| format!("Could not run Git: {error}. Install Git and retry."))
}

fn detail(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

fn path_arg(path_bytes: &[u8]) -> Result<PathBuf, String> {
    gix::path::from_bstr(gix::bstr::BStr::new(path_bytes))
        .map(|path| path.into_owned())
        .map_err(|error| format!("Could not use submodule path: {error}"))
}

/// Read one gitlink's parent-recorded and nested state.
pub fn read(
    path: &Path,
    path_bytes: &[u8],
    recorded: Option<&str>,
    before: Option<&str>,
) -> Result<Details, String> {
    let display = gg_engine::review::display_path(path_bytes);
    let relative = path_arg(path_bytes)?;
    let mut note = None;
    let (section, url, branch) = module_values(path, &display);
    if section.is_none() {
        note = Some("Gitlink has no .gitmodules entry.".into());
    }
    let configured_url = section
        .as_ref()
        .and_then(|section| parent_config_value(path, &format!("{section}.url")));
    let index = index_gitlink(path, &relative)?;
    let status = submodule_status(path, &relative).unwrap_or_else(|error| {
        note.get_or_insert(error);
        None
    });
    let nested = path.join(&relative);
    let initialized = nested.join(".git").exists()
        && command(&nested)
            .args(["rev-parse", "--git-dir"])
            .output()
            .is_ok_and(|output| output.status.success());
    let checked_out = initialized
        .then(|| run(&nested, &["rev-parse", "HEAD"]).ok())
        .flatten()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty());
    let dirty = initialized
        .then(|| run(&nested, &["status", "--porcelain", "-z"]).ok())
        .flatten()
        .and_then(|output| output.status.success().then_some(!output.stdout.is_empty()));
    let target = recorded.or(index.as_deref());
    let recorded_available = if initialized {
        target.map(|commit| {
            run(
                &nested,
                &["cat-file", "-e", &format!("{commit}^{{commit}}")],
            )
            .is_ok_and(|output| output.status.success())
        })
    } else {
        None
    };
    Ok(Details {
        path: display,
        path_bytes: path_bytes.to_vec(),
        recorded: recorded.map(str::to_owned),
        before: before.map(str::to_owned),
        index,
        checked_out,
        initialized,
        dirty,
        url,
        branch,
        configured_url,
        recorded_available,
        status,
        note,
    })
}

fn module_values(path: &Path, display: &str) -> (Option<String>, Option<String>, Option<String>) {
    let output = run(
        path,
        &[
            "config",
            "-f",
            ".gitmodules",
            "--get-regexp",
            r"^submodule\..*\.path$",
        ],
    );
    let Ok(output) = output else {
        return (None, None, None);
    };
    if !output.status.success() {
        return (None, None, None);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(section) = text.lines().find_map(|line| {
        let (key, value) = line.split_once(' ')?;
        (value == display)
            .then(|| key.strip_suffix(".path").map(str::to_owned))
            .flatten()
    }) else {
        return (None, None, None);
    };
    let url = config_value(path, &format!("{section}.url"));
    let branch = config_value(path, &format!("{section}.branch"));
    (Some(section), url, branch)
}

fn config_value(path: &Path, key: &str) -> Option<String> {
    run(path, &["config", "-f", ".gitmodules", "--get", key])
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parent_config_value(path: &Path, key: &str) -> Option<String> {
    run(path, &["config", "--get", key])
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn index_gitlink(path: &Path, relative: &Path) -> Result<Option<String>, String> {
    let output = command(path)
        .args(["ls-files", "--stage", "--"])
        .arg(relative)
        .output()
        .map_err(|error| format!("Could not read the parent index: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Reading the parent index failed ({}).{}",
            output.status,
            match detail(&output) {
                detail if detail.is_empty() => String::new(),
                detail => format!("\n{detail}"),
            }
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        let mode = fields.next()?;
        let id = fields.next()?;
        (mode == "160000").then(|| id.to_string())
    }))
}

fn submodule_status(path: &Path, relative: &Path) -> Result<Option<String>, String> {
    let output = command(path)
        .args(["submodule", "status", "--"])
        .arg(relative)
        .output()
        .map_err(|error| format!("Could not read submodule status: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Submodule status failed ({}).{}",
            output.status,
            match detail(&output) {
                detail if detail.is_empty() => String::new(),
                detail => format!("\n{detail}"),
            }
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .find(|line| !line.is_empty())
        .and_then(|line| line.chars().next())
        .map(|prefix| prefix.to_string()))
}

/// Update the gitlink to the parent index commit after validating nested state.
pub fn update(path: &Path, path_bytes: &[u8]) -> Result<(Details, Receipt), String> {
    let details = read(path, path_bytes, None, None)?;
    details.can_update()?;
    let relative = path_arg(path_bytes)?;
    let output = command(path)
        .args(["submodule", "update", "--init", "--"])
        .arg(&relative)
        .output()
        .map_err(|error| format!("Could not update the submodule: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Submodule update failed ({}).{}",
            output.status,
            match detail(&output) {
                detail if detail.is_empty() => String::new(),
                detail => format!("\n{detail}"),
            }
        ));
    }
    let updated = read(path, path_bytes, None, None)?;
    Ok((updated.clone(), Receipt::from_details(&updated)))
}
