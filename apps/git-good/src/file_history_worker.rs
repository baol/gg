//! One file-history session; opening a new file supersedes older work.
use gg_engine::file_history::{PRELOAD, Timeline};
use gg_git::file_history::Session;
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
};

#[derive(Clone)]
enum Request {
    Open {
        generation: u64,
        repo: PathBuf,
        revision: String,
        path_bytes: Vec<u8>,
    },
    Need {
        generation: u64,
        count: usize,
    },
    Choose {
        generation: u64,
        index: usize,
    },
}

impl Request {
    fn generation(&self) -> u64 {
        match self {
            Self::Open { generation, .. }
            | Self::Need { generation, .. }
            | Self::Choose { generation, .. } => *generation,
        }
    }
}

pub struct Update {
    pub generation: u64,
    pub timeline: Option<Timeline>,
    pub busy: bool,
    pub error: Option<String>,
}

pub struct Worker {
    input: Arc<Mutex<Option<Request>>>,
    output: Arc<Mutex<Option<Update>>>,
    generation: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    thread: thread::Thread,
}

impl Worker {
    pub fn start(ctx: eframe::egui::Context) -> Result<Self, String> {
        let input = Arc::new(Mutex::new(None::<Request>));
        let output = Arc::new(Mutex::new(None));
        let generation = Arc::new(AtomicU64::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (i, o, g, s) = (
            input.clone(),
            output.clone(),
            generation.clone(),
            stop.clone(),
        );
        let handle = thread::Builder::new()
            .name("gg-file-history".into())
            .spawn(move || {
                let mut loaded: Option<(u64, Session)> = None;
                while !s.load(Ordering::Relaxed) {
                    let request = i.lock().ok().and_then(|mut slot| slot.take());
                    let Some(request) = request else {
                        thread::park();
                        continue;
                    };
                    let generation = request.generation();
                    if loaded
                        .as_ref()
                        .is_some_and(|(current, _)| *current != generation)
                    {
                        loaded = None;
                    }
                    let cancelled =
                        || s.load(Ordering::Relaxed) || g.load(Ordering::Relaxed) != generation;
                    match request {
                        Request::Open {
                            repo,
                            revision,
                            path_bytes,
                            ..
                        } => {
                            match Session::open(&repo, &revision, &path_bytes, &cancelled) {
                                Ok(session) => loaded = Some((generation, session)),
                                Err(error) => {
                                    loaded = None;
                                    emit(&o, &ctx, generation, None, false, Some(error));
                                    continue;
                                }
                            }
                            let Some((_, session)) = loaded.as_ref() else {
                                continue;
                            };
                            emit(&o, &ctx, generation, Some(session.timeline()), true, None);
                            preload(&o, &ctx, generation, loaded.as_mut(), PRELOAD, &cancelled);
                        }
                        Request::Need { count, .. } => {
                            let Some((_, session)) = loaded.as_mut() else {
                                continue;
                            };
                            emit(&o, &ctx, generation, Some(session.timeline()), true, None);
                            preload(&o, &ctx, generation, loaded.as_mut(), count, &cancelled);
                        }
                        Request::Choose { index, .. } => {
                            let target = {
                                let Some((_, session)) = loaded.as_mut() else {
                                    continue;
                                };
                                if let Err(error) = session.choose(index) {
                                    emit(
                                        &o,
                                        &ctx,
                                        generation,
                                        Some(session.timeline()),
                                        false,
                                        Some(error),
                                    );
                                    continue;
                                }
                                session.versions().len() + 1
                            };
                            let Some((_, session)) = loaded.as_ref() else {
                                continue;
                            };
                            emit(&o, &ctx, generation, Some(session.timeline()), true, None);
                            preload(&o, &ctx, generation, loaded.as_mut(), target, &cancelled);
                        }
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            input,
            output,
            generation,
            stop,
            thread: handle.thread().clone(),
        })
    }

    pub fn open(&self, generation: u64, repo: PathBuf, revision: String, path_bytes: Vec<u8>) {
        self.submit(Request::Open {
            generation,
            repo,
            revision,
            path_bytes,
        });
    }

    pub fn need(&self, generation: u64, count: usize) {
        self.submit(Request::Need { generation, count });
    }

    pub fn choose(&self, generation: u64, index: usize) {
        self.submit(Request::Choose { generation, index });
    }

    fn submit(&self, request: Request) {
        self.generation
            .store(request.generation(), Ordering::Relaxed);
        if let Ok(mut slot) = self.input.lock() {
            *slot = Some(request);
            self.thread.unpark();
        }
    }

    pub fn poll(&self) -> Option<Update> {
        self.output.try_lock().ok()?.take()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.unpark();
    }
}

fn preload(
    output: &Mutex<Option<Update>>,
    ctx: &eframe::egui::Context,
    generation: u64,
    loaded: Option<&mut (u64, Session)>,
    target: usize,
    cancelled: &dyn Fn() -> bool,
) {
    let Some((_, session)) = loaded else {
        return;
    };
    let mut failure = None;
    while session.versions().len() < target && !session.complete() {
        if cancelled() {
            return;
        }
        match session.next(cancelled) {
            Ok(true) => emit(
                output,
                ctx,
                generation,
                Some(session.timeline()),
                true,
                None,
            ),
            Ok(false) => break,
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    if cancelled() {
        return;
    }
    emit(
        output,
        ctx,
        generation,
        Some(session.timeline()),
        false,
        failure,
    );
}

fn emit(
    output: &Mutex<Option<Update>>,
    ctx: &eframe::egui::Context,
    generation: u64,
    timeline: Option<Timeline>,
    busy: bool,
    error: Option<String>,
) {
    if let Ok(mut slot) = output.lock() {
        *slot = Some(Update {
            generation,
            timeline,
            busy,
            error,
        });
    }
    ctx.request_repaint();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::BTreeMap,
        error::Error,
        path::{Path, PathBuf},
        process::Command,
        time::Duration,
    };

    type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

    fn git(path: &Path, args: &[&str]) -> Result {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .env("GIT_AUTHOR_NAME", "Test Author")
            .env("GIT_AUTHOR_EMAIL", "test@example.org")
            .env("GIT_COMMITTER_NAME", "Test Author")
            .env("GIT_COMMITTER_EMAIL", "test@example.org")
            .env("GIT_AUTHOR_DATE", "2026-01-01T12:00:00+00:00")
            .env("GIT_COMMITTER_DATE", "2026-01-01T12:00:00+00:00")
            .output()?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
        }
        Ok(())
    }

    fn fixture() -> Result<tempfile::TempDir> {
        let dir = tempfile::tempdir()?;
        git(dir.path(), &["init", "-b", "main"])?;
        git(dir.path(), &["config", "core.autocrlf", "false"])?;
        std::fs::write(dir.path().join("f.txt"), "one\n")?;
        git(dir.path(), &["add", "-A"])?;
        git(dir.path(), &["commit", "-m", "Add"])?;
        std::fs::write(dir.path().join("f.txt"), "two\n")?;
        git(dir.path(), &["add", "-A"])?;
        git(dir.path(), &["commit", "-m", "Change"])?;
        Ok(dir)
    }

    fn wait(worker: &Worker, generation: u64) -> Result<Update> {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(update) = worker.poll()
                && update.generation == generation
                && !update.busy
            {
                return Ok(update);
            }
            if std::time::Instant::now() > deadline {
                return Err("file history worker timed out".into());
            }
            thread::sleep(Duration::from_millis(2));
        }
    }

    fn snapshot(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
        fn visit(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) -> Result {
            for entry in std::fs::read_dir(dir)? {
                let path = entry?.path();
                if path.is_dir() {
                    visit(root, &path, files)?;
                } else {
                    files.insert(path.strip_prefix(root)?.to_path_buf(), std::fs::read(path)?);
                }
            }
            Ok(())
        }
        let mut files = BTreeMap::new();
        visit(root, root, &mut files)?;
        Ok(files)
    }

    #[test]
    fn reading_versions_preserves_the_repository() -> Result {
        let dir = fixture()?;
        let before = snapshot(dir.path())?;
        let worker = Worker::start(eframe::egui::Context::default())?;
        worker.open(1, dir.path().into(), "HEAD".into(), b"f.txt".to_vec());
        let update = wait(&worker, 1)?;
        assert!(update.error.is_none(), "{:?}", update.error);
        worker.need(1, timeline_versions(&update) + PRELOAD);
        let update = wait(&worker, 1)?;
        assert!(update.error.is_none(), "{:?}", update.error);
        drop(worker);
        assert_eq!(
            snapshot(dir.path())?,
            before,
            "reading file history changed repository files"
        );
        Ok(())
    }

    fn timeline_versions(update: &Update) -> usize {
        update
            .timeline
            .as_ref()
            .map_or(0, |timeline| timeline.versions.len())
    }

    #[test]
    fn opening_preloads_and_can_page_to_the_oldest_version() -> Result {
        let dir = fixture()?;
        let worker = Worker::start(eframe::egui::Context::default())?;
        worker.open(1, dir.path().into(), "HEAD".into(), b"f.txt".to_vec());
        let update = wait(&worker, 1)?;
        assert!(update.error.is_none(), "{:?}", update.error);
        let timeline = update.timeline.ok_or("the timeline is missing")?;
        assert_eq!(timeline.versions.len(), 2);
        assert!(timeline.complete);
        assert_eq!(timeline.versions[0].subject, "Change");
        assert_eq!(timeline.versions[1].subject, "Add");
        Ok(())
    }

    #[test]
    fn opening_another_file_discards_the_previous_session() -> Result {
        let dir = fixture()?;
        let worker = Worker::start(eframe::egui::Context::default())?;
        worker.open(1, dir.path().into(), "HEAD".into(), b"f.txt".to_vec());
        let first = wait(&worker, 1)?;
        assert!(first.timeline.is_some());
        worker.open(2, dir.path().into(), "HEAD".into(), b"missing.txt".to_vec());
        let second = wait(&worker, 2)?;
        let timeline = second.timeline.ok_or("the timeline is missing")?;
        assert_eq!(timeline.versions.len(), 1);
        assert_eq!(timeline.versions[0].side, None);
        assert!(matches!(
            timeline.versions[0].previous,
            gg_engine::file_history::Predecessor::Oldest
        ));
        Ok(())
    }
}
