//! Read LFS pointer identity from review or working-tree sides, fetch objects explicitly, and read bounded previews.
use gg_engine::{
    lfs::{Details, FetchReceipt, Parse, SidePointer, parse},
    review::{FileChange, FileSide},
    status::{Area, WorkingRequest},
};
use std::{
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const POINTER_LIMIT: u64 = 4096;

fn command(path: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(path)
        .stdin(Stdio::null())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    command
}

fn detail(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

fn path_arg(bytes: &[u8]) -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        std::ffi::OsStr::from_bytes(bytes).to_os_string()
    }
    #[cfg(not(unix))]
    {
        OsString::from(String::from_utf8_lossy(bytes).into_owned())
    }
}

fn pointer_from(bytes: &[u8], note: &mut Option<String>) -> Option<SidePointer> {
    match parse(bytes) {
        Parse::Pointer(pointer) => Some(SidePointer {
            oid: pointer.oid,
            size: pointer.size,
            available: false,
        }),
        Parse::Malformed(error) => {
            note.get_or_insert(error);
            None
        }
        Parse::NotPointer => None,
    }
}

fn read_side(repo: &Path, side: &FileSide, note: &mut Option<String>) -> Option<SidePointer> {
    let bytes = match crate::blob::preview(repo, side, POINTER_LIMIT, || false) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return None,
        Err(error) => {
            note.get_or_insert(error);
            return None;
        }
    };
    pointer_from(&bytes, note)
}

fn read_working_side(
    repo: &Path,
    path_bytes: &[u8],
    note: &mut Option<String>,
) -> Option<SidePointer> {
    let path = repo.join(PathBuf::from(path_arg(path_bytes)));
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            note.get_or_insert(format!("Read working file: {error}"));
            return None;
        }
    };
    if !metadata.file_type().is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    match fs::File::open(&path)
        .and_then(|file| file.take(POINTER_LIMIT + 1).read_to_end(&mut bytes))
    {
        Ok(_) => {}
        Err(error) => {
            note.get_or_insert(format!("Read working file: {error}"));
            return None;
        }
    }
    if bytes.len() as u64 > POINTER_LIMIT {
        return None;
    }
    pointer_from(&bytes, note)
}

fn head_side(repo: &Path, path_bytes: &[u8]) -> Option<FileSide> {
    let output = command(repo)
        .args(["ls-tree", "-z", "HEAD", "--"])
        .arg(path_arg(path_bytes))
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    parse_ls_tree(&output.stdout)
}

fn parse_ls_tree(bytes: &[u8]) -> Option<FileSide> {
    let record = bytes.split(|byte| *byte == 0).next()?;
    let meta = record.split(|byte| *byte == b'\t').next()?;
    let meta = std::str::from_utf8(meta).ok()?;
    let mut fields = meta.split(' ');
    let mode = u16::from_str_radix(fields.next()?, 8).ok()?;
    let kind = fields.next()?;
    let id = fields.next()?.to_string();
    (kind == "blob").then_some(FileSide { id, mode })
}

fn index_side(repo: &Path, path_bytes: &[u8]) -> Option<FileSide> {
    let output = command(repo)
        .args(["ls-files", "-s", "-z", "--"])
        .arg(path_arg(path_bytes))
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    parse_ls_files(&output.stdout)
}

fn parse_ls_files(bytes: &[u8]) -> Option<FileSide> {
    let record = bytes.split(|byte| *byte == 0).next()?;
    let meta = record.split(|byte| *byte == b'\t').next()?;
    let meta = std::str::from_utf8(meta).ok()?;
    let mut fields = meta.split(' ');
    let mode = u16::from_str_radix(fields.next()?, 8).ok()?;
    let id = fields.next()?.to_string();
    let stage = fields.next()?;
    (stage == "0").then_some(FileSide { id, mode })
}

