//! Read branch, remote, tag, and HEAD labels without changing repository state.
use gg_engine::refs::{Head, RefItem, RefKind, Snapshot};
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
    for reference in repo.references()?.local_branches()? {
        let mut reference = reference?;
        let commit = reference.peel_to_commit()?.id.to_string();
        let name = reference.name().shorten().to_string();
        items.push(RefItem {
            name,
            full_name: reference.name().as_bstr().to_string(),
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
    Ok(Snapshot {
        head: Some(head),
        items,
    })
}
