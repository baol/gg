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

/// What the reader's find bar looks for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindQuery {
    pub text: String,
    /// Match letter case exactly.
    pub case: bool,
    /// Only whole words: no letter, digit or `_` on either side.
    pub word: bool,
}

/// One match: a document row (a diff row, or a line in the full file) and
/// the characters it covers in that row's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindMatch {
    pub row: usize,
    pub chars: Range<usize>,
}

/// At most this many matches are listed; the count then reads `n+`.
pub const FIND_LIMIT: usize = 10_000;

/// Every match of `query` in `rows` (row index, text), in order, at most
/// [`FIND_LIMIT`]. Case-insensitive matching compares lowercased characters
/// one for one, so a match's character range is the same in the original.
pub fn find_in<'a>(
    rows: impl Iterator<Item = (usize, &'a str)>,
    query: &FindQuery,
) -> Vec<FindMatch> {
    let needle: Vec<char> = query.text.chars().collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let fold = |ch: char| {
        if query.case {
            ch
        } else {
            ch.to_lowercase().next().unwrap_or(ch)
        }
    };
    let needle: Vec<char> = needle.into_iter().map(fold).collect();
    let word = |ch: Option<&char>| ch.is_some_and(|ch| ch.is_alphanumeric() || *ch == '_');
    let mut found = Vec::new();
    for (row, text) in rows {
        let hay: Vec<char> = text.chars().collect();
        let mut at = 0;
        while at + needle.len() <= hay.len() {
            let hit = hay[at..at + needle.len()]
                .iter()
                .zip(&needle)
                .all(|(a, b)| fold(*a) == *b);
            let bounded = !query.word
                || (!word(at.checked_sub(1).and_then(|before| hay.get(before)))
                    && !word(hay.get(at + needle.len())));
            if hit && bounded {
                found.push(FindMatch {
                    row,
                    chars: at..at + needle.len(),
                });
                if found.len() >= FIND_LIMIT {
                    return found;
                }
                at += needle.len();
            } else {
                at += 1;
            }
        }
    }
    found
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
    /// Context and whitespace; a change comes with a new generation.
    options: gg_engine::diff::Options,
    /// The find bar's query, searched over the whole document once.
    find: Option<FindQuery>,
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
    /// Side-by-side pairing of the whole document's rows.
    pub split: Arc<crate::split_rows::Layout>,
    /// Displayed characters of every document row, for wrapping unloaded rows.
    pub widths: Arc<Vec<u32>>,
    /// The find query's matches over the whole document, once searched.
    pub find: Option<(FindQuery, Arc<Vec<FindMatch>>)>,
    /// Changed words of this diff page's changed lines: document row and
    /// character ranges (`gg_engine::diff::word_emphasis`).
    pub emphasis: Vec<(usize, Vec<Range<usize>>)>,
}

pub struct Worker {
    input: Arc<Mutex<Option<Request>>>,
    output: Arc<Mutex<Option<Update>>>,
    generation: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    thread: thread::Thread,
    options: std::cell::Cell<gg_engine::diff::Options>,
    find: std::cell::RefCell<Option<FindQuery>>,
}

