//! Commit drafts survive quit and crash. Each repository's commit message is
//! written to `gg/drafts/<id>.json` in the configuration folder once typing
//! pauses for [`PAUSE`], when Local Changes closes, and on quit; restored
//! when Local Changes opens on that repository again; and rewritten at once
//! on commit, which clears the committed text. A draft with no text has no
//! file. Files are written by one background thread, through a temporary
//! file and a rename, so the UI thread never waits on the disk and a crash
//! never leaves half a file. Source: photocraft
//! `apps/photocraft/src/services.rs` (`RecoveryStore`).

use crate::status_view::Draft;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant};

/// How long typing must pause before the draft is written.
pub const PAUSE: Duration = Duration::from_secs(1);

/// What a draft file holds. The selected file is not kept: it may be gone
/// when the repository reopens.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Stored {
    /// The repository the draft belongs to, checked on load.
    repository: String,
    #[serde(default)]
    normal: String,
    #[serde(default)]
    amend: String,
    #[serde(default)]
    amend_mode: bool,
    #[serde(default)]
    amend_base: Option<String>,
    #[serde(default)]
    sign: bool,
}

impl Stored {
    fn new(key: &Path, draft: &Draft) -> Self {
        Self {
            repository: key.to_string_lossy().into_owned(),
            normal: draft.normal.clone(),
            amend: draft.amend.clone(),
            amend_mode: draft.amend_mode,
            amend_base: draft.amend_base.clone(),
            sign: draft.sign,
        }
    }

    fn empty(&self) -> bool {
        self.normal.trim().is_empty() && self.amend.trim().is_empty()
    }

    fn draft(self) -> Draft {
        Draft {
            normal: self.normal,
            amend: self.amend,
            amend_mode: self.amend_mode,
            amend_base: self.amend_base,
            sign: self.sign,
            selected: None,
        }
    }
}

/// A file to write, or to delete when there is nothing to keep.
type Write = (PathBuf, Option<Vec<u8>>);

pub struct Store {
    dir: Option<PathBuf>,
    writer: Option<Sender<Write>>,
    thread: Option<std::thread::JoinHandle<()>>,
    /// Per repository, what its file holds (or is about to): an empty draft
    /// for none.
    written: HashMap<PathBuf, Stored>,
    /// A change waiting for typing to pause, and when it was last seen to
    /// change.
    pending: Option<(PathBuf, Stored, Instant)>,
}

impl Store {
    /// Keep drafts in `gg/drafts` under the configuration folder.
    pub fn open() -> Self {
        Self::in_dir(crate::settings::config_dir().map(|dir| dir.join("drafts")))
    }

    /// Keep drafts in `dir`; `None` keeps nothing (tests).
    pub fn in_dir(dir: Option<PathBuf>) -> Self {
        Self {
            dir,
            writer: None,
            thread: None,
            written: HashMap::new(),
            pending: None,
        }
    }

    /// The file for the repository `key`: named by a hash of its path, so
    /// any path gives a valid file name.
    fn file(&self, key: &Path) -> Option<PathBuf> {
        use sha2::Digest as _;
        let hash = sha2::Sha256::digest(key.to_string_lossy().as_bytes());
        let name: String = hash[..8].iter().map(|byte| format!("{byte:02x}")).collect();
        self.dir
            .as_ref()
            .map(|dir| dir.join(format!("{name}.json")))
    }

    /// The draft kept for the repository `key`, if any.
    pub fn load(&mut self, key: &Path) -> Option<Draft> {
        let file = self.file(key)?;
        let bytes = std::fs::read(file).ok()?;
        let stored: Stored = serde_json::from_slice(&bytes).ok()?;
        if stored.repository != key.to_string_lossy() || stored.empty() {
            return None;
        }
        self.written.insert(key.to_path_buf(), stored.clone());
        Some(stored.draft())
    }

    /// The draft of `key` as it is now, once a frame while Local Changes is
    /// open. It is written once it stopped changing for [`PAUSE`]; until
    /// then, how long until it is due, for the frame to wake at.
    pub fn note(&mut self, key: &Path, draft: &Draft, now: Instant) -> Option<Duration> {
        let stored = Stored::new(key, draft);
        let changed = match &self.pending {
            Some((pending_key, pending, _)) if pending_key == key => *pending != stored,
            Some(_) => {
                self.flush();
                true
            }
            None => true,
        };
        if changed {
            if self.is_written(key, &stored) {
                self.pending = None;
                return None;
            }
            self.pending = Some((key.to_path_buf(), stored, now));
        }
        let (_, _, since) = self.pending.as_ref()?;
        let waited = now.saturating_duration_since(*since);
        if waited >= PAUSE {
            self.flush();
            return None;
        }
        Some(PAUSE - waited)
    }

    /// Write the draft of `key` now, as after a commit.
    pub fn save_now(&mut self, key: &Path, draft: &Draft) {
        if self
            .pending
            .as_ref()
            .is_some_and(|(pending, _, _)| pending == key)
        {
            self.pending = None;
        }
        let stored = Stored::new(key, draft);
        if !self.is_written(key, &stored) {
            self.send(key, stored);
        }
    }

