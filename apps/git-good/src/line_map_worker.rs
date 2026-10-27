//! Map one selected line range through a content edge without blocking the UI.
use gg_engine::{
    file::ReadingSide,
    file_history::{LineMap, map_lines},
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
struct Request {
    generation: u64,
    repo: PathBuf,
    change: FileChange,
    from: ReadingSide,
    start: usize,
    end: usize,
}

pub struct Update {
    pub generation: u64,
    pub result: Result<LineMap, String>,
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
            .name("gg-line-map".into())
            .spawn(move || {
                while !s.load(Ordering::Relaxed) {
                    let request = i.lock().ok().and_then(|mut slot| slot.take());
                    let Some(request) = request else {
                        thread::park();
                        continue;
                    };
                    let current = request.generation;
                    let cancelled_closure =
                        || s.load(Ordering::Relaxed) || g.load(Ordering::Relaxed) != current;
                    let cancelled: &dyn Fn() -> bool = &cancelled_closure;
                    let result =
                        gg_git::diff::read(&request.repo, &request.change, cancelled).map(|diff| {
                            map_lines(&diff.rows, request.from, request.start, request.end)
                        });
                    if cancelled() {
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
            .map_err(|error| format!("Start line-history worker: {error}"))?;
        Ok(Self {
            input,
            output,
            generation,
            stop,
            thread: handle.thread().clone(),
        })
    }

    pub fn request(
        &self,
        generation: u64,
        repo: PathBuf,
        change: FileChange,
        from: ReadingSide,
        start: usize,
        end: usize,
    ) {
        self.generation.store(generation, Ordering::Relaxed);
        if let Ok(mut slot) = self.input.lock() {
            *slot = Some(Request {
                generation,
                repo,
                change,
                from,
                start,
                end,
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
