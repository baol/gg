//! What a destructive dialog will remove (files, commits), read with Git on
//! a background thread when the dialog opens so frames never wait.
//!
//! As in the dialogs proposal, Discard all, Reset, Delete branch, and force
//! push list what they lose before the red primary button names the count.
//! On confirm a [`Guard`] reads the same lists again in the background and
//! runs the action only when they still match what the dialog showed.
use eframe::egui;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

/// At most this many lines are read and listed; the count says when there
/// are more.
const LIMIT: usize = 200;

/// What a dialog shows after a confirmation found the repository changed.
pub const CHANGED: &str = "The repository changed; review the updated list and confirm again.";

/// One dialog's read, kept so a confirmation can run it again.
type Read = Arc<dyn Fn() -> Result<Vec<String>, String> + Send + Sync>;

/// Lines of `git` output (or another background read) for one dialog.
pub struct Lines {
    receiver: Option<Receiver<Result<Vec<String>, String>>>,
    value: Option<Result<Vec<String>, String>>,
    /// The `git` arguments, so a dialog can tell whether it must re-read.
    pub key: Vec<String>,
    read: Read,
}

impl Lines {
    /// Run `git -C <repo> <args>` in the background, one entry per line.
    pub fn git(repo: &Path, args: &[&str], ctx: &egui::Context) -> Self {
        let repo = repo.to_path_buf();
        let owned = args
            .iter()
            .map(|arg| (*arg).to_string())
            .collect::<Vec<_>>();
        let key = owned.clone();
        Self::with(key, ctx, move || {
            let output = gg_git::process::git()
                .env("GIT_OPTIONAL_LOCKS", "0")
                .arg("-C")
                .arg(&repo)
                .args(&owned)
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .output();
            match output {
                Ok(output) if output.status.success() => {
                    Ok(String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .filter(|line| !line.trim().is_empty())
                        .map(str::to_owned)
                        .collect())
                }
                Ok(output) => Err(String::from_utf8_lossy(&output.stderr).trim().to_owned()),
                Err(error) => Err(error.to_string()),
            }
        })
    }

    /// Run `read` in the background; `key` names what it reads.
    pub fn with(
        key: Vec<String>,
        ctx: &egui::Context,
        read: impl Fn() -> Result<Vec<String>, String> + Send + Sync + 'static,
    ) -> Self {
        Self::start(key, Arc::new(read), ctx)
    }

    fn start(key: Vec<String>, read: Read, ctx: &egui::Context) -> Self {
        let (sender, receiver) = channel();
        let ctx = ctx.clone();
        let reader = Arc::clone(&read);
        let _ = std::thread::Builder::new()
            .name("gg-consequence".into())
            .spawn(move || {
                let result = reader().map(|mut lines| {
                    lines.truncate(LIMIT + 1);
                    lines
                });
                let _ = sender.send(result);
                ctx.request_repaint();
            });
        Self {
            receiver: Some(receiver),
            value: None,
            key,
            read,
        }
    }

    /// The same read, started again in the background.
    pub fn again(&self, ctx: &egui::Context) -> Self {
        Self::start(self.key.clone(), Arc::clone(&self.read), ctx)
    }

    /// The lines once read; `None` while Git runs.
    pub fn get(&mut self) -> Option<&Result<Vec<String>, String>> {
        if let Some(receiver) = &self.receiver
            && let Ok(value) = receiver.try_recv()
        {
            self.value = Some(value);
            self.receiver = None;
        }
        self.value.as_ref()
    }

    /// How many entries were read, when known (`LIMIT + 1` means more).
    pub fn count(&mut self) -> Option<usize> {
        match self.get() {
            Some(Ok(lines)) => Some(lines.len()),
            _ => None,
        }
    }
}

/// Re-reads what a destructive dialog showed when the user confirms, so an
/// action never runs on a repository that changed after the dialog opened.
#[derive(Default)]
pub struct Guard {
    pending: Option<Vec<(&'static str, Lines)>>,
    /// A re-read differed from what the dialog showed; the dialog shows
    /// [`CHANGED`] until it closes.
    pub changed: bool,
}

impl Guard {
    /// Start reading `slots` of `shown` again in the background.
    pub fn confirm(
        &mut self,
        shown: &BTreeMap<&'static str, Lines>,
        slots: &[&'static str],
        ctx: &egui::Context,
    ) {
        if self.pending.is_none() {
            self.pending = Some(
                slots
                    .iter()
                    .filter_map(|slot| shown.get(slot).map(|lines| (*slot, lines.again(ctx))))
                    .collect(),
            );
        }
    }

    /// A confirmation is being checked.
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Once every re-read arrived: `Some(true)` when all match what was
    /// shown (run the action), `Some(false)` after putting the new lists in
    /// `shown` in place. `None` while reading or when nothing was confirmed.
    pub fn poll(&mut self, shown: &mut BTreeMap<&'static str, Lines>) -> Option<bool> {
        let pending = self.pending.as_mut()?;
        if pending.iter_mut().any(|(_, lines)| lines.get().is_none()) {
            return None;
        }
        let mut same = true;
        for (slot, mut fresh) in self.pending.take().unwrap_or_default() {
            let old = shown.get_mut(slot).and_then(|lines| lines.get().cloned());
            if old.as_ref() != fresh.get() {
                same = false;
                shown.insert(slot, fresh);
            }
        }
        self.changed |= !same;
        Some(same)
    }
}

/// `3 commits`, `1 file`, `200+ files`.
pub fn counted(count: usize, one: &str, many: &str) -> String {
    match count {
        1 => format!("1 {one}"),
        count if count > LIMIT => format!("{LIMIT}+ {many}"),
        count => format!("{count} {many}"),
    }
}

/// A titled list of `lines` in a fixed-height slot (so the dialog never
/// resizes when the read finishes), each line in `color`.
pub fn list(ui: &mut egui::Ui, title: &str, lines: &mut Lines, color: egui::Color32) {
    ui.label(egui::RichText::new(title).small().weak());
    let rows = 6.0;
    let height = rows * crate::density::row();
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.set_min_height(height);
            egui::ScrollArea::vertical()
                .id_salt(("consequence", title))
                .max_height(height)
                .auto_shrink([false, true])
                .show(ui, |ui| match lines.get() {
                    // The slot stays empty while Git runs.
                    None => {}
                    Some(Err(error)) => {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    Some(Ok(entries)) if entries.is_empty() => {
                        ui.weak("Nothing");
                    }
                    Some(Ok(entries)) => {
                        for entry in entries.iter().take(LIMIT) {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(entry)
                                        .monospace()
                                        .size(11.5)
                                        .color(color),
                                )
                                .truncate(),
                            );
                        }
                        if entries.len() > LIMIT {
                            ui.weak("…");
                        }
                    }
                });
        });
}