fn installed(repo: &Path) -> bool {
    command(repo)
        .args(["lfs", "version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn common_dir(repo: &Path) -> Option<PathBuf> {
    let output = command(repo)
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if dir.is_empty() {
        return None;
    }
    let path = PathBuf::from(dir);
    Some(if path.is_absolute() {
        path
    } else {
        repo.join(path)
    })
}

fn object_path_in(common: &Path, oid: &str) -> Option<PathBuf> {
    if oid.len() < 4 || !oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(
        common
            .join("lfs")
            .join("objects")
            .join(&oid[..2])
            .join(&oid[2..4])
            .join(oid),
    )
}

/// Locate one object in the repository's Git LFS object store.
pub fn object_path(repo: &Path, oid: &str) -> Option<PathBuf> {
    object_path_in(&common_dir(repo)?, oid)
}

fn remote(repo: &Path) -> Option<String> {
    let output = command(repo)
        .args(["remote"])
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let text = String::from_utf8_lossy(&output.stdout);
    let names = text
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if names.contains(&"origin") {
        Some("origin".to_string())
    } else {
        names.first().map(|name| (*name).to_string())
    }
}

fn details(
    repo: &Path,
    path: &str,
    path_bytes: &[u8],
    mut before: Option<SidePointer>,
    mut after: Option<SidePointer>,
    mut note: Option<String>,
) -> Details {
    let lfs_installed = installed(repo);
    let common = common_dir(repo);
    for pointer in [&mut before, &mut after].into_iter().flatten() {
        pointer.available = common
            .as_deref()
            .and_then(|common| object_path_in(common, &pointer.oid))
            .is_some_and(|object| object.is_file());
    }
    let remote = remote(repo);
    if before.is_none() && after.is_none() && note.is_none() {
        note = Some("No LFS pointer on either side.".into());
    }
    if (before.is_some() || after.is_some()) && !lfs_installed {
        note = Some("git-lfs is not installed; fetch is unavailable.".into());
    }
    Details {
        path: path.to_string(),
        path_bytes: path_bytes.to_vec(),
        before,
        after,
        lfs_installed,
        remote,
        note,
    }
}

/// Read the LFS pointer state for both immutable sides of a file.
pub fn read(repo: &Path, change: &FileChange) -> Result<Details, String> {
    let mut note = None;
    let before = change
        .old
        .as_ref()
        .and_then(|side| read_side(repo, side, &mut note));
    let after = change
        .new
        .as_ref()
        .and_then(|side| read_side(repo, side, &mut note));
    Ok(details(
        repo,
        &change.path,
        &change.path_bytes,
        before,
        after,
        note,
    ))
}

/// Read the LFS pointer state for one Local Changes comparison.
pub fn read_working(repo: &Path, request: &WorkingRequest) -> Result<Details, String> {
    let entry = &request.entry;
    let mut note = None;
    let (before, after) = match entry.area {
        Area::Staged => (
            head_side(repo, &entry.path_bytes)
                .as_ref()
                .and_then(|side| read_side(repo, side, &mut note)),
            index_side(repo, &entry.path_bytes)
                .as_ref()
                .and_then(|side| read_side(repo, side, &mut note)),
        ),
        Area::Unstaged => (
            index_side(repo, &entry.path_bytes)
                .as_ref()
                .and_then(|side| read_side(repo, side, &mut note)),
            read_working_side(repo, &entry.path_bytes, &mut note),
        ),
        Area::Untracked => (None, read_working_side(repo, &entry.path_bytes, &mut note)),
        Area::Conflict => {
            note =
                Some("Unmerged path; resolve the conflict before inspecting LFS content.".into());
            (None, None)
        }
    };
    Ok(details(
        repo,
        &entry.path,
        &entry.path_bytes,
        before,
        after,
        note,
    ))
}

fn fetch_inner(
    repo: &Path,
    path: &str,
    path_bytes: &[u8],
    details: Details,
    reread: impl FnOnce(&Path) -> Result<Details, String>,
) -> Result<(Details, FetchReceipt), String> {
    details.can_fetch()?;
    let target = details
        .target()
        .cloned()
        .ok_or("No LFS pointer is selected.")?;
    let mut command = command(repo);
    command.arg("lfs").arg("fetch");
    if let Some(remote) = &details.remote {
        command.arg(remote);
    }
    command.arg(format!("--include={path}"));
    let output = command
        .output()
        .map_err(|error| format!("Could not run git-lfs: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "LFS fetch failed ({}).{}",
            output.status,
            match detail(&output) {
                detail if detail.is_empty() => String::new(),
                detail => format!("\n{detail}"),
            }
        ));
    }
    let updated = reread(repo)?;
    let available = updated.target().is_some_and(|pointer| pointer.available);
    Ok((
        updated,
        FetchReceipt {
            path: path.to_string(),
            path_bytes: path_bytes.to_vec(),
            oid: target.oid,
            size: target.size,
            remote: details.remote,
            available,
        },
    ))
}

/// Fetch the selected After/Before pointer object without smudging or staging.
pub fn fetch(repo: &Path, change: &FileChange) -> Result<(Details, FetchReceipt), String> {
    fetch_inner(
        repo,
        &change.path,
        &change.path_bytes,
        read(repo, change)?,
        |repo| read(repo, change),
    )
}

/// Fetch the selected working-tree pointer object without smudging or staging.
pub fn fetch_working(
    repo: &Path,
    request: &WorkingRequest,
) -> Result<(Details, FetchReceipt), String> {
    fetch_inner(
        repo,
        &request.entry.path,
        &request.entry.path_bytes,
        read_working(repo, request)?,
        |repo| read_working(repo, request),
    )
}

/// Read a bounded LFS object for preview; `None` means the object is not present.
pub fn read_object(repo: &Path, oid: &str, limit: u64) -> Result<Option<Vec<u8>>, String> {
    let Some(path) = object_path(repo, oid) else {
        return Ok(None);
    };
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Read LFS object: {error}")),
    };
    if !metadata.is_file() {
        return Ok(None);
    }
    if metadata.len() > limit {
        return Err(format!(
            "LFS object is {}; the content preview is limited to {}.",
            size_label(metadata.len()),
            size_label(limit)
        ));
    }
    fs::read(&path)
        .map(Some)
        .map_err(|error| format!("Read LFS object: {error}"))
}

fn size_label(value: u64) -> String {
    if value < 1024 {
        format!("{value} B")
    } else if value < 1024 * 1024 {
        format!("{:.1} KiB", value as f64 / 1024.0)
    } else {
        format!("{:.1} MiB", value as f64 / (1024.0 * 1024.0))
    }
}
