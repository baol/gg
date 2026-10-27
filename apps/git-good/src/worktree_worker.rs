//! One worktree inventory or explicit add/remove; a newer request supersedes stale output.
use gg_engine::worktree::{AddRequest, Snapshot};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
};

#[derive(Clone)]
pub enum Action {
    List,
    Add(AddRequest),
    Remove(PathBuf),
}

#[derive(Clone)]
struct Request {
    generation: u64,
    repo: PathBuf,
    action: Action,
}

pub struct Update {
    pub generation: u64,
    pub result: Result<Snapshot, String>,
    pub notice: Option<String>,
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
            .name("gg-worktrees".into())
            .spawn(move || {
                while !s.load(Ordering::Relaxed) {
                    let request = i.lock().ok().and_then(|mut slot| slot.take());
                    let Some(request) = request else {
                        thread::park();
                        continue;
                    };
                    let current = request.generation;
                    if g.load(Ordering::Relaxed) != current {
                        continue;
                    }
                    let (result, notice) = match &request.action {
                        Action::List => (gg_git::worktree::list(&request.repo), None),
                        Action::Add(add) => match gg_git::worktree::add(&request.repo, add) {
                            Ok(receipt) => (
                                gg_git::worktree::list(&request.repo),
                                Some(format!(
                                    "Added {} on {}.",
                                    receipt.path,
                                    receipt.branch.as_deref().unwrap_or("detached HEAD")
                                )),
                            ),
                            Err(error) => (Err(error), None),
                        },
                        Action::Remove(path) => match gg_git::worktree::remove(&request.repo, path)
                        {
                            Ok(receipt) => (
                                gg_git::worktree::list(&request.repo),
                                Some(format!(
                                    "Removed {}.",
                                    receipt.branch.as_deref().unwrap_or("the worktree")
                                )),
                            ),
                            Err(error) => (Err(error), None),
                        },
                    };
                    if g.load(Ordering::Relaxed) != current {
                        continue;
                    }
                    if let Ok(mut slot) = o.lock() {
                        *slot = Some(Update {
                            generation: current,
                            result,
                            notice,
                        });
                    }
                    ctx.request_repaint();
                }
            })
            .map_err(|error| format!("Start worktree worker: {error}"))?;
        Ok(Self {
            input,
            output,
            generation,
            stop,
            thread: handle.thread().clone(),
        })
    }

    pub fn request(&self, generation: u64, repo: PathBuf, action: Action) {
        self.generation.store(generation, Ordering::Relaxed);
        if let Ok(mut slot) = self.input.lock() {
            *slot = Some(Request {
                generation,
                repo,
                action,
            });
        }
        self.thread.unpark();
    }

    pub fn poll(&self) -> Option<Update> {
        self.output.lock().ok().and_then(|mut slot| slot.take())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.unpark();
    }
}
