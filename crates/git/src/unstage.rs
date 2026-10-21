//! Restore one index path from pinned HEAD under the index lock.
use gg_engine::status::{Area, WorkingRequest};
use std::path::Component;

pub fn file(request: &WorkingRequest) -> Result<(), String> {
    apply(request).map_err(|e| format!("Unstage {}: {e}", request.entry.path))
}
fn apply(request: &WorkingRequest) -> Result<(), Box<dyn std::error::Error>> {
    if request.entry.area != Area::Staged {
        return Err("Select a staged file".into());
    }
    let relative = gix::path::from_bstr(gix::bstr::BStr::new(&request.entry.path_bytes))?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Use a repository-relative file path".into());
    }
    let repo = gix::discover(&request.repository.path)?;
    let mut lock = gix::lock::File::acquire_to_update_resource(
        repo.index_path(),
        gix::lock::acquire::Fail::Immediately,
        None,
        0,
    )?;
    crate::status::validate(&request.repository)?;
    let mut index = (**repo.index_or_empty()?).clone();
    if index.is_sparse() {
        return Err("Unstage requires an expanded index".into());
    }
    let path = gix::bstr::BStr::new(&request.entry.path_bytes);
    if index
        .entries()
        .iter()
        .any(|e| e.path(&index) == path && e.stage() != gix::index::entry::Stage::Unconflicted)
    {
        return Err("Resolve this path's conflict before unstaging".into());
    }
    let before = if let Some(head) = &request.repository.head {
        repo.find_commit(gix::ObjectId::from_hex(head.as_bytes())?)?
            .tree()?
            .lookup_entry_by_path(&relative)?
            .map(|e| (e.id().detach(), e.mode().value()))
    } else {
        None
    };
    if before.is_some_and(|(_, mode)| mode == 0o040000) {
        return Err("Select an individual file; this path is a directory in HEAD".into());
    }
    // Restoring a file must not introduce a file/directory collision in the index.
    if before.is_some()
        && index.entries().iter().any(|e| {
            let p: &[u8] = e.path(&index).as_ref();
            let wanted = &request.entry.path_bytes;
            (p.starts_with(wanted) && p.get(wanted.len()) == Some(&b'/'))
                || (wanted.starts_with(p) && wanted.get(p.len()) == Some(&b'/'))
        })
    {
        return Err("Unstage the related file/directory changes first".into());
    }
    index.remove_entries(|_, p, _| p == path);
    if let Some((id, mode)) = before {
        index.dangerously_push_entry(
            Default::default(),
            id,
            gix::index::entry::Flags::empty(),
            gix::index::entry::Mode::from_bits_truncate(mode as u32),
            path,
        );
    }
    index.sort_entries();
    index.remove_tree();
    index.write_to(&mut lock, Default::default())?;
    // HEAD can change independently of the index lock.
    crate::status::validate(&request.repository)?;
    lock.commit()?;
    Ok(())
}
