//! One LFS pointer inspection or explicit object fetch.
use gg_engine::{
    lfs::{Details, FetchReceipt},
    review::FileChange,
};
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
    Read(FileChange),
    Fetch(FileChange),
}

#[derive(Clone)]
struct Request {
    generation: u64,
    repo: PathBuf,
    action: Action,
}

pub struct Update {
    pub generation: u64,
    pub result: Result<(Details, Option<FetchReceipt>), String>,
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
            .name("gg-lfs".into())
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
                    let result = match &request.action {
                        Action::Read(change) => {
                            gg_git::lfs::read(&request.repo, change).map(|details| (details, None))
                        }
                        Action::Fetch(change) => gg_git::lfs::fetch(&request.repo, change)
                            .map(|(details, receipt)| (details, Some(receipt))),
                    };
                    if g.load(Ordering::Relaxed) != current {
                        continue;
                    }
                    if let Ok(mut slot) = o.lock() {
                        *slot = Some(Update {
                            generation: current,
                            result,
                        });
                    }
                    ctx.request_repaint();
                }
            })
            .map_err(|error| format!("Start LFS worker: {error}"))?;
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
