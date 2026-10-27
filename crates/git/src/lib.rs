//! Native Git object access through gitoxide.
use gg_engine::{Commit, History, HistoryRequest, HistorySource};
use std::{
    collections::{BinaryHeap, HashSet},
    io::{BufRead, BufReader, Read, Write},
    process::{Child, ChildStderr, ChildStdout, Command, Stdio},
};

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
    walk: Walk,
}

enum Walk {
    Frontier {
        frontier: BinaryHeap<(i64, gix::ObjectId)>,
        seen: HashSet<gix::ObjectId>,
        shallow: HashSet<gix::ObjectId>,
    },
    Topo(Box<dyn Iterator<Item = Result<gix::ObjectId, String>>>),
    Git(GitWalk),
}

struct GitWalk {
    child: Child,
    reader: BufReader<ChildStdout>,
    stderr: Option<ChildStderr>,
}

impl GitWalk {
    fn open(path: &std::path::Path, tips: &[gix::ObjectId]) -> Result<Self, String> {
        let mut child = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["rev-list", "--topo-order", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .spawn()
            .map_err(|error| format!("Could not start Git history traversal: {error}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            for tip in tips {
                writeln!(stdin, "{tip}").map_err(|error| error.to_string())?;
            }
        }
        let stdout = child
            .stdout
            .take()
            .ok_or("Git history traversal has no stdout")?;
        let stderr = child.stderr.take();
        Ok(Self {
            child,
            reader: BufReader::new(stdout),
            stderr,
        })
    }
}

impl Drop for GitWalk {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Session {
    pub fn open(request: &HistoryRequest) -> Result<Self, String> {
        Self::open_inner(request, None)
            .map_err(|e| format!("Could not read {}: {e}", request.path.display()))
    }

    /// Open with an already-read refs snapshot, avoiding a second ref scan.
    pub fn open_with_refs(
        request: &HistoryRequest,
        refs: gg_engine::refs::Snapshot,
    ) -> Result<Self, String> {
        Self::open_inner(request, Some(refs))
            .map_err(|e| format!("Could not read {}: {e}", request.path.display()))
    }

    fn open_inner(
        request: &HistoryRequest,
        refs: Option<gg_engine::refs::Snapshot>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
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
        let refs = match refs {
            Some(refs) => refs,
            None => crate::refs::read(&request.path)?,
        };
        let shallow: HashSet<_> = repo
            .shallow_commits()?
            .map(|ids| ids.iter().copied().collect())
            .unwrap_or_default();
        let mut tips = Vec::new();
        let mut tip_set = HashSet::new();
        if let Some(id) = id {
            tips.push(id);
            tip_set.insert(id);
        }
        for item in refs.items {
            let id = gix::ObjectId::from_hex(item.commit.as_bytes())?;
            if tip_set.insert(id) {
                tips.push(id);
            }
        }
        let walk = if tips.len() > 512 && shallow.is_empty() {
            Walk::Git(GitWalk::open(&request.path, &tips)?)
        } else if tips.len() > 1 && shallow.is_empty() {
            let walk = gix::traverse::commit::topo::Builder::new(repo.objects.clone())
                .with_tips(tips)
                .sorting(gix::traverse::commit::topo::Sorting::TopoOrder)
                .build()?
                .map(|info| info.map(|info| info.id).map_err(|error| error.to_string()));
            Walk::Topo(Box::new(walk))
        } else {
            let mut frontier = BinaryHeap::new();
            let mut seen = HashSet::new();
            for id in tips {
                if seen.insert(id) {
                    let time = repo.find_commit(id)?.committer()?.time()?.seconds;
                    frontier.push((time, id));
                }
            }
            Walk::Frontier {
                frontier,
                seen,
                shallow,
            }
        };
        Ok(Self {
            repo,
            identity,
            walk,
        })
    }

    pub fn next_commit(&mut self) -> Result<Option<Commit>, String> {
        self.next_inner()
            .map_err(|e| format!("Could not read {}: {e}", self.identity.path.display()))
    }

    fn next_inner(&mut self) -> Result<Option<Commit>, Box<dyn std::error::Error>> {
        let id = match &mut self.walk {
            Walk::Topo(walk) => match walk.next() {
                Some(Ok(id)) => id,
                Some(Err(error)) => return Err(std::io::Error::other(error).into()),
                None => return Ok(None),
            },
            Walk::Git(walk) => {
                let mut line = String::new();
                if walk.reader.read_line(&mut line)? == 0 {
                    let status = walk.child.wait()?;
                    if !status.success() {
                        let mut detail = String::new();
                        if let Some(mut stderr) = walk.stderr.take() {
                            stderr.read_to_string(&mut detail)?;
                        }
                        return Err(std::io::Error::other(format!(
                            "Git history traversal failed: {}",
                            detail.trim()
                        ))
                        .into());
                    }
                    return Ok(None);
                }
                gix::ObjectId::from_hex(line.trim().as_bytes())?
            }
            Walk::Frontier { frontier, seen, .. } => {
                let Some((_, id)) = frontier.pop() else {
                    seen.clear();
                    seen.shrink_to_fit();
                    return Ok(None);
                };
                id
            }
        };
        let (commit, parents) = self.read_commit(id)?;
        if let Walk::Frontier {
            frontier,
            seen,
            shallow,
        } = &mut self.walk
            && !shallow.contains(&id)
        {
            for parent in parents {
                if seen.insert(parent) {
                    let time = self.repo.find_commit(parent)?.committer()?.time()?.seconds;
                    frontier.push((time, parent));
                }
            }
        }
        Ok(Some(commit))
    }

    fn read_commit(
        &self,
        id: gix::ObjectId,
    ) -> Result<(Commit, Vec<gix::ObjectId>), Box<dyn std::error::Error>> {
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
        Ok((commit, parents))
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

pub mod backup;
pub mod blame;
pub mod blob;
pub mod branch;
pub mod commit;
pub mod conflict;
pub mod diff;
pub mod drop;
pub mod file;
pub mod file_history;
pub mod merge;
pub mod mutation;
pub mod patch;
pub mod rebase;
pub mod reflog;
pub mod refs;
pub mod reset;
pub mod review;
pub mod rewrite;
pub mod stash;
pub mod status;
pub mod sync;
pub mod tag;
pub mod working_diff;
pub mod worktree;

pub mod unstage;

pub mod stage;
