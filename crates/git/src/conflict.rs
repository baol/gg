//! Read unmerged stages and mark conflict sides resolved.
use gg_engine::conflict::{Content, Resolution, Side, Snapshot};
use std::{
    io::Write,
    path::{Component, Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
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

#[derive(Clone)]
struct StageEntry {
    oid: String,
    mode: String,
}

type Stages = (Option<StageEntry>, Option<StageEntry>, Option<StageEntry>);

static TEMP: AtomicU64 = AtomicU64::new(0);

/// Capture the exact conflict stages and working bytes for a resolution preview.
pub fn snapshot(path: &Path, file: &str) -> Result<Snapshot, String> {
    validate(file)?;
    let (base, ours, theirs) = stages(path, file)?;
    if base.is_none() && ours.is_none() && theirs.is_none() {
        return Err(format!("No unmerged stages exist for {file}."));
    }
    let working = read_working(path, file)?;
    Ok(Snapshot {
        base: base.map(|entry| entry.oid),
        ours: ours.map(|entry| entry.oid),
        theirs: theirs.map(|entry| entry.oid),
        working,
    })
}

/// Publish one previewed resolution after revalidating every conflict input.
pub fn publish(
    path: &Path,
    file: &str,
    snapshot: &Snapshot,
    resolution: Resolution,
) -> Result<(), String> {
    validate(file)?;
    let (base, ours, theirs) = stages(path, file)?;
    if base.as_ref().map(|entry| entry.oid.clone()) != snapshot.base
        || ours.as_ref().map(|entry| entry.oid.clone()) != snapshot.ours
        || theirs.as_ref().map(|entry| entry.oid.clone()) != snapshot.theirs
    {
        return Err("The conflict changed. Refresh the resolution preview.".into());
    }
    if read_working(path, file)? != snapshot.working {
        return Err("The working file changed. Refresh external edits before staging.".into());
    }
    if resolution != Resolution::Working {
        let entry = match resolution {
            Resolution::Base => base,
            Resolution::Ours => ours,
            Resolution::Theirs => theirs,
            Resolution::Working => unreachable!(),
        }
        .ok_or_else(|| format!("The selected side is absent for {file}."))?;
        if !matches!(entry.mode.as_str(), "100644" | "100755") {
            return Err(
                "Resolve mode, symlink, or submodule conflicts with Git or an external editor."
                    .into(),
            );
        }
        let bytes = blob(path, &entry.oid)?;
        write_working(path, file, &bytes, &entry.mode)?;
    }
    stage(path, file)
}

fn stages(path: &Path, file: &str) -> Result<Stages, String> {
    let output = run(
        path,
        &[
            "ls-files".into(),
            "-u".into(),
            "-z".into(),
            "--".into(),
            file.to_string(),
        ],
    )?;
    if !output.status.success() {
        return Err(failure("Read conflict stages", &output));
    }
    let mut base = None;
    let mut ours = None;
    let mut theirs = None;
    for field in output.stdout.split(|byte| *byte == 0) {
        if field.is_empty() {
            continue;
        }
        let text = String::from_utf8_lossy(field);
        let Some((meta, _)) = text.split_once('\t') else {
            continue;
        };
        let mut parts = meta.split_whitespace();
        let (Some(mode), Some(oid), Some(stage)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let entry = StageEntry {
            oid: oid.to_string(),
            mode: mode.to_string(),
        };
        match stage {
            "1" => base = Some(entry),
            "2" => ours = Some(entry),
            "3" => theirs = Some(entry),
            _ => {}
        }
    }
    Ok((base, ours, theirs))
}

fn blob(path: &Path, oid: &str) -> Result<Vec<u8>, String> {
    let output = run(path, &["cat-file".into(), "blob".into(), oid.to_string()])?;
    if !output.status.success() {
        return Err(failure("Read conflict side", &output));
    }
    Ok(output.stdout)
}

fn read_working(path: &Path, file: &str) -> Result<Option<Vec<u8>>, String> {
    let target = worktree_path(path, file)?;
    match std::fs::read(&target) {
        Ok(bytes) if bytes.len() <= LIMIT => Ok(Some(bytes)),
        Ok(_) => Err("The conflict file exceeds the 2 MiB resolution preview budget.".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Read working file: {error}")),
    }
}

fn write_working(path: &Path, file: &str, bytes: &[u8], mode: &str) -> Result<(), String> {
    let target = worktree_path(path, file)?;
    #[cfg(not(unix))]
    let _ = mode;
    let parent = target
        .parent()
        .ok_or("The conflict path has no parent directory.")?;
    std::fs::create_dir_all(parent).map_err(|error| format!("Create directory: {error}"))?;
    let temp = parent.join(format!(
        ".gg-resolution-{}-{}",
        std::process::id(),
        TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| -> Result<(), std::io::Error> {
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        #[cfg(unix)]
        if mode == "100755" {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o755))?;
        }
        std::fs::rename(&temp, &target)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.map_err(|error| format!("Write working file: {error}"))
}

fn worktree_path(root: &Path, file: &str) -> Result<PathBuf, String> {
    let relative = Path::new(file);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("Use a repository-relative conflict path.".into());
    }
    Ok(root.join(relative))
}
