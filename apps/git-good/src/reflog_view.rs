//! Read-only reflog browser with confirmed recovery actions.
use eframe::egui;
use gg_engine::reflog::Entry;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub enum Action {
    Open {
        commit: String,
        subject: String,
        selector: String,
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

pub struct ReflogView {
    path: PathBuf,
    reference: String,
    entries: Vec<Entry>,
    selected: Option<usize>,
    error: Option<String>,
    notice: Option<String>,
}

impl ReflogView {
    pub fn new(path: PathBuf, reference: String) -> Self {
        let mut view = Self {
            path,
            reference,
            entries: Vec::new(),
            selected: None,
            error: None,
            notice: None,
        };
        view.load();
        view
    }

    pub fn load(&mut self) {
        match gg_git::reflog::read(&self.path, &self.reference) {
            Ok(snapshot) => {
                self.entries = snapshot.entries;
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

    pub fn select_commit(&mut self, commit: &str) {
        if let Some(index) = self.entries.iter().position(|entry| entry.id == commit) {
            self.selected = Some(index);
        }
    }

    pub fn notice(&mut self, notice: String) {
        self.notice = Some(notice);
        self.error = None;
        self.load();
    }

    #[cfg(test)]
    pub fn draw(
        &mut self,
        ctx: &egui::Context,
        references: &[String],
        open: &mut bool,
    ) -> Option<Action> {
        let mut action = None;
        egui::Window::new("Reflog")
            .id(egui::Id::unique("reflog"))
            .open(open)
            .collapsible(false)
            .resizable(true)
            .default_width(680.0)
            .default_height(460.0)
            .default_pos(ctx.content_rect().center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                action = self.content(ui, references);
            });
        action
    }

    /// Render the reflog body without creating a window.
    pub fn content(&mut self, ui: &mut egui::Ui, references: &[String]) -> Option<Action> {
        let mut action = None;
        ui.horizontal(|ui| {
            ui.label("Reference");
            let mut reference = self.reference.clone();
            egui::ComboBox::from_id_salt("reflog-reference")
                .selected_text(&reference)
                .width(200.0)
                .show_ui(ui, |ui| {
                    for candidate in references {
                        ui.selectable_value(&mut reference, candidate.clone(), candidate);
                    }
                });
            if reference != self.reference {
                self.reference = reference;
                self.selected = None;
                self.load();
            }
            if ui.button("Refresh").clicked() {
                self.load();
            }
        });
        ui.separator();
        if let Some(notice) = &self.notice {
            ui.colored_label(ui.visuals().hyperlink_color, notice);
        }
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        if self.entries.is_empty() {
            if self.error.is_none() {
                ui.label("No reflog entries.");
            }
            return action;
        }
        egui::ScrollArea::vertical()
            .id_salt("reflog-entries")
            .max_height(300.0)
            .show(ui, |ui| {
                for (index, entry) in self.entries.iter().enumerate() {
                    let selected = self.selected == Some(index);
                    let text = format!(
                        "{}  {}  {}",
                        entry.selector,
                        entry.id.get(..8).unwrap_or(&entry.id),
                        summary(entry)
                    );
                    if ui.selectable_label(selected, text).clicked() {
                        self.selected = Some(index);
                    }
                }
            });
        ui.separator();
        let Some(entry) = self.selected.and_then(|index| self.entries.get(index)) else {
            ui.label("Select an entry to inspect it.");
            return action;
        };
        ui.label(egui::RichText::new(&entry.id).monospace());
        ui.label(format!(
            "{} · {}",
            relative_time(entry.time),
            summary(entry)
        ));
        ui.horizontal(|ui| {
            if ui.button("Open commit").clicked() {
                action = Some(Action::Open {
                    commit: entry.id.clone(),
                    subject: summary(entry),
                    selector: entry.selector.clone(),
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
                            commit: entry.id.clone(),
                            mode,
                        });
                        ui.close();
                    }
                }
            });
            if ui.button("Create branch here…").clicked() {
                action = Some(Action::CreateBranch {
                    commit: entry.id.clone(),
                    selector: entry.selector.clone(),
                });
            }
            if ui.button("Copy hash").clicked() {
                ui.ctx().copy_text(entry.id.clone());
            }
        });
        action
    }
}

fn summary(entry: &Entry) -> String {
    if entry.message.is_empty() {
        entry.action.clone()
    } else {
        format!("{}: {}", entry.action, entry.message)
    }
}

fn relative_time(seconds: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default();
    let delta = (now - seconds).max(0);
    if delta < 60 {
        "just now".into()
    } else if delta < 3_600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86_400 {
        format!("{}h ago", delta / 3_600)
    } else {
        format!("{}d ago", delta / 86_400)
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
        std::fs::write(dir.path().join("a"), b"next\n")?;
        git(dir.path(), &["commit", "-qam", "next"])?;
        Ok(dir)
    }

    #[test]
    fn loads_entries_and_renders_recovery_actions() -> Result {
        let dir = repo()?;
        let mut view = ReflogView::new(dir.path().into(), "HEAD".into());
        assert!(view.error.is_none(), "{:?}", view.error);
        assert!(view.entries.len() >= 2);
        assert_eq!(view.selected, Some(0));

        let ctx = egui::Context::default();
        let mut open = true;
        let references = vec!["HEAD".to_string(), "main".to_string()];
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1120.0, 740.0),
            )),
            ..Default::default()
        };
        let _ = crate::tests::run_ui(&ctx, input(), |ui| {
            view.draw(ui.ctx(), &references, &mut open);
        });
        let output = crate::tests::run_ui(
            &ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1120.0, 740.0),
                )),
                ..Default::default()
            },
            |ui| {
                view.draw(ui.ctx(), &references, &mut open);
            },
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for expected in [
            "Reflog",
            "HEAD@{0}",
            "Reset branch here",
            "Create branch here…",
            "Copy hash",
        ] {
            assert!(
                text.iter().any(|line| line.contains(expected)),
                "missing {expected}: {text:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn missing_reference_shows_an_error_instead_of_stale_entries() -> Result {
        let dir = repo()?;
        let mut view = ReflogView::new(dir.path().into(), "HEAD".into());
        assert!(!view.entries.is_empty());
        view.reference = "missing".into();
        view.load();
        assert!(view.entries.is_empty());
        assert!(view.error.as_deref().is_some_and(|e| e.contains("failed")));
        Ok(())
    }
}
