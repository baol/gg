//! Build a bounded newest-to-oldest file timeline from content states.
//!
//! A version is a content state, not every commit. Commits that carry the
//! same path, blob, and mode are skipped. A unique predecessor is followed
//! silently; genuinely different predecessors are offered as a choice.
use gg_engine::{
    file_history::{
        FileVersion, HistoryKind, MAX_VERSIONS, Predecessor, Timeline, VersionCandidate,
        display_version_path, edge_kind,
    },
    review::FileSide,
};
use gix::bstr::ByteSlice;
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Bound for one predecessor search. Reaching it is an explicit boundary.
pub const TRACE_LIMIT: usize = 20_000;

#[derive(Clone, Debug, PartialEq, Eq)]
struct State {
    path: Vec<u8>,
    side: Option<FileSide>,
}

impl State {
    fn key(&self) -> Option<(String, u16)> {
        self.side.as_ref().map(|side| (side.id.clone(), side.mode))
    }

    fn dedupe(&self) -> Option<(String, u16)> {
        self.key()
    }
}

#[derive(Clone, Debug)]
struct Boundary {
    commit: gix::ObjectId,
    state: State,
}

struct Trace {
    introducer: Option<gix::ObjectId>,
    boundaries: Vec<Boundary>,
    limited: Option<String>,
}

struct Meta {
    subject: String,
    author: String,
    time: i64,
}

type BoundaryKey = (gix::ObjectId, Vec<u8>, Option<(String, u16)>);

/// A persistent, append-only timeline builder for one file and revision.
pub struct Session {
    repo: gix::Repository,
    repo_path: PathBuf,
    versions: Vec<FileVersion>,
    pending: Option<Boundary>,
    complete: bool,
    note: Option<String>,
}

