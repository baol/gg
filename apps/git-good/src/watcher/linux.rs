//! The Linux backend: inotify, asked only for the events that can change
//! what gg shows. notify's inotify backend always asks for open events too,
//! and Git opens hundreds of files per command, so every command run while
//! a window watched its repository woke notify's thread hundreds of times
//! and took 2–3 ms longer than from a shell (RULE-16). This one also stays
//! out of the Git directory's objects and logs, which [`classify`] ignores.

use super::{Changes, Layout, classify};
use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask, Watches};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Instant;

/// Created, deleted, moved, written and closed, or its mode changed. Not
/// opened or read, and not each write: a file is reported once it closes.
fn mask() -> WatchMask {
    WatchMask::CREATE
        | WatchMask::DELETE
        | WatchMask::CLOSE_WRITE
        | WatchMask::MOVED_FROM
        | WatchMask::MOVED_TO
        | WatchMask::ATTRIB
}

/// Whether anything inside `dir` can change what gg shows. In a Git
/// directory only its top level and the ref and operation state folders
/// can; the whole worktree can.
fn wanted(layout: &Layout, dir: &Path) -> bool {
    for git in [&layout.git_dir, &layout.common_dir] {
        let Ok(relative) = dir.strip_prefix(git) else {
            continue;
        };
        return match relative.components().next() {
            None => true,
            Some(first) => matches!(
                first.as_os_str().to_str(),
                Some("refs" | "rebase-merge" | "rebase-apply" | "sequencer")
            ),
        };
    }
    dir.starts_with(&layout.workdir)
}

/// Watches the repository until dropped.
pub struct Backend {
    watches: Watches,
    root: WatchDescriptor,
    stop: Arc<AtomicBool>,
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Removing a watch queues an event, which wakes the thread to stop.
        let _ = self.watches.remove(self.root.clone());
    }
}

/// Watch `layout`'s worktree and Git directories; send what changed and
/// call `repaint` after each burst of events read together.
pub fn start(
    layout: Layout,
    sender: Sender<(Changes, Instant)>,
    repaint: impl Fn() + Send + 'static,
) -> Result<Backend, String> {
    let inotify = Inotify::init().map_err(|error| format!("Watch the repository: {error}"))?;
    let mut watches = inotify.watches();
    let root = watches
        .add(&layout.workdir, mask())
        .map_err(|error| format!("Watch the repository: {error}"))?;
    let stop = Arc::new(AtomicBool::new(false));
    let thread = Thread {
        inotify,
        watches: watches.clone(),
        dirs: HashMap::from([(root.clone(), layout.workdir.clone())]),
        layout,
        sender,
        stop: stop.clone(),
    };
    std::thread::Builder::new()
        .name("gg-watcher".into())
        .spawn(move || thread.run(repaint))
        .map_err(|error| format!("Watch the repository: {error}"))?;
    Ok(Backend {
        watches,
        root,
        stop,
    })
}

struct Thread {
    inotify: Inotify,
    watches: Watches,
    /// The directory each watch is on.
    dirs: HashMap<WatchDescriptor, PathBuf>,
    layout: Layout,
    sender: Sender<(Changes, Instant)>,
    stop: Arc<AtomicBool>,
}

impl Thread {
    fn run(mut self, repaint: impl Fn()) {
        // Off the caller's thread: a large worktree takes a while to walk.
        let workdir = self.layout.workdir.clone();
        self.add_children(&workdir);
        for dir in [self.layout.git_dir.clone(), self.layout.common_dir.clone()] {
            if !dir.starts_with(&workdir) {
                self.add_tree(&dir);
            }
        }
        let mut buffer = vec![0; 64 * 1024];
        loop {
            let changes = match self.read(&mut buffer) {
                Ok(changes) => changes,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return,
            };
            if self.stop.load(Ordering::Relaxed) {
                return;
            }
            if changes.any() {
                if self.sender.send((changes, Instant::now())).is_err() {
                    return;
                }
                repaint();
            }
        }
    }

