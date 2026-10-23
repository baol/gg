//! Private rewrite backups with recovery actions.
use eframe::egui;
use gg_engine::recovery::Backup;
use std::path::PathBuf;

pub enum Action {
    Open {
        commit: String,
        subject: String,
    },
    Reset {
        commit: String,
        mode: gg_engine::reset::Mode,
    },
    CreateBranch {
        commit: String,
        selector: String,
    },
}

pub struct BackupsView {
    path: PathBuf,
    entries: Vec<Backup>,
    selected: Option<usize>,
    error: Option<String>,
}

impl BackupsView {
    pub fn new(path: PathBuf) -> Self {
        let mut view = Self {
            path,
            entries: Vec::new(),
            selected: None,
            error: None,
        };
        view.load();
        view
    }

    pub fn load(&mut self) {
        match gg_git::backup::list(&self.path) {
            Ok(entries) => {
                self.entries = entries;
                self.selected = self
                    .selected
                    .filter(|selected| *selected < self.entries.len())
                    .or_else(|| (!self.entries.is_empty()).then_some(0));
                self.error = None;
            }
            Err(error) => {
                self.entries.clear();
                self.selected = None;
                self.error = Some(error);
            }
        }
    }

    pub fn draw(&mut self, ctx: &egui::Context, open: &mut bool) -> Option<Action> {
        let mut action = None;
        egui::Window::new("Backups")
            .id(egui::Id::unique("rewrite-backups"))
            .open(open)
            .collapsible(false)
            .resizable(true)
            .default_width(720.0)
            .default_height(460.0)
            .default_pos(ctx.content_rect().center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Private refs saved before history rewrites. They stay out of the refs sidebar.");
                    if ui.button("Refresh").clicked() {
                        self.load();
                    }
                });
                ui.separator();
                if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                if self.entries.is_empty() {
                    if self.error.is_none() {
                        ui.label("No rewrite backups.");
                    }
                    return;
                }
                egui::ScrollArea::vertical()
                    .id_salt("rewrite-backup-entries")
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for (index, entry) in self.entries.iter().enumerate() {
                            let selected = self.selected == Some(index);
                            let text = format!(
                                "{}  {}  {}",
                                entry.label,
                                entry.commit.get(..8).unwrap_or(&entry.commit),
                                entry.subject
                            );
                            if ui.selectable_label(selected, text).clicked() {
                                self.selected = Some(index);
                            }
                        }
                    });
                ui.separator();
                let Some(entry) = self.selected.and_then(|index| self.entries.get(index)) else {
                    ui.label("Select a backup to inspect it.");
                    return;
                };
                ui.label(egui::RichText::new(&entry.reference).monospace());
                ui.label(format!(
                    "{} · {} · {}",
                    entry.label,
                    entry.commit,
                    entry.time
                ));
                ui.horizontal(|ui| {
                    if ui.button("Open commit").clicked() {
                        action = Some(Action::Open {
                            commit: entry.commit.clone(),
                            subject: entry.subject.clone(),
                        });
                    }
                    ui.menu_button("Reset branch here", |ui| {
                        for (label, mode) in [
                            ("Soft", gg_engine::reset::Mode::Soft),
                            ("Mixed", gg_engine::reset::Mode::Mixed),
                            ("Hard", gg_engine::reset::Mode::Hard),
                        ] {
                            if ui.button(label).clicked() {
                                action = Some(Action::Reset {
                                    commit: entry.commit.clone(),
                                    mode,
                                });
                                ui.close();
                            }
                        }
                    });
                    if ui.button("Create branch here…").clicked() {
                        action = Some(Action::CreateBranch {
                            commit: entry.commit.clone(),
                            selector: entry.reference.clone(),
                        });
                    }
                    if ui.button("Copy hash").clicked() {
                        ui.ctx().copy_text(entry.commit.clone());
                    }
                    if ui.button("Copy ref").clicked() {
                        ui.ctx().copy_text(entry.reference.clone());
                    }
                });
            });
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{error::Error, path::Path, process::Command};
    type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

    fn git(path: &Path, args: &[&str]) -> Result {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.org")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.org")
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }

    fn repo() -> Result<tempfile::TempDir> {
        let dir = tempfile::tempdir()?;
        git(dir.path(), &["init", "-b", "main"])?;
        git(dir.path(), &["config", "user.name", "Test"])?;
        git(dir.path(), &["config", "user.email", "test@example.org"])?;
        std::fs::write(dir.path().join("a"), b"base\n")?;
        git(dir.path(), &["add", "a"])?;
        git(dir.path(), &["commit", "-qm", "base"])?;
        let head = String::from_utf8(
            Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(["rev-parse", "HEAD"])
                .output()?
                .stdout,
        )?
        .trim()
        .to_string();
        gg_git::backup::create(dir.path(), "rebase", &head)?;
        Ok(dir)
    }

    #[test]
    fn lists_and_renders_rewrite_backups() -> Result {
        let dir = repo()?;
        let mut view = BackupsView::new(dir.path().into());
        assert!(view.error.is_none(), "{:?}", view.error);
        assert_eq!(view.entries.len(), 1);
        assert_eq!(view.entries[0].label, "rebase");
        assert_eq!(view.selected, Some(0));

        let ctx = egui::Context::default();
        let mut open = true;
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1120.0, 740.0),
            )),
            ..Default::default()
        };
        let _ = crate::tests::run_ui(&ctx, input(), |ui| {
            view.draw(ui.ctx(), &mut open);
        });
        let output = crate::tests::run_ui(&ctx, input(), |ui| {
            view.draw(ui.ctx(), &mut open);
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for expected in [
            "Backups",
            "rebase",
            "Open commit",
            "Reset branch here",
            "Create branch here…",
            "Copy ref",
        ] {
            assert!(
                text.iter().any(|line| line.contains(expected)),
                "missing {expected}: {text:?}"
            );
        }
        Ok(())
    }
}
