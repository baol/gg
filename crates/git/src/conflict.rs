//! Read unmerged stages, preview a resolution, and publish it explicitly.
use gg_engine::conflict::{Content, Resolution, Segment, Side, Snapshot};
use std::{
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

const LIMIT: usize = 2 * 1024 * 1024;

#[derive(Clone)]
struct StageEntry {
    oid: String,
    mode: String,
}

type Stages = (Option<StageEntry>, Option<StageEntry>, Option<StageEntry>);

#[derive(Clone, PartialEq, Eq)]
struct Working {
    bytes: Option<Vec<u8>>,
    hash: Option<u64>,
    note: Option<String>,
}

static TEMP: AtomicU64 = AtomicU64::new(0);

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
        working: working.bytes,
        working_hash: working.hash,
        working_note: working.note,
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
        return Err("The conflict changed. Refresh external edits and retry.".into());
    }
    let current = read_working(path, file)?;
    if current != working_snapshot(snapshot) {
        return Err("The working file changed. Refresh external edits before staging.".into());
    }
    if resolution == Resolution::Working {
        if current.bytes.is_none() && current.hash.is_none() {
            return Err(current.note.unwrap_or_else(|| {
                "The working file cannot be staged as a previewed result.".into()
            }));
        }
        return stage(path, file);
    }
    let (bytes, mode) = {
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
        (blob(path, &entry.oid)?, entry.mode)
    };
    write_working(path, file, &bytes, &mode)?;
    let written = read_working(path, file)?;
    let written_ok = if bytes.len() > LIMIT {
        written.bytes.is_none() && written.hash == Some(hash(&bytes))
    } else {
        written.bytes.as_deref() == Some(bytes.as_slice())
    };
    if !written_ok {
        return Err(
            "The working file changed while writing the resolution. Refresh external edits and retry."
                .into(),
        );
    }
    if let Err(error) = stage(path, file) {
        return Err(format!(
            "The working file was written but staging failed: {error}\nRefresh status and stage the resolution again."
        ));
    }
    Ok(())
}

