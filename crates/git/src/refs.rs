//! Read branch, remote, tag, and HEAD labels without changing repository state.
use gg_engine::refs::{Head, RefItem, RefKind, Remote, Snapshot, Tracking};
use gix::{bstr::ByteSlice, remote::Direction};
use std::path::Path;

pub fn read(path: &Path) -> Result<Snapshot, String> {
    read_inner(path).map_err(|e| format!("Read refs {}: {e}", path.display()))
}

fn read_inner(path: &Path) -> std::result::Result<Snapshot, Box<dyn std::error::Error>> {
    let repo = gix::discover(path)?;
    let head = repo.head()?;
    let head = Head {
        name: head.referent_name().map(ToString::to_string),
        detached: head.is_detached(),
        commit: head.id().map(|id| id.detach().to_string()),
    };
    let mut items = vec![];
    let mut branches = vec![];
    for reference in repo.references()?.local_branches()? {
        let mut reference = reference?;
        let commit = reference.peel_to_commit()?.id.to_string();
        let full = reference.name().to_owned();
        let name = full.shorten().to_string();
        let full_name = full.as_bstr().to_string();
        branches.push((full, name.clone(), full_name.clone(), commit.clone()));
        items.push(RefItem {
            name,
            full_name,
            kind: RefKind::Branch,
            commit,
        });
    }
    for reference in repo.references()?.remote_branches()? {
        let mut reference = reference?;
        let commit = reference.peel_to_commit()?.id.to_string();
        let name = reference
            .name()
            .as_bstr()
            .to_string()
            .strip_prefix("refs/remotes/")
            .unwrap_or_default()
            .to_string();
        if name == "HEAD" || name.ends_with("/HEAD") {
            continue;
        }
        items.push(RefItem {
            name,
            full_name: reference.name().as_bstr().to_string(),
            kind: RefKind::Remote,
            commit,
        });
    }
    for reference in repo.references()?.tags()? {
        let mut reference = reference?;
        let Ok(commit) = reference.peel_to_commit() else {
            continue;
        };
        let name = reference.name().shorten().to_string();
        items.push(RefItem {
            name,
            full_name: reference.name().as_bstr().to_string(),
            kind: RefKind::Tag,
            commit: commit.id.to_string(),
        });
    }
    items.sort_by(|a, b| {
        (a.kind as u8, a.name.as_str(), a.full_name.as_str()).cmp(&(
            b.kind as u8,
            b.name.as_str(),
            b.full_name.as_str(),
        ))
    });
    // gix can yield the same remote ref from both loose and packed storage.
    // The overlay iterator favors the loose value, so keep the first entry.
    items.dedup_by(|a, b| a.full_name == b.full_name && a.kind == b.kind);

    let mut remotes = vec![];
    for name in repo.remote_names() {
        let name = name.to_string();
        let remote = repo.find_remote(name.as_str())?;
        let fetch_url = display_url(remote.url(Direction::Fetch));
        let push_url =
            display_url(remote.url(Direction::Push)).filter(|url| Some(url) != fetch_url.as_ref());
        remotes.push(Remote {
            name,
            fetch_url,
            push_url,
        });
    }

    let tracking = branches
        .into_iter()
        .filter_map(|(full, branch, full_name, commit)| {
            tracking_for(&repo, full, branch, full_name, commit).transpose()
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(Snapshot {
        head: Some(head),
        items,
        remotes,
        tracking,
    })
}

fn tracking_for(
    repo: &gix::Repository,
    full: gix::refs::FullName,
    branch: String,
    full_name: String,
    commit: String,
) -> std::result::Result<Option<Tracking>, Box<dyn std::error::Error>> {
    let Some(upstream_full_name) =
        repo.branch_remote_tracking_ref_name(full.as_ref(), Direction::Fetch)
    else {
        return Ok(None);
    };
    let upstream_full_name = upstream_full_name?;
    let remote = repo
        .branch_remote_name(full.shorten(), Direction::Fetch)
        .map(|name| name.as_bstr().to_string())
        .unwrap_or_default();
    let remote_branch = repo
        .branch_remote_ref_name(full.as_ref(), Direction::Fetch)
        .transpose()?
        .map(|name| {
            let name = name.as_bstr().to_string();
            name.strip_prefix("refs/heads/")
                .unwrap_or(&name)
                .to_string()
        })
        .unwrap_or_else(|| {
            let short = upstream_full_name.shorten().as_bstr().to_string();
            short
                .split_once('/')
                .map(|(_, branch)| branch.to_string())
                .unwrap_or_default()
        });
    let upstream = upstream_full_name.shorten().to_string();
    let upstream_commit = repo
        .try_find_reference(upstream_full_name.as_bstr().to_string().as_str())?
        .map(|mut reference| reference.peel_to_commit().map(|commit| commit.id))
        .transpose()?;
    let (ahead, behind) = match upstream_commit {
        Some(upstream) => {
            let local = gix::ObjectId::from_hex(commit.as_bytes())?;
            ahead_behind(repo, local, upstream)?
        }
        None => (None, None),
    };
    Ok(Some(Tracking {
        branch,
        full_name,
        upstream,
        upstream_full_name: upstream_full_name.as_bstr().to_string(),
        remote,
        remote_branch,
        ahead,
        behind,
    }))
}

fn ahead_behind(
    repo: &gix::Repository,
    local: gix::ObjectId,
    upstream: gix::ObjectId,
) -> std::result::Result<(Option<u64>, Option<u64>), Box<dyn std::error::Error>> {
    let base = repo.merge_base(local, upstream)?.map(|id| id.detach());
    Ok((
        Some(count_commits(repo, local, base)?),
        Some(count_commits(repo, upstream, base)?),
    ))
}

fn count_commits(
    repo: &gix::Repository,
    tip: gix::ObjectId,
    boundary: Option<gix::ObjectId>,
) -> std::result::Result<u64, Box<dyn std::error::Error>> {
    let mut walk = repo.rev_walk([tip]);
    if let Some(boundary) = boundary {
        walk = walk.with_boundary([boundary]);
    }
    let mut count = 0;
    for commit in walk.all()? {
        commit?;
        count += 1;
    }
    Ok(count)
}

fn display_url(url: Option<&gix::Url>) -> Option<String> {
    let mut url = url?.clone();
    url.password = None;
    Some(String::from_utf8_lossy(&url.to_bstring()).into_owned())
}
