//! Bounded scan delivery and disk-backed viewport pages for both Local Changes lists.
use gg_engine::status::{Area, StatusEntry, StatusIdentity};
use std::{
    collections::HashMap,
    ops::Range,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ranges {
    /// Visible rows in the Unstaged group (Unstaged, Untracked, and Conflict entries).
    pub unstaged: Range<usize>,
    /// Visible rows in the Staged group.
    pub staged: Range<usize>,
    /// Selected comparison, searched across the completed scan.
    pub selected: Option<(Vec<u8>, Area)>,
}
#[derive(Clone)]
pub struct Update {
    pub identity: StatusIdentity,
    /// Pages in list coordinates: `[Unstaged group, Staged group]`.
    pub rows: [Vec<(usize, StatusEntry)>; 2],
    pub count: usize,
    pub staged: usize,
    pub complete: bool,
    pub error: Option<String>,
    /// Selected comparison as `(list, row, entry)` once the scan reaches it.
    pub selection: Option<(usize, usize, StatusEntry)>,
    /// The comparison these lookups answer (`Ranges::selected`).
    pub selected: Option<(Vec<u8>, Area)>,
    /// The same path's entry on the other side (Staged for a working entry,
    /// Unstaged or Untracked for a staged one) once the scan reaches it.
    pub counterpart: Option<(usize, usize, StatusEntry)>,
}
impl Update {
    pub fn unstaged_count(&self) -> usize {
        self.count.saturating_sub(self.staged)
    }
}
pub struct Worker {
    input: Arc<Mutex<Ranges>>,
    output: Arc<Mutex<Option<Result<Update, String>>>>,
    stop: Arc<AtomicBool>,
    thread: thread::Thread,
}
enum Event {
    Identity(StatusIdentity),
    Entry(StatusEntry),
    Done(Result<(), String>),
}
fn send(tx: &mpsc::SyncSender<Event>, mut event: Event, stop: &AtomicBool) -> Result<(), String> {
    loop {
        if stop.load(Ordering::Relaxed) {
            return Err("Status cancelled".into());
        }
        match tx.try_send(event) {
            Ok(()) => return Ok(()),
            Err(mpsc::TrySendError::Full(value)) => {
                event = value;
                thread::sleep(Duration::from_millis(1));
            }
            Err(mpsc::TrySendError::Disconnected(_)) => return Err("Status closed".into()),
        }
    }
}
impl Worker {
    pub fn start(
        path: PathBuf,
        ctx: eframe::egui::Context,
        base_tree: Option<String>,
    ) -> Result<Self, String> {
        let input = Arc::new(Mutex::new(Ranges {
            unstaged: 0..64,
            staged: 0..64,
            selected: None,
        }));
        let output = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let (i, o, s) = (input.clone(), output.clone(), stop.clone());
        let handle = thread::Builder::new()
            .name("gg-status-pages".into())
            .spawn(move || {
                let result = run(path, base_tree, &i, &o, &s, &ctx);
                if let Err(e) = result {
                    if let Ok(mut out) = o.lock() {
                        *out = Some(Err(e));
                    }
                    ctx.request_repaint();
                }
                s.store(true, Ordering::Relaxed);
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            input,
            output,
            stop,
            thread: handle.thread().clone(),
        })
    }
    pub fn request(&self, ranges: Ranges) {
        if let Ok(mut slot) = self.input.lock()
            && *slot != ranges
        {
            *slot = ranges;
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
fn run(
    path: PathBuf,
    base_tree: Option<String>,
    input: &Mutex<Ranges>,
    output: &Mutex<Option<Result<Update, String>>>,
    stop: &Arc<AtomicBool>,
    ctx: &eframe::egui::Context,
) -> Result<(), String> {
    let (tx, rx) = mpsc::sync_channel(128);
    let s = stop.clone();
    thread::Builder::new()
        .name("gg-status-scan".into())
        .spawn(move || {
            let result = gg_git::status::Session::open_with_base_tree(&path, base_tree).and_then(
                |session| {
                    send(&tx, Event::Identity(session.identity.clone()), &s)?;
                    session.scan(&s, |entry| send(&tx, Event::Entry(entry), &s))
                },
            );
            let _ = send(&tx, Event::Done(result), &s);
        })
        .map_err(|e| e.to_string())?;
    let mut store = crate::records::Store::new()?;
    let mut identity = None;
    let mut staged = 0;
    let mut done = false;
    let mut error = None;
    let mut positions: HashMap<(Vec<u8>, Area), usize> = HashMap::new();
    let mut saw_working = false;
    while !stop.load(Ordering::Relaxed) {
        let ranges = input.lock().map_err(|e| e.to_string())?.clone();
        if !done {
            let started = Instant::now();
            for _ in 0..32 {
                match rx.recv_timeout(Duration::from_millis(4)) {
                    Ok(Event::Identity(value)) => identity = Some(value),
                    Ok(Event::Entry(entry)) => {
                        if entry.area == Area::Staged {
                            debug_assert!(
                                !saw_working,
                                "staged rows must precede every working row"
                            );
                            staged += 1;
                        } else {
                            saw_working = true;
                        }
                        store.append(&entry)?;
                        positions.insert(
                            (entry.path_bytes.clone(), entry.area),
                            store.count.saturating_sub(1),
                        );
                    }
                    Ok(Event::Done(result)) => {
                        done = true;
                        error = result.err();
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        done = true;
                        error = Some("Status worker stopped".into());
                        break;
                    }
                }
                if started.elapsed() >= Duration::from_millis(4) {
                    break;
                }
            }
        }
        if let Some(identity) = &identity {
            let staged_end = staged.min(store.count);
            let staged_start = ranges.staged.start.min(staged_end);
            let staged_range = staged_start..ranges.staged.end.min(staged_end).max(staged_start);
            let working_start = staged
                .saturating_add(ranges.unstaged.start)
                .min(store.count);
            let working_end = staged
                .saturating_add(ranges.unstaged.end)
                .min(store.count)
                .max(working_start);
            let mut locate = |global: usize| {
                store
                    .page(global..global + 1)
                    .ok()?
                    .into_iter()
                    .next()
                    .map(|(_, entry)| {
                        let list = if entry.area == Area::Staged { 1 } else { 0 };
                        let row = if list == 1 {
                            global
                        } else {
                            global.saturating_sub(staged)
                        };
                        (list, row, entry)
                    })
            };
            let selection = ranges
                .selected
                .as_ref()
                .and_then(|key| positions.get(key).copied())
                .and_then(&mut locate);
            let counterpart = ranges.selected.as_ref().and_then(|(path, area)| {
                let others: &[Area] = match area {
                    Area::Staged => &[Area::Unstaged, Area::Untracked],
                    Area::Unstaged | Area::Untracked => &[Area::Staged],
                    Area::Conflict => &[],
                };
                others
                    .iter()
                    .find_map(|other| positions.get(&(path.clone(), *other)).copied())
                    .and_then(&mut locate)
            });
            *output.lock().map_err(|e| e.to_string())? = Some(Ok(Update {
                identity: identity.clone(),
                rows: [
                    store
                        .page(working_start..working_end)?
                        .into_iter()
                        .map(|(row, entry)| (row.saturating_sub(staged), entry))
                        .collect(),
                    store.page(staged_range)?,
                ],
                count: store.count,
                staged,
                complete: done && error.is_none(),
                error: error.clone(),
                selection,
                selected: ranges.selected.clone(),
                counterpart,
            }));
            ctx.request_repaint();
        } else if let Some(error) = &error {
            return Err(error.clone());
        }
        if done {
            while !stop.load(Ordering::Relaxed)
                && *input.lock().map_err(|e| e.to_string())? == ranges
            {
                thread::park();
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_queue_cancels_without_waiting_for_a_consumer() -> Result<(), Box<dyn std::error::Error>>
    {
        let (tx, _rx) = mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        send(&tx, Event::Done(Ok(())), &stop)?;
        let s = stop.clone();
        let thread = std::thread::spawn(move || send(&tx, Event::Done(Ok(())), &s));
        stop.store(true, Ordering::Relaxed);
        assert!(thread.join().map_err(|_| "producer panicked")?.is_err());
        Ok(())
    }
    #[test]
    fn large_status_pages_are_complete_bounded_and_revisitable()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(["init", "-q"])
                .status()?
                .success()
        );
        for i in 0..1000 {
            std::fs::write(dir.path().join(format!("file-{i:04}")), b"content")?;
        }
        let worker = Worker::start(dir.path().into(), eframe::egui::Context::default(), None)?;
        let ranges = |unstaged: Range<usize>, staged: Range<usize>| Ranges {
            unstaged,
            staged,
            selected: None,
        };
        let wait = |ranges: Ranges,
                    complete: bool,
                    list: usize,
                    row: usize|
         -> Result<Update, Box<dyn std::error::Error>> {
            worker.request(ranges);
            let deadline = std::time::Instant::now() + Duration::from_secs(15);
            loop {
                if let Some(result) = worker.poll() {
                    let update = result?;
                    assert!(update.rows.iter().all(|rows| rows.len() <= 256));
                    assert!(update.error.is_none(), "{:?}", update.error);
                    if (!complete || update.complete)
                        && update.rows[list].iter().any(|(index, _)| *index == row)
                    {
                        return Ok(update);
                    }
                }
                if std::time::Instant::now() > deadline {
                    return Err("status timed out".into());
                }
                thread::sleep(Duration::from_millis(2));
            }
        };
        let first = wait(ranges(0..32, 0..32), false, 0, 0)?;
        assert!(first.unstaged_count() <= 1000);
        assert!(first.rows[1].is_empty());
        let far = wait(ranges(990..1000, 0..32), true, 0, 999)?;
        assert_eq!(far.unstaged_count(), 1000);
        let staged = wait(ranges(0..32, 0..32), true, 0, 0)?;
        assert_eq!(staged.rows[0][0].1, first.rows[0][0].1);
        Ok(())
    }
    #[test]
    fn staged_and_unstaged_requests_page_independently() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let git = |args: &[&str]| -> Result<(), Box<dyn std::error::Error>> {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env(
                    "GIT_CONFIG_GLOBAL",
                    if cfg!(windows) { "NUL" } else { "/dev/null" },
                )
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.org")
                .env("GIT_COMMITTER_EMAIL", "test@example.org")
                .output()?;
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            Ok(())
        };
        git(&["init", "-q"])?;
        std::fs::write(dir.path().join("a"), b"base\n")?;
        git(&["add", "a"])?;
        git(&["commit", "-qm", "base"])?;
        std::fs::write(dir.path().join("a"), b"staged\n")?;
        git(&["add", "a"])?;
        std::fs::write(dir.path().join("a"), b"working\n")?;
        std::fs::write(dir.path().join("new"), b"untracked\n")?;
        let worker = Worker::start(dir.path().into(), eframe::egui::Context::default(), None)?;
        worker.request(Ranges {
            unstaged: 0..2,
            staged: 0..1,
            selected: None,
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        'outer: loop {
            if let Some(result) = worker.poll() {
                let update = result?;
                if !update.complete || update.rows[0].is_empty() || update.rows[1].is_empty() {
                    if std::time::Instant::now() > deadline {
                        return Err("status timed out".into());
                    }
                    thread::sleep(Duration::from_millis(2));
                    continue;
                }
                assert_eq!((update.count, update.staged), (3, 1));
                assert_eq!(
                    update.rows[0]
                        .iter()
                        .map(|(index, _)| *index)
                        .collect::<Vec<_>>(),
                    vec![0, 1],
                    "working pages use their own list coordinates"
                );
                assert_eq!(update.rows[1][0].1.area, Area::Staged);
                assert_eq!(update.rows[0][0].1.area, Area::Unstaged);
                assert_eq!(update.rows[0][1].1.area, Area::Untracked);
                worker.request(Ranges {
                    unstaged: 0..1,
                    staged: 0..1,
                    selected: Some((b"new".to_vec(), Area::Untracked)),
                });
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                loop {
                    if let Some(result) = worker.poll() {
                        let update = result?;
                        if let Some((list, row, entry)) = update.selection {
                            assert_eq!(list, 0);
                            assert_eq!(row, 1);
                            assert_eq!(entry.path, "new");
                            assert!(update.counterpart.is_none(), "nothing staged for new");
                            break;
                        }
                    }
                    if std::time::Instant::now() > deadline {
                        return Err("selected row was not found".into());
                    }
                    thread::sleep(Duration::from_millis(2));
                }
                worker.request(Ranges {
                    unstaged: 0..0,
                    staged: 0..0,
                    selected: Some((b"a".to_vec(), Area::Staged)),
                });
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                loop {
                    if let Some(result) = worker.poll() {
                        let update = result?;
                        if let Some((list, row, entry)) = update.counterpart {
                            assert_eq!((list, row), (0, 0));
                            assert_eq!(entry.area, Area::Unstaged);
                            assert_eq!(
                                update.selected,
                                Some((b"a".to_vec(), Area::Staged)),
                                "the lookup names the comparison it answers"
                            );
                            break 'outer;
                        }
                    }
                    if std::time::Instant::now() > deadline {
                        return Err("counterpart was not found".into());
                    }
                    thread::sleep(Duration::from_millis(2));
                }
            }
            if std::time::Instant::now() > deadline {
                return Err("status timed out".into());
            }
            thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }
}
