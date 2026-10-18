//! One selected-file worker; generation changes cancel and supersede older work.
use gg_engine::{diff::FileDiff, review::FileChange};
use std::{
    ops::Range,
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
    path: PathBuf,
    change: FileChange,
    range: Range<usize>,
}
#[derive(Clone)]
pub struct Update {
    pub generation: u64,
    pub count: usize,
    pub columns: usize,
    pub page_start: usize,
    pub result: Result<FileDiff, String>,
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
            .name("gg-diff".into())
            .spawn(move || {
                let mut document = None;
                let mut loaded = 0;
                let mut max_columns = 0;
                while !s.load(Ordering::Relaxed) {
                    let request = i.lock().ok().and_then(|mut r| r.take());
                    let Some(request) = request else {
                        thread::park();
                        continue;
                    };
                    if request.generation != loaded {
                        document = None;
                        let cancelled = || {
                            s.load(Ordering::Relaxed)
                                || g.load(Ordering::Relaxed) != request.generation
                        };
                        let result = gg_git::diff::read(&request.path, &request.change, cancelled);
                        if cancelled() {
                            continue;
                        }
                        max_columns = result
                            .as_ref()
                            .map(|doc| {
                                doc.rows
                                    .iter()
                                    .map(|r| r.text.chars().count())
                                    .max()
                                    .unwrap_or(0)
                            })
                            .unwrap_or(0);
                        document = Some(result);
                        loaded = request.generation;
                    }
                    if g.load(Ordering::Relaxed) != request.generation {
                        continue;
                    }
                    if let Some(document) = &document {
                        let (count, columns, result) = match document {
                            Ok(doc) => {
                                let mut page = gg_engine::diff::FileDiff {
                                    kind: doc.kind,
                                    old_size: doc.old_size,
                                    new_size: doc.new_size,
                                    complete: doc.complete,
                                    note: doc.note.clone(),
                                    rows: vec![],
                                };
                                page.rows = doc
                                    .rows
                                    .iter()
                                    .skip(request.range.start)
                                    .take(request.range.len().min(256))
                                    .cloned()
                                    .collect();
                                (doc.rows.len(), max_columns, Ok(page))
                            }
                            Err(e) => (0, 0, Err(e.clone())),
                        };
                        if let Ok(mut slot) = o.lock() {
                            *slot = Some(Update {
                                generation: loaded,
                                count,
                                columns,
                                page_start: request.range.start,
                                result,
                            });
                        }
                        ctx.request_repaint();
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            input,
            output,
            generation,
            stop,
            thread: handle.thread().clone(),
        })
    }
    pub fn request(&self, generation: u64, path: PathBuf, change: FileChange, range: Range<usize>) {
        self.generation.store(generation, Ordering::Relaxed);
        if let Ok(mut slot) = self.input.lock() {
            *slot = Some(Request {
                generation,
                path,
                change,
                range,
            });
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
