//! The one place Git Good starts a Git process (RULE-16).
//!
//! Every production `git` spawn goes through [`git`], which returns a
//! [`GitCommand`]: a thin builder over [`std::process::Command`] whose
//! `output`, `status` and `spawn` note the command line in a [`Recording`]
//! when one is active. Callers keep their own arguments and environment;
//! this module only names the program and observes the spawn.
//!
//! The recorder is for tests: with no recording active a spawn costs one
//! relaxed atomic load. A recording keeps only the spawns that name one of
//! its roots (in an argument or the working directory), so tests that run in
//! parallel on their own fixtures never see each other's processes.
use std::{
    ffi::OsStr,
    io,
    path::Path,
    process::{Child, Command, ExitStatus, Output, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread::ThreadId,
};

/// A Git command line under construction. Mirrors the parts of
/// [`Command`] Git Good uses; spawning records the command line when a
/// [`Recording`] is active.
#[derive(Debug)]
pub struct GitCommand {
    command: Command,
}

/// Start building a `git` command. The only production constructor of a
/// Git process; a test (`git_processes_start_at_the_chokepoint`) keeps it so.
///
/// Git writes its own messages in English whatever the user's locale: Git
/// Good tells some failures apart by Git's message (a rejected push, nothing
/// to stash). The locale itself is left alone, so the programs Git starts
/// (hooks, credential helpers, editors) keep the user's language.
pub fn git() -> GitCommand {
    let mut command = Command::new("git");
    if let Some(dir) = no_translations() {
        command.env("GIT_TEXTDOMAINDIR", dir);
    } else {
        english_messages(&mut command);
    }
    GitCommand { command }
}

/// Where Git looks for its translations: a folder that cannot exist, inside
/// Git Good's own executable file, so Git finds none and speaks English.
/// Only Git reads `GIT_TEXTDOMAINDIR`; Git commands a hook runs speak
/// English too.
fn no_translations() -> Option<&'static Path> {
    static DIR: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let exe = std::env::current_exe().ok()?;
        exe.is_file().then(|| exe.join("no-translations"))
    })
    .as_deref()
}

/// Without that folder: `LC_MESSAGES=C` names the message language for Git
/// and everything it starts. `LC_ALL` would override it, so its value moves
/// to `LC_CTYPE`; `LANGUAGE` is dropped.
fn english_messages(command: &mut Command) {
    if let Some(all) = std::env::var_os("LC_ALL").filter(|value| !value.is_empty()) {
        command.env_remove("LC_ALL").env("LC_CTYPE", all);
    }
    command.env("LC_MESSAGES", "C").env_remove("LANGUAGE");
}

impl GitCommand {
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.command.arg(arg);
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    pub fn env<K: AsRef<OsStr>, V: AsRef<OsStr>>(&mut self, key: K, value: V) -> &mut Self {
        self.command.env(key, value);
        self
    }

    pub fn env_remove<K: AsRef<OsStr>>(&mut self, key: K) -> &mut Self {
        self.command.env_remove(key);
        self
    }

    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.command.current_dir(dir);
        self
    }

    pub fn stdin<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
        self.command.stdin(cfg);
        self
    }

    pub fn stdout<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
        self.command.stdout(cfg);
        self
    }

    pub fn stderr<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
        self.command.stderr(cfg);
        self
    }

    pub fn output(&mut self) -> io::Result<Output> {
        self.note();
        self.command.output()
    }

    pub fn status(&mut self) -> io::Result<ExitStatus> {
        self.note();
        self.command.status()
    }

    pub fn spawn(&mut self) -> io::Result<Child> {
        self.note();
        self.command.spawn()
    }

    #[inline]
    fn note(&self) {
        if ACTIVE.load(Ordering::Relaxed) != 0 {
            record(&self.command);
        }
    }
}

/// One recorded Git process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// The arguments after `git`, with each recording root replaced by its
    /// placeholder.
    pub args: Vec<String>,
    /// The working directory, placeholder-substituted, when one was set.
    pub dir: Option<String>,
    /// Whether the process started on the thread that began the recording
    /// (the UI thread in GUI tests).
    pub on_recording_thread: bool,
}

impl Record {
    /// `[dir] arg arg …`, for assertions and failure messages.
    pub fn line(&self) -> String {
        let mut line = String::new();
        if let Some(dir) = &self.dir {
            line.push('[');
            line.push_str(dir);
            line.push_str("] ");
        }
        line.push_str(&self.args.join(" "));
        line
    }
}

struct Scope {
    thread: ThreadId,
    /// (text to find, placeholder), longest first.
    roots: Vec<(String, String)>,
    records: Mutex<Vec<Record>>,
}

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static SCOPES: Mutex<Vec<Arc<Scope>>> = Mutex::new(Vec::new());

/// Records the Git processes that name its roots until dropped.
pub struct Recording {
    scope: Arc<Scope>,
}