    /// Write a change still waiting for the pause.
    pub fn flush(&mut self) {
        if let Some((key, stored, _)) = self.pending.take() {
            self.send(&key, stored);
        }
    }

    /// Write what is waiting and wait for every write to finish (quit).
    pub fn finish(&mut self) {
        self.flush();
        self.writer = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }

    fn is_written(&self, key: &Path, stored: &Stored) -> bool {
        match self.written.get(key) {
            Some(written) => written == stored || (written.empty() && stored.empty()),
            None => stored.empty(),
        }
    }

    fn send(&mut self, key: &Path, stored: Stored) {
        let Some(file) = self.file(key) else {
            return;
        };
        let bytes = if stored.empty() {
            None
        } else {
            match serde_json::to_vec_pretty(&stored) {
                Ok(bytes) => Some(bytes),
                Err(_) => return,
            }
        };
        self.written.insert(key.to_path_buf(), stored);
        let Some(writer) = self.writer() else {
            return;
        };
        let _ = writer.send((file, bytes));
    }

    /// The writer thread, started on the first write.
    fn writer(&mut self) -> Option<&Sender<Write>> {
        if self.writer.is_none() {
            let (sender, receiver) = channel::<Write>();
            let thread = std::thread::Builder::new()
                .name("gg-drafts".into())
                .spawn(move || {
                    for (file, bytes) in receiver {
                        let _ = write(&file, bytes.as_deref());
                    }
                })
                .ok()?;
            self.writer = Some(sender);
            self.thread = Some(thread);
        }
        self.writer.as_ref()
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        self.finish();
    }
}

/// Replace `file` with `bytes` in one step, or delete it for `None`.
fn write(file: &Path, bytes: Option<&[u8]>) -> std::io::Result<()> {
    let Some(bytes) = bytes else {
        return match std::fs::remove_file(file) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        };
    };
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = file.with_extension(format!("json.{}.tmp", std::process::id()));
    std::fs::write(&temporary, bytes)
        .and_then(|()| std::fs::rename(&temporary, file))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&temporary);
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(normal: &str) -> Draft {
        Draft {
            normal: normal.into(),
            ..Default::default()
        }
    }

    #[test]
    fn a_draft_is_written_after_a_pause_and_read_back() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let key = Path::new("/projects/repo");
        let start = Instant::now();
        let mut store = Store::in_dir(Some(dir.path().to_path_buf()));
        assert_eq!(store.note(key, &draft("Fix"), start), Some(PAUSE));
        let typing = start + PAUSE / 2;
        assert_eq!(
            store.note(key, &draft("Fix the"), typing),
            Some(PAUSE),
            "typing restarts the pause"
        );
        assert_eq!(
            store.note(key, &draft("Fix the"), typing + PAUSE / 2),
            Some(PAUSE / 2)
        );
        assert_eq!(store.note(key, &draft("Fix the"), typing + PAUSE), None);
        assert_eq!(
            store.note(key, &draft("Fix the"), typing + PAUSE * 2),
            None,
            "nothing more to write"
        );
        store.finish();

        let mut reopened = Store::in_dir(Some(dir.path().to_path_buf()));
        let restored = reopened.load(key).ok_or("no draft")?;
        assert_eq!(restored.normal, "Fix the");
        assert!(reopened.load(Path::new("/projects/other")).is_none());
        Ok(())
    }

    #[test]
    fn a_commit_rewrites_at_once_and_an_empty_draft_has_no_file()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let key = Path::new("/projects/repo");
        let mut store = Store::in_dir(Some(dir.path().to_path_buf()));
        let mut kept = draft("Subject");
        kept.amend = "Amended".into();
        store.save_now(key, &kept);
        store.finish();
        assert_eq!(std::fs::read_dir(dir.path())?.count(), 1);

        let mut store = Store::in_dir(Some(dir.path().to_path_buf()));
        assert!(store.load(key).is_some());
        store.save_now(key, &draft(""));
        store.finish();
        assert_eq!(std::fs::read_dir(dir.path())?.count(), 0, "committed: gone");
        Ok(())
    }

    #[test]
    fn closing_writes_what_is_still_waiting() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let key = Path::new("/projects/repo");
        {
            let mut store = Store::in_dir(Some(dir.path().to_path_buf()));
            store.note(key, &draft("Half typed"), Instant::now());
        }
        let mut store = Store::in_dir(Some(dir.path().to_path_buf()));
        assert_eq!(
            store.load(key).map(|draft| draft.normal).as_deref(),
            Some("Half typed")
        );
        Ok(())
    }

    #[test]
    fn a_file_for_another_repository_is_not_restored() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let key = Path::new("/projects/repo");
        let mut store = Store::in_dir(Some(dir.path().to_path_buf()));
        let file = store.file(key).ok_or("no file")?;
        std::fs::write(
            &file,
            serde_json::to_vec(&Stored {
                repository: "/projects/elsewhere".into(),
                normal: "Not mine".into(),
                ..Default::default()
            })?,
        )?;
        assert!(store.load(key).is_none());
        std::fs::write(&file, b"{ half")?;
        assert!(store.load(key).is_none());
        Ok(())
    }
}
