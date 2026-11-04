//! Crash and launch diagnostics for the desktop app. Each launch writes
//! `gg/logs/gg.log` in the configuration folder and keeps the previous run's
//! as `gg.previous.log`; a log stops growing at [`CAP`]. It records the
//! launch (version, OS, Git), error outcomes, panics with their backtrace,
//! and a launch that fails, which also shows a native dialog when no
//! terminal is attached. Passwords in URLs are redacted, as in Git's own
//! messages that `sync.rs` reports. Only the window logs: the CLI commands
//! and tests write nothing. Sources: photocraft
//! `apps/photocraft/src/{logging,crash_guard,startup_report}.rs`.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The most one run writes to its log.
pub const CAP: u64 = 16 * 1024 * 1024;

struct Log {
    file: std::fs::File,
    written: u64,
}

static LOG: Mutex<Option<Log>> = Mutex::new(None);
static PATH: OnceLock<PathBuf> = OnceLock::new();
static GIT: OnceLock<Option<String>> = OnceLock::new();

/// `gg/logs/gg.log` under the configuration folder.
pub fn log_path() -> Option<PathBuf> {
    crate::settings::config_dir().map(|dir| dir.join("logs").join("gg.log"))
}

/// Start this run's log: keep the last run's beside it, install the panic
/// hook, and record the launch. Git's version is read off the main thread.
pub fn start() {
    let Some(path) = log_path() else {
        return;
    };
    if open(&path).is_err() {
        return;
    }
    install_panic_hook();
    log("launch", &launch_line());
    std::thread::Builder::new()
        .name("gg-startup-report".into())
        .spawn(|| log("launch", git_version().unwrap_or("git not found")))
        .ok();
}

/// Rotate and open `path` as this run's log.
fn open(path: &Path) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if path.exists() {
        std::fs::rename(path, path.with_file_name("gg.previous.log"))?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    if let Ok(mut log) = LOG.lock() {
        *log = Some(Log { file, written: 0 });
    }
    let _ = PATH.set(path.to_path_buf());
    Ok(())
}

/// Where this run logs, once [`start`] opened it.
pub fn current_path() -> Option<&'static Path> {
    PATH.get().map(PathBuf::as_path)
}

/// Append one entry, `<UTC time> <kind> <text>`, with credentials redacted.
/// Does nothing before [`start`] or once the log reached [`CAP`].
pub fn log(kind: &str, text: &str) {
    let Ok(mut guard) = LOG.lock() else {
        return;
    };
    let Some(log) = guard.as_mut() else {
        return;
    };
    let mut entry = format!("{} {kind} {}\n", utc_now(), redact(text));
    if log.written >= CAP {
        return;
    }
    if log.written + entry.len() as u64 > CAP {
        entry = format!(
            "{} log reached its 16 MiB cap; later entries are dropped\n",
            utc_now()
        );
        log.written = CAP;
    } else {
        log.written += entry.len() as u64;
    }
    let _ = log.file.write_all(entry.as_bytes());
    let _ = log.file.flush();
}

/// Log a panic, with where it happened and its backtrace, before Rust's
/// usual report.
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|text| (*text).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(no message)".into());
        let place = info
            .location()
            .map(|place| format!("{}:{}", place.file(), place.line()))
            .unwrap_or_default();
        let thread = std::thread::current();
        let backtrace = std::backtrace::Backtrace::force_capture();
        log(
            "panic",
            &format!(
                "thread '{}' at {place}: {message}\n{backtrace}",
                thread.name().unwrap_or("unnamed")
            ),
        );
        previous(info);
    }));
}

/// The window could not start: log it and, launched from the desktop with
/// no terminal to read, say so in a native dialog naming the log.
pub fn launch_failed(error: &dyn std::fmt::Display) {
    let error = redact(&error.to_string());
    log("launch-failed", &error);
    eprintln!("Git Good could not start: {error}");
    use std::io::IsTerminal as _;
    if std::io::stderr().is_terminal() {
        return;
    }
    let log = current_path()
        .map(|path| format!("\n\nDetails are in {}.", path.display()))
        .unwrap_or_default();
    let _ = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Git Good could not start")
        .set_description(format!("{error}{log}"))
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// `git version 2.x`, read once.
pub fn git_version() -> Option<&'static str> {
    GIT.get_or_init(|| {
        let output = gg_git::process::git().arg("--version").output().ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    })
    .as_deref()
}

/// Git's version if it was already read, without waiting for Git.
pub fn known_git_version() -> Option<&'static str> {
    GIT.get().and_then(|version| version.as_deref())
}

/// The operating system, its release where it is cheap to read, and the
/// CPU architecture: `linux (Ubuntu 24.04.1 LTS) x86_64`.
pub fn os() -> String {
    let release = os_release()
        .map(|name| format!(" ({name})"))
        .unwrap_or_default();
    format!(
        "{}{release} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

fn os_release() -> Option<String> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let text = std::fs::read_to_string("/etc/os-release").ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix("PRETTY_NAME="))
        .map(|name| name.trim_matches('"').to_string())
        .filter(|name| !name.is_empty())
}

