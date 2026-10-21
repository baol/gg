//! Native Git object access through gitoxide.
use gg_engine::{Commit, History, HistoryRequest, HistorySource};
use std::collections::{BinaryHeap, HashSet};

pub struct Git;

impl HistorySource for Git {
    fn history(&self, request: &HistoryRequest) -> Result<History, String> {
        let mut session = Session::open(request)?;
        let mut history = session.identity.clone();
        while let Some(commit) = session.next_commit()? {
            history.commits.push(commit);
        }
        Ok(history)
    }
}

/// A pinned HEAD and deterministic, resumable discovery frontier.
pub struct Session {
    repo: gix::Repository,
    pub identity: History,
    frontier: BinaryHeap<(i64, gix::ObjectId)>,
    seen: HashSet<gix::ObjectId>,
    shallow: HashSet<gix::ObjectId>,
}

impl Session {
    pub fn open(request: &HistoryRequest) -> Result<Self, String> {
        Self::open_inner(request)
            .map_err(|e| format!("Could not read {}: {e}", request.path.display()))
    }

    fn open_inner(request: &HistoryRequest) -> Result<Self, Box<dyn std::error::Error>> {
        let repo = gix::discover(&request.path)?;
        let head = repo.head()?;
        let label = head
            .referent_name()
            .map(|name| name.shorten().to_string())
            .unwrap_or_else(|| "Detached HEAD".into());
        let identity = History {
            path: repo.workdir().unwrap_or(repo.path()).to_path_buf(),
            head: label,
            commits: Vec::new(),
            truncated: false,
            shallow: repo.is_shallow()?,
        };
        let id = if head.is_unborn() {
            None
        } else {
            Some(repo.head_commit()?.id)
        };
        let shallow = repo
            .shallow_commits()?
            .map(|ids| ids.iter().copied().collect())
            .unwrap_or_default();
        let mut session = Self {
            repo,
            identity,
            frontier: BinaryHeap::new(),
            seen: HashSet::new(),
            shallow,
        };
        if let Some(id) = id {
            session.enqueue(id)?;
        }
        Ok(session)
    }

    fn enqueue(&mut self, id: gix::ObjectId) -> Result<(), Box<dyn std::error::Error>> {
        if self.seen.insert(id) {
            let time = self.repo.find_commit(id)?.committer()?.time()?.seconds;
            self.frontier.push((time, id));
        }
        Ok(())
    }

    pub fn next_commit(&mut self) -> Result<Option<Commit>, String> {
        self.next_inner()
            .map_err(|e| format!("Could not read {}: {e}", self.identity.path.display()))
    }

    fn next_inner(&mut self) -> Result<Option<Commit>, Box<dyn std::error::Error>> {
        let Some((_, id)) = self.frontier.pop() else {
            self.seen.clear();
            self.seen.shrink_to_fit();
            return Ok(None);
        };
        let object = self.repo.find_commit(id)?;
        let decoded = object.decode()?;
        let author = decoded.author()?;
        let parents: Vec<_> = decoded.parents().collect();
        let commit = Commit {
            id: id.to_string(),
            parents: parents.iter().map(ToString::to_string).collect(),
            author: author.name.to_string(),
            email: author.email.to_string(),
            time: author.time()?.format(gix::date::time::format::ISO8601)?,
            message: decoded.message.to_string(),
        };
        drop(decoded);
        drop(object);
        if !self.shallow.contains(&id) {
            for parent in parents {
                self.enqueue(parent)?;
            }
        }
        Ok(Some(commit))
    }
}

impl gg_engine::HistoryReader for Session {
    fn identity(&self) -> &History {
        &self.identity
    }
    fn next_commit(&mut self) -> Result<Option<Commit>, String> {
        Session::next_commit(self)
    }
}

pub mod blob;
pub mod diff;
pub mod review;
pub mod status;
pub mod working_diff;

pub mod unstage;

pub mod stage;