impl Session {
    pub fn open(
        path: &Path,
        revision: &str,
        path_bytes: &[u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self, String> {
        let repo = gix::discover(path)
            .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
        let id = {
            let object = repo
                .rev_parse_single(revision)
                .map_err(|error| format!("Could not resolve {revision}: {error}"))?
                .object()
                .map_err(|error| format!("Could not read {revision}: {error}"))?
                .peel_to_kind(gix::object::Kind::Commit)
                .map_err(|error| format!("{revision} is not a commit: {error}"))?;
            object
                .try_into_commit()
                .map_err(|error| format!("Could not read commit {revision}: {error}"))?
                .id
        };
        let repo_path = repo.workdir().unwrap_or_else(|| repo.path()).to_path_buf();
        let mut session = Self {
            repo,
            repo_path,
            versions: vec![],
            pending: None,
            complete: false,
            note: None,
        };
        let side = session.state_at(id, path_bytes)?;
        let state = State {
            path: path_bytes.to_vec(),
            side,
        };
        let (version, pending) = session.make_version(id, state, cancelled, TRACE_LIMIT)?;
        session.versions.push(version);
        session.pending = pending;
        session.complete = session.pending.is_none();
        Ok(session)
    }

    pub fn versions(&self) -> &[FileVersion] {
        &self.versions
    }

    pub fn complete(&self) -> bool {
        self.complete
    }

    pub fn timeline(&self) -> Timeline {
        Timeline::new(self.versions.clone(), self.complete, self.note.clone())
    }

    /// Build the next older version. Returns false when nothing remains.
    pub fn next(&mut self, cancelled: &dyn Fn() -> bool) -> Result<bool, String> {
        if self.complete {
            return Ok(false);
        }
        let Some(boundary) = self.pending.clone() else {
            self.complete = true;
            return Ok(false);
        };
        if self.versions.len() >= MAX_VERSIONS {
            if let Some(last) = self.versions.last_mut() {
                last.previous = Predecessor::Limited {
                    reason: format!("History limit: {MAX_VERSIONS} versions"),
                };
            }
            self.complete = true;
            return Ok(false);
        }
        // Keep the pending boundary on failure so a retry can resume it.
        let (version, pending) =
            self.make_version(boundary.commit, boundary.state, cancelled, TRACE_LIMIT)?;
        if let Some(last) = self.versions.last_mut() {
            last.previous = Predecessor::Linked;
        }
        self.pending = pending;
        self.versions.push(version);
        self.complete = self.pending.is_none();
        Ok(true)
    }

    /// Resolve a predecessor choice for the oldest loaded version.
    pub fn choose(&mut self, index: usize) -> Result<bool, String> {
        let Some(last) = self.versions.last_mut() else {
            return Err("There is no version to choose a predecessor for.".into());
        };
        let Predecessor::Candidates { choices } = &last.previous else {
            return Err("No predecessor choice is pending.".into());
        };
        let Some(choice) = choices.get(index).cloned() else {
            return Err("The predecessor choice is outside the offered range.".into());
        };
        let commit = gix::ObjectId::from_hex(choice.commit.as_bytes())
            .map_err(|error| format!("Invalid predecessor commit: {error}"))?;
        last.kind = Some(choice.kind);
        last.old_path_bytes = choice.old_path_bytes.clone();
        last.old_path = choice.old_path.clone();
        last.previous = Predecessor::More;
        self.pending = Some(Boundary {
            commit,
            state: State {
                path: choice.path_bytes,
                side: choice.side,
            },
        });
        self.complete = false;
        Ok(true)
    }

    fn make_version(
        &self,
        anchor: gix::ObjectId,
        state: State,
        cancelled: &dyn Fn() -> bool,
        limit: usize,
    ) -> Result<(FileVersion, Option<Boundary>), String> {
        let trace = self.trace(anchor, state.clone(), cancelled, limit)?;
        let commit = trace.introducer.unwrap_or(anchor);
        let meta = self.meta(commit)?;
        let (previous, pending, kind, old_path_bytes) = if let Some(reason) = trace.limited {
            (Predecessor::Limited { reason }, None, None, None)
        } else if trace.boundaries.is_empty() {
            let kind = state.side.as_ref().map(|_| HistoryKind::Added);
            (Predecessor::Oldest, None, kind, None)
        } else if trace.boundaries.len() == 1 {
            let boundary = &trace.boundaries[0];
            let kind = edge_kind(
                boundary.state.side.as_ref(),
                state.side.as_ref(),
                &boundary.state.path,
                &state.path,
            );
            let old_path = (boundary.state.path != state.path).then(|| boundary.state.path.clone());
            (
                Predecessor::More,
                Some(boundary.clone()),
                Some(kind),
                old_path,
            )
        } else {
            let mut choices = Vec::with_capacity(trace.boundaries.len());
            for boundary in &trace.boundaries {
                let candidate_meta = self.meta(boundary.commit)?;
                let changed = boundary.state.path != state.path;
                choices.push(VersionCandidate {
                    commit: boundary.commit.to_string(),
                    path_bytes: boundary.state.path.clone(),
                    path: display_version_path(&boundary.state.path),
                    side: boundary.state.side.clone(),
                    subject: candidate_meta.subject,
                    author: candidate_meta.author,
                    time: candidate_meta.time,
                    kind: edge_kind(
                        boundary.state.side.as_ref(),
                        state.side.as_ref(),
                        &boundary.state.path,
                        &state.path,
                    ),
                    old_path_bytes: changed.then(|| boundary.state.path.clone()),
                    old_path: changed.then(|| display_version_path(&boundary.state.path)),
                });
            }
            // Newest predecessor first, with the commit ID as a stable tie-break.
            choices.sort_by(|a, b| b.time.cmp(&a.time).then_with(|| b.commit.cmp(&a.commit)));
            (Predecessor::Candidates { choices }, None, None, None)
        };
        Ok((
            FileVersion {
                path_bytes: state.path.clone(),
                path: display_version_path(&state.path),
                side: state.side,
                commit: commit.to_string(),
                subject: meta.subject,
                author: meta.author,
                time: meta.time,
                kind,
                old_path_bytes: old_path_bytes.clone(),
                old_path: old_path_bytes.as_deref().map(display_version_path),
                previous,
            },
            pending,
        ))
    }

    /// Follow carriers of `state` backwards until content differs.
    fn trace(
        &self,
        anchor: gix::ObjectId,
        state: State,
        cancelled: &dyn Fn() -> bool,
        limit: usize,
    ) -> Result<Trace, String> {
        let mut frontier = VecDeque::new();
        let mut seen = HashSet::new();
        let mut introducers = Vec::new();
        let mut boundaries: BTreeMap<BoundaryKey, Boundary> = BTreeMap::new();
        let mut visited = 0usize;
        seen.insert((anchor, state.path.clone(), state.dedupe()));
        frontier.push_back((anchor, state));
        while let Some((commit, current)) = frontier.pop_front() {
            if cancelled() {
                return Err("Operation cancelled".into());
            }
            if visited >= limit {
                return Ok(Trace {
                    introducer: self.newest(introducers)?.or(Some(anchor)),
                    boundaries: vec![],
                    limited: Some(format!("History search stopped after {limit} commits")),
                });
            }
            visited += 1;
            let parents = self.parents(commit)?;
            let mut carriers = Vec::new();
            let mut differing = Vec::new();
            for parent in parents {
                if cancelled() {
                    return Err("Operation cancelled".into());
                }
                for parent_state in
                    self.parent_states(commit, parent, &current.path, current.side.as_ref())?
                {
                    if parent_state == current {
                        carriers.push((parent, parent_state));
                    } else {
                        differing.push((parent, parent_state));
                    }
                }
            }
            if carriers.is_empty() {
                introducers.push(commit);
                for (parent, parent_state) in differing {
                    boundaries.insert(
                        (parent, parent_state.path.clone(), parent_state.dedupe()),
                        Boundary {
                            commit: parent,
                            state: parent_state,
                        },
                    );
                }
            } else {
                for (parent, parent_state) in carriers {
                    let key = (parent, parent_state.path.clone(), parent_state.dedupe());
                    if seen.insert(key) {
                        frontier.push_back((parent, parent_state));
                    }
                }
            }
        }
        let introducer = self.newest(introducers)?;
        Ok(Trace {
            introducer,
            boundaries: boundaries.into_values().collect(),
            limited: None,
        })
    }

    fn newest(&self, ids: Vec<gix::ObjectId>) -> Result<Option<gix::ObjectId>, String> {
        let mut newest: Option<(i64, gix::ObjectId)> = None;
        for id in ids {
            let meta = self.meta(id)?;
            if newest.is_none_or(|(time, current)| (meta.time, id) > (time, current)) {
                newest = Some((meta.time, id));
            }
        }
        Ok(newest.map(|(_, id)| id))
    }

    fn parents(&self, id: gix::ObjectId) -> Result<Vec<gix::ObjectId>, String> {
        self.repo
            .find_commit(id)
            .map_err(|error| format!("Could not read commit {id}: {error}"))
            .map(|commit| commit.parent_ids().map(|id| id.detach()).collect())
    }

    fn state_at(&self, commit: gix::ObjectId, path: &[u8]) -> Result<Option<FileSide>, String> {
        if path.is_empty() {
            return Ok(None);
        }
        let tree = self
            .repo
            .find_commit(commit)
            .map_err(|error| format!("Could not read commit {commit}: {error}"))?
            .tree()
            .map_err(|error| format!("Could not read tree of {commit}: {error}"))?;
        let components: Vec<&[u8]> = path
            .split(|byte| *byte == b'/')
            .filter(|component| !component.is_empty())
            .collect();
        let entry = tree.lookup_entry(components).map_err(|error| {
            format!("Could not look up {}: {error}", display_version_path(path))
        })?;
        Ok(entry
            .filter(|entry| !entry.mode().is_tree())
            .map(|entry| FileSide {
                id: entry.object_id().to_string(),
                mode: entry.mode().value(),
            }))
    }

    /// Resolve the path at `parent`, following a rename when the child path is
    /// absent there. Multiple rename sources are returned as distinct states.
    fn parent_states(
        &self,
        child: gix::ObjectId,
        parent: gix::ObjectId,
        path: &[u8],
        side: Option<&FileSide>,
    ) -> Result<Vec<State>, String> {
        if let Some(found) = self.state_at(parent, path)? {
            return Ok(vec![State {
                path: path.to_vec(),
                side: Some(found),
            }]);
        }
        if side.is_none() {
            return Ok(vec![State {
                path: path.to_vec(),
                side: None,
            }]);
        }
        let sources = self.rename_sources(parent, child, path)?;
        if sources.is_empty() {
            return Ok(vec![State {
                path: path.to_vec(),
                side: None,
            }]);
        }
        let mut states = Vec::with_capacity(sources.len());
        for source in sources {
            if let Some(found) = self.state_at(parent, &source)? {
                states.push(State {
                    path: source,
                    side: Some(found),
                });
            }
        }
        if states.is_empty() {
            Ok(vec![State {
                path: path.to_vec(),
                side: None,
            }])
        } else {
            Ok(states)
        }
    }

    fn rename_sources(
        &self,
        parent: gix::ObjectId,
        child: gix::ObjectId,
        destination: &[u8],
    ) -> Result<Vec<Vec<u8>>, String> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args([
                "diff-tree",
                "--no-commit-id",
                "-r",
                "-M",
                "-z",
                "--name-status",
            ])
            .arg(parent.to_string())
            .arg(child.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .map_err(|error| {
                format!("Could not inspect renames with Git: {error}. Install Git and retry.")
            })?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(format!(
                "Rename inspection failed ({}).{}",
                output.status,
                if detail.is_empty() {
                    String::new()
                } else {
                    format!("\n{detail}")
                }
            ));
        }
        let mut fields = output.stdout.split(|byte| *byte == 0);
        let mut sources = Vec::new();
        while let Some(status) = fields.next() {
            if status.is_empty() {
                continue;
            }
            let status = String::from_utf8_lossy(status);
            if status.starts_with('R') || status.starts_with('C') {
                let source = fields.next().unwrap_or_default();
                let target = fields.next().unwrap_or_default();
                if target == destination && !source.is_empty() {
                    sources.push(source.to_vec());
                }
            } else {
                let _path = fields.next();
            }
        }
        sources.sort();
        sources.dedup();
        Ok(sources)
    }

    fn meta(&self, id: gix::ObjectId) -> Result<Meta, String> {
        let commit = self
            .repo
            .find_commit(id)
            .map_err(|error| format!("Could not read commit {id}: {error}"))?;
        let decoded = commit
            .decode()
            .map_err(|error| format!("Could not decode commit {id}: {error}"))?;
        let author = decoded
            .author()
            .map_err(|error| format!("Could not read the author of {id}: {error}"))?;
        let time = author
            .time()
            .map_err(|error| format!("Could not read the time of {id}: {error}"))?
            .seconds;
        Ok(Meta {
            subject: decoded
                .message
                .lines()
                .next()
                .map(|line| line.to_str_lossy().into_owned())
                .unwrap_or_else(|| "(empty message)".into()),
            author: author.name.to_string(),
            time,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gg_engine::file_history::HistoryKind;

    #[test]
    fn state_keys_distinguish_absent_and_content() {
        let absent = State {
            path: b"a".to_vec(),
            side: None,
        };
        assert_eq!(absent.key(), None);
        let present = State {
            path: b"a".to_vec(),
            side: Some(FileSide {
                id: "b".into(),
                mode: 0o100644,
            }),
        };
        assert_eq!(present.key(), Some(("b".into(), 0o100644)));
        assert_ne!(absent, present);
    }

    #[test]
    fn history_kind_labels_are_stable() {
        assert_eq!(HistoryKind::Renamed.label(), "Renamed");
        assert_eq!(HistoryKind::TypeChanged.label(), "Type changed");
    }
}
