//! Bounded scan delivery and disk-backed viewport pages.
use gg_engine::status::{StatusEntry, StatusIdentity};
use std::{
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
#[derive(Clone)]
pub struct Update {
    pub identity: StatusIdentity,
    pub rows: Vec<(usize, StatusEntry)>,
    pub count: usize,
    pub complete: bool,
    pub error: Option<String>,
}
pub struct Worker {
    input: Arc<Mutex<Range<usize>>>,
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
    pub fn start(path: PathBuf, ctx: eframe::egui::Context) -> Result<Self, String> {
        let input = Arc::new(Mutex::new(0..64));
        let output = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let (i, o, s) = (input.clone(), output.clone(), stop.clone());
        let handle = thread::Builder::new()
            .name("gg-status-pages".into())
            .spawn(move || {
                let result = run(path, &i, &o, &s, &ctx);
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
    pub fn request(&self, range: Range<usize>) {
        if let Ok(mut slot) = self.input.lock()
            && *slot != range
        {
            *slot = range;
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
    input: &Mutex<Range<usize>>,
    output: &Mutex<Option<Result<Update, String>>>,
    stop: &Arc<AtomicBool>,
    ctx: &eframe::egui::Context,
) -> Result<(), String> {
    let (tx, rx) = mpsc::sync_channel(128);
    let s = stop.clone();
    thread::Builder::new()
        .name("gg-status-scan".into())
        .spawn(move || {
            let result = gg_git::status::Session::open(&path).and_then(|session| {
                send(&tx, Event::Identity(session.identity.clone()), &s)?;
                session.scan(&s, |entry| send(&tx, Event::Entry(entry), &s))
            });
            let _ = send(&tx, Event::Done(result), &s);
        })
        .map_err(|e| e.to_string())?;
    let mut store = crate::records::Store::new()?;
    let mut identity = None;
    let mut done = false;
    let mut error = None;
    while !stop.load(Ordering::Relaxed) {
        let range = input.lock().map_err(|e| e.to_string())?.clone();
        if !done {
            let started = Instant::now();
            for _ in 0..32 {
                match rx.recv_timeout(Duration::from_millis(4)) {
                    Ok(Event::Identity(value)) => identity = Some(value),
                    Ok(Event::Entry(entry)) => store.append(&entry)?,
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
            *output.lock().map_err(|e| e.to_string())? = Some(Ok(Update {
                identity: identity.clone(),
                rows: store.page(range.clone())?,
                count: store.count,
                complete: done && error.is_none(),
                error: error.clone(),
            }));
            ctx.request_repaint();
        } else if let Some(error) = &error {
            return Err(error.clone());
        }
        if done {
            while !stop.load(Ordering::Relaxed)
                && *input.lock().map_err(|e| e.to_string())? == range
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
        let worker = Worker::start(dir.path().into(), eframe::egui::Context::default())?;
        let wait = |complete: bool, row: usize| -> Result<Update, Box<dyn std::error::Error>> {
            let deadline = std::time::Instant::now() + Duration::from_secs(15);
            loop {
                if let Some(result) = worker.poll() {
                    let update = result?;
                    assert!(update.rows.len() <= 256);
                    assert!(update.error.is_none(), "{:?}", update.error);
                    if (!complete || update.complete) && update.rows.iter().any(|(i, _)| *i == row)
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
        let first = wait(false, 0)?;
        assert!(first.count <= 1000);
        worker.request(990..1000);
        assert_eq!(wait(true, 999)?.count, 1000);
        worker.request(0..32);
        assert_eq!(wait(true, 0)?.rows[0].1, first.rows[0].1);
        Ok(())
    }
}