impl Worker {
    pub fn start(ctx: eframe::egui::Context) -> Result<Self, String> {
        Self::with_readers(
            ctx,
            |request, cancelled| {
                if let Some(working) = &request.working {
                    gg_git::working_diff::read_with(working, request.options, cancelled)
                } else {
                    gg_git::diff::read_with(
                        &request.path,
                        &request.change,
                        request.options,
                        cancelled,
                    )
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
                let mut split = Arc::new(crate::split_rows::Layout::default());
                let mut widths: Arc<Vec<u32>> = Arc::default();
                let mut preview = None;
                let mut full: Option<(u64, ReadingSide, Result<FileText, String>)> = None;
                // The last search: document, mode, side and query, and its matches.
                type Found = (u64, Mode, ReadingSide, FindQuery, Arc<Vec<FindMatch>>);
                let mut found: Option<Found> = None;
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
                        split = Arc::default();
                        widths = Arc::default();
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
                        widths = Arc::new(
                            result
                                .as_ref()
                                .map(|doc| {
                                    doc.rows
                                        .iter()
                                        .map(|r| {
                                            u32::try_from(crate::wrap_rows::reach(&r.text))
                                                .unwrap_or(u32::MAX)
                                        })
                                        .collect()
                                })
                                .unwrap_or_default(),
                        );
                        // Wide characters take two columns.
                        max_columns = result.as_ref().map_or(0, |doc| {
                            doc.rows
                                .iter()
                                .map(|r| crate::wrap_rows::columns(&r.text))
                                .max()
                                .unwrap_or(0)
                        });
                        if let Ok(doc) = &result {
                            hunks = doc
                                .rows
                                .iter()
                                .enumerate()
                                .filter(|(_, row)| row.kind == gg_engine::diff::RowKind::Hunk)
                                .map(|(index, _)| index)
                                .collect();
                            split = Arc::new(crate::split_rows::Layout::from_kinds(
                                doc.rows.iter().map(|row| row.kind),
                            ));
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
                                split: Arc::default(),
                                widths: Arc::default(),
                                find: None,
                                emphasis: Vec::new(),
                            });
                            ctx.request_repaint();
                        }
                        continue;
                    }
                    let doc = match document.as_ref() {
                        Some(Ok(doc)) => doc,
                        _ => continue,
                    };
                    let mut update = if request.mode == Mode::Full {
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
                            split: split.clone(),
                            widths: widths.clone(),
                            find: None,
                            emphasis: Vec::new(),
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
                        // Changed words, for this page only and bounded.
                        let emphasis = gg_engine::diff::word_emphasis(
                            &doc.rows,
                            start..start + page.rows.len(),
                            256,
                        );
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
                            split: split.clone(),
                            widths: widths.clone(),
                            find: None,
                            emphasis,
                        }
                    };
                    // The find query, searched once per document, mode and side.
                    if let Some(query) = request.find.clone().filter(|query| !query.text.is_empty())
                    {
                        let key = (loaded, request.mode, request.side);
                        let cached =
                            found
                                .as_ref()
                                .filter(|(generation, mode, side, previous, _)| {
                                    (*generation, *mode, *side) == key && *previous == query
                                });
                        let matches = match cached {
                            Some((_, _, _, _, matches)) => matches.clone(),
                            None => {
                                let matches = Arc::new(match request.mode {
                                    Mode::Full => match full.as_ref() {
                                        Some((_, _, Ok(text))) => find_in(
                                            text.lines
                                                .iter()
                                                .enumerate()
                                                .map(|(row, line)| (row, line.text.as_str())),
                                            &query,
                                        ),
                                        _ => Vec::new(),
                                    },
                                    Mode::Diff => find_in(
                                        doc.rows
                                            .iter()
                                            .enumerate()
                                            .filter(|(_, row)| {
                                                row.kind != gg_engine::diff::RowKind::Hunk
                                            })
                                            .map(|(index, row)| (index, row.text.as_str())),
                                        &query,
                                    ),
                                });
                                found = Some((key.0, key.1, key.2, query.clone(), matches.clone()));
                                matches
                            }
                        };
                        update.find = Some((query, matches));
                    }
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
            options: std::cell::Cell::default(),
            find: std::cell::RefCell::default(),
        })
    }

    /// Context and whitespace for the following requests. The reader starts
    /// a new generation whenever they change, so a document never mixes them.
    pub fn set_options(&self, options: gg_engine::diff::Options) {
        self.options.set(options);
    }

    /// The find bar's query for the following requests; `None` when closed.
    pub fn set_find(&self, find: Option<FindQuery>) {
        if let Ok(mut slot) = self.find.try_borrow_mut() {
            *slot = find;
        }
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
            old_path: None,
            old_path_bytes: None,
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
                options: self.options.get(),
                find: self.find.borrow().clone(),
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
    fn find_matches_case_whole_words_and_wide_characters() {
        let rows = [
            "let total = Total + total_x;",
            "TOTAL",
            "naïve Ünïcode total",
        ];
        let find = |text: &str, case: bool, word: bool| {
            find_in(
                rows.iter().enumerate().map(|(row, text)| (row, *text)),
                &FindQuery {
                    text: text.into(),
                    case,
                    word,
                },
            )
            .into_iter()
            .map(|found| (found.row, found.chars))
            .collect::<Vec<_>>()
        };
        assert_eq!(
            find("total", false, false),
            [(0, 4..9), (0, 12..17), (0, 20..25), (1, 0..5), (2, 14..19)]
        );
        assert_eq!(
            find("total", true, false),
            [(0, 4..9), (0, 20..25), (2, 14..19)]
        );
        assert_eq!(
            find("total", false, true),
            [(0, 4..9), (0, 12..17), (1, 0..5), (2, 14..19)],
            "total_x is not the whole word"
        );
        // Character ranges, not bytes, past wide characters.
        assert_eq!(find("ünï", false, false), [(2, 6..9)]);
        assert!(find("", false, false).is_empty());
        assert_eq!(
            find("aa", false, false),
            Vec::<(usize, Range<usize>)>::new()
        );
    }

    #[test]
    fn the_whole_document_is_searched_once_per_query() -> Result<(), String> {
        let worker = Worker::with_readers(
            eframe::egui::Context::default(),
            move |_, _| {
                let mut doc = FileDiff::summary(DiffKind::Text, 0, 2000, "");
                doc.rows = (0..1000)
                    .map(|n| DiffRow {
                        kind: if n % 100 == 0 {
                            RowKind::Hunk
                        } else {
                            RowKind::Added
                        },
                        old_line: None,
                        new_line: Some(n + 1),
                        text: format!("line {n} needle"),
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
            old_path: None,
            old_path_bytes: None,
        };
        worker.set_find(Some(FindQuery {
            text: "NEEDLE".into(),
            ..Default::default()
        }));
        worker.request(
            1,
            "repo".into(),
            change,
            0..64,
            Mode::Diff,
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
        let (query, matches) = update.find.ok_or("no matches sent")?;
        assert_eq!(query.text, "NEEDLE");
        // Every page, not just the one sent; hunk rows are not searched.
        assert_eq!(matches.len(), 990);
        assert_eq!(matches[0].row, 1);
        assert_eq!(matches.last().map(|found| found.row), Some(999));
        Ok(())
    }

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
            old_path: None,
            old_path_bytes: None,
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
            old_path: None,
            old_path_bytes: None,
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
