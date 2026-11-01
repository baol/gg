//! Explicit export to a new destination; temporary output is removed on failure/cancel.
use crate::navigation::ModalWindow;
use eframe::egui;
use gg_engine::review::FileSide;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

pub fn save(
    repo: &Path,
    side: &FileSide,
    destination: &Path,
    cancelled: impl Fn() -> bool,
) -> Result<u64, String> {
    gg_git::blob::check_destination(repo, destination)?;
    if destination.exists() {
        return Err("Destination already exists; choose a new file".into());
    }
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    let size = gg_git::blob::export(repo, side, file.as_file_mut(), &cancelled)?;
    if cancelled() {
        return Err("Export cancelled".into());
    }
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    if cancelled() {
        return Err("Export cancelled".into());
    }
    file.persist_noclobber(destination)
        .map_err(|e| e.error.to_string())?;
    Ok(size)
}
/// What ⋯ › Export before…/after… names in the dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub path: String,
    /// `Before` or `After` (or the comparison's own side labels).
    pub side: &'static str,
    /// The side's endpoint, such as a short commit ID or `index`.
    pub version: Option<String>,
    pub size: Option<u64>,
}

impl Choice {
    /// `Export after version`.
    pub fn title(&self) -> String {
        format!("Export {} version", self.side.to_lowercase())
    }

    /// `After 96274d95 · 22.7 KiB`.
    pub fn version_line(&self) -> String {
        let mut line = self.side.to_string();
        if let Some(version) = &self.version {
            line.push(' ');
            line.push_str(version);
        }
        if let Some(size) = self.size {
            line.push_str(" · ");
            line.push_str(&bytes(size));
        }
        line
    }

    /// The suggested file name: `logo-after.png`.
    pub fn file_name(&self) -> String {
        let name = self.path.rsplit('/').next().unwrap_or(&self.path);
        let side = self.side.to_lowercase();
        match name.rsplit_once('.') {
            Some((stem, extension)) if !stem.is_empty() => format!("{stem}-{side}.{extension}"),
            _ => format!("{name}-{side}"),
        }
    }
}

fn bytes(value: u64) -> String {
    if value < 1024 {
        format!("{value} B")
    } else if value < 1024 * 1024 {
        format!("{:.1} KiB", value as f64 / 1024.0)
    } else {
        format!("{:.1} MiB", value as f64 / (1024.0 * 1024.0))
    }
}