    /// Wait for events; what they changed. Watches follow folders created,
    /// moved in, or moved away.
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<Changes> {
        let mut changes = Changes::default();
        let mut added = Vec::new();
        let mut moved = Vec::new();
        for event in self.inotify.read_events_blocking(buffer)? {
            if event.mask.contains(EventMask::Q_OVERFLOW) {
                // Events were lost: anything may have changed.
                changes.add(Changes {
                    worktree: true,
                    index: true,
                    refs: true,
                    config: true,
                });
                continue;
            }
            if event.mask.contains(EventMask::IGNORED) {
                self.dirs.remove(&event.wd);
                continue;
            }
            let Some(dir) = self.dirs.get(&event.wd) else {
                continue;
            };
            let path = match event.name {
                Some(name) => dir.join(name),
                None => dir.clone(),
            };
            if event.mask.contains(EventMask::ISDIR) {
                if event.mask.contains(EventMask::MOVED_FROM) {
                    moved.push(path.clone());
                }
                if event
                    .mask
                    .intersects(EventMask::CREATE | EventMask::MOVED_TO)
                {
                    added.push(path.clone());
                }
            }
            if let Some(found) = classify(&self.layout, &path) {
                changes.add(found);
            }
        }
        for gone in moved {
            self.unwatch_tree(&gone);
        }
        // Files made in a new folder before its watch was added are not
        // reported, but the folder itself was.
        for dir in added {
            self.add_tree(&dir);
        }
        Ok(changes)
    }

    /// Watch `dir` and the folders inside it that matter.
    fn add_tree(&mut self, dir: &Path) {
        if !wanted(&self.layout, dir) {
            return;
        }
        let Ok(watch) = self.watches.add(dir, mask() | WatchMask::DONT_FOLLOW) else {
            return;
        };
        self.dirs.insert(watch, dir.to_path_buf());
        self.add_children(dir);
    }

    fn add_children(&mut self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            // Not through symbolic links, which may leave the repository.
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                self.add_tree(&entry.path());
            }
        }
    }

    /// Stop watching a folder moved away, and everything inside it: their
    /// paths are stale. A move within the repository watches it again.
    fn unwatch_tree(&mut self, gone: &Path) {
        let stale: Vec<_> = self
            .dirs
            .iter()
            .filter(|(_, dir)| dir.starts_with(gone))
            .map(|(watch, _)| watch.clone())
            .collect();
        for watch in stale {
            self.dirs.remove(&watch);
            let _ = self.watches.remove(watch);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_folders_that_can_change_what_gg_shows_are_watched() {
        let layout = Layout {
            workdir: "/r".into(),
            git_dir: "/r/.git".into(),
            common_dir: "/r/.git".into(),
        };
        let wanted = |dir: &str| wanted(&layout, Path::new(dir));
        assert!(wanted("/r"));
        assert!(wanted("/r/src/deep"));
        assert!(wanted("/r/.git"));
        assert!(wanted("/r/.git/refs/heads/topic"));
        assert!(wanted("/r/.git/rebase-merge"));
        assert!(!wanted("/r/.git/objects"));
        assert!(!wanted("/r/.git/objects/ab"));
        assert!(!wanted("/r/.git/logs/refs"));
        assert!(!wanted("/elsewhere"));
    }

    #[test]
    fn a_linked_worktree_watches_its_own_git_directory_whole() {
        let layout = Layout {
            workdir: "/w".into(),
            git_dir: "/r/.git/worktrees/w".into(),
            common_dir: "/r/.git".into(),
        };
        let wanted = |dir: &str| wanted(&layout, Path::new(dir));
        assert!(wanted("/r/.git/worktrees/w"));
        assert!(wanted("/r/.git/worktrees/w/rebase-merge"));
        assert!(wanted("/r/.git/refs/heads"));
        assert!(!wanted("/r/.git/worktrees/w/logs"));
        assert!(!wanted("/r/.git/objects"));
    }
}
