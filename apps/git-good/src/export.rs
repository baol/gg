//! Explicit export to a new destination; temporary output is removed on failure/cancel.
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
#[derive(Default)]
pub struct Export {
    selected: Option<(PathBuf, FileSide, String)>,
    destination: String,
    message: String,
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
    pub fn choose(&mut self, repo: PathBuf, side: FileSide, label: String) {
        if self.job.is_none() {
            self.selected = Some((repo, side, label));
            self.message.clear();
        }
    }
    pub fn draw(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.job.as_ref().and_then(|job| job.try_recv().ok()) {
            self.job = None;
            self.message = match result {
                Ok(n) => format!("Exported {n} bytes to {}", self.destination),
                Err(e) => e,
            };
        }
        let Some((repo, side, label)) = self.selected.clone() else {
            return;
        };
        let mut open = !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        egui::Window::new("Export file version")
            .id(egui::Id::unique("export-file"))
            .open(&mut open)
            .resizable(true)
            .default_width(460.0)
            .show(ctx, |ui| {
                crate::bounded_text::field(ui, &label, egui::FontId::proportional(13.0), 22.0);
                crate::bounded_text::field(ui, &side.id, egui::FontId::monospace(11.0), 20.0);
                ui.label("Save exact bytes to a new file");
                ui.add_enabled(
                    self.job.is_none(),
                    egui::TextEdit::singleline(&mut self.destination)
                        .hint_text("Destination path")
                        .desired_width(f32::INFINITY),
                );
                if self.job.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Exporting…");
                        if ui.button("Cancel").clicked() {
                            self.stop.store(true, Ordering::Relaxed);
                        }
                    });
                } else if ui
                    .add_enabled(
                        !self.destination.trim().is_empty(),
                        egui::Button::new("Export"),
                    )
                    .clicked()
                {
                    self.stop = Arc::new(AtomicBool::new(false));
                    let stop = self.stop.clone();
                    let destination = PathBuf::from(&self.destination);
                    let (send, receive) = mpsc::channel();
                    let ctx = ctx.clone();
                    match std::thread::Builder::new()
                        .name("gg-export".into())
                        .spawn(move || {
                            let result =
                                save(&repo, &side, &destination, || stop.load(Ordering::Relaxed));
                            let _ = send.send(result);
                            ctx.request_repaint();
                        }) {
                        Ok(_) => {
                            self.job = Some(receive);
                            self.message.clear();
                        }
                        Err(e) => self.message = e.to_string(),
                    }
                }
                if !self.message.is_empty() {
                    crate::bounded_text::field(
                        ui,
                        &self.message,
                        egui::FontId::proportional(12.0),
                        24.0,
                    );
                }
            });
        if !open {
            self.stop.store(true, Ordering::Relaxed);
            self.selected = None;
        }
    }
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
            "After file".into(),
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
