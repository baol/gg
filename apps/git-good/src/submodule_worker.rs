//! One submodule inspection or explicit index update.
use gg_engine::submodule::{Details, Receipt};
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
    Read {
        before: Option<String>,
        after: Option<String>,
    },
    Update,
}

#[derive(Clone)]
struct Request {
    generation: u64,
    repo: PathBuf,
    path_bytes: Vec<u8>,
    action: Action,
}

pub struct Update {
    pub generation: u64,
    pub result: Result<(Details, Option<Receipt>), String>,
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
            .name("gg-submodule".into())
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
                        Action::Read { before, after } => gg_git::submodule::read(
                            &request.repo,
                            &request.path_bytes,
                            after.as_deref(),
                            before.as_deref(),
                        )
                        .map(|details| (details, None)),
                        Action::Update => {
                            gg_git::submodule::update(&request.repo, &request.path_bytes)
                                .map(|(details, receipt)| (details, Some(receipt)))
                        }
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
            .map_err(|error| format!("Start submodule worker: {error}"))?;
        Ok(Self {
            input,
            output,
            generation,
            stop,
            thread: handle.thread().clone(),
        })
    }

    pub fn request(&self, generation: u64, repo: PathBuf, path_bytes: Vec<u8>, action: Action) {
        self.generation.store(generation, Ordering::Relaxed);
        if let Ok(mut slot) = self.input.lock() {
            *slot = Some(Request {
                generation,
                repo,
                path_bytes,
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