/// The Export dialog, in the Preferences dialog shape: the title names the
/// side, a File / Version / Save to grid with **Browse…** (the native save
/// dialog), an inline error under the field, and **Cancel** (`Esc`) beside
/// the filled **Export** (`Enter`), which reads **Exporting…** while busy.
/// The result is reported in the window footer.
#[derive(Default)]
pub struct Export {
    selected: Option<(PathBuf, FileSide, Choice)>,
    destination: String,
    error: Option<String>,
    receipt: Option<String>,
    job: Option<mpsc::Receiver<Result<u64, String>>>,
    stop: Arc<AtomicBool>,
}
impl Export {
    pub fn is_open(&self) -> bool {
        self.selected.is_some()
    }
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn choose(&mut self, repo: PathBuf, side: FileSide, choice: Choice) {
        if self.job.is_none() {
            self.selected = Some((repo, side, choice));
            self.error = None;
        }
    }
    /// The footer receipt of a finished export: `Exported 23 244 bytes to …`.
    pub fn take_receipt(&mut self) -> Option<String> {
        self.receipt.take()
    }
    fn start(&mut self, ctx: &egui::Context, repo: PathBuf, side: FileSide) {
        self.stop = Arc::new(AtomicBool::new(false));
        let stop = self.stop.clone();
        let destination = PathBuf::from(self.destination.trim());
        let (send, receive) = mpsc::channel();
        let ctx = ctx.clone();
        match std::thread::Builder::new()
            .name("gg-export".into())
            .spawn(move || {
                let result = save(&repo, &side, &destination, || stop.load(Ordering::Relaxed));
                let _ = send.send(result);
                ctx.request_repaint();
            }) {
            Ok(_) => {
                self.job = Some(receive);
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }
    pub fn draw(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.job.as_ref().and_then(|job| job.try_recv().ok()) {
            self.job = None;
            match result {
                Ok(n) => {
                    self.receipt = Some(format!(
                        "Exported {} bytes to {}",
                        grouped(n),
                        self.destination.trim()
                    ));
                    self.selected = None;
                }
                Err(e) if self.stop.load(Ordering::Relaxed) && self.selected.is_none() => {
                    let _ = e;
                }
                Err(e) => {
                    self.error = Some(if e.contains("already exists") {
                        "That file already exists. Choose a new name.".into()
                    } else {
                        e
                    });
                }
            }
        }
        let Some((repo, side, choice)) = self.selected.clone() else {
            return;
        };
        let busy = self.job.is_some();
        let mut close = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        let mut export = false;
        let id = egui::Id::unique("export-file");
        egui::Window::new(choice.title())
            .id(id)
            .modal(ctx, id)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_pos(ctx.content_rect().center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                ui.set_width(480.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(choice.title()).size(16.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close |= ui.button("×").on_hover_text("Close (Esc)").clicked();
                    });
                });
                ui.separator();
                egui::Grid::new("export-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.weak("File");
                        ui.monospace(&choice.path).on_hover_text(&side.id);
                        ui.end_row();
                        ui.weak("Version");
                        ui.label(choice.version_line());
                        ui.end_row();
                        ui.weak("Save to");
                        ui.horizontal(|ui| {
                            ui.add_enabled(
                                !busy,
                                egui::TextEdit::singleline(&mut self.destination)
                                    .hint_text("Destination path")
                                    .desired_width(320.0),
                            );
                            if ui
                                .add_enabled(!busy, egui::Button::new("Browse…"))
                                .on_hover_text("Choose where to save with the desktop dialog")
                                .clicked()
                            {
                                let mut dialog = rfd::FileDialog::new()
                                    .set_title(choice.title())
                                    .set_file_name(choice.file_name());
                                if let Some(folder) = Path::new(self.destination.trim())
                                    .parent()
                                    .filter(|folder| folder.is_dir())
                                {
                                    dialog = dialog.set_directory(folder);
                                }
                                if let Some(path) = dialog.save_file() {
                                    self.destination = path.to_string_lossy().into_owned();
                                    self.error = None;
                                }
                            }
                        });
                        ui.end_row();
                        ui.label("");
                        // One reserved line, so the footer never moves.
                        ui.allocate_ui(egui::vec2(400.0, 18.0), |ui| {
                            ui.set_min_height(18.0);
                            if let Some(error) = &self.error {
                                ui.colored_label(ui.visuals().error_fg_color, error);
                            }
                        });
                        ui.end_row();
                    });
                ui.separator();
                let ready = !busy && !self.destination.trim().is_empty();
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let primary = if busy {
                            egui::Button::new(
                                egui::RichText::new("Exporting…").color(egui::Color32::WHITE),
                            )
                        } else {
                            egui::Button::new((
                                egui::RichText::new("Export").color(egui::Color32::WHITE),
                                egui::RichText::new("Enter")
                                    .small()
                                    .color(egui::Color32::from_rgb(184, 212, 230)),
                            ))
                        }
                        .fill(crate::welcome_view::PRIMARY);
                        export |= ui
                            .add_enabled(ready, primary)
                            .on_disabled_hover_text(if busy {
                                "Exporting…"
                            } else {
                                "Choose where to save the file"
                            })
                            .clicked();
                        close |= ui
                            .button(("Cancel", egui::RichText::new("Esc").small().weak()))
                            .clicked();
                    });
                });
                export |= ready && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
        if close {
            self.stop.store(true, Ordering::Relaxed);
            self.selected = None;
        } else if export && !busy {
            self.start(ctx, repo, side);
        }
    }
}

