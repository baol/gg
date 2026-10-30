//! Automatic refresh: watch the worktree and the Git directory, and report
//! what changed once the burst of events settles.
//!
//! Working files, the index, and refs or operation state are told apart so
//! the window re-reads only what changed: status for files and the index, the
//! history and refs for refs and HEAD.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

/// Quiet time after the last event before the changes are reported.
const SETTLE: Duration = Duration::from_millis(300);

/// What changed since the last report.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Changes {
    /// A file in the worktree.
    pub worktree: bool,
    /// The index.
    pub index: bool,
    /// HEAD, a ref, or the state of a merge, rebase, cherry-pick, or revert.
    pub refs: bool,
}

impl Changes {
    pub fn any(self) -> bool {
        self.worktree || self.index || self.refs
    }

    fn add(&mut self, other: Changes) {
        self.worktree |= other.worktree;
        self.index |= other.index;
        self.refs |= other.refs;
    }
}

/// Where the repository keeps its files.
#[derive(Clone, Debug)]
pub struct Layout {
    pub workdir: PathBuf,
    /// This worktree's Git directory (`HEAD`, `index`, operation state).
    pub git_dir: PathBuf,
    /// The shared Git directory (`refs`, `packed-refs`); the same as
    /// `git_dir` outside linked worktrees.
    pub common_dir: PathBuf,
}

/// Sort one changed path; `None` for paths that never change what gg shows
/// (objects, logs, lock files, gg's own scratch files).
pub fn classify(layout: &Layout, path: &Path) -> Option<Changes> {
    let git = |dir: &Path| path.strip_prefix(dir).ok();
    for dir in [&layout.git_dir, &layout.common_dir] {
        let Some(relative) = git(dir) else {
            continue;
        };
        let name = relative.to_string_lossy().replace('\\', "/");
        if name.ends_with(".lock") {
            return None;
        }
        let first = name.split('/').next().unwrap_or("");
        return match first {
            "index" => Some(Changes {
                index: true,
                ..Default::default()
            }),
            "HEAD" | "ORIG_HEAD" | "packed-refs" | "refs" | "MERGE_HEAD" | "CHERRY_PICK_HEAD"
            | "REVERT_HEAD" | "rebase-merge" | "rebase-apply" | "sequencer" => Some(Changes {
                refs: true,
                ..Default::default()
            }),
            _ => None,
        };
    }
    path.starts_with(&layout.workdir).then_some(Changes {
        worktree: true,
        ..Default::default()
    })
}

/// Collects classified events and reports them once they settle.
#[derive(Default)]
pub struct Batch {
    changes: Changes,
    first: Option<Instant>,
    last: Option<Instant>,
}

impl Batch {
    pub fn push(&mut self, changes: Changes, at: Instant) {
        if !changes.any() {
            return;
        }
        self.changes.add(changes);
        self.first.get_or_insert(at);
        self.last = Some(at);
    }

    /// The changes and when the first of them happened, once quiet for
    /// [`SETTLE`], or at once when `now` is `None`.
    pub fn take(&mut self, now: Option<Instant>) -> Option<(Changes, Instant)> {
        let (first, last) = (self.first?, self.last?);
        if now.is_some_and(|now| now.duration_since(last) < SETTLE) {
            return None;
        }
        let changes = std::mem::take(&mut self.changes);
        self.first = None;
        self.last = None;
        Some((changes, first))
    }
}

pub struct Watcher {
    _inner: notify::RecommendedWatcher,
    receiver: Receiver<Changes>,
    batch: Batch,
    pub workdir: PathBuf,
}

