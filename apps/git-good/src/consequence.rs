//! What a destructive dialog will remove (files, commits), read with Git on
//! a background thread when the dialog opens so frames never wait.
//!
//! As in the dialogs proposal, Discard all, Reset, Delete branch, and force
//! push list what they lose before the red primary button names the count.
use eframe::egui;
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};

/// At most this many lines are read and listed; the count says when there
/// are more.
const LIMIT: usize = 200;

/// Lines of `git` output for one dialog.
pub struct Lines {
    receiver: Option<Receiver<Result<Vec<String>, String>>>,
    value: Option<Result<Vec<String>, String>>,
    /// The `git` arguments, so a dialog can tell whether it must re-read.
    pub key: Vec<String>,
}

impl Lines {
    /// Run `git -C <repo> <args>` in the background, one entry per line.
    pub fn git(repo: &Path, args: &[&str], ctx: &egui::Context) -> Self {
        let (sender, receiver) = channel();
        let repo = repo.to_path_buf();
        let owned = args
            .iter()
            .map(|arg| (*arg).to_string())
            .collect::<Vec<_>>();
        let key = owned.clone();
        let ctx = ctx.clone();
        let _ = std::thread::Builder::new()
            .name("gg-consequence".into())
            .spawn(move || {
                let output = std::process::Command::new("git")
                    .arg("-C")
                    .arg(&repo)
                    .args(&owned)
                    .env_remove("GIT_DIR")
                    .env_remove("GIT_WORK_TREE")
                    .output();
                let result = match output {
                    Ok(output) if output.status.success() => {
                        Ok(String::from_utf8_lossy(&output.stdout)
                            .lines()
                            .filter(|line| !line.trim().is_empty())
                            .take(LIMIT + 1)
                            .map(str::to_owned)
                            .collect())
                    }
                    Ok(output) => Err(String::from_utf8_lossy(&output.stderr).trim().to_owned()),
                    Err(error) => Err(error.to_string()),
                };
                let _ = sender.send(result);
                ctx.request_repaint();
            });
        Self {
            receiver: Some(receiver),
            value: None,
            key,
        }
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

/// The filled red primary button of a destructive dialog.
pub fn danger_button<'a>(label: impl Into<String>, enabled: bool) -> egui::Button<'a> {
    let text = egui::RichText::new(label.into()).color(if enabled {
        egui::Color32::from_rgb(245, 160, 160)
    } else {
        egui::Color32::from_gray(110)
    });
    egui::Button::new((text, egui::RichText::new("Enter").small().weak())).fill(if enabled {
        egui::Color32::from_rgb(74, 36, 36)
    } else {
        egui::Color32::from_gray(42)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_read_naturally() {
        assert_eq!(counted(1, "file", "files"), "1 file");
        assert_eq!(counted(4, "file", "files"), "4 files");
        assert_eq!(counted(LIMIT + 1, "commit", "commits"), "200+ commits");
    }

    #[test]
    fn lines_come_from_git_in_the_background() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["init", "-q"])
            .status()
            .map_err(|e| e.to_string())?;
        assert!(status.success());
        std::fs::write(dir.path().join("a"), "x").map_err(|e| e.to_string())?;
        let ctx = egui::Context::default();
        let mut lines = Lines::git(dir.path(), &["status", "--porcelain=v1"], &ctx);
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while lines.get().is_none() && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(lines.get(), Some(&Ok(vec!["?? a".to_string()])));
        Ok(())
    }
}
