//! Read LFS pointers from immutable sides and fetch objects explicitly.
use gg_engine::{
    lfs::{Details, FetchReceipt, Parse, SidePointer, parse},
    review::{FileChange, FileSide},
};
use std::{
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

fn read_side(
    repo: &Path,
    side: &Option<FileSide>,
    note: &mut Option<String>,
) -> Option<SidePointer> {
    let side = side.as_ref()?;
    let bytes = match crate::blob::preview(repo, side, POINTER_LIMIT, || false) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return None,
        Err(error) => {
            note.get_or_insert(error);
            return None;
        }
    };
    match parse(&bytes) {
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

/// Read the LFS pointer state for both immutable sides of a file.
pub fn read(repo: &Path, change: &FileChange) -> Result<Details, String> {
    let mut note = None;
    let mut before = read_side(repo, &change.old, &mut note);
    let mut after = read_side(repo, &change.new, &mut note);
    let lfs_installed = command(repo)
        .args(["lfs", "version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    let common = command(repo)
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty());
    let common = common.map(|dir| {
        let path = PathBuf::from(dir);
        if path.is_absolute() {
            path
        } else {
            repo.join(path)
        }
    });
    for pointer in [&mut before, &mut after].into_iter().flatten() {
        if let Some(common) = &common {
            let object = common
                .join("lfs")
                .join("objects")
                .join(&pointer.oid[..2])
                .join(&pointer.oid[2..4])
                .join(&pointer.oid);
            pointer.available = object.is_file();
        }
    }
    let remote = command(repo)
        .args(["remote"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut names = text.lines().filter(|line| !line.is_empty());
            let first = names.next();
            if text.lines().any(|line| line == "origin") {
                Some("origin".to_string())
            } else {
                first.map(str::to_owned)
            }
        });
    if before.is_none() && after.is_none() && note.is_none() {
        note = Some("No LFS pointer on either side.".into());
    }
    if after.as_ref().or(before.as_ref()).is_some() && !lfs_installed {
        note = Some("git-lfs is not installed; fetch is unavailable.".into());
    }
    Ok(Details {
        path: change.path.clone(),
        path_bytes: change.path_bytes.clone(),
        before,
        after,
        lfs_installed,
        remote,
        note,
    })
}

/// Fetch the selected After/Before pointer object without smudging or staging.
pub fn fetch(repo: &Path, change: &FileChange) -> Result<(Details, FetchReceipt), String> {
    let details = read(repo, change)?;
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
    command.arg(format!("--include={}", change.path));
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
    let updated = read(repo, change)?;
    let available = updated.target().is_some_and(|pointer| pointer.available);
    Ok((
        updated,
        FetchReceipt {
            path: change.path.clone(),
            path_bytes: change.path_bytes.clone(),
            oid: target.oid,
            size: target.size,
            remote: details.remote,
            available,
        },
    ))
}
