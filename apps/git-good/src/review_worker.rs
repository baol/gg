//! Cancellable comparison worker with disk-backed records and a single page mailbox.
use gg_engine::review::*;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    ops::Range,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub const PAGE: usize = 256;
#[derive(Clone)]
pub struct Update {
    pub comparison: Comparison,
    pub count: usize,
    pub complete: bool,
    pub error: Option<String>,
    pub rows: Vec<(usize, FileChange)>,
}
pub struct Worker {
    request: Arc<Mutex<Range<usize>>>,
    output: Arc<Mutex<Option<Result<Update, String>>>>,
    stop: Arc<AtomicBool>,
    thread: thread::Thread,
}
impl Worker {
    pub fn start(request: ReviewRequest, ctx: eframe::egui::Context) -> Result<Self, String> {
        let range = Arc::new(Mutex::new(0..64));
        let output = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let (r, o, s) = (range.clone(), output.clone(), stop.clone());
        let handle = thread::Builder::new()
            .name("gg-review".into())
            .spawn(move || {
                let result = gg_git::review::ReviewSession::open(&request)
                    .and_then(|reader| run(reader, &r, &o, &s, &ctx));
                if let Err(error) = result {
                    if let Ok(mut slot) = o.lock() {
                        *slot = Some(Err(error));
                    }
                    ctx.request_repaint();
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            request: range,
            output,
            stop,
            thread: handle.thread().clone(),
        })
    }
    pub fn request(&self, range: Range<usize>) {
        if let Ok(mut current) = self.request.lock()
            && *current != range
        {
            *current = range;
            self.thread.unpark();
        }
    }
    pub fn poll(&self) -> Option<Result<Update, String>> {
        self.output.try_lock().ok()?.take()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.unpark();
    }
}
fn error(e: impl std::fmt::Display) -> String {
    format!("Could not store comparison: {e}")
}
struct Store {
    data: File,
    index: File,
    count: usize,
}
impl Store {
    fn new() -> Result<Self, String> {
        Ok(Self {
            data: tempfile::tempfile().map_err(error)?,
            index: tempfile::tempfile().map_err(error)?,
            count: 0,
        })
    }
    fn append(&mut self, change: &FileChange) -> Result<(), String> {
        let bytes = serde_json::to_vec(change).map_err(error)?;
        let offset = self.data.seek(SeekFrom::End(0)).map_err(error)?;
        self.data.write_all(&bytes).map_err(error)?;
        self.index.seek(SeekFrom::End(0)).map_err(error)?;
        self.index.write_all(&offset.to_le_bytes()).map_err(error)?;
        self.index
            .write_all(&(bytes.len() as u64).to_le_bytes())
            .map_err(error)?;
        self.count += 1;
        Ok(())
    }
    fn page(&mut self, range: Range<usize>) -> Result<Vec<(usize, FileChange)>, String> {
        let mut rows = Vec::new();
        for row in range.start
            ..range
                .end
                .min(self.count)
                .min(range.start.saturating_add(PAGE))
        {
            self.index
                .seek(SeekFrom::Start(row as u64 * 16))
                .map_err(error)?;
            let (mut offset, mut len) = ([0; 8], [0; 8]);
            self.index.read_exact(&mut offset).map_err(error)?;
            self.index.read_exact(&mut len).map_err(error)?;
            self.data
                .seek(SeekFrom::Start(u64::from_le_bytes(offset)))
                .map_err(error)?;
            let mut bytes = vec![0; u64::from_le_bytes(len) as usize];
            self.data.read_exact(&mut bytes).map_err(error)?;
            rows.push((row, serde_json::from_slice(&bytes).map_err(error)?));
        }
        Ok(rows)
    }
}
fn run(
    mut reader: impl ReviewReader,
    input: &Mutex<Range<usize>>,
    output: &Mutex<Option<Result<Update, String>>>,
    stop: &AtomicBool,
    ctx: &eframe::egui::Context,
) -> Result<(), String> {
    let mut store = Store::new()?;
    let mut complete = false;
    let mut failure = None;
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let range = input.lock().map_err(error)?.clone();
        let start = Instant::now();
        for _ in 0..32 {
            if stop.load(Ordering::Relaxed) {
                return Ok(());
            }
            if complete || failure.is_some() {
                break;
            }
            match reader.step() {
                Ok(ReviewStep::Change(change)) => {
                    if let Err(e) = store.append(&change) {
                        failure = Some(e);
                    }
                }
                Ok(ReviewStep::Pending) => {}
                Ok(ReviewStep::Complete) => complete = true,
                Err(e) => failure = Some(e),
            }
            if start.elapsed() >= Duration::from_millis(4) {
                break;
            }
        }
        let rows = match store.page(range.clone()) {
            Ok(rows) => rows,
            Err(e) => {
                failure = Some(e);
                Vec::new()
            }
        };
        *output.lock().map_err(error)? = Some(Ok(Update {
            comparison: reader.comparison().clone(),
            count: store.count,
            complete: complete && failure.is_none(),
            error: failure.clone(),
            rows,
        }));
        ctx.request_repaint();
        if complete || failure.is_some() {
            while !stop.load(Ordering::Relaxed) && *input.lock().map_err(error)? == range {
                thread::park();
            }
        } else {
            thread::yield_now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    struct Reader {
        identity: Comparison,
        next: usize,
        visits: Arc<AtomicUsize>,
        gate: Arc<AtomicBool>,
        stop: Arc<AtomicBool>,
        fail: bool,
    }
    impl ReviewReader for Reader {
        fn comparison(&self) -> &Comparison {
            &self.identity
        }
        fn step(&mut self) -> Result<ReviewStep, String> {
            while self.next == 32
                && !self.gate.load(Ordering::Relaxed)
                && !self.stop.load(Ordering::Relaxed)
            {
                thread::sleep(Duration::from_millis(1));
            }
            if self.fail && self.next == 32 {
                return Err("Missing tree".into());
            }
            if self.next == 10_000 {
                return Ok(ReviewStep::Complete);
            }
            let path = format!("file-{:05}", self.next);
            self.next += 1;
            self.visits.store(self.next, Ordering::Relaxed);
            Ok(ReviewStep::Change(FileChange {
                path_bytes: path.as_bytes().to_vec(),
                path,
                kind: ChangeKind::Added,
                old: None,
                new: None,
            }))
        }
    }
    fn wait(
        output: &Mutex<Option<Result<Update, String>>>,
        predicate: impl Fn(&Update) -> bool,
    ) -> Result<Update, String> {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(10) {
            if let Some(result) = output.lock().map_err(error)?.take() {
                let update = result?;
                if predicate(&update) {
                    return Ok(update);
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
        Err("Timed out waiting for comparison".into())
    }
    #[test]
    fn streams_early_pages_reloads_and_cancels_after_completion_or_failure() -> Result<(), String> {
        for fail in [false, true] {
            let input = Arc::new(Mutex::new(0..64));
            let output = Arc::new(Mutex::new(None));
            let stop = Arc::new(AtomicBool::new(false));
            let gate = Arc::new(AtomicBool::new(false));
            let visits = Arc::new(AtomicUsize::new(0));
            let reader = Reader {
                identity: Comparison {
                    path: "repo".into(),
                    commit: "commit".into(),
                    parent: None,
                    parents: vec![],
                    subject: "root".into(),
                },
                next: 0,
                visits: visits.clone(),
                gate: gate.clone(),
                stop: stop.clone(),
                fail,
            };
            let (i, o, s) = (input.clone(), output.clone(), stop.clone());
            let handle =
                thread::spawn(move || run(reader, &i, &o, &s, &eframe::egui::Context::default()));
            let result = (|| -> Result<(), String> {
                let first = wait(&output, |u| u.count > 0)?;
                assert!(!first.complete);
                assert!(first.count <= 32);
                assert!(visits.load(Ordering::Relaxed) <= 32);
                gate.store(true, Ordering::Relaxed);
                let done = wait(&output, |u| u.complete || u.error.is_some())?;
                assert_eq!(done.complete, !fail);
                assert_eq!(done.count, if fail { 32 } else { 10_000 });
                if fail {
                    assert_eq!(done.error.as_deref(), Some("Missing tree"));
                    assert!(!done.rows.is_empty());
                } else {
                    for range in [9900..10_000, 0..64] {
                        *input.lock().map_err(error)? = range.clone();
                        handle.thread().unpark();
                        let page = wait(&output, |u| {
                            u.rows.first().is_some_and(|(row, _)| *row == range.start)
                        })?;
                        assert!(page.rows.len() <= PAGE);
                        assert_eq!(page.rows[0].1.path, format!("file-{:05}", range.start));
                        assert_eq!(visits.load(Ordering::Relaxed), 10_000);
                    }
                }
                Ok(())
            })();
            gate.store(true, Ordering::Relaxed);
            stop.store(true, Ordering::Relaxed);
            handle.thread().unpark();
            handle.join().map_err(|_| "Comparison worker panicked")??;
            result?;
        }
        Ok(())
    }
}
