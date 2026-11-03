//! The repository's signing configuration (`gg_git::signature::config`: six
//! `git config --get` reads and a probe of the signing tool), read off the
//! UI thread and kept for the window.
//!
//! Local Changes and New tag start a read as they open and show the last
//! known configuration until it lands; focus and a watcher report of a
//! changed `config` file read it again. A commit or tag worker that needs the
//! configuration as it is now waits for the read in flight, or reads it
//! itself, so the dialog never blocks on Git (RULE-16).
use eframe::egui;
use gg_engine::signature::Config;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};

/// One read's result, shared with the workers that wait for it.
type Slot = Arc<(Mutex<Option<Config>>, Condvar)>;

#[derive(Default)]
struct Inner {
    /// The repository the configuration belongs to.
    repo: Option<PathBuf>,
    config: Option<Config>,
    /// Counts stored results, so views apply each one once.
    generation: u64,
    reading: Option<(PathBuf, Slot)>,
    repaint: Option<egui::Context>,
}

/// Cheap to clone: every clone shares one configuration.
#[derive(Clone, Default)]
pub struct Cache(Arc<Mutex<Inner>>);

impl Cache {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Repaint `ctx` when a background read lands.
    pub fn attach(&self, ctx: &egui::Context) {
        let mut inner = self.lock();
        if inner.repaint.is_none() {
            inner.repaint = Some(ctx.clone());
        }
    }

    /// Start a background read for `repo` unless one is already running.
    pub fn refresh(&self, repo: &Path) {
        let slot: Slot = Default::default();
        {
            let mut inner = self.lock();
            if inner
                .reading
                .as_ref()
                .is_some_and(|(reading, _)| reading == repo)
            {
                return;
            }
            inner.reading = Some((repo.to_path_buf(), Arc::clone(&slot)));
        }
        let cache = self.clone();
        let path = repo.to_path_buf();
        let worker_slot = Arc::clone(&slot);
        let spawned = std::thread::Builder::new()
            .name("gg-signing".into())
            .spawn(move || {
                let config = gg_git::signature::config(&path);
                cache.store(&path, &config, Some(&worker_slot));
                let (value, ready) = &*worker_slot;
                *value
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(config);
                ready.notify_all();
                let repaint = cache.lock().repaint.clone();
                if let Some(ctx) = repaint {
                    ctx.request_repaint();
                }
            });
        if spawned.is_err() {
            self.lock().reading = None;
        }
    }

    fn store(&self, repo: &Path, config: &Config, slot: Option<&Slot>) {
        let mut inner = self.lock();
        if let (Some((_, reading)), Some(slot)) = (&inner.reading, slot)
            && Arc::ptr_eq(reading, slot)
        {
            inner.reading = None;
        }
        inner.repo = Some(repo.to_path_buf());
        inner.config = Some(config.clone());
        inner.generation += 1;
    }

    /// The last configuration read for `repo` and its generation.
    pub fn current(&self, repo: &Path) -> Option<(u64, Config)> {
        let inner = self.lock();
        let config = inner.config.as_ref()?;
        (inner.repo.as_deref() == Some(repo)).then(|| (inner.generation, config.clone()))
    }

    /// For a worker: the read in flight for `repo`, waited for, or else the
    /// last configuration read.
    pub fn settled(&self, repo: &Path) -> Option<Config> {
        let slot = {
            let inner = self.lock();
            match &inner.reading {
                Some((reading, slot)) if reading == repo => Arc::clone(slot),
                _ => {
                    return inner
                        .config
                        .clone()
                        .filter(|_| inner.repo.as_deref() == Some(repo));
                }
            }
        };
        Some(wait(&slot))
    }

    /// For a worker: the configuration as it is now. Waits for the read in
    /// flight, otherwise reads it on the calling thread, and keeps it.
    pub fn fresh(&self, repo: &Path) -> Config {
        let slot = {
            let inner = self.lock();
            match &inner.reading {
                Some((reading, slot)) if reading == repo => Some(Arc::clone(slot)),
                _ => None,
            }
        };
        if let Some(slot) = slot {
            return wait(&slot);
        }
        let config = gg_git::signature::config(repo);
        self.store(repo, &config, None);
        let repaint = self.lock().repaint.clone();
        if let Some(ctx) = repaint {
            ctx.request_repaint();
        }
        config
    }
}

fn wait(slot: &Slot) -> Config {
    let (value, ready) = &**slot;
    let mut value = value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    loop {
        if let Some(config) = value.as_ref() {
            return config.clone();
        }
        value = ready
            .wait(value)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_worker_waits_for_the_read_in_flight_and_views_see_it_once()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let cache = Cache::default();
        assert!(cache.current(dir.path()).is_none());
        cache.refresh(dir.path());
        let config = cache.settled(dir.path()).ok_or("the read in flight")?;
        let (generation, current) = cache.current(dir.path()).ok_or("stored")?;
        assert_eq!(current.format, config.format);
        assert_eq!(cache.current(dir.path()).map(|(g, _)| g), Some(generation));
        assert!(cache.current(Path::new("/elsewhere")).is_none());
        let _ = cache.fresh(dir.path());
        assert!(
            cache
                .current(dir.path())
                .is_some_and(|(g, _)| g > generation)
        );
        Ok(())
    }
}
