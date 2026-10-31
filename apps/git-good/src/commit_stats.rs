//! Changed-file and line counts for the History inspector
//! (`· 5 files +86 −41`), read on a background thread per selected commit.
use gg_engine::review::{LineStat, ReviewReader, ReviewRequest, ReviewStep};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

/// Stop counting after this many files or this long; the inspector then
/// shows `N+ files` and the lines counted so far.
const MAX_FILES: usize = 2000;
const BUDGET: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub files: usize,
    pub added: u64,
    pub deleted: u64,
    /// Every changed file was listed and counted.
    pub complete: bool,
}

impl Stats {
    /// `5 files +86 −41`, or `2000+ files …` when counting stopped early.
    pub fn label(&self) -> String {
        let files = match (self.files, self.complete) {
            (1, true) => "1 file".to_string(),
            (count, true) => format!("{count} files"),
            (count, false) => format!("{count}+ files"),
        };
        format!("{files} +{} −{}", self.added, self.deleted)
    }
}

pub struct Worker {
    pub commit: String,
    output: Arc<Mutex<Option<Stats>>>,
    stop: Arc<AtomicBool>,
}

impl Worker {
    pub fn start(path: PathBuf, commit: String, ctx: eframe::egui::Context) -> Self {
        let output = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let (o, s, id) = (output.clone(), stop.clone(), commit.clone());
        // Stats are an enhancement: on any failure the slot stays empty.
        let _ = thread::Builder::new()
            .name("gg-commit-stats".into())
            .spawn(move || {
                if let Some(stats) = count(path, id, &s) {
                    if let Ok(mut slot) = o.lock() {
                        *slot = Some(stats);
                    }
                    ctx.request_repaint();
                }
            });
        Self {
            commit,
            output,
            stop,
        }
    }

    pub fn poll(&self) -> Option<Stats> {
        self.output.try_lock().ok()?.take()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn count(path: PathBuf, commit: String, stop: &AtomicBool) -> Option<Stats> {
    let request = ReviewRequest {
        path: path.clone(),
        revision: commit,
        parent: None,
        against: None,
    };
    let mut reader = gg_git::review::ReviewSession::open(&request).ok()?;
    let lines = gg_git::review::LineStats::open(&path).ok()?;
    let deadline = Instant::now() + BUDGET;
    let mut stats = Stats::default();
    loop {
        if stop.load(Ordering::Relaxed) {
            return None;
        }
        if stats.files >= MAX_FILES || Instant::now() > deadline {
            return Some(stats);
        }
        match reader.step().ok()? {
            ReviewStep::Complete => {
                stats.complete = true;
                return Some(stats);
            }
            ReviewStep::Pending => {}
            ReviewStep::Change(change) => {
                stats.files += 1;
                if let Ok(LineStat::Lines { added, deleted }) = lines.count(&change) {
                    stats.added += u64::from(added);
                    stats.deleted += u64::from(deleted);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_read_like_the_mockup() {
        let stats = Stats {
            files: 5,
            added: 86,
            deleted: 41,
            complete: true,
        };
        assert_eq!(stats.label(), "5 files +86 −41");
        let one = Stats {
            files: 1,
            complete: true,
            ..Stats::default()
        };
        assert_eq!(one.label(), "1 file +0 −0");
        let partial = Stats {
            files: 2000,
            ..Stats::default()
        };
        assert_eq!(partial.label(), "2000+ files +0 −0");
    }

    #[test]
    fn counts_a_commit_against_its_parent() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let git = |args: &[&str]| -> Result<String, String> {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .output()
                .map_err(|e| e.to_string())?;
            output
                .status
                .success()
                .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
                .ok_or_else(|| String::from_utf8_lossy(&output.stderr).into_owned())
        };
        git(&["init", "-q"])?;
        std::fs::write(dir.path().join("a"), "1\n2\n").map_err(|e| e.to_string())?;
        git(&["add", "."])?;
        git(&["commit", "-qm", "one"])?;
        std::fs::write(dir.path().join("a"), "1\nchanged\n3\n").map_err(|e| e.to_string())?;
        std::fs::write(dir.path().join("b"), "new\n").map_err(|e| e.to_string())?;
        git(&["add", "."])?;
        git(&["commit", "-qm", "two"])?;
        let head = git(&["rev-parse", "HEAD"])?;
        let stats = count(dir.path().to_path_buf(), head, &AtomicBool::new(false));
        assert_eq!(
            stats,
            Some(Stats {
                files: 2,
                added: 3,
                deleted: 1,
                complete: true,
            })
        );
        Ok(())
    }
}
