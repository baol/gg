//! Tree-only comparison: skip identical subtrees and read blobs on demand later.
use gg_engine::review::*;
use std::collections::{BTreeMap, HashMap, HashSet};

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
    /// Renames as `git diff -M` pairs them, read once the first added or
    /// deleted file shows up; `None` until then.
    renames: Option<Renames>,
}

/// Rename pairs of one comparison: each destination with its source side.
#[derive(Default)]
struct Renames {
    by_destination: HashMap<Vec<u8>, (Vec<u8>, FileSide)>,
    sources: HashSet<Vec<u8>>,
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
        let against = request
            .against
            .as_ref()
            .map(
                |reference| -> Result<(gix::ObjectId, gix::ObjectId), Error> {
                    let object = repo
                        .rev_parse_single(reference.as_str())?
                        .object()?
                        .peel_to_kind(gix::object::Kind::Commit)?;
                    let base = object.try_into_commit()?;
                    let id = base.id;
                    let tree = base.tree_id()?.detach();
                    Ok((id, tree))
                },
            )
            .transpose()?;
        let parent = if against.is_none() {
            match (request.parent, parents.is_empty()) {
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
            }
        } else {
            None
        };
        let old = if let Some((_, tree)) = &against {
            Some(Side {
                id: *tree,
                mode: 0o040000,
            })
        } else {
            parent
                .map(|id| -> Result<Side, Error> {
                    Ok(Side {
                        id: repo.find_commit(id)?.tree_id()?.detach(),
                        mode: 0o040000,
                    })
                })
                .transpose()?
        };
        let new = Some(Side {
            id: commit.tree_id()?.detach(),
            mode: 0o040000,
        });
        let subject = if let Some((id, _)) = &against {
            format!(
                "Compare {} to {}",
                &id.to_string()[..8],
                &commit.id.to_string()[..8]
            )
        } else {
            commit
                .decode()?
                .message
                .to_string()
                .lines()
                .next()
                .unwrap_or("(empty message)")
                .into()
        };
        let (author, date) = {
            let decoded = commit.decode()?;
            let author = decoded.author()?;
            let time = author.time()?.format(gix::date::time::format::ISO8601)?;
            (
                author.name.to_string(),
                time.get(..10).unwrap_or(&time).to_string(),
            )
        };
        let comparison = Comparison {
            path: repo.workdir().unwrap_or(repo.path()).to_path_buf(),
            commit: commit.id.to_string(),
            parent: against
                .as_ref()
                .map(|(id, _)| id.to_string())
                .or_else(|| parent.map(|id| id.to_string())),
            parents: if against.is_some() {
                Vec::new()
            } else {
                parents.iter().map(ToString::to_string).collect()
            },
            subject,
            author,
            date,
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
            renames: None,
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
    /// The comparison's renames, paired as `git diff -M` pairs them (50 %
    /// similarity, at most 1000 candidates), the first time an added or
    /// deleted file needs them. Read in process: no Git command runs, so a
    /// review started beside a mutation never adds to its commands. A root
    /// commit has none; a read that fails (a blob missing from a partial
    /// clone) pairs nothing, and the files list as added and deleted.
    fn renames(&mut self) -> &Renames {
        if self.renames.is_none() {
            let renames = self.read_renames().unwrap_or_default();
            self.renames = Some(renames);
        }
        self.renames.get_or_insert_with(Renames::default)
    }
    fn read_renames(&self) -> Result<Renames, Error> {
        let Some(parent) = &self.comparison.parent else {
            return Ok(Renames::default());
        };
        let tree = |id: &str| -> Result<gix::Tree<'_>, Error> {
            Ok(self
                .repo
                .find_commit(gix::ObjectId::from_hex(id.as_bytes())?)?
                .tree()?)
        };
        let (old, new) = (tree(parent)?, tree(&self.comparison.commit)?);
        let mut options = gix::diff::Options::default();
        options
            .track_path()
            .track_rewrites(Some(gix::diff::Rewrites::default()));
        let mut renames = Renames::default();
        for change in self.repo.diff_tree_to_tree(&old, &new, options)? {
            if let gix::object::tree::diff::ChangeDetached::Rewrite {
                source_location,
                source_entry_mode,
                source_id,
                entry_mode,
                location,
                copy: false,
                ..
            } = change
                && !entry_mode.is_tree()
                && !source_entry_mode.is_tree()
            {
                renames.sources.insert(source_location.to_vec());
                renames.by_destination.insert(
                    location.to_vec(),
                    (
                        source_location.to_vec(),
                        FileSide {
                            id: source_id.to_string(),
                            mode: source_entry_mode.value(),
                        },
                    ),
                );
            }
        }
        Ok(renames)
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
        if matches!(kind, ChangeKind::Added | ChangeKind::Deleted) {
            let renames = self.renames();
            if kind == ChangeKind::Deleted && renames.sources.contains(&node.path) {
                // Listed once, at its new path.
                return Ok(ReviewStep::Pending);
            }
            if kind == ChangeKind::Added
                && let Some((source, side)) = renames.by_destination.get(&node.path)
            {
                return Ok(ReviewStep::Change(FileChange {
                    path: display_path(&node.path),
                    path_bytes: node.path,
                    kind: ChangeKind::Renamed,
                    old: Some(side.clone()),
                    new,
                    old_path: Some(display_path(source)),
                    old_path_bytes: Some(source.clone()),
                }));
            }
        }
        Ok(ReviewStep::Change(FileChange {
            path: display_path(&node.path),
            path_bytes: node.path,
            kind,
            old,
            new,
            old_path: None,
            old_path_bytes: None,
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

/// Counts added and deleted lines of changed files in one repository.
pub struct LineStats {
    repo: gix::Repository,
}
impl LineStats {
    pub fn open(path: &std::path::Path) -> Result<Self, String> {
        let repo = gix::discover(path).map_err(|e| e.to_string())?;
        Ok(Self { repo })
    }
    /// Count one change; object headers are checked before content is read.
    pub fn count(&self, change: &FileChange) -> Result<LineStat, String> {
        self.count_inner(change)
            .map_err(|e| format!("Could not count lines of {}: {e}", change.path))
    }
    fn count_inner(&self, change: &FileChange) -> Result<LineStat, Error> {
        let gitlink = |side: &Option<FileSide>| side.as_ref().is_some_and(|s| s.mode == 0o160000);
        if gitlink(&change.old) || gitlink(&change.new) {
            return Ok(LineStat::Skipped);
        }
        if let (Some(old), Some(new)) = (&change.old, &change.new)
            && old.id == new.id
        {
            return Ok(LineStat::Lines {
                added: 0,
                deleted: 0,
            });
        }
        let size = |side: &Option<FileSide>| -> Result<u64, Error> {
            match side {
                Some(side) => Ok(self
                    .repo
                    .find_header(gix::ObjectId::from_hex(side.id.as_bytes())?)?
                    .size()),
                None => Ok(0),
            }
        };
        let limit = gg_engine::diff::MAX_BLOB_BYTES as u64;
        if size(&change.old)? > limit || size(&change.new)? > limit {
            return Ok(LineStat::Skipped);
        }
        let bytes = |side: &Option<FileSide>| -> Result<Vec<u8>, Error> {
            match side {
                Some(side) => Ok(self
                    .repo
                    .find_blob(gix::ObjectId::from_hex(side.id.as_bytes())?)?
                    .data
                    .clone()),
                None => Ok(Vec::new()),
            }
        };
        let (old, new) = (bytes(&change.old)?, bytes(&change.new)?);
        // Git splits a file/symlink type change into a deletion and an
        // addition, so every line of each side counts.
        if change.kind == ChangeKind::TypeChanged {
            return Ok(
                match (LineStat::compare(&old, b""), LineStat::compare(b"", &new)) {
                    (LineStat::Lines { deleted, .. }, LineStat::Lines { added, .. }) => {
                        LineStat::Lines { added, deleted }
                    }
                    (LineStat::Binary, _) | (_, LineStat::Binary) => LineStat::Binary,
                    _ => LineStat::Skipped,
                },
            );
        }
        Ok(LineStat::compare(&old, &new))
    }
}