impl Recording {
    /// Start recording spawns that mention any of `roots` (a repository, a
    /// remote…), replacing each with its placeholder such as `<repo>`.
    pub fn start(roots: &[(&Path, &str)]) -> Self {
        let mut forms = Vec::new();
        for (path, placeholder) in roots {
            let mut texts = vec![path.to_string_lossy().into_owned()];
            if let Ok(canonical) = std::fs::canonicalize(path) {
                let canonical = canonical.to_string_lossy().into_owned();
                if let Some(stripped) = canonical.strip_prefix(r"\\?\") {
                    texts.push(stripped.to_string());
                }
                texts.push(canonical);
            }
            for text in texts.clone() {
                texts.push(text.replace('\\', "/"));
            }
            for text in texts {
                if !text.is_empty() && !forms.iter().any(|(known, _)| *known == text) {
                    forms.push((text, (*placeholder).to_string()));
                }
            }
        }
        forms.sort_by_key(|(text, _): &(String, String)| std::cmp::Reverse(text.len()));
        let scope = Arc::new(Scope {
            thread: std::thread::current().id(),
            roots: forms,
            records: Mutex::new(Vec::new()),
        });
        if let Ok(mut scopes) = SCOPES.lock() {
            scopes.push(Arc::clone(&scope));
        }
        ACTIVE.fetch_add(1, Ordering::Relaxed);
        Self { scope }
    }

    /// The processes recorded so far, in spawn order.
    pub fn records(&self) -> Vec<Record> {
        self.scope
            .records
            .lock()
            .map(|records| records.clone())
            .unwrap_or_default()
    }

    /// The processes recorded so far; the recording continues empty.
    pub fn take(&self) -> Vec<Record> {
        self.scope
            .records
            .lock()
            .map(|mut records| std::mem::take(&mut *records))
            .unwrap_or_default()
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        if let Ok(mut scopes) = SCOPES.lock() {
            scopes.retain(|scope| !Arc::ptr_eq(scope, &self.scope));
        }
        ACTIVE.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cold]
fn record(command: &Command) {
    let Ok(scopes) = SCOPES.lock() else {
        return;
    };
    if scopes.is_empty() {
        return;
    }
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let dir = command
        .get_current_dir()
        .map(|dir| dir.to_string_lossy().into_owned());
    let thread = std::thread::current().id();
    for scope in scopes.iter() {
        let mentions = |text: &str| scope.roots.iter().any(|(root, _)| text.contains(root));
        if !(args.iter().any(|arg| mentions(arg)) || dir.as_deref().is_some_and(mentions)) {
            continue;
        }
        let replace = |text: &str| {
            let mut text = text.to_string();
            for (root, placeholder) in &scope.roots {
                text = text.replace(root, placeholder);
            }
            text
        };
        let record = Record {
            args: args.iter().map(|arg| replace(arg)).collect(),
            dir: dir.as_deref().map(replace),
            on_recording_thread: thread == scope.thread,
        };
        if let Ok(mut records) = scope.records.lock() {
            records.push(record);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recording_keeps_only_its_own_roots() -> Result<(), Box<dyn std::error::Error>> {
        let mine = tempfile::tempdir()?;
        let other = tempfile::tempdir()?;
        let recording = Recording::start(&[(mine.path(), "<repo>")]);
        git()
            .arg("-C")
            .arg(mine.path())
            .args(["rev-parse", "--git-dir"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        git()
            .current_dir(other.path())
            .arg("--version")
            .stdout(Stdio::null())
            .status()?;
        let path = mine.path().to_path_buf();
        std::thread::spawn(move || {
            git()
                .current_dir(&path)
                .arg("--version")
                .stdout(Stdio::null())
                .status()
        })
        .join()
        .map_err(|_| "worker panicked")??;
        let lines = recording
            .records()
            .iter()
            .map(|record| (record.line(), record.on_recording_thread))
            .collect::<Vec<_>>();
        assert_eq!(
            lines,
            [
                ("-C <repo> rev-parse --git-dir".to_string(), true),
                ("[<repo>] --version".to_string(), false),
            ]
        );
        drop(recording);
        let after = Recording::start(&[(other.path(), "<other>")]);
        assert!(after.records().is_empty());
        Ok(())
    }

    fn env_of<'a>(command: &'a GitCommand, name: &str) -> Option<Option<&'a std::ffi::OsStr>> {
        command
            .command
            .get_envs()
            .find(|(key, _)| *key == std::ffi::OsStr::new(name))
            .map(|(_, value)| value)
    }

    #[test]
    fn git_speaks_english_and_what_it_starts_keeps_the_user_locale() {
        let command = git();
        let dir = env_of(&command, "GIT_TEXTDOMAINDIR")
            .flatten()
            .map(Path::new);
        assert!(
            dir.is_some_and(|dir| !dir.exists() && dir.parent().is_some_and(Path::is_file)),
            "{dir:?}"
        );
        for name in ["LC_ALL", "LC_CTYPE", "LC_MESSAGES", "LANGUAGE", "LANG"] {
            assert_eq!(env_of(&command, name), None, "{name} is the user's");
        }
    }
}
