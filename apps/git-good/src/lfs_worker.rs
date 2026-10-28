//! One LFS pointer inspection, explicit object fetch, and bounded content preview.
use crate::{binary_preview, diff_view::LfsRequest};
use gg_engine::lfs::{Details, FetchReceipt};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
};

/// Content previews stop at 2 MiB; export remains exact.
const PREVIEW_LIMIT: u64 = 2 * 1024 * 1024;

#[derive(Clone)]
pub enum Action {
    Read(LfsRequest),
    Fetch(LfsRequest),
}

#[derive(Clone)]
struct Request {
    generation: u64,
    repo: PathBuf,
    action: Action,
}

pub struct Loaded {
    pub details: Details,
    pub receipt: Option<FetchReceipt>,
    pub content: Option<binary_preview::Preview>,
    pub preview_note: Option<String>,
}

pub struct Update {
    pub generation: u64,
    pub result: Result<Loaded, String>,
}

pub struct Worker {
    input: Arc<Mutex<Option<Request>>>,
    output: Arc<Mutex<Option<Update>>>,
    generation: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    thread: thread::Thread,
}

fn loaded(repo: &std::path::Path, details: Details, receipt: Option<FetchReceipt>) -> Loaded {
    let preview_note = match details.target() {
        None => None,
        Some(pointer) if !pointer.available => {
            Some("Fetch the object to preview its content.".into())
        }
        Some(pointer) => match gg_git::lfs::read_object(repo, &pointer.oid, PREVIEW_LIMIT) {
            Ok(Some(bytes)) => {
                return Loaded {
                    details,
                    receipt,
                    content: Some(binary_preview::decode(&bytes)),
                    preview_note: None,
                };
            }
            Ok(None) => Some("The LFS object is not available locally.".into()),
            Err(error) => Some(error),
        },
    };
    Loaded {
        details,
        receipt,
        content: None,
        preview_note,
    }
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
                        Action::Read(target) => match target {
                            LfsRequest::Change(change) => gg_git::lfs::read(&request.repo, change)
                                .map(|details| loaded(&request.repo, details, None)),
                            LfsRequest::Working(working) => {
                                gg_git::lfs::read_working(&request.repo, working)
                                    .map(|details| loaded(&request.repo, details, None))
                            }
                        },
                        Action::Fetch(target) => {
                            let fetched = match target {
                                LfsRequest::Change(change) => {
                                    gg_git::lfs::fetch(&request.repo, change)
                                }
                                LfsRequest::Working(working) => {
                                    gg_git::lfs::fetch_working(&request.repo, working)
                                }
                            };
                            fetched.map(|(details, receipt)| {
                                loaded(&request.repo, details, Some(receipt))
                            })
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
