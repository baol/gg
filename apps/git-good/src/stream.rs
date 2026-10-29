//! Background traversal, disk-backed rows, and coalesced viewport requests.
use crate::graph;
use gg_engine::{Commit, History, HistoryReader, HistoryRequest, refs::Snapshot};
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
    pub revision: u64,
    pub query: String,
    pub range: Range<usize>,
    pub selected_id: Option<String>,
    pub selected_row: Option<usize>,
    pub anchor_id: Option<String>,
}
impl Default for Request {
    fn default() -> Self {
        Self {
            revision: 0,
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
    pub graphs: Vec<(usize, graph::Row)>,
    pub refs: Snapshot,
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
        let initial_revision = request.revision;
        let input = Arc::new(Mutex::new(request));
        let output = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let (i, o, s) = (input.clone(), output.clone(), stop.clone());
        let handle = thread::Builder::new()
            .name("gg-history".into())
            .spawn(move || {
                let result = run(path, previous_path, initial_revision, &i, &o, &s, &ctx);
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
const SPOOL_BUFFER: usize = 64 * 1024;

struct SpoolFile {
    file: File,
    end: u64,
    pos: u64,
    write_buf: Vec<u8>,
    read_buf: Vec<u8>,
    read_start: u64,
    read_len: usize,
}
impl SpoolFile {
    fn new() -> Result<Self, String> {
        Ok(Self {
            file: tempfile::tempfile().map_err(err)?,
            end: 0,
            pos: 0,
            write_buf: Vec::with_capacity(SPOOL_BUFFER),
            read_buf: Vec::new(),
            read_start: 0,
            read_len: 0,
        })
    }
    fn flush(&mut self) -> Result<(), String> {
        if self.write_buf.is_empty() {
            return Ok(());
        }
        let start = self.end - self.write_buf.len() as u64;
        if self.pos != start {
            self.file.seek(SeekFrom::Start(start)).map_err(err)?;
        }
        self.file.write_all(&self.write_buf).map_err(err)?;
        self.pos = self.end;
        self.write_buf.clear();
        Ok(())
    }
    fn append(&mut self, bytes: &[u8]) -> Result<u64, String> {
        let offset = self.end;
        self.end += bytes.len() as u64;
        self.write_buf.extend_from_slice(bytes);
        if self.write_buf.len() >= SPOOL_BUFFER {
            self.flush()?;
        }
        Ok(offset)
    }
    fn read_at(&mut self, offset: u64, out: &mut [u8]) -> Result<(), String> {
        self.flush()?;
        let end = offset.saturating_add(out.len() as u64);
        if offset >= self.read_start && end <= self.read_start + self.read_len as u64 {
            let rel = (offset - self.read_start) as usize;
            out.copy_from_slice(&self.read_buf[rel..rel + out.len()]);
            return Ok(());
        }
        if out.len() >= SPOOL_BUFFER {
            if self.pos != offset {
                self.file.seek(SeekFrom::Start(offset)).map_err(err)?;
            }
            self.file.read_exact(out).map_err(err)?;
            self.pos = end;
            return Ok(());
        }
        let available = self.end.saturating_sub(offset) as usize;
        let chunk = available.min(SPOOL_BUFFER);
        if chunk < out.len() {
            return Err(err("unexpected end of history spool"));
        }
        if self.pos != offset {
            self.file.seek(SeekFrom::Start(offset)).map_err(err)?;
        }
        if self.read_buf.len() < chunk {
            self.read_buf.resize(chunk, 0);
        }
        self.file
            .read_exact(&mut self.read_buf[..chunk])
            .map_err(err)?;
        self.pos = offset + chunk as u64;
        self.read_start = offset;
        self.read_len = chunk;
        out.copy_from_slice(&self.read_buf[..out.len()]);
        Ok(())
    }
    fn clear(&mut self) -> Result<(), String> {
        self.write_buf.clear();
        self.read_start = 0;
        self.read_len = 0;
        self.file.set_len(0).map_err(err)?;
        self.file.seek(SeekFrom::Start(0)).map_err(err)?;
        self.end = 0;
        self.pos = 0;
        Ok(())
    }
}

struct Spool {
    data: SpoolFile,
    index: SpoolFile,
    matches: SpoolFile,
    count: usize,
    matched: usize,
}
impl Spool {
    fn new() -> Result<Self, String> {
        Ok(Self {
            data: SpoolFile::new()?,
            index: SpoolFile::new()?,
            matches: SpoolFile::new()?,
            count: 0,
            matched: 0,
        })
    }
    fn append(&mut self, commit: &Commit) -> Result<[u8; 16], String> {
        let bytes = serde_json::to_vec(commit).map_err(err)?;
        let offset = self.data.append(&bytes)?;
        let mut record = [0u8; 16];
        record[..8].copy_from_slice(&offset.to_le_bytes());
        record[8..].copy_from_slice(&(bytes.len() as u64).to_le_bytes());
        self.index.append(&record)?;
        self.count += 1;
        Ok(record)
    }
    fn read(&mut self, row: usize, filtered: bool) -> Result<(Commit, [u8; 16]), String> {
        let target = row as u64 * 16;
        let mut record = [0u8; 16];
        if filtered {
            self.matches.read_at(target, &mut record)?;
        } else {
            self.index.read_at(target, &mut record)?;
        }
        let mut offset = [0u8; 8];
        let mut len = [0u8; 8];
        offset.copy_from_slice(&record[..8]);
        len.copy_from_slice(&record[8..]);
        let offset = u64::from_le_bytes(offset);
        let len = u64::from_le_bytes(len) as usize;
        let mut bytes = vec![0; len];
        self.data.read_at(offset, &mut bytes)?;
        Ok((serde_json::from_slice(&bytes).map_err(err)?, record))
    }
    fn include(&mut self, record: [u8; 16]) -> Result<(), String> {
        self.matches.append(&record)?;
        self.matched += 1;
        Ok(())
    }
    fn reset_filter(&mut self) -> Result<(), String> {
        self.matches.clear()?;
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
    initial_revision: u64,
    input: &Mutex<Request>,
    output: &Mutex<Option<Result<Update, String>>>,
    stop: &AtomicBool,
    ctx: &eframe::egui::Context,
) -> Result<(), String> {
    let refs = gg_git::refs::read(&path)?;
    let session = gg_git::Session::open_with_refs(&HistoryRequest { path }, refs.clone())?;
    if previous_path
        .as_ref()
        .is_some_and(|old| *old != session.identity.path)
    {
        let mut request = input.lock().map_err(err)?;
        let query = if request.revision != initial_revision {
            request.query.clone()
        } else {
            String::new()
        };
        *request = Request {
            revision: request.revision,
            query,
            ..Default::default()
        };
    }
    run_session(session, refs, input, output, stop, ctx)
}

fn run_session(
    mut session: impl HistoryReader,
    refs: Snapshot,
    input: &Mutex<Request>,
    output: &Mutex<Option<Result<Update, String>>>,
    stop: &AtomicBool,
    ctx: &eframe::egui::Context,
) -> Result<(), String> {
    let mut spool = Spool::new()?;
    let mut graph_state = graph::Graph::default();
    let mut graph_rows = Vec::new();
    let mut ids = Vec::new();
    let mut current = input.lock().map_err(err)?.clone();
    let mut scanned = 0;
    let mut finished = false;
    let mut failure = None;
    let mut selection = None;
    let mut anchor_row = None;
    let mut cached_range = 0..0;
    let mut cached_rows = Vec::new();
    let mut cached_graphs = Vec::new();
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let next = input.lock().map_err(err)?.clone();
        if next.query != current.query || next.revision != current.revision {
            spool.reset_filter()?;
            cached_range = 0..0;
            cached_rows.clear();
            scanned = 0;
            selection = None;
        }
        if next.selected_id != current.selected_id || next.selected_row != current.selected_row {
            selection = None;
        }
        if next.anchor_id != current.anchor_id
            || next.query != current.query
            || next.revision != current.revision
        {
            anchor_row = None;
        }
        current = next;
        let start = Instant::now();
        // A small first batch reaches the screen before counting the remainder.
        let batch = if spool.count == 0 { 32 } else { 2048 };
        for _ in 0..batch {
            if stop.load(Ordering::Relaxed) {
                return Ok(());
            }
            let mut appended = None;
            if scanned == spool.count && !finished {
                match session.next_commit() {
                    Ok(Some(commit)) => {
                        let record = spool.append(&commit)?;
                        graph_rows.push(graph_state.push(&commit));
                        ids.push(commit.id.clone());
                        appended = Some((commit, record));
                    }
                    Ok(None) => finished = true,
                    Err(error) => {
                        failure = Some(error);
                        finished = true;
                    }
                }
            }
            if scanned < spool.count {
                let (commit, record) = if let Some(pair) = appended {
                    pair
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
                    spool.include(record)?;
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
                selection = Some((row, spool.read(row, true)?.0));
            }
        } else if selection.is_none()
            && finished
            && scanned == spool.count
            && current.query.is_empty()
            && let Some(id) = current.selected_id.as_deref()
            && let Some(row) = ids.iter().position(|candidate| candidate == id)
        {
            selection = Some((row, spool.read(row, true)?.0));
        } else if selection.is_none() && current.selected_id.is_none() && spool.matched > 0 {
            selection = Some((0, spool.read(0, true)?.0));
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
            cached_graphs.clear();
            for row in range.clone() {
                let mut commit = spool.read(row, true)?.0;
                commit.message = commit.subject().to_owned();
                cached_rows.push((row, commit));
                cached_graphs.push((row, graph_rows[row].clone()));
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
            graphs: cached_graphs.clone(),
            refs: refs.clone(),
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
pub struct Driver {
    request: Arc<Mutex<Request>>,
    output: Arc<Mutex<Option<Result<Update, String>>>>,
}
#[cfg(test)]
impl Worker {
    pub fn controlled() -> (Self, Driver) {
        let request = Arc::new(Mutex::new(Request::default()));
        let output = Arc::new(Mutex::new(None));
        (
            Self {
                request: request.clone(),
                output: output.clone(),
                stop: Arc::new(AtomicBool::new(false)),
                thread: thread::current(),
            },
            Driver { request, output },
        )
    }
}
#[cfg(test)]
impl Driver {
    pub fn publish(&self, update: Result<Update, String>) -> Result<(), String> {
        *self.output.lock().map_err(err)? = Some(update);
        Ok(())
    }
    pub fn requested(&self) -> Result<Request, String> {
        Ok(self.request.lock().map_err(err)?.clone())
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
        while start.elapsed() < Duration::from_secs(90) {
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
        let handle = thread::spawn(move || {
            run_session(
                fake,
                Default::default(),
                &i,
                &o,
                &s,
                &eframe::egui::Context::default(),
            )
        });
        let result = (|| -> TestResult {
            // Time slicing may publish fewer than 32 rows before the reader reaches its gate.
            // Accept that first batch: waiting for exactly 32 can deadlock the test itself.
            let first = wait(&output, |u| u.count > 0)?;
            assert!(!first.complete);
            assert!(visits.load(Ordering::Relaxed) <= 32);
            assert_eq!(first.rows.len(), first.count);
            assert!(first.count <= 32);
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
                let target = format!("{:040x}", 99_999);
                let mut request = input.lock().map_err(err)?;
                request.selected_id = Some(target.clone());
                request.selected_row = None;
                request.range = 0..64;
            }
            handle.thread().unpark();
            let target = format!("{:040x}", 99_999);
            let jumped = wait(&output, |u| {
                u.selection
                    .as_ref()
                    .is_some_and(|(_, commit)| commit.id == target)
            })?;
            assert_eq!(jumped.selection.as_ref().map(|(row, _)| *row), Some(99_999));

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
            assert!(
                found
                    .graphs
                    .iter()
                    .all(|(row, _)| found.rows.iter().any(|(visible, _)| visible == row))
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
