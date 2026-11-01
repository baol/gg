//! Repository › New repository… (and the welcome view's Initialize here…):
//! a folder, the first branch, and whether to commit what is there.
use crate::welcome_view::tilde;
use eframe::egui;
use std::{
    path::{Path, PathBuf},
    sync::mpsc::Receiver,
};

/// What the background read learned about the chosen folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inspection {
    pub exists: bool,
    /// The repository that already contains the folder.
    pub enclosing: Option<PathBuf>,
    pub count: Option<gg_git::init::Count>,
}

pub enum Action {
    Created(gg_git::init::Receipt),
    /// Open the repository the folder is already inside.
    OpenRepository(PathBuf),
    Closed,
}

pub struct Dialog {
    pub folder: String,
    pub branch: String,
    pub commit: bool,
    pub error: Option<String>,
    inspection: Option<(String, Inspection)>,
    inspecting: Option<(String, Receiver<Inspection>)>,
    job: Option<Receiver<Result<gg_git::init::Receipt, String>>>,
}

pub fn inspect(path: &Path) -> Inspection {
    let exists = path.is_dir();
    let enclosing = gg_git::init::enclosing_repository(path);
    let count = (exists && enclosing.is_none())
        .then(|| gg_git::init::count_files(path).ok())
        .flatten();
    Inspection {
        exists,
        enclosing,
        count,
    }
}

impl Dialog {
    pub fn new(folder: Option<PathBuf>) -> Self {
        Self {
            folder: folder
                .map(|folder| folder.to_string_lossy().into_owned())
                .unwrap_or_default(),
            branch: "main".into(),
            commit: true,
            error: None,
            inspection: None,
            inspecting: None,
            job: None,
        }
    }

    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// The read for the folder as typed, once it has arrived.
    pub fn inspection(&self) -> Option<&Inspection> {
        self.inspection
            .as_ref()
            .filter(|(folder, _)| *folder == self.folder.trim())
            .map(|(_, inspection)| inspection)
    }

    /// Start reading the folder when it changed; keep the result that
    /// matches what is typed.
    fn poll_inspection(&mut self, ctx: &egui::Context) {
        let folder = self.folder.trim().to_string();
        if let Some((asked, rx)) = &self.inspecting
            && let Ok(inspection) = rx.try_recv()
        {
            self.inspection = Some((asked.clone(), inspection));
            self.inspecting = None;
        }
        let current = self
            .inspection
            .as_ref()
            .is_some_and(|(asked, _)| *asked == folder)
            || self
                .inspecting
                .as_ref()
                .is_some_and(|(asked, _)| *asked == folder);
        if current || !Path::new(&folder).is_absolute() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let repaint = ctx.clone();
        let path = PathBuf::from(&folder);
        std::thread::spawn(move || {
            let _ = tx.send(inspect(&path));
            repaint.request_repaint();
        });
        self.inspecting = Some((folder, rx));
    }

