//! Cancellable comparison worker with disk-backed records and a single page
//! mailbox, plus a line counter that fills per-file stats behind the listing.
use gg_engine::review::*;
use std::{
    ops::Range,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Update {
    pub comparison: Comparison,
    pub count: usize,
    pub complete: bool,
    pub error: Option<String>,
    pub rows: Vec<(usize, FileChange)>,
}
/// Line counts for the requested rows that are counted so far, and the
/// running total over every counted file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatsUpdate {
    pub rows: Vec<(usize, LineStat)>,
    pub added: u64,
    pub deleted: u64,
    /// The listing finished and every listed file is counted.
    pub complete: bool,
}
pub struct Worker {
    request: Arc<Mutex<Range<usize>>>,
    output: Arc<Mutex<Option<Result<Update, String>>>>,
    stats: Arc<Mutex<Option<StatsUpdate>>>,
    stop: Arc<AtomicBool>,
    thread: thread::Thread,
    counter: thread::Thread,
}
/// Listing states shared with the line counter.
const LISTING: u8 = 0;
const LISTED: u8 = 1;
const FAILED: u8 = 2;
impl Worker {
    pub fn start(request: ReviewRequest, ctx: eframe::egui::Context) -> Result<Self, String> {
        let range = Arc::new(Mutex::new(0..64));
        let output = Arc::new(Mutex::new(None));
        let stats = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let store = Arc::new(Mutex::new(Store::new()?));
        let listed = Arc::new(AtomicU8::new(LISTING));
        let (r, o, s, st, l, c) = (
            range.clone(),
            output.clone(),
            stop.clone(),
            store.clone(),
            listed.clone(),
            ctx.clone(),
        );
        let path = request.path.clone();
        let handle = thread::Builder::new()
            .name("gg-review".into())
            .spawn(move || {
                let result = gg_git::review::ReviewSession::open(&request)
                    .and_then(|reader| run(reader, &r, &st, &l, &o, &s, &c));
                if let Err(error) = result {
                    l.store(FAILED, Ordering::Release);
                    if let Ok(mut slot) = o.lock() {
                        *slot = Some(Err(error));
                    }
                    c.request_repaint();
                }
            })
            .map_err(|e| e.to_string())?;
        let (r, o, s) = (range.clone(), stats.clone(), stop.clone());
        let counter = thread::Builder::new()
            .name("gg-review-stats".into())
            .spawn(move || {
                // Stats are an enhancement: without them the slots stay empty.
                if let Ok(lines) = gg_git::review::LineStats::open(&path) {
                    let count =
                        |change: &FileChange| lines.count(change).unwrap_or(LineStat::Skipped);
                    let _ = run_stats(count, &r, &store, &listed, &o, &s, &ctx);
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            request: range,
            output,
            stats,
            stop,
            thread: handle.thread().clone(),
            counter: counter.thread().clone(),
        })
    }
    pub fn request(&self, range: Range<usize>) {
        if let Ok(mut current) = self.request.lock()
            && *current != range
        {
            *current = range;
            self.thread.unpark();
            self.counter.unpark();
        }
    }
    pub fn poll(&self) -> Option<Result<Update, String>> {
        self.output.try_lock().ok()?.take()
    }
    pub fn poll_stats(&self) -> Option<StatsUpdate> {
        self.stats.try_lock().ok()?.take()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.unpark();
        self.counter.unpark();
    }
}
fn error(e: impl std::fmt::Display) -> String {
    format!("Could not store comparison: {e}")
}
type Store = crate::records::Store<FileChange>;

fn run(
    mut reader: impl ReviewReader,
    input: &Mutex<Range<usize>>,
    store: &Mutex<Store>,
    listed: &AtomicU8,
    output: &Mutex<Option<Result<Update, String>>>,
    stop: &AtomicBool,
    ctx: &eframe::egui::Context,
) -> Result<(), String> {
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
                    if let Err(e) = store.lock().map_err(error)?.append(&change) {
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
        let (rows, count) = {
            let mut store = store.lock().map_err(error)?;
            match store.page(range.clone()) {
                Ok(rows) => (rows, store.count),
                Err(e) => {
                    failure = Some(e);
                    (Vec::new(), store.count)
                }
            }
        };
        if failure.is_some() {
            listed.store(FAILED, Ordering::Release);
        } else if complete {
            listed.store(LISTED, Ordering::Release);
        }
        *output.lock().map_err(error)? = Some(Ok(Update {
            comparison: reader.comparison().clone(),
            count,
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

/// Count lines of listed files: requested rows first, then the rest in order,
/// in 8-ms slices. Publishes the requested rows and the running total.
fn run_stats(
    count: impl Fn(&FileChange) -> LineStat,
    input: &Mutex<Range<usize>>,
    store: &Mutex<Store>,
    listed: &AtomicU8,
    output: &Mutex<Option<StatsUpdate>>,
    stop: &AtomicBool,
    ctx: &eframe::egui::Context,
) -> Result<(), String> {
    let mut stats: Vec<Option<LineStat>> = Vec::new();
    let (mut cursor, mut added, mut deleted) = (0usize, 0u64, 0u64);
    let mut published: Option<(Range<usize>, u8)> = None;
    let mut changed = false;
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let range = input.lock().map_err(error)?.clone();
        // Read the listing state before the count, so a finished listing
        // implies the count below is final.
        let state = listed.load(Ordering::Acquire);
        let total = store.lock().map_err(error)?.count;
        if stats.len() < total {
            stats.resize(total, None);
        }
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(8) && !stop.load(Ordering::Relaxed) {
            while cursor < total && stats[cursor].is_some() {
                cursor += 1;
            }
            let visible = (range.start..range.end.min(total)).find(|row| stats[*row].is_none());
            let Some(row) = visible.or((cursor < total).then_some(cursor)) else {
                break;
            };
            let Some((_, change)) = store.lock().map_err(error)?.page(row..row + 1)?.pop() else {
                break;
            };
            let stat = count(&change);
            if let LineStat::Lines {
                added: a,
                deleted: d,
            } = stat
            {
                added += u64::from(a);
                deleted += u64::from(d);
            }
            stats[row] = Some(stat);
            changed = true;
        }
        while cursor < total && stats[cursor].is_some() {
            cursor += 1;
        }
        let counted = cursor >= total;
        let complete = counted && state == LISTED;
        if changed || published.as_ref() != Some(&(range.clone(), state)) {
            let rows = (range.start..range.end.min(total).min(range.start.saturating_add(256)))
                .filter_map(|row| stats[row].map(|stat| (row, stat)))
                .collect();
            *output.lock().map_err(error)? = Some(StatsUpdate {
                rows,
                added,
                deleted,
                complete,
            });
            ctx.request_repaint();
            published = Some((range.clone(), state));
            changed = false;
        }
        if counted {
            if state == LISTING {
                // More files may arrive; check again shortly.
                thread::park_timeout(Duration::from_millis(10));
            } else {
                while !stop.load(Ordering::Relaxed) && *input.lock().map_err(error)? == range {
                    thread::park();
                }
            }
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
    fn wait<T>(output: &Mutex<Option<T>>, predicate: impl Fn(&T) -> bool) -> Result<T, String> {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(10) {
            if let Some(update) = output.lock().map_err(error)?.take()
                && predicate(&update)
            {
                return Ok(update);
            }
            thread::sleep(Duration::from_millis(1));
        }
        Err("Timed out waiting for comparison".into())
    }
    fn wait_page(
        output: &Mutex<Option<Result<Update, String>>>,
        predicate: impl Fn(&Update) -> bool,
    ) -> Result<Update, String> {
        wait(output, |result| result.as_ref().is_ok_and(&predicate))?
    }
    /// Line counts of the fixture's `file-NNNNN`: the number as added lines.
    fn fixture_count(change: &FileChange) -> LineStat {
        row_count(change.path[5..].parse::<u32>().unwrap_or(0))
    }
    fn row_count(number: u32) -> LineStat {
        if number % 100 == 99 {
            LineStat::Binary
        } else {
            LineStat::Lines {
                added: number % 7,
                deleted: number % 3,
            }
        }
    }
    #[test]
    fn streams_early_pages_reloads_and_cancels_after_completion_or_failure() -> Result<(), String> {
        for fail in [false, true] {
            let input = Arc::new(Mutex::new(0..64));
            let output = Arc::new(Mutex::new(None));
            let stats = Arc::new(Mutex::new(None));
            let stop = Arc::new(AtomicBool::new(false));
            let gate = Arc::new(AtomicBool::new(false));
            let visits = Arc::new(AtomicUsize::new(0));
            let store = Arc::new(Mutex::new(Store::new()?));
            let listed = Arc::new(AtomicU8::new(LISTING));
            let reader = Reader {
                identity: Comparison {
                    path: "repo".into(),
                    commit: "commit".into(),
                    parent: None,
                    parents: vec![],
                    subject: "root".into(),
                    author: String::new(),
                    date: String::new(),
                },
                next: 0,
                visits: visits.clone(),
                gate: gate.clone(),
                stop: stop.clone(),
                fail,
            };
            let (i, st, l, o, s) = (
                input.clone(),
                store.clone(),
                listed.clone(),
                output.clone(),
                stop.clone(),
            );
            let handle = thread::spawn(move || {
                let result = run(reader, &i, &st, &l, &o, &s, &Default::default());
                if result.is_err() {
                    l.store(FAILED, Ordering::Release);
                }
                result
            });
            let (i, st, l, o, s) = (
                input.clone(),
                store.clone(),
                listed.clone(),
                stats.clone(),
                stop.clone(),
            );
            let counter = thread::spawn(move || {
                run_stats(fixture_count, &i, &st, &l, &o, &s, &Default::default())
            });
            let result = (|| -> Result<(), String> {
                let first = wait_page(&output, |u| u.count > 0)?;
                assert!(!first.complete);
                assert!(first.count <= 32);
                assert!(visits.load(Ordering::Relaxed) <= 32);
                // Stats for the listed rows arrive while the listing waits,
                // and the total stays incomplete.
                let early = wait(&stats, |u| !u.rows.is_empty())?;
                assert!(!early.complete);
                gate.store(true, Ordering::Relaxed);
                let done = wait_page(&output, |u| u.complete || u.error.is_some())?;
                assert_eq!(done.complete, !fail);
                assert_eq!(done.count, if fail { 32 } else { 10_000 });
                let files = done.count as u32;
                let expected: u64 = (0..files)
                    .filter(|n| n % 100 != 99)
                    .map(|n| u64::from(n % 7))
                    .sum();
                let counted = wait(&stats, |u| u.added == expected && (fail || u.complete))?;
                assert_eq!(counted.complete, !fail);
                if fail {
                    assert_eq!(done.error.as_deref(), Some("Missing tree"));
                    assert!(!done.rows.is_empty());
                } else {
                    for range in [9900..10_000, 0..64] {
                        *input.lock().map_err(error)? = range.clone();
                        handle.thread().unpark();
                        counter.thread().unpark();
                        let page = wait_page(&output, |u| {
                            u.rows.first().is_some_and(|(row, _)| *row == range.start)
                        })?;
                        assert!(page.rows.len() <= 256);
                        assert_eq!(page.rows[0].1.path, format!("file-{:05}", range.start));
                        assert_eq!(visits.load(Ordering::Relaxed), 10_000);
                        let rows = wait(&stats, |u| {
                            u.rows.first().is_some_and(|(row, _)| *row == range.start)
                        })?;
                        assert_eq!(rows.rows.len(), range.len());
                        assert!(rows.complete);
                        assert!(
                            rows.rows
                                .iter()
                                .all(|(row, stat)| *stat == row_count(*row as u32))
                        );
                    }
                }
                Ok(())
            })();
            gate.store(true, Ordering::Relaxed);
            stop.store(true, Ordering::Relaxed);
            handle.thread().unpark();
            counter.thread().unpark();
            handle.join().map_err(|_| "Comparison worker panicked")??;
            counter.join().map_err(|_| "Line counter panicked")??;
            result?;
        }
        Ok(())
    }
}
