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
    pub preview: Option<Arc<crate::binary_preview::Pair>>,
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
        Self::with_reader(ctx, |request, cancelled| {
            gg_git::diff::read(&request.path, &request.change, cancelled)
        })
    }
    fn with_reader(
        ctx: eframe::egui::Context,
        read: impl Fn(&Request, &dyn Fn() -> bool) -> Result<FileDiff, String> + Send + 'static,
    ) -> Result<Self, String> {
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
                let mut preview = None;
                while !s.load(Ordering::Relaxed) {
                    let request = i.lock().ok().and_then(|mut r| r.take());
                    let Some(request) = request else {
                        thread::park();
                        continue;
                    };
                    if request.generation != loaded {
                        drop(document.take());
                        let cancelled = || {
                            s.load(Ordering::Relaxed)
                                || g.load(Ordering::Relaxed) != request.generation
                        };
                        let result = read(&request, &cancelled);
                        preview = None;
                        if result.as_ref().is_ok_and(|diff| {
                            matches!(
                                diff.kind,
                                gg_engine::diff::DiffKind::Binary
                                    | gg_engine::diff::DiffKind::Limited
                            )
                        }) && !cancelled()
                        {
                            preview = Some(Arc::new(crate::binary_preview::read(
                                &request.path,
                                &request.change,
                                &cancelled,
                            )));
                        }
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
                                preview: preview.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use gg_engine::{
        diff::{DiffKind, DiffRow, RowKind},
        review::ChangeKind,
    };
    #[test]
    fn selection_cancels_old_work_and_pages_reuse_the_document() -> Result<(), String> {
        let (started, wait) = std::sync::mpsc::channel();
        let reads = Arc::new(AtomicU64::new(0));
        let count = reads.clone();
        let worker = Worker::with_reader(
            eframe::egui::Context::default(),
            move |request, cancelled| {
                count.fetch_add(1, Ordering::Relaxed);
                if request.generation == 1 {
                    started.send(()).map_err(|e| e.to_string())?;
                    while !cancelled() {
                        thread::yield_now();
                    }
                    return Err("obsolete".into());
                }
                let mut doc = FileDiff::summary(DiffKind::Text, 0, 2000, "");
                doc.rows = (0..1000)
                    .map(|n| DiffRow {
                        kind: RowKind::Added,
                        old_line: None,
                        new_line: Some(n + 1),
                        text: n.to_string(),
                        bytes: vec![],
                    })
                    .collect();
                Ok(doc)
            },
        )?;
        let change = FileChange {
            path: "file".into(),
            path_bytes: b"file".to_vec(),
            kind: ChangeKind::Added,
            old: None,
            new: None,
        };
        worker.request(1, "repo".into(), change.clone(), 0..1000);
        wait.recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        worker.request(2, "repo".into(), change.clone(), 0..1000);
        let poll = || -> Result<Update, String> {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                if let Some(update) = worker.poll() {
                    return Ok(update);
                }
                if std::time::Instant::now() > deadline {
                    return Err("worker timed out".into());
                }
                thread::sleep(std::time::Duration::from_millis(1));
            }
        };
        let update = poll()?;
        assert_eq!(update.generation, 2);
        assert_eq!(update.count, 1000);
        assert_eq!(update.result?.rows.len(), 256);
        worker.request(2, "repo".into(), change, 900..1000);
        let update = poll()?;
        assert_eq!(update.page_start, 900);
        assert_eq!(update.result?.rows[0].new_line, Some(901));
        assert_eq!(reads.load(Ordering::Relaxed), 2);
        Ok(())
    }
}
