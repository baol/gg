//! Tree-only comparison: skip identical subtrees and read blobs on demand later.
use gg_engine::review::*;
use std::collections::BTreeMap;

type Error = Box<dyn std::error::Error>;
#[derive(Clone, PartialEq, Eq)]
struct Side {
    id: gix::ObjectId,
    mode: u16,
}
impl Side {
    fn tree(&self) -> bool {
        self.mode == 0o040000
    }
    fn summary(&self) -> FileSide {
        FileSide {
            id: self.id.to_string(),
            mode: self.mode,
        }
    }
}
struct Node {
    path: Vec<u8>,
    old: Option<Side>,
    new: Option<Side>,
}
pub struct ReviewSession {
    repo: gix::Repository,
    comparison: Comparison,
    pending: Vec<Node>,
}
impl ReviewSession {
    pub fn open(request: &ReviewRequest) -> Result<Self, String> {
        Self::open_inner(request).map_err(|e| {
            format!(
                "Could not compare {} in {}: {e}",
                request.revision,
                request.path.display()
            )
        })
    }
    fn open_inner(request: &ReviewRequest) -> Result<Self, Error> {
        let repo = gix::discover(&request.path)?;
        let object = repo
            .rev_parse_single(request.revision.as_str())?
            .object()?
            .peel_to_kind(gix::object::Kind::Commit)?;
        let commit = object.try_into_commit()?;
        let parents: Vec<_> = commit.parent_ids().map(|id| id.detach()).collect();
        let parent = match (request.parent, parents.is_empty()) {
            (None, true) => None,
            (value, _) => Some(
                *parents
                    .get(
                        value
                            .unwrap_or(1)
                            .checked_sub(1)
                            .ok_or("Parent numbers start at 1")?,
                    )
                    .ok_or("Parent is outside this commit's parent list")?,
            ),
        };
        let old = parent
            .map(|id| -> Result<Side, Error> {
                Ok(Side {
                    id: repo.find_commit(id)?.tree_id()?.detach(),
                    mode: 0o040000,
                })
            })
            .transpose()?;
        let new = Some(Side {
            id: commit.tree_id()?.detach(),
            mode: 0o040000,
        });
        let comparison = Comparison {
            path: repo.workdir().unwrap_or(repo.path()).to_path_buf(),
            commit: commit.id.to_string(),
            parent: parent.map(|id| id.to_string()),
            parents: parents.iter().map(ToString::to_string).collect(),
            subject: commit
                .decode()?
                .message
                .to_string()
                .lines()
                .next()
                .unwrap_or("(empty message)")
                .into(),
        };
        drop(commit);
        Ok(Self {
            repo,
            comparison,
            pending: vec![Node {
                path: vec![],
                old,
                new,
            }],
        })
    }
    fn expand(&mut self, node: &Node) -> Result<(), Error> {
        let mut entries: BTreeMap<Vec<u8>, (Option<Side>, Option<Side>)> = BTreeMap::new();
        for (side, old) in [(&node.old, true), (&node.new, false)] {
            if let Some(side) = side.as_ref().filter(|s| s.tree()) {
                for entry in self.repo.find_tree(side.id)?.iter() {
                    let entry = entry?;
                    let pair = entries.entry(entry.filename().to_vec()).or_default();
                    let value = Some(Side {
                        id: entry.object_id(),
                        mode: entry.mode().value(),
                    });
                    if old {
                        pair.0 = value;
                    } else {
                        pair.1 = value;
                    }
                }
            }
        }
        // Git tree ordering compares directory names with a trailing slash.
        let mut ordered = Vec::new();
        for (name, (old, new)) in entries {
            // A file/directory replacement occupies two positions in Git's ordering.
            if matches!((&old, &new), (Some(a), Some(b)) if a.tree() != b.tree()) {
                ordered.push((name.clone(), (old, None)));
                ordered.push((name, (None, new)));
            } else {
                ordered.push((name, (old, new)));
            }
        }
        let mut entries = ordered;
        entries.sort_by_cached_key(|(name, (old, new))| {
            let mut key = name.clone();
            if old.as_ref().or(new.as_ref()).is_some_and(Side::tree) {
                key.push(b'/');
            }
            key
        });
        for (name, (old, new)) in entries.into_iter().rev() {
            if old == new {
                continue;
            }
            let mut path = node.path.clone();
            if !path.is_empty() {
                path.push(b'/');
            }
            path.extend(name);
            self.pending.push(Node { path, old, new });
        }
        Ok(())
    }
    fn next(&mut self) -> Result<ReviewStep, Error> {
        let Some(node) = self.pending.pop() else {
            return Ok(ReviewStep::Complete);
        };
        if node.old == node.new {
            return Ok(ReviewStep::Pending);
        }
        if node.old.as_ref().is_some_and(Side::tree) || node.new.as_ref().is_some_and(Side::tree) {
            self.expand(&node)?;
        }
        let old = node.old.filter(|s| !s.tree()).map(|s| s.summary());
        let new = node.new.filter(|s| !s.tree()).map(|s| s.summary());
        let kind = match (&old, &new) {
            (None, None) => return Ok(ReviewStep::Pending),
            (None, Some(_)) => ChangeKind::Added,
            (Some(_), None) => ChangeKind::Deleted,
            (Some(a), Some(b)) if a.mode & 0o170000 != b.mode & 0o170000 => ChangeKind::TypeChanged,
            _ => ChangeKind::Modified,
        };
        Ok(ReviewStep::Change(FileChange {
            path: String::from_utf8_lossy(&node.path).into_owned(),
            path_bytes: node.path,
            kind,
            old,
            new,
        }))
    }
}
impl ReviewReader for ReviewSession {
    fn comparison(&self) -> &Comparison {
        &self.comparison
    }
    fn step(&mut self) -> Result<ReviewStep, String> {
        self.next().map_err(|e| {
            format!(
                "Could not compare {} in {}: {e}",
                self.comparison.commit,
                self.comparison.path.display()
            )
        })
    }
}