/// One reserved row above a destructive dialog's footer for `text` (a
/// notice or why the primary is disabled), so the footer never moves.
pub fn notice(ui: &mut egui::Ui, text: Option<&str>) {
    let row = crate::density::row();
    ui.allocate_ui(egui::vec2(ui.available_width(), row), |ui| {
        ui.set_min_height(row);
        if let Some(text) = text {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(text).color(egui::Color32::from_rgb(220, 195, 125)),
                )
                .truncate(),
            )
            .on_hover_text(text);
        }
    });
}

/// The filled red primary button of a destructive dialog.
///
/// It shows no key: a destructive dialog focuses Cancel, so Enter never
/// presses it.
pub fn danger_button<'a>(label: impl Into<String>, enabled: bool) -> egui::Button<'a> {
    egui::Button::new(danger_text(label, enabled)).fill(danger_fill(enabled))
}

/// [`danger_button`] with a key hint, for the few red buttons a key does
/// press (the conflict question and the stash drop question, where Enter
/// accepts).
pub fn danger_button_with_key<'a>(
    label: impl Into<String>,
    enabled: bool,
    key: &'a str,
) -> egui::Button<'a> {
    egui::Button::new((
        danger_text(label, enabled),
        egui::RichText::new(key).small().weak(),
    ))
    .fill(danger_fill(enabled))
}

fn danger_text(label: impl Into<String>, enabled: bool) -> egui::RichText {
    egui::RichText::new(label.into()).color(if enabled {
        egui::Color32::from_rgb(245, 160, 160)
    } else {
        egui::Color32::from_gray(110)
    })
}

fn danger_fill(enabled: bool) -> egui::Color32 {
    if enabled {
        egui::Color32::from_rgb(74, 36, 36)
    } else {
        egui::Color32::from_gray(42)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait(lines: &mut Lines) {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while lines.get().is_none() && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    fn init() -> Result<tempfile::TempDir, String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["init", "-q"])
            .status()
            .map_err(|e| e.to_string())?;
        assert!(status.success());
        Ok(dir)
    }

    #[test]
    fn counts_read_naturally() {
        assert_eq!(counted(1, "file", "files"), "1 file");
        assert_eq!(counted(4, "file", "files"), "4 files");
        assert_eq!(counted(LIMIT + 1, "commit", "commits"), "200+ commits");
    }

    #[test]
    fn lines_come_from_git_in_the_background() -> Result<(), String> {
        let dir = init()?;
        std::fs::write(dir.path().join("a"), "x").map_err(|e| e.to_string())?;
        let ctx = egui::Context::default();
        let mut lines = Lines::git(dir.path(), &["status", "--porcelain=v1"], &ctx);
        wait(&mut lines);
        assert_eq!(lines.get(), Some(&Ok(vec!["?? a".to_string()])));
        Ok(())
    }

    #[test]
    fn guard_confirms_only_what_the_dialog_showed() -> Result<(), String> {
        let dir = init()?;
        std::fs::write(dir.path().join("a"), "x").map_err(|e| e.to_string())?;
        let ctx = egui::Context::default();
        let mut shown = BTreeMap::new();
        let mut lines = Lines::git(dir.path(), &["status", "--porcelain=v1"], &ctx);
        wait(&mut lines);
        shown.insert("files", lines);
        let settle = |guard: &mut Guard, shown: &mut BTreeMap<&'static str, Lines>| {
            let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                if let Some(verdict) = guard.poll(shown) {
                    return Some(verdict);
                }
                if std::time::Instant::now() > until {
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        };

        let mut guard = Guard::default();
        assert_eq!(guard.poll(&mut shown), None, "nothing confirmed yet");
        guard.confirm(&shown, &["files"], &ctx);
        assert!(guard.busy());
        assert_eq!(settle(&mut guard, &mut shown), Some(true), "unchanged");
        assert!(!guard.changed);

        // Changed outside gg between opening and confirming.
        std::fs::write(dir.path().join("b"), "y").map_err(|e| e.to_string())?;
        guard.confirm(&shown, &["files"], &ctx);
        assert_eq!(settle(&mut guard, &mut shown), Some(false), "changed");
        assert!(guard.changed);
        let updated = shown.get_mut("files").and_then(Lines::get).cloned();
        assert_eq!(
            updated,
            Some(Ok(vec!["?? a".to_string(), "?? b".to_string()])),
            "the list is updated in place"
        );
        guard.confirm(&shown, &["files"], &ctx);
        assert_eq!(settle(&mut guard, &mut shown), Some(true), "confirm again");
        Ok(())
    }
}
