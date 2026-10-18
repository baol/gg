//! Browser transitions independent of rendering and worker timing.
use gg_engine::{Commit, History};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Default)]
pub struct State {
    pub history: Option<History>,
    pub rows: BTreeMap<usize, Commit>,
    pub count: usize,
    pub viewport: std::ops::Range<usize>,
    pub scanned: usize,
    pub finished: bool,
    pub selected_row: Option<usize>,
    pub details: Option<Commit>,
    pub query: String,
    pub filter_revision: u64,
    pub selected: Option<String>,
    pub error: Option<String>,
    pub requested: Option<PathBuf>,
    pub generation: u64,
}

impl State {
    pub fn begin(&mut self, path: PathBuf) -> u64 {
        self.generation = self.generation.saturating_add(1);
        self.requested = Some(path);
        self.error = None;
        self.generation
    }

    #[cfg(test)]
    pub fn complete(&mut self, generation: u64, result: Result<History, String>) {
        if generation != self.generation {
            return;
        }
        match result {
            Ok(history) => {
                let same_repo = self
                    .history
                    .as_ref()
                    .is_some_and(|old| old.path == history.path);
                if !same_repo {
                    self.query.clear();
                    self.selected = None;
                }
                self.rows = history.commits.iter().cloned().enumerate().collect();
                self.count = self.rows.len();
                self.scanned = self.count;
                self.finished = true;
                self.history = Some(history);
                self.reconcile_selection();
                self.requested = None;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub fn is_current(&self, update: &crate::stream::Update) -> bool {
        update.query == self.query && update.request.revision == self.filter_revision
    }

    pub fn accept(
        &mut self,
        update: crate::stream::Update,
        replace: bool,
        safe: bool,
        pinned: Option<usize>,
    ) {
        if !replace && !self.is_current(&update) {
            return;
        }
        let selection_current = replace
            || (update.request.selected_id == self.selected
                && update.request.selected_row == self.selected_row);
        let center = update
            .request
            .range
            .start
            .saturating_add(update.request.range.len() / 2);
        if replace {
            self.rows.clear();
            self.history = Some(update.identity);
            self.query = update.query;
            self.filter_revision = update.request.revision;
            self.count = 0;
            self.selected_row = None;
            self.details = None;
            self.requested = None;
        }
        if safe || replace {
            self.count = update.count;
            self.scanned = update.scanned;
            self.finished = update.complete;
            if self.requested.is_none() {
                self.error = update.error.clone();
            }
        }
        if safe || replace || self.details.is_none() {
            if selection_current && let Some((row, commit)) = update.selection {
                self.selected = Some(commit.id.clone());
                self.selected_row = Some(row);
                self.details = Some(commit);
            } else if update.complete && update.count == 0 {
                self.selected = None;
                self.selected_row = None;
                self.details = None;
            }
        }
        for (row, commit) in update.rows {
            if pinned != Some(row) || self.rows.contains_key(&row) {
                self.rows.entry(row).or_insert(commit);
            }
        }
        while self.rows.len() > crate::stream::CACHE_ROWS {
            let victim = self
                .rows
                .keys()
                .copied()
                .filter(|r| Some(*r) != pinned && !self.viewport.contains(r))
                .max_by_key(|r| r.abs_diff(center));
            if let Some(row) = victim {
                self.rows.remove(&row);
            } else {
                break;
            }
        }
    }

    pub fn start_filter(&mut self) {
        self.filter_revision = self.filter_revision.saturating_add(1);
        self.rows.clear();
        self.count = 0;
        self.scanned = 0;
        self.finished = false;
        self.selected_row = None;
        if !self
            .details
            .as_ref()
            .is_some_and(|c| c.matches(&self.query))
        {
            self.selected = None;
            self.details = None;
        }
    }

    pub fn displayed_details(&self) -> Option<&Commit> {
        self.details.as_ref().or_else(|| {
            self.selected_row
                .and_then(|row| self.rows.get(&row))
                .filter(|commit| Some(&commit.id) == self.selected.as_ref())
        })
    }

    pub fn select_row(&mut self, row: usize) {
        self.selected_row = Some(row);
        if let Some(commit) = self.rows.get(&row) {
            self.selected = Some(commit.id.clone());
            // A matching full detail remains until its replacement arrives.
            if self.details.as_ref().is_none_or(|d| d.id != commit.id) {
                self.details = None;
            }
        } else {
            self.selected = None;
            self.details = None;
        }
    }

    pub fn reconcile_selection(&mut self) {
        let mut matches = self
            .history
            .iter()
            .flat_map(|h| &h.commits)
            .filter(|c| c.matches(&self.query));
        let first = matches.next();
        let selected_visible = first.is_some_and(|c| Some(&c.id) == self.selected.as_ref())
            || matches.any(|c| Some(&c.id) == self.selected.as_ref());
        if !selected_visible {
            self.selected = first.map(|c| c.id.clone());
        }
        self.selected_row = self
            .rows
            .iter()
            .find(|(_, c)| Some(&c.id) == self.selected.as_ref())
            .map(|(i, _)| *i);
        self.details = self.selected_row.and_then(|i| self.rows.get(&i)).cloned();
    }

    pub fn move_selection(&mut self, direction: i32) {
        let ids: Vec<_> = self
            .history
            .iter()
            .flat_map(|h| &h.commits)
            .filter(|c| c.matches(&self.query))
            .map(|c| &c.id)
            .collect();
        if ids.is_empty() {
            self.selected = None;
            return;
        }
        let current = ids
            .iter()
            .position(|id| Some(*id) == self.selected.as_ref())
            .unwrap_or(0);
        let next = current
            .saturating_add_signed(direction as isize)
            .min(ids.len().saturating_sub(1));
        self.selected = ids.get(next).map(|id| (*id).clone());
        self.selected_row = self
            .rows
            .iter()
            .find(|(_, c)| Some(&c.id) == self.selected.as_ref())
            .map(|(i, _)| *i);
        self.details = self.selected_row.and_then(|i| self.rows.get(&i)).cloned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn history(path: &str, ids: &[&str]) -> History {
        History {
            path: path.into(),
            head: "main".into(),
            truncated: false,
            shallow: false,
            commits: ids
                .iter()
                .map(|id| gg_engine::Commit {
                    id: (*id).into(),
                    parents: vec![],
                    author: "Ada".into(),
                    email: String::new(),
                    time: String::new(),
                    message: (*id).into(),
                })
                .collect(),
        }
    }
    fn loaded() -> State {
        let mut state = State::default();
        let generation = state.begin("repo".into());
        state.complete(generation, Ok(history("repo", &["one", "two", "three"])));
        state
    }
    #[test]
    fn filtering_preserves_visible_selection_and_clears_empty_details() {
        let mut s = loaded();
        s.selected = Some("two".into());
        s.query = "t".into();
        s.reconcile_selection();
        assert_eq!(s.selected.as_deref(), Some("two"));
        s.query = "three".into();
        s.reconcile_selection();
        assert_eq!(s.selected.as_deref(), Some("three"));
        s.query = "missing".into();
        s.reconcile_selection();
        assert!(s.selected.is_none());
        s.query.clear();
        s.reconcile_selection();
        assert_eq!(s.selected.as_deref(), Some("one"));
    }
    #[test]
    fn refresh_failure_retry_and_stale_generation() {
        let mut s = loaded();
        s.selected = Some("two".into());
        let old = s.generation;
        let current = s.begin("repo".into());
        s.complete(old, Ok(history("wrong", &["stale"])));
        assert_eq!(
            s.history.as_ref().map(|h| h.path.as_path()),
            Some(std::path::Path::new("repo"))
        );
        s.complete(current, Err("Read failed".into()));
        assert_eq!(s.selected.as_deref(), Some("two"));
        assert!(s.error.is_some());
        assert!(s.requested.is_some());
        let retry = s.begin("repo".into());
        s.complete(retry, Ok(history("repo", &["new", "two"])));
        assert_eq!(s.selected.as_deref(), Some("two"));
        assert!(s.error.is_none());
        assert!(s.requested.is_none());
        let next = s.begin("repo".into());
        s.complete(next, Ok(history("repo", &["new"])));
        assert_eq!(s.selected.as_deref(), Some("new"));
    }
    #[test]
    fn switching_repository_resets_filter_and_selection() {
        let mut s = loaded();
        s.query = "one".into();
        let generation = s.begin("other".into());
        s.complete(generation, Ok(history("other", &["other-id"])));
        assert!(s.query.is_empty());
        assert_eq!(s.selected.as_deref(), Some("other-id"));
    }
    #[test]
    fn keyboard_moves_through_all_matches_and_stops_at_edges() {
        let mut s = loaded();
        s.move_selection(-1);
        assert_eq!(s.selected.as_deref(), Some("one"));
        s.move_selection(1);
        assert_eq!(s.selected.as_deref(), Some("two"));
        s.move_selection(1);
        s.move_selection(1);
        assert_eq!(s.selected.as_deref(), Some("three"));
        s.query = "t".into();
        s.reconcile_selection();
        s.move_selection(-1);
        assert_eq!(s.selected.as_deref(), Some("two"));
    }
}

#[cfg(test)]
mod streaming_tests {
    use super::*;
    fn update(start: usize, count: usize) -> crate::stream::Update {
        let rows: Vec<_> = (start..(start + 64).min(count))
            .map(|i| {
                (
                    i,
                    Commit {
                        id: format!("{i:040x}"),
                        parents: vec![],
                        author: "Ada".into(),
                        email: String::new(),
                        time: String::new(),
                        message: format!("Commit {i}"),
                    },
                )
            })
            .collect();
        crate::stream::Update {
            request: crate::stream::Request {
                range: start..start + 64,
                ..Default::default()
            },
            anchor_row: None,
            identity: History {
                path: "repo".into(),
                head: "main".into(),
                commits: vec![],
                truncated: false,
                shallow: false,
            },
            query: String::new(),
            count,
            scanned: count,
            complete: true,
            selection: rows.first().cloned(),
            rows,
            error: None,
        }
    }
    #[test]
    fn hover_pins_placeholder_and_extent_while_other_rows_arrive() {
        let mut state = State::default();
        state.accept(update(0, 32), true, true, None);
        state.rows.remove(&10);
        state.rows.remove(&11);
        state.accept(update(0, 1000), false, false, Some(10));
        assert_eq!(state.count, 32);
        assert!(!state.rows.contains_key(&10));
        assert!(state.rows.contains_key(&11));
        assert_eq!(state.selected_row, Some(0));
        state.accept(update(0, 1000), false, true, None);
        assert_eq!(state.count, 1000);
        assert!(state.rows.contains_key(&10));
    }
    #[test]
    fn pages_evict_and_reload_without_losing_identity_or_selection() {
        let mut state = State::default();
        state.accept(update(0, 100_000), true, true, None);
        for start in (64..2048).step_by(64) {
            state.accept(update(start, 100_000), false, true, None);
            assert!(state.rows.len() <= crate::stream::CACHE_ROWS);
            assert!(state.rows.contains_key(&start));
            assert_eq!(state.selected_row, Some(0));
        }
        assert!(!state.rows.contains_key(&0));
        state.accept(update(0, 100_000), false, true, None);
        assert_eq!(
            state.rows.get(&0).map(|c| c.id.as_str()),
            Some("0000000000000000000000000000000000000000")
        );
    }
    #[test]
    fn late_query_and_selection_results_do_not_replace_new_intent() {
        let mut state = State::default();
        state.accept(update(0, 100), true, true, None);
        state.select_row(10);
        state.accept(update(0, 100), false, true, None);
        assert_eq!(state.selected_row, Some(10));
        state.query = "new filter".into();
        state.start_filter();
        state.accept(update(0, 100), false, true, None);
        assert_eq!(state.count, 0);
        assert!(state.rows.is_empty());
    }
}