/// The file's conflict as common text and blocks, from a fresh three-way
/// merge of its stages (`git merge-file --diff3`), so the result does not
/// depend on the working file or the user's conflict style.
pub fn blocks(path: &Path, file: &str) -> Result<Vec<Segment>, String> {
    validate(file)?;
    let (base, ours, theirs) = stages(path, file)?;
    let (Some(ours), Some(theirs)) = (ours, theirs) else {
        return Err("One side deleted this file. Choose a whole side instead.".into());
    };
    for entry in [&ours, &theirs].into_iter().chain(base.as_ref()) {
        if !matches!(entry.mode.as_str(), "100644" | "100755") {
            return Err(
                "Mode, symlink, or submodule conflicts have no text blocks. Choose a whole side."
                    .into(),
            );
        }
    }
    let text = |entry: Option<&StageEntry>| -> Result<Vec<u8>, String> {
        let bytes = match entry {
            Some(entry) => blob(path, &entry.oid)?,
            None => Vec::new(),
        };
        if bytes.len() > LIMIT || bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
            return Err(
                "This file is binary or too large for text blocks. Choose a whole side.".into(),
            );
        }
        Ok(bytes)
    };
    let sides = [
        ("ours", text(Some(&ours))?),
        ("base", text(base.as_ref())?),
        ("theirs", text(Some(&theirs))?),
    ];
    let dir = std::env::temp_dir().join(format!(
        "gg-merge-{}-{}",
        std::process::id(),
        TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).map_err(|error| format!("Prepare the merge: {error}"))?;
    let result = (|| {
        let mut args = vec!["merge-file".to_string(), "-p".into(), "--diff3".into()];
        for (name, _) in &sides {
            args.push("-L".into());
            args.push((*name).into());
        }
        for (name, bytes) in &sides {
            let side = dir.join(name);
            std::fs::write(&side, bytes).map_err(|error| format!("Prepare the merge: {error}"))?;
            args.push(side.to_string_lossy().into_owned());
        }
        let output = run(path, &args)?;
        // merge-file exits with the number of conflicts, or above 127 on error.
        if output
            .status
            .code()
            .is_none_or(|code| !(0..=127).contains(&code))
        {
            return Err(failure("Merge the conflict sides", &output));
        }
        gg_engine::conflict::parse_diff3(&String::from_utf8_lossy(&output.stdout))
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// Publish composed text as the file's resolution after revalidating every
/// conflict input, keeping the executable bit of our side.
pub fn publish_text(
    path: &Path,
    file: &str,
    snapshot: &Snapshot,
    text: &str,
) -> Result<(), String> {
    validate(file)?;
    let (base, ours, theirs) = stages(path, file)?;
    if base.as_ref().map(|entry| entry.oid.clone()) != snapshot.base
        || ours.as_ref().map(|entry| entry.oid.clone()) != snapshot.ours
        || theirs.as_ref().map(|entry| entry.oid.clone()) != snapshot.theirs
    {
        return Err("The conflict changed. Refresh external edits and retry.".into());
    }
    if read_working(path, file)? != working_snapshot(snapshot) {
        return Err("The working file changed. Refresh external edits before staging.".into());
    }
    let mode = ours
        .or(theirs)
        .map(|entry| entry.mode)
        .filter(|mode| matches!(mode.as_str(), "100644" | "100755"))
        .ok_or("Resolve mode, symlink, or submodule conflicts with Git or an external editor.")?;
    let bytes = text.as_bytes();
    write_working(path, file, bytes, &mode)?;
    if read_working(path, file)?.bytes.as_deref() != Some(bytes) {
        return Err(
            "The working file changed while writing the resolution. Refresh external edits and retry."
                .into(),
        );
    }
    stage(path, file).map_err(|error| {
        format!(
            "The working file was written but staging failed: {error}\nRefresh status and stage the resolution again."
        )
    })
}

fn hash(bytes: &[u8]) -> u64 {
    let mut digest = 0xcbf29ce484222325u64;
    for byte in bytes {
        digest ^= u64::from(*byte);
        digest = digest.wrapping_mul(0x100000001b3);
    }
    digest
}

fn working_snapshot(snapshot: &Snapshot) -> Working {
    Working {
        bytes: snapshot.working.clone(),
        hash: snapshot.working_hash,
        note: snapshot.working_note.clone(),
    }
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

fn read_working(path: &Path, file: &str) -> Result<Working, String> {
    let target = worktree_path(path, file)?;
    let metadata = match std::fs::symlink_metadata(&target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Working {
                bytes: None,
                hash: None,
                note: Some("Working file absent.".into()),
            });
        }
        Err(error) => return Err(format!("Read working file: {error}")),
    };
    if metadata.file_type().is_symlink() {
        return Ok(Working {
            bytes: None,
            hash: None,
            note: Some("Working path is a symlink; resolve it externally.".into()),
        });
    }
    if !metadata.is_file() {
        return Ok(Working {
            bytes: None,
            hash: None,
            note: Some(
                "Working path is not a regular file; choose a side or resolve externally.".into(),
            ),
        });
    }
    if metadata.len() > LIMIT as u64 {
        let mut file =
            std::fs::File::open(&target).map_err(|error| format!("Read working file: {error}"))?;
        let mut buffer = [0u8; 64 * 1024];
        let mut digest = 0xcbf29ce484222325u64;
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|error| format!("Read working file: {error}"))?;
            if read == 0 {
                break;
            }
            for byte in &buffer[..read] {
                digest ^= u64::from(*byte);
                digest = digest.wrapping_mul(0x100000001b3);
            }
        }
        return Ok(Working {
            bytes: None,
            hash: Some(digest),
            note: Some(format!(
                "Working file is {} bytes; exact bytes are validated without a preview.",
                metadata.len()
            )),
        });
    }
    let bytes = std::fs::read(&target).map_err(|error| format!("Read working file: {error}"))?;
    Ok(Working {
        bytes: Some(bytes),
        hash: None,
        note: None,
    })
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
    let repo = gix::discover(path)
        .map_err(|error| format!("Could not read repository {}: {error}", path.display()))?;
    let root = repo.workdir().unwrap_or_else(|| repo.path());
    Command::new("git")
        .current_dir(root)
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
