//! Explicit clone, fetch, and push commands with JSON receipts.
use gg_engine::sync::{Operation, Progress};
use std::{
    ffi::OsString,
    io::IsTerminal,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub fn fetch(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = args;
    let mut path = std::env::current_dir()?;
    let mut remote = None;
    let mut prune = false;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--prune") if !prune => prune = true,
            Some(value) if value.starts_with("--") => return Err("Unknown fetch option".into()),
            _ if remote.is_none() => {
                remote = Some(arg.into_string().map_err(|_| "Remote must be UTF-8")?);
            }
            _ => return Err("Usage: git good fetch [REMOTE] [--prune] [--repo PATH]".into()),
        }
    }
    let mut progress = progress_writer();
    let receipt = gg_git::sync::fetch(
        &path,
        remote.as_deref(),
        prune,
        std::io::stdin().is_terminal(),
        &std::sync::atomic::AtomicBool::new(false),
        &mut progress.writer,
    )?;
    progress.finish();
    write_receipt(serde_json::json!({
        "operation": Operation::Fetch,
        "repository": display_repository(&path),
        "remote": receipt.remote,
        "pruned": receipt.pruned,
        "complete": true,
    }))?;
    Ok(())
}

pub fn push(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = args;
    let mut path = std::env::current_dir()?;
    let mut remote = None;
    let mut branch = None;
    let mut set_upstream = false;
    let mut force_with_lease = false;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--repo") => path = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            Some("--set-upstream") if !set_upstream => set_upstream = true,
            Some("--force-with-lease") if !force_with_lease => force_with_lease = true,
            Some(value) if value.starts_with("--") => return Err("Unknown push option".into()),
            _ if remote.is_none() => {
                remote = Some(arg.into_string().map_err(|_| "Remote must be UTF-8")?);
            }
            _ if branch.is_none() => {
                branch = Some(arg.into_string().map_err(|_| "Branch must be UTF-8")?);
            }
            _ => {
                return Err(
                    "Usage: git good push [REMOTE] [BRANCH] [--set-upstream] [--force-with-lease] [--repo PATH]".into(),
                );
            }
        }
    }
    let lease = if force_with_lease {
        Some(gg_git::sync::push_lease(
            &path,
            remote.as_deref(),
            branch.as_deref(),
        )?)
    } else {
        None
    };
    let mut progress = progress_writer();
    let receipt = gg_git::sync::push(
        &path,
        remote.as_deref(),
        branch.as_deref(),
        gg_git::sync::PushOptions {
            set_upstream,
            force_with_lease: lease,
        },
        std::io::stdin().is_terminal(),
        &std::sync::atomic::AtomicBool::new(false),
        &mut progress.writer,
    )?;
    progress.finish();
    write_receipt(serde_json::json!({
        "operation": Operation::Push,
        "repository": display_repository(&path),
        "remote": receipt.remote,
        "branch": receipt.branch,
        "upstream": receipt.upstream,
        "set_upstream": receipt.set_upstream,
        "force_with_lease": receipt.force_with_lease,
        "lease": receipt.lease,
        "complete": true,
    }))?;
    Ok(())
}

pub fn clone(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut url = None;
    let mut destination = None;
    for arg in args {
        match arg.to_str() {
            Some(value) if value.starts_with("--") => return Err("Unknown clone option".into()),
            _ if url.is_none() => {
                url = Some(arg.into_string().map_err(|_| "URL must be UTF-8")?);
            }
            _ if destination.is_none() => destination = Some(PathBuf::from(arg)),
            _ => return Err("Usage: git good clone URL [PATH]".into()),
        }
    }
    let url = url.ok_or("Clone requires a repository URL")?;
    let mut progress = progress_writer();
    let receipt = gg_git::sync::clone(
        &url,
        destination.as_deref(),
        std::io::stdin().is_terminal(),
        &std::sync::atomic::AtomicBool::new(false),
        &mut progress.writer,
    )?;
    progress.finish();
    write_receipt(serde_json::json!({
        "operation": Operation::Clone,
        "path": receipt.path,
        "head": receipt.head,
        "branch": receipt.branch,
        "complete": true,
    }))?;
    Ok(())
}

fn write_receipt(receipt: serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer(std::io::stdout().lock(), &receipt)?;
    println!();
    Ok(())
}

fn display_repository(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

struct ProgressWriter {
    enabled: bool,
    printed: Arc<AtomicBool>,
    writer: Box<dyn FnMut(Progress)>,
}

impl ProgressWriter {
    fn finish(&self) {
        if self.enabled && self.printed.load(Ordering::Relaxed) {
            eprintln!();
        }
    }
}

fn progress_writer() -> ProgressWriter {
    let enabled = std::io::stderr().is_terminal();
    let printed = Arc::new(AtomicBool::new(false));
    let writer_printed = Arc::clone(&printed);
    ProgressWriter {
        enabled,
        printed,
        writer: Box::new(move |progress| {
            if !enabled {
                return;
            }
            writer_printed.store(true, Ordering::Relaxed);
            let percent = progress
                .percent
                .map(|percent| format!(" {percent}%"))
                .unwrap_or_default();
            eprint!(
                "\r{} · {}{percent} · {:<48}",
                progress.operation.as_str(),
                progress.phase,
                progress.message
            );
        }),
    }
}
