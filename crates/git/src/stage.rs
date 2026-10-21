//! Stage one working path into the index under the index lock.
use gg_engine::status::{Area, Kind, WorkingRequest};
use gix::bstr::BStr;
use std::{io::Read, path::Component};

pub fn file(request: &WorkingRequest) -> Result<(), String> {
    apply(request).map_err(|e| format!("Stage {}: {e}", request.entry.path))
}
fn apply(request: &WorkingRequest) -> Result<(), Box<dyn std::error::Error>> {
    match request.entry.area {
        Area::Unstaged | Area::Untracked => {}
        Area::Staged => return Err("Path is already staged".into()),
        Area::Conflict => return Err("Resolve this path's conflict before staging".into()),
    }
    let relative = gix::path::from_bstr(BStr::new(&request.entry.path_bytes))?;
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
        return Err("Stage requires an expanded index".into());
    }
    let path = BStr::new(&request.entry.path_bytes);
    if index
        .entries()
        .iter()
        .any(|e| e.path(&index) == path && e.stage() != gix::index::entry::Stage::Unconflicted)
    {
        return Err("Resolve this path's conflict before staging".into());
    }
    let previous = index.entry_by_path(path).map(|e| e.mode.bits() as u16);
    match request.entry.area {
        Area::Unstaged if previous.is_none() => {
            return Err("Path is no longer tracked. Refresh status.".into());
        }
        Area::Untracked if previous.is_some() => {
            return Err("Path is already tracked. Refresh status.".into());
        }
        _ => {}
    }
    if previous == Some(0o160000) && request.entry.kind != Kind::Deleted {
        return Err("Submodule changes: stage inside the submodule".into());
    }
    // Staging a file must not collide with an indexed file or directory at the same path.
    if index.entries().iter().any(|e| {
        let p: &[u8] = e.path(&index).as_ref();
        let wanted = &request.entry.path_bytes;
        (p.starts_with(wanted) && p.get(wanted.len()) == Some(&b'/'))
            || (wanted.starts_with(p) && wanted.get(p.len()) == Some(&b'/'))
    }) {
        return Err("Stage the related file/directory changes first".into());
    }
    let captured = crate::working_diff::capture(&request.repository.path, &relative, None)?;
    if request.entry.kind == Kind::Deleted {
        if captured.is_some() {
            return Err("Working path reappeared after the status scan. Refresh status.".into());
        }
        index.remove_entries(|_, p, _| p == path);
    } else {
        let captured = captured
            .as_ref()
            .ok_or("Working path is absent. Refresh status.")?;
        let capabilities = repo.filesystem_options()?;
        let mode = crate::working_diff::effective_mode(&capabilities, previous, captured.mode);
        let mut bytes = captured.bytes.clone();
        if mode != 0o120000 {
            let (mut pipeline, _) = repo.filter_pipeline(None)?;
            let mut normalized = vec![];
            pipeline
                .convert_to_git(std::io::Cursor::new(&bytes), &relative, &index)?
                .read_to_end(&mut normalized)?;
            bytes = normalized;
        }
        let id = repo.write_blob(&bytes)?.detach();
        let full = request.repository.path.join(&relative);
        let stat = gix::index::entry::Stat::from_fs(
            &gix::index::fs::Metadata::from_path_no_follow(&full)?,
        )?;
        index.remove_entries(|_, p, _| p == path);
        index.dangerously_push_entry(
            stat,
            id,
            gix::index::entry::Flags::empty(),
            gix::index::entry::Mode::from_bits_truncate(mode as u32),
            path,
        );
    }
    index.sort_entries();
    index.remove_tree();
    // The working file can change independently of the index lock.
    if crate::working_diff::capture(&request.repository.path, &relative, None)? != captured {
        return Err("Working path changed while staging. Refresh status.".into());
    }
    index.write_to(&mut lock, Default::default())?;
    // HEAD can change independently of the index lock.
    crate::status::validate(&request.repository)?;
    lock.commit()?;
    Ok(())
}
