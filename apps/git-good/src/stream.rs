//! Background traversal, disk-backed rows, and coalesced viewport requests.
use gg_engine::{Commit, History, HistoryReader, HistoryRequest};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    ops::Range,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub const PAGE_ROWS: usize = 256;
pub const CACHE_ROWS: usize = 768;

#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub query: String,
    pub range: Range<usize>,
    pub selected_id: Option<String>,
    pub selected_row: Option<usize>,
    pub anchor_id: Option<String>,
}
impl Default for Request {
    fn default() -> Self {
        Self {
            query: String::new(),
            range: 0..64,
            selected_id: None,
            selected_row: None,
            anchor_id: None,
        }
    }
}

#[derive(Clone)]
pub struct Update {
    pub request: Request,
    pub anchor_row: Option<usize>,
    pub identity: History,
    pub query: String,
    pub count: usize,
    pub scanned: usize,
    pub complete: bool,
    pub rows: Vec<(usize, Commit)>,
    pub selection: Option<(usize, Commit)>,
    pub error: Option<String>,
}

/// Both directions retain only the latest message; slow UI frames cannot grow queues.
pub struct Worker {
    request: Arc<Mutex<Request>>,
    output: Arc<Mutex<Option<Result<Update, String>>>>,
    stop: Arc<AtomicBool>,
    thread: thread::Thread,
}
impl Worker {
    pub fn start(
        path: PathBuf,
        request: Request,
        previous_path: Option<PathBuf>,
        ctx: eframe::egui::Context,
    ) -> Result<Self, String> {
        let input = Arc::new(Mutex::new(request));
        let output = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let (i, o, s) = (input.clone(), output.clone(), stop.clone());
        let handle = thread::Builder::new()
            .name("gg-history".into())
            .spawn(move || {
                let result = run(path, previous_path, &i, &o, &s, &ctx);
                if let Err(error) = result {
                    if let Ok(mut output) = o.lock() {
                        *output = Some(Err(error));
                    }
                    ctx.request_repaint();
                }
            })
            .map_err(|e| format!("Could not start history reader: {e}"))?;
        Ok(Self {
            request: input,
            output,
            stop,
            thread: handle.thread().clone(),
        })
    }
    pub fn request(&self, request: Request) -> bool {
        let Ok(mut current) = self.request.try_lock() else {
            return false;
        };
        if *current != request {
            *current = request;
            self.thread.unpark();
        }
        true
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

/// Fixed-size offset records keep arbitrary-depth lookup independent of RAM capacity.
struct Spool {
    data: File,
    index: File,
    matches: File,
    count: usize,
    matched: usize,
}
impl Spool {
    fn new() -> Result<Self, String> {
        Ok(Self {
            data: tempfile::tempfile().map_err(err)?,
            index: tempfile::tempfile().map_err(err)?,
            matches: tempfile::tempfile().map_err(err)?,
            count: 0,
            matched: 0,
        })
    }
    fn append(&mut self, commit: &Commit) -> Result<(), String> {
        let bytes = serde_json::to_vec(commit).map_err(err)?;
        let offset = self.data.seek(SeekFrom::End(0)).map_err(err)?;
        self.data.write_all(&bytes).map_err(err)?;
        self.index.seek(SeekFrom::End(0)).map_err(err)?;
        self.index.write_all(&offset.to_le_bytes()).map_err(err)?;
        self.index
            .write_all(&(bytes.len() as u64).to_le_bytes())
            .map_err(err)?;
        self.count += 1;
        Ok(())
    }
    fn read(&mut self, row: usize, filtered: bool) -> Result<Commit, String> {
        let index = if filtered {
            &mut self.matches
        } else {
            &mut self.index
        };
        index.seek(SeekFrom::Start(row as u64 * 16)).map_err(err)?;
        let mut offset = [0; 8];
        let mut len = [0; 8];
        index.read_exact(&mut offset).map_err(err)?;
        index.read_exact(&mut len).map_err(err)?;
        self.data
            .seek(SeekFrom::Start(u64::from_le_bytes(offset)))
            .map_err(err)?;
        let mut bytes = vec![0; u64::from_le_bytes(len) as usize];
        self.data.read_exact(&mut bytes).map_err(err)?;
        serde_json::from_slice(&bytes).map_err(err)
    }
    fn include(&mut self, row: usize) -> Result<(), String> {
        self.index
            .seek(SeekFrom::Start(row as u64 * 16))
            .map_err(err)?;
        let mut record = [0; 16];
        self.index.read_exact(&mut record).map_err(err)?;
        self.matches.seek(SeekFrom::End(0)).map_err(err)?;
        self.matches.write_all(&record).map_err(err)?;
        self.matched += 1;
        Ok(())
    }
    fn reset_filter(&mut self) -> Result<(), String> {
        self.matches.set_len(0).map_err(err)?;
        self.matched = 0;
        Ok(())
    }
}
fn err(error: impl std::fmt::Display) -> String {
    format!("History session: {error}")
}

fn run(
    path: PathBuf,
    previous_path: Option<PathBuf>,
    input: &Mutex<Request>,
    output: &Mutex<Option<Result<Update, String>>>,
    stop: &AtomicBool,
    ctx: &eframe::egui::Context,
) -> Result<(), String> {
    let session = gg_git::Session::open(&HistoryRequest { path })?;
    if previous_path
        .as_ref()
        .is_some_and(|old| *old != session.identity.path)
    {
        *input.lock().map_err(err)? = Request::default();
    }
    run_session(session, input, output, stop, ctx)
}

fn run_session(
    mut session: impl HistoryReader,
    input: &Mutex<Request>,
    output: &Mutex<Option<Result<Update, String>>>,
    stop: &AtomicBool,
    ctx: &eframe::egui::Context,
) -> Result<(), String> {
    let mut spool = Spool::new()?;
    let mut current = input.lock().map_err(err)?.clone();
    let mut scanned = 0;
    let mut finished = false;
    let mut failure = None;
    let mut selection = None;
    let mut anchor_row = None;
    let mut cached_range = 0..0;
    let mut cached_rows = Vec::new();
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let next = input.lock().map_err(err)?.clone();
        if next.query != current.query {
            spool.reset_filter()?;
            cached_range = 0..0;
            cached_rows.clear();
            scanned = 0;
            selection = None;
        }
        if next.selected_id != current.selected_id || next.selected_row != current.selected_row {
            selection = None;
        }
        if next.anchor_id != current.anchor_id || next.query != current.query {
            anchor_row = None;
        }
        current = next;
        let start = Instant::now();
        // A small first batch reaches the screen before counting the remainder.
        for _ in 0..32 {
            if stop.load(Ordering::Relaxed) {
                return Ok(());
            }
            let mut appended = None;
            if scanned == spool.count && !finished {
                match session.next_commit() {
                    Ok(Some(commit)) => {
                        spool.append(&commit)?;
                        appended = Some(commit);
                    }
                    Ok(None) => finished = true,
                    Err(error) => {
                        failure = Some(error);
                        finished = true;
                    }
                }
            }
            if scanned < spool.count {
                let commit = if let Some(commit) = appended {
                    commit
                } else {
                    spool.read(scanned, false)?
                };
                if commit.matches(&current.query) {
                    if current.anchor_id.as_deref() == Some(&commit.id) {
                        anchor_row = Some(spool.matched);
                    }
                    if current.selected_id.as_deref() == Some(&commit.id) {
                        selection = Some((spool.matched, commit.clone()));
                    }
                    spool.include(scanned)?;
                }
                scanned += 1;
            } else {
                break;
            }
            if start.elapsed() >= Duration::from_millis(4) {
                break;
            }
        }
        if let Some(row) = current.selected_row.filter(|row| *row < spool.matched) {
            if selection.as_ref().map(|(index, _)| *index) != Some(row) {
                selection = Some((row, spool.read(row, true)?));
            }
        } else if selection.is_none()
            && spool.matched > 0
            && (current.selected_id.is_none() || (finished && scanned == spool.count))
        {
            selection = Some((0, spool.read(0, true)?));
        }
        let requested = if let Some(row) = anchor_row {
            row.saturating_sub(24)..row.saturating_add(64)
        } else {
            current.range.clone()
        };
        let range = requested.start.min(spool.matched)
            ..requested
                .end
                .min(spool.matched)
                .min(requested.start.saturating_add(PAGE_ROWS));
        if range != cached_range {
            cached_rows.clear();
            for row in range.clone() {
                let mut commit = spool.read(row, true)?;
                commit.message = commit.subject().to_owned();
                cached_rows.push((row, commit));
            }
            cached_range = range;
        }
        let done = finished && scanned == spool.count;
        let complete = done && failure.is_none();
        let update = Update {
            request: current.clone(),
            anchor_row,
            identity: session.identity().clone(),
            query: current.query.clone(),
            count: spool.matched,
            scanned,
            complete,
            rows: cached_rows.clone(),
            selection: selection.clone(),
            error: failure.clone(),
        };
        *output.lock().map_err(err)? = Some(Ok(update));
        ctx.request_repaint();
        if done {
            // Unpark tokens prevent a request arriving just before parking from being lost.
            while !stop.load(Ordering::Relaxed) && *input.lock().map_err(err)? == current {
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
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    struct Fake {
        identity: History,
        next: usize,
        visits: Arc<AtomicUsize>,
        gate: Arc<AtomicBool>,
    }
    impl HistoryReader for Fake {
        fn identity(&self) -> &History {
            &self.identity
        }
        fn next_commit(&mut self) -> Result<Option<Commit>, String> {
            // Hold traversal after the first page so early delivery is deterministic.
            while self.next == 32 && !self.gate.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(1));
            }
            if self.next == 100_000 {
                return Ok(None);
            }
            let i = self.next;
            self.next += 1;
            self.visits.store(self.next, Ordering::Relaxed);
            Ok(Some(Commit {
                id: format!("{i:040x}"),
                parents: vec![],
                author: "Ada".into(),
                email: String::new(),
                time: String::new(),
                message: format!(
                    "Commit {i}\n\n{}",
                    if i == 99_999 {
                        "needle in full message"
                    } else {
                        "body"
                    }
                ),
            }))
        }
    }

    fn wait(
        output: &Mutex<Option<Result<Update, String>>>,
        predicate: impl Fn(&Update) -> bool,
    ) -> Result<Update, String> {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(30) {
            if let Some(result) = output.lock().map_err(err)?.take() {
                let update = result?;
                if predicate(&update) {
                    return Ok(update);
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
        Err("Timed out waiting for history update".into())
    }

    #[test]
    fn first_page_precedes_completion_and_disk_pages_support_whole_history_search() -> TestResult {
        let input = Arc::new(Mutex::new(Request::default()));
        let output = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let visits = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new(AtomicBool::new(false));
        let fake = Fake {
            identity: History {
                path: "repo".into(),
                head: "main".into(),
                commits: vec![],
                truncated: false,
                shallow: false,
            },
            next: 0,
            visits: visits.clone(),
            gate: gate.clone(),
        };
        let (i, o, s) = (input.clone(), output.clone(), stop.clone());
        let handle =
            thread::spawn(move || run_session(fake, &i, &o, &s, &eframe::egui::Context::default()));
        let result = (|| -> TestResult {
            let first = wait(&output, |u| u.count == 32)?;
            assert!(!first.complete);
            assert_eq!(visits.load(Ordering::Relaxed), 32);
            assert_eq!(first.rows.len(), 32);
            assert!(first.rows.iter().all(|(_, c)| !c.message.contains('\n')));
            assert!(
                first
                    .selection
                    .as_ref()
                    .is_some_and(|(_, c)| c.message.contains("body"))
            );
            gate.store(true, Ordering::Relaxed);
            let done = wait(&output, |u| u.complete)?;
            assert_eq!(done.count, 100_000);
            for range in [99_950..100_000, 0..32] {
                input.lock().map_err(err)?.range = range.clone();
                handle.thread().unpark();
                let page = wait(&output, |u| u.request.range == range)?;
                assert_eq!(page.rows.first().map(|(row, _)| *row), Some(range.start));
                assert!(page.rows.len() <= PAGE_ROWS);
                assert_eq!(visits.load(Ordering::Relaxed), 100_000);
            }
            {
                let mut request = input.lock().map_err(err)?;
                request.query = "needle".into();
                request.range = 0..64;
            }
            handle.thread().unpark();
            let found = wait(&output, |u| u.query == "needle" && u.complete)?;
            assert_eq!(found.count, 1);
            assert_eq!(
                found.rows.first().map(|(_, c)| c.subject()),
                Some("Commit 99999")
            );
            input.lock().map_err(err)?.query = "absent".into();
            handle.thread().unpark();
            let empty = wait(&output, |u| u.query == "absent" && u.complete)?;
            assert_eq!(empty.count, 0);
            assert!(empty.selection.is_none());
            Ok(())
        })();
        gate.store(true, Ordering::Relaxed);
        stop.store(true, Ordering::Relaxed);
        handle.thread().unpark();
        handle.join().map_err(|_| "History worker panicked")??;
        result
    }
}
