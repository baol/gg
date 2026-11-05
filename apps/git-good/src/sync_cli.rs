//! Explicit clone, fetch, and push commands with JSON receipts.
use crate::cli_args::usage_error;
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
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut remote = None;
    let mut prune = false;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--prune") if !prune => prune = true,
            Some(value) if value.starts_with("--") => {
                return Err(usage_error("Unknown fetch option"));
            }
            _ if remote.is_none() => {
                remote = Some(
                    arg.into_string()
                        .map_err(|_| usage_error("Remote must be UTF-8"))?,
                );
            }
            _ => {
                return Err(usage_error(
                    "Usage: git good --cli fetch [REMOTE] [--prune] [--repo PATH]",
                ));
            }
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
    let mut args = crate::cli_args::Args::new(args);
    let mut path = std::env::current_dir()?;
    let mut remote = None;
    let mut branch = None;
    let mut set_upstream = false;
    let mut force_with_lease = false;
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some("--repo") => {
                path = PathBuf::from(
                    args.value()
                        .ok_or_else(|| usage_error("--repo needs a path"))?,
                )
            }
            Some("--set-upstream") if !set_upstream => set_upstream = true,
            Some("--force-with-lease") if !force_with_lease => force_with_lease = true,
            Some(value) if value.starts_with("--") => {
                return Err(usage_error("Unknown push option"));
            }
            _ if remote.is_none() => {
                remote = Some(
                    arg.into_string()
                        .map_err(|_| usage_error("Remote must be UTF-8"))?,
                );
            }
            _ if branch.is_none() => {
                branch = Some(
                    arg.into_string()
                        .map_err(|_| usage_error("Branch must be UTF-8"))?,
                );
            }
            _ => {
                return Err(usage_error(
                    "Usage: git good --cli push [REMOTE] [BRANCH] [--set-upstream] [--force-with-lease] [--repo PATH]",
                ));
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
    let mut args = crate::cli_args::Args::new(args);
    while let Some(arg) = args.next() {
        match args.option(&arg) {
            Some(value) if value.starts_with("--") => {
                return Err(usage_error("Unknown clone option"));
            }
            _ if url.is_none() => {
                url = Some(
                    arg.into_string()
                        .map_err(|_| usage_error("URL must be UTF-8"))?,
                );
            }
            _ if destination.is_none() => destination = Some(PathBuf::from(arg)),
            _ => return Err(usage_error("Usage: git good --cli clone URL [PATH]")),
        }
    }
    let url = url.ok_or_else(|| usage_error("Clone requires a repository URL"))?;
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
    crate::cli_args::emit(&receipt)?;
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