/// `23244` → `23 244`, as the board writes byte counts.
fn grouped(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(' ');
        }
        out.push(digit);
    }
    out
}
impl Drop for Export {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escape_closes_export_and_cancels_its_work() {
        let mut export = Export::default();
        export.choose(
            "repo".into(),
            FileSide {
                id: "a".repeat(40),
                mode: 0o100644,
            },
            Choice {
                path: "assets/logo.png".into(),
                side: "After",
                version: Some("96274d95".into()),
                size: Some(23_244),
            },
        );
        let ctx = egui::Context::default();
        let event = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(720.0, 480.0, vec![event]),
            |ctx| export.draw(ctx.ctx()),
        );
        assert!(!export.is_open());
        assert!(export.stop.load(Ordering::Relaxed));
    }
    #[test]
    fn the_dialog_names_the_side_version_and_a_suggested_file() {
        let choice = Choice {
            path: "assets/logo.png".into(),
            side: "After",
            version: Some("96274d95".into()),
            size: Some(23_244),
        };
        assert_eq!(choice.title(), "Export after version");
        assert_eq!(choice.version_line(), "After 96274d95 · 22.7 KiB");
        assert_eq!(choice.file_name(), "logo-after.png");
        assert_eq!(grouped(23_244), "23 244");
        assert_eq!(grouped(7), "7");
    }

    #[test]
    fn a_finished_export_closes_and_reports_in_the_footer() -> Result<(), Box<dyn std::error::Error>>
    {
        use std::process::Command;
        let repo = tempfile::tempdir()?;
        let out = tempfile::tempdir()?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(repo.path())
                .arg("init")
                .output()?
                .status
                .success()
        );
        std::fs::write(repo.path().join("blob"), b"hello")?;
        let hash = Command::new("git")
            .arg("-C")
            .arg(repo.path())
            .args(["hash-object", "-w", "blob"])
            .output()?;
        let side = FileSide {
            id: String::from_utf8(hash.stdout)?.trim().into(),
            mode: 0o100644,
        };
        let mut export = Export::default();
        let choice = Choice {
            path: "blob".into(),
            side: "After",
            version: None,
            size: Some(5),
        };
        export.choose(repo.path().into(), side.clone(), choice.clone());
        let destination = out.path().join("saved");
        export.destination = destination.to_string_lossy().into_owned();
        let ctx = egui::Context::default();
        let enter = egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let frame = |export: &mut Export, events| {
            let _ = crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(720.0, 480.0, events),
                |ui| export.draw(ui.ctx()),
            );
        };
        frame(&mut export, vec![]);
        frame(&mut export, vec![enter.clone()]);
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while export.busy() && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(5));
            frame(&mut export, vec![]);
        }
        assert!(!export.is_open(), "the dialog closes on success");
        assert_eq!(
            export.take_receipt(),
            Some(format!("Exported 5 bytes to {}", destination.display()))
        );
        assert_eq!(std::fs::read(&destination)?, b"hello");

        // Exporting over an existing file keeps the dialog with an inline error.
        export.choose(repo.path().into(), side, choice);
        frame(&mut export, vec![]);
        frame(&mut export, vec![enter]);
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while export.busy() && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(5));
            frame(&mut export, vec![]);
        }
        assert!(export.is_open());
        assert_eq!(
            export.error.as_deref(),
            Some("That file already exists. Choose a new name.")
        );
        Ok(())
    }

    #[test]
    fn cancelled_export_removes_partial_file() -> Result<(), Box<dyn std::error::Error>> {
        use std::{cell::Cell, process::Command};
        let repo = tempfile::tempdir()?;
        let destination = tempfile::tempdir()?;
        let init = Command::new("git")
            .arg("-C")
            .arg(repo.path())
            .arg("init")
            .output()?;
        assert!(init.status.success());
        let content = vec![5u8; 256 * 1024];
        std::fs::write(repo.path().join("blob"), &content)?;
        let hash = Command::new("git")
            .arg("-C")
            .arg(repo.path())
            .args(["hash-object", "-w", "blob"])
            .output()?;
        assert!(hash.status.success());
        let side = FileSide {
            id: String::from_utf8(hash.stdout)?.trim().into(),
            mode: 0o100644,
        };
        let calls = Cell::new(0);
        let result = save(
            repo.path(),
            &side,
            &destination.path().join("saved"),
            || {
                calls.set(calls.get() + 1);
                calls.get() >= 4
            },
        );
        assert!(result.is_err());
        assert_eq!(std::fs::read_dir(destination.path())?.count(), 0);
        assert_eq!(std::fs::read(repo.path().join("blob"))?, content);
        Ok(())
    }
}