impl Watcher {
    /// Watch the repository at `workdir`; `repaint` wakes the window after
    /// each event so the batch can settle.
    pub fn start(workdir: &Path, repaint: impl Fn() + Send + 'static) -> Result<Self, String> {
        use notify::Watcher as _;
        let layout = layout(workdir)?;
        let (sender, receiver) = channel();
        let classify_layout = layout.clone();
        let mut inner = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            let Ok(event) = event else {
                return;
            };
            if matches!(event.kind, notify::EventKind::Access(_)) {
                return;
            }
            let mut changes = Changes::default();
            for path in &event.paths {
                if let Some(found) = classify(&classify_layout, path) {
                    changes.add(found);
                }
            }
            if changes.any() && sender.send(changes).is_ok() {
                repaint();
            }
        })
        .map_err(|error| format!("Watch the repository: {error}"))?;
        inner
            .watch(&layout.workdir, notify::RecursiveMode::Recursive)
            .map_err(|error| format!("Watch the repository: {error}"))?;
        for dir in [&layout.git_dir, &layout.common_dir] {
            if !dir.starts_with(&layout.workdir) {
                inner
                    .watch(dir, notify::RecursiveMode::Recursive)
                    .map_err(|error| format!("Watch the Git directory: {error}"))?;
            }
        }
        Ok(Self {
            _inner: inner,
            receiver,
            batch: Batch::default(),
            workdir: workdir.to_path_buf(),
        })
    }

    fn drain(&mut self) {
        let now = Instant::now();
        while let Ok(changes) = self.receiver.try_recv() {
            self.batch.push(changes, now);
        }
    }

    /// Settled changes, if any; call once per frame.
    pub fn poll(&mut self) -> Option<(Changes, Instant)> {
        self.drain();
        self.batch.take(Some(Instant::now()))
    }

    /// Everything seen so far, settled or not: used before a command runs.
    pub fn flush(&mut self) -> Option<(Changes, Instant)> {
        self.drain();
        self.batch.take(None)
    }
}

/// Ask Git where this worktree's Git directories are.
fn layout(workdir: &Path) -> Result<Layout, String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args([
            "rev-parse",
            "--path-format=absolute",
            "--show-toplevel",
            "--git-dir",
            "--git-common-dir",
        ])
        .output()
        .map_err(|error| format!("Run git rev-parse: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines = text.lines().map(PathBuf::from);
    match (lines.next(), lines.next(), lines.next()) {
        (Some(workdir), Some(git_dir), Some(common_dir)) => Ok(Layout {
            workdir,
            git_dir,
            common_dir,
        }),
        _ => Err("git rev-parse did not name the Git directories".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout {
            workdir: "/r".into(),
            git_dir: "/r/.git".into(),
            common_dir: "/r/.git".into(),
        }
    }

    #[test]
    fn paths_are_sorted_into_worktree_index_and_refs() {
        let layout = layout();
        let kind = |path: &str| classify(&layout, Path::new(path));
        let worktree = Some(Changes {
            worktree: true,
            ..Default::default()
        });
        let index = Some(Changes {
            index: true,
            ..Default::default()
        });
        let refs = Some(Changes {
            refs: true,
            ..Default::default()
        });
        assert_eq!(kind("/r/src/main.rs"), worktree);
        assert_eq!(kind("/r/.git/index"), index);
        assert_eq!(kind("/r/.git/HEAD"), refs);
        assert_eq!(kind("/r/.git/refs/heads/main"), refs);
        assert_eq!(kind("/r/.git/rebase-merge/done"), refs);
        assert_eq!(kind("/r/.git/index.lock"), None);
        assert_eq!(kind("/r/.git/objects/ab/cdef"), None);
        assert_eq!(kind("/r/.git/logs/HEAD"), None);
        assert_eq!(kind("/elsewhere/file"), None);
    }

    #[test]
    fn a_batch_reports_after_it_settles_or_when_flushed() {
        let mut batch = Batch::default();
        let start = Instant::now();
        let file = Changes {
            worktree: true,
            ..Default::default()
        };
        batch.push(file, start);
        batch.push(
            Changes {
                refs: true,
                ..Default::default()
            },
            start + Duration::from_millis(100),
        );
        assert_eq!(batch.take(Some(start + Duration::from_millis(200))), None);
        let Some((changes, first)) = batch.take(Some(start + Duration::from_millis(500))) else {
            unreachable!("the batch settled");
        };
        assert!(changes.worktree && changes.refs && !changes.index);
        assert_eq!(first, start);
        assert!(!batch.changes.any());
        batch.push(file, start);
        assert!(batch.take(None).is_some(), "a flush does not wait");
    }

    #[test]
    fn a_real_repository_reports_a_changed_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["init", "-q"])
            .status()?;
        assert!(status.success());
        let mut watcher = Watcher::start(dir.path(), || {})?;
        std::fs::write(dir.path().join("file"), "x")?;
        let until = Instant::now() + Duration::from_secs(5);
        let mut seen = Changes::default();
        while Instant::now() < until && !seen.worktree {
            std::thread::sleep(Duration::from_millis(50));
            if let Some((changes, _)) = watcher.flush() {
                seen.add(changes);
            }
        }
        assert!(seen.worktree);
        Ok(())
    }
}