    fn start(&mut self, ctx: &egui::Context) {
        let path = PathBuf::from(self.folder.trim());
        let branch = self.branch.trim().to_string();
        let commit = self.commit;
        let (tx, rx) = std::sync::mpsc::channel();
        let repaint = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(gg_git::init::init(&path, &branch, commit));
            repaint.request_repaint();
        });
        self.error = None;
        self.job = Some(rx);
    }

    /// Wait for a running create; `Some` once it finished well.
    pub fn poll_job(&mut self) -> Option<gg_git::init::Receipt> {
        let result = self.job.as_ref()?.try_recv().ok()?;
        self.job = None;
        match result {
            Ok(receipt) => Some(receipt),
            Err(error) => {
                self.error = Some(error);
                None
            }
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, home: Option<&str>) -> Option<Action> {
        self.poll_inspection(ctx);
        if let Some(receipt) = self.poll_job() {
            return Some(Action::Created(receipt));
        }
        let mut action = None;
        let mut create = false;
        let busy = self.busy();
        let modal = egui::Modal::new(egui::Id::unique("new-repository"))
            .backdrop_color(egui::Color32::from_black_alpha(140))
            .show(ctx, |ui| {
                ui.set_width(520.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("New repository").size(16.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("×").on_hover_text("Close (Esc)").clicked() {
                            action = Some(Action::Closed);
                        }
                    });
                });
                ui.separator();
                ui.label(egui::RichText::new("FOLDER").small().weak());
                ui.horizontal(|ui| {
                    let field = ui.add_enabled(
                        !busy,
                        egui::TextEdit::singleline(&mut self.folder)
                            .desired_width(420.0)
                            .hint_text("Folder for the new repository"),
                    );
                    if self.folder.is_empty() && !field.has_focus() {
                        field.request_focus();
                    }
                    if ui
                        .add_enabled(!busy, egui::Button::new("Choose…"))
                        .on_hover_text("Choose a folder with the desktop dialog")
                        .clicked()
                        && let Some(path) = crate::folder_picker::pick_folder(
                            "New repository",
                            Some(Path::new(self.folder.trim())),
                        )
                    {
                        self.folder = path.to_string_lossy().into_owned();
                    }
                });
                ui.add_space(6.0);
                ui.label(egui::RichText::new("FIRST BRANCH").small().weak());
                ui.add_enabled(
                    !busy,
                    egui::TextEdit::singleline(&mut self.branch).desired_width(200.0),
                );
                ui.add_space(6.0);
                ui.add_enabled(
                    !busy,
                    egui::Checkbox::new(
                        &mut self.commit,
                        "Commit the existing files as \u{201c}Initial commit\u{201d}",
                    ),
                );
                // One reserved line: what is in the folder, or why not here.
                let folder = PathBuf::from(self.folder.trim());
                let shown = tilde(&folder, home);
                let inspection = self.inspection().cloned();
                let warn = ui.visuals().warn_fg_color;
                ui.allocate_ui_with_layout(
                    egui::vec2(500.0, 24.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_size(egui::vec2(500.0, 24.0));
                        if self.folder.trim().is_empty() {
                        } else if !folder.is_absolute() {
                            ui.colored_label(warn, "Choose an absolute folder.");
                        } else if let Some(inspection) = &inspection {
                            if let Some(repository) = &inspection.enclosing {
                                let repository_shown = tilde(repository, home);
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(format!(
                                            "{shown} is inside the repository {repository_shown}. Open it instead?"
                                        ))
                                        .color(warn),
                                    )
                                    .truncate(),
                                );
                                if ui.button(format!("Open {repository_shown}")).clicked() {
                                    action = Some(Action::OpenRepository(repository.clone()));
                                }
                            } else if !inspection.exists {
                                ui.weak(format!("{shown} will be created."));
                            } else if let Some(count) = inspection.count {
                                ui.weak(files_line(&shown, count));
                            }
                        }
                    },
                );
                if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.separator();
                let ready = !busy
                    && folder.is_absolute()
                    && !self.branch.trim().is_empty()
                    && inspection
                        .as_ref()
                        .is_some_and(|inspection| inspection.enclosing.is_none());
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let primary = egui::Button::new((
                            egui::RichText::new("Create repository").color(egui::Color32::WHITE),
                            egui::RichText::new("Enter")
                                .small()
                                .color(egui::Color32::from_rgb(184, 212, 230)),
                        ))
                        .fill(crate::welcome_view::PRIMARY);
                        create |= ui
                            .add_enabled(ready, primary)
                            .on_disabled_hover_text(
                                "Choose an absolute folder outside any repository and name the first branch",
                            )
                            .clicked();
                        if ui
                            .button(("Cancel", egui::RichText::new("Esc").small().weak()))
                            .clicked()
                        {
                            action = Some(Action::Closed);
                        }
                    });
                });
                create |= ready && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
        if modal.should_close() && !busy {
            action = Some(Action::Closed);
        }
        if create && action.is_none() {
            self.start(ctx);
        }
        action
    }
}

/// `~/notes has 14 files (3 ignored by .gitignore).`
pub fn files_line(shown: &str, count: gg_git::init::Count) -> String {
    let files = match count.files {
        0 => return format!("{shown} is empty."),
        1 => "1 file".to_string(),
        files => format!("{files} files"),
    };
    if count.ignored == 0 {
        format!("{shown} has {files}.")
    } else {
        format!(
            "{shown} has {files} ({} ignored by .gitignore).",
            count.ignored
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_files_line_counts_what_will_be_committed() {
        let count = |files, ignored| gg_git::init::Count { files, ignored };
        assert_eq!(
            files_line("~/notes", count(14, 3)),
            "~/notes has 14 files (3 ignored by .gitignore)."
        );
        assert_eq!(files_line("~/notes", count(1, 0)), "~/notes has 1 file.");
        assert_eq!(files_line("~/notes", count(0, 0)), "~/notes is empty.");
    }
}
