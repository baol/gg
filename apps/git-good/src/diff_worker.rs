//! One selected-file worker; generation, mode, and side changes supersede older work.
use gg_engine::{
    diff::{DiffRow, FileDiff},
    file::{FileText, ReadingSide, changed_lines},
    review::FileChange,
};
use std::{
    collections::BTreeSet,
    ops::Range,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Diff,
    Full,
}

fn side_line(row: &DiffRow, side: ReadingSide) -> Option<usize> {
    match side {
        ReadingSide::Before => row.old_line,
        ReadingSide::After => row.new_line.or(row.old_line),
    }
}

#[derive(Clone)]
struct Request {
    generation: u64,
    path: PathBuf,
    change: FileChange,
    range: Range<usize>,
    working: Option<gg_engine::status::WorkingRequest>,
    mode: Mode,
    side: ReadingSide,
    /// One-based line to center the first diff page on after a mode or file change.
    line: Option<usize>,
}

#[derive(Clone)]
pub struct Update {
    pub generation: u64,
    pub mode: Mode,
    pub side: ReadingSide,
    pub preview: Option<Arc<crate::binary_preview::Pair>>,
    pub count: usize,
    pub columns: usize,
    pub page_start: usize,
    pub result: Result<FileDiff, String>,
    pub full: Option<Result<FileText, String>>,
    /// Row indices of diff hunks, so change navigation can reach any page.
    pub hunks: Vec<usize>,
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
        Self::with_readers(
            ctx,
            |request, cancelled| {
                if let Some(working) = &request.working {
                    gg_git::working_diff::read(working, cancelled)
                } else {
                    gg_git::diff::read(&request.path, &request.change, cancelled)
                }
            },
            |request, side, changed, cancelled| {
                if let Some(working) = &request.working {
                    gg_git::working_diff::read_side(working, side, changed, cancelled)
                } else {
                    gg_git::file::read(&request.path, &request.change, side, changed, cancelled)
                }
            },
        )
    }

    fn with_readers(
        ctx: eframe::egui::Context,
        read: impl Fn(&Request, &dyn Fn() -> bool) -> Result<FileDiff, String> + Send + 'static,
        read_file: impl Fn(
            &Request,
            ReadingSide,
            &BTreeSet<usize>,
            &dyn Fn() -> bool,
        ) -> Result<FileText, String>
        + Send
        + 'static,
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
                let mut document: Option<Result<FileDiff, String>> = None;
                let mut loaded = 0;
                let mut max_columns = 0;
                let mut hunks: Vec<usize> = vec![];
                let mut preview = None;
                let mut full: Option<(u64, ReadingSide, Result<FileText, String>)> = None;
                while !s.load(Ordering::Relaxed) {
                    let request = i.lock().ok().and_then(|mut r| r.take());
                    let Some(request) = request else {
                        thread::park();
                        continue;
                    };
                    if request.generation != loaded {
                        document = None;
                        full = None;
                        hunks.clear();
                        let cancelled = || {
                            s.load(Ordering::Relaxed)
                                || g.load(Ordering::Relaxed) != request.generation
                        };
                        let result = read(&request, &cancelled);
                        preview = None;
                        if request.working.is_none()
                            && result.as_ref().is_ok_and(|diff| {
                                matches!(
                                    diff.kind,
                                    gg_engine::diff::DiffKind::Binary
                                        | gg_engine::diff::DiffKind::Limited
                                )
                            })
                            && !cancelled()
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
                        if let Ok(doc) = &result {
                            hunks = doc
                                .rows
                                .iter()
                                .enumerate()
                                .filter(|(_, row)| row.kind == gg_engine::diff::RowKind::Hunk)
                                .map(|(index, _)| index)
                                .collect();
                        }
                        document = Some(result);
                        loaded = request.generation;
                    }
                    if g.load(Ordering::Relaxed) != request.generation {
                        continue;
                    }
                    if !matches!(document, Some(Ok(_))) {
                        if let Some(Err(error)) = &document
                            && let Ok(mut slot) = o.lock()
                        {
                            *slot = Some(Update {
                                generation: loaded,
                                mode: request.mode,
                                side: request.side,
                                preview: preview.clone(),
                                count: 0,
                                columns: 0,
                                page_start: 0,
                                result: Err(error.clone()),
                                full: None,
                                hunks: vec![],
                            });
                            ctx.request_repaint();
                        }
                        continue;
                    }
                    let doc = match document.as_ref() {
                        Some(Ok(doc)) => doc,
                        _ => continue,
                    };
                    let update = if request.mode == Mode::Full {
                        let key_matches = full.as_ref().is_some_and(|(loaded, side, _)| {
                            *loaded == request.generation && *side == request.side
                        });
                        if !key_matches {
                            let changed = changed_lines(&doc.rows, request.side);
                            let cancelled = || {
                                s.load(Ordering::Relaxed)
                                    || g.load(Ordering::Relaxed) != request.generation
                            };
                            let result = read_file(&request, request.side, &changed, &cancelled);
                            if cancelled() {
                                continue;
                            }
                            full = Some((loaded, request.side, result));
                        }
                        let (count, columns, result, body) = match full.as_ref() {
                            Some((_, _, Ok(text))) => (
                                text.lines.len(),
                                text.lines
                                    .iter()
                                    .map(|line| line.text.chars().count())
                                    .max()
                                    .unwrap_or(0),
                                Ok(FileDiff {
                                    kind: doc.kind,
                                    old_size: doc.old_size,
                                    new_size: doc.new_size,
                                    complete: doc.complete,
                                    note: doc.note.clone(),
                                    rows: vec![],
                                }),
                                Ok(text.clone()),
                            ),
                            Some((_, _, Err(error))) => {
                                (0, 0, Err(error.clone()), Err(error.clone()))
                            }
                            None => continue,
                        };
                        Update {
                            generation: loaded,
                            mode: Mode::Full,
                            side: request.side,
                            preview: None,
                            count,
                            columns,
                            page_start: 0,
                            result,
                            full: Some(body),
                            hunks: hunks.clone(),
                        }
                    } else {
                        let start = match request.line {
                            Some(line) => doc
                                .rows
                                .iter()
                                .position(|row| side_line(row, request.side) == Some(line))
                                .or_else(|| {
                                    doc.rows
                                        .iter()
                                        .enumerate()
                                        .filter_map(|(index, row)| {
                                            side_line(row, request.side)
                                                .map(|number| (index, number.abs_diff(line)))
                                        })
                                        .min_by_key(|(_, distance)| *distance)
                                        .map(|(index, _)| index)
                                })
                                .unwrap_or(0)
                                .saturating_sub(128)
                                .min(doc.rows.len().saturating_sub(1)),
                            None => request.range.start,
                        };
                        let mut page = FileDiff {
                            kind: doc.kind,
                            old_size: doc.old_size,
                            new_size: doc.new_size,
                            complete: doc.complete,
                            note: doc.note.clone(),
                            rows: vec![],
                        };
                        page.rows = doc.rows.iter().skip(start).take(256).cloned().collect();
                        Update {
                            generation: loaded,
                            mode: Mode::Diff,
                            side: request.side,
                            preview: preview.clone(),
                            count: doc.rows.len(),
                            columns: max_columns,
                            page_start: start,
                            result: Ok(page),
                            full: None,
                            hunks: hunks.clone(),
                        }
                    };
                    if g.load(Ordering::Relaxed) != request.generation {
                        continue;
                    }
                    if let Ok(mut slot) = o.lock() {
                        *slot = Some(update);
                    }
                    ctx.request_repaint();
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

    #[allow(clippy::too_many_arguments)]
    pub fn request(
        &self,
        generation: u64,
        path: PathBuf,
        change: FileChange,
        range: Range<usize>,
        mode: Mode,
        side: ReadingSide,
        line: Option<usize>,
    ) {
        self.submit(generation, path, change, range, mode, side, line, None);
    }

    pub fn request_working(
        &self,
        generation: u64,
        working: gg_engine::status::WorkingRequest,
        range: Range<usize>,
        mode: Mode,
        side: ReadingSide,
        line: Option<usize>,
    ) {
        let change = FileChange {
            path: working.entry.path.clone(),
            path_bytes: working.entry.path_bytes.clone(),
            kind: gg_engine::review::ChangeKind::Modified,
            old: None,
            new: None,
        };
        self.submit(
            generation,
            working.repository.path.clone(),
            change,
            range,
            mode,
            side,
            line,
            Some(working),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn submit(
        &self,
        generation: u64,
        path: PathBuf,
        change: FileChange,
        range: Range<usize>,
        mode: Mode,
        side: ReadingSide,
        line: Option<usize>,
        working: Option<gg_engine::status::WorkingRequest>,
    ) {
        self.generation.store(generation, Ordering::Relaxed);
        if let Ok(mut slot) = self.input.lock() {
            *slot = Some(Request {
                generation,
                path,
                change,
                range,
                working,
                mode,
                side,
                line,
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
        file::FileKind,
        review::ChangeKind,
    };

    #[test]
    fn selection_cancels_old_work_and_pages_reuse_the_document() -> Result<(), String> {
        let (started, wait) = std::sync::mpsc::channel();
        let reads = Arc::new(AtomicU64::new(0));
        let count = reads.clone();
        let worker = Worker::with_readers(
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
            |_, _, _, _| Err("unused".into()),
        )?;
        let change = FileChange {
            path: "file".into(),
            path_bytes: b"file".to_vec(),
            kind: ChangeKind::Added,
            old: None,
            new: None,
        };
        worker.request(
            1,
            "repo".into(),
            change.clone(),
            0..1000,
            Mode::Diff,
            ReadingSide::After,
            None,
        );
        wait.recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        worker.request(
            2,
            "repo".into(),
            change.clone(),
            0..1000,
            Mode::Diff,
            ReadingSide::After,
            None,
        );
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
        worker.request(
            2,
            "repo".into(),
            change.clone(),
            900..1000,
            Mode::Diff,
            ReadingSide::After,
            None,
        );
        let update = poll()?;
        assert_eq!(update.page_start, 900);
        assert_eq!(update.result?.rows[0].new_line, Some(901));
        worker.request(
            2,
            "repo".into(),
            change,
            0..64,
            Mode::Diff,
            ReadingSide::After,
            Some(500),
        );
        let update = poll()?;
        assert_eq!(update.page_start, 371);
        assert_eq!(update.result?.rows[0].new_line, Some(372));
        assert_eq!(reads.load(Ordering::Relaxed), 2);
        Ok(())
    }

    #[test]
    fn full_file_reads_one_side_and_marks_changes() -> Result<(), String> {
        let reads = Arc::new(AtomicU64::new(0));
        let count = reads.clone();
        let worker = Worker::with_readers(
            eframe::egui::Context::default(),
            |_, _| {
                let mut doc = FileDiff::summary(DiffKind::Text, 0, 8, "");
                doc.rows = vec![
                    DiffRow {
                        kind: RowKind::Hunk,
                        old_line: None,
                        new_line: None,
                        text: "@@".into(),
                        bytes: vec![],
                    },
                    DiffRow {
                        kind: RowKind::Added,
                        old_line: None,
                        new_line: Some(2),
                        text: "new".into(),
                        bytes: vec![],
                    },
                ];
                Ok(doc)
            },
            move |_, side, changed, _| {
                count.fetch_add(1, Ordering::Relaxed);
                assert_eq!(side, ReadingSide::After);
                assert!(changed.contains(&2));
                Ok(FileText::from_bytes(side, b"one\ntwo\n", changed))
            },
        )?;
        let change = FileChange {
            path: "file".into(),
            path_bytes: b"file".to_vec(),
            kind: ChangeKind::Modified,
            old: None,
            new: None,
        };
        worker.request(
            1,
            "repo".into(),
            change.clone(),
            0..64,
            Mode::Full,
            ReadingSide::After,
            None,
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let update = loop {
            if let Some(update) = worker.poll() {
                break update;
            }
            if std::time::Instant::now() > deadline {
                return Err("worker timed out".into());
            }
            thread::sleep(std::time::Duration::from_millis(1));
        };
        let full = update.full.ok_or("missing full file")??;
        assert_eq!(full.kind, FileKind::Text);
        assert_eq!(full.lines.len(), 2);
        assert!(full.lines[1].changed);
        assert_eq!(update.hunks, vec![0]);
        worker.request(
            1,
            "repo".into(),
            change,
            0..64,
            Mode::Full,
            ReadingSide::After,
            None,
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if worker.poll().is_some() {
                break;
            }
            if std::time::Instant::now() > deadline {
                return Err("worker timed out".into());
            }
            thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(reads.load(Ordering::Relaxed), 1);
        Ok(())
    }
}