fn launch_line() -> String {
    let about = crate::branding::about();
    format!(
        "Git Good {} (commit {}, built {}, {}) on {}",
        about.version,
        about.commit,
        about.built,
        about.target,
        os()
    )
}

/// `text` with the password of every `scheme://user:password@host` URL
/// removed, keeping the user and host, as `sync.rs` does for Git's messages.
pub fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(scheme) = rest.find("://") {
        let (before, after) = rest.split_at(scheme + 3);
        out.push_str(before);
        let end = after
            .find(|c: char| c == '/' || c.is_whitespace() || c == '\'' || c == '"')
            .unwrap_or(after.len());
        let authority = &after[..end];
        match authority.rfind('@') {
            Some(at) if authority[..at].contains(':') => {
                let user = authority[..at].split(':').next().unwrap_or("");
                out.push_str(user);
                out.push_str(":***");
                out.push_str(&authority[at..]);
            }
            _ => out.push_str(authority),
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// Now as `YYYY-MM-DDTHH:MM:SS.mmmZ`.
fn utc_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    utc(now.as_secs(), now.subsec_millis())
}

fn utc(seconds: u64, millis: u32) -> String {
    let days = (seconds / 86_400) as i64;
    let time = seconds % 86_400;
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        time / 3600,
        time % 3600 / 60,
        time % 60
    )
}

/// The wgpu setup that avoids two known driver crashes: no OpenGL backend
/// on Windows, where Vulkan or DirectX 12 is always there, and no
/// indirect-call validation, whose compute shader some drivers fail on. The
/// `WGPU_BACKEND` and `WGPU_VALIDATION_INDIRECT_CALL` variables still win.
/// Source: filmcraft `apps/filmcraft/src/graphics.rs`.
pub fn wgpu_options() -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
    let mut setup = WgpuSetupCreateNew::without_display_handle();
    let instance = &mut setup.instance_descriptor;
    if std::env::var_os("WGPU_VALIDATION_INDIRECT_CALL").is_none() {
        instance
            .flags
            .remove(wgpu::InstanceFlags::VALIDATION_INDIRECT_CALL);
    }
    if cfg!(windows) && wgpu::Backends::from_env().is_none() {
        instance.backends.remove(wgpu::Backends::GL);
    }
    WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(setup),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_passwords_are_redacted_and_the_rest_kept() {
        assert_eq!(
            redact("fatal: unable to access 'https://ada:s3cret@example.org/gg.git/': 403"),
            "fatal: unable to access 'https://ada:***@example.org/gg.git/': 403"
        );
        assert_eq!(
            redact("ssh://git@example.org/gg.git and https://example.org/x"),
            "ssh://git@example.org/gg.git and https://example.org/x"
        );
        assert_eq!(redact("no urls here"), "no urls here");
        assert_eq!(redact("trailing https://u:p@h"), "trailing https://u:***@h");
    }

    #[test]
    fn times_are_utc_dates() {
        assert_eq!(utc(0, 0), "1970-01-01T00:00:00.000Z");
        assert_eq!(utc(951_782_400, 5), "2000-02-29T00:00:00.005Z");
        assert_eq!(utc(1_793_750_400 + 3_661, 120), "2026-11-04T01:01:01.120Z");
    }

    #[test]
    fn a_launch_keeps_the_previous_log_and_redacts_and_caps()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("logs").join("gg.log");
        open(&path)?;
        log("launch", "first run");
        open(&path)?;
        log(
            "error",
            "push to https://ada:s3cret@example.org/gg.git failed",
        );
        let previous = std::fs::read_to_string(dir.path().join("logs/gg.previous.log"))?;
        assert!(previous.contains("launch first run"), "{previous}");
        let current = std::fs::read_to_string(&path)?;
        assert!(current.contains("https://ada:***@example.org"), "{current}");
        assert!(!current.contains("s3cret") && !current.contains("first run"));

        if let Ok(mut log) = LOG.lock()
            && let Some(log) = log.as_mut()
        {
            log.written = CAP - 10;
        }
        log("error", "this entry does not fit");
        log("error", "nor this one");
        let capped = std::fs::read_to_string(&path)?;
        assert!(capped.contains("16 MiB cap"), "{capped}");
        assert!(!capped.contains("does not fit") && !capped.contains("nor this one"));
        if let Ok(mut log) = LOG.lock() {
            *log = None;
        }
        Ok(())
    }

    #[test]
    fn the_os_names_the_system_and_architecture() {
        let os = os();
        assert!(os.starts_with(std::env::consts::OS), "{os}");
        assert!(os.ends_with(std::env::consts::ARCH), "{os}");
    }
}
