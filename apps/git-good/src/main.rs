use eframe::egui;
use gg_engine::{History, HistoryRequest};
use std::{path::PathBuf, sync::mpsc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let first = args.next();
    if first.as_deref() == Some(std::ffi::OsStr::new("--help")) {
        println!(
            "gg · good git\n\ngit-good [PATH]\ngit-good history [PATH]   Print history as JSON\n\nBrowse the latest 500 commits reachable from HEAD."
        );
        return Ok(());
    }
    let headless = first.as_deref() == Some(std::ffi::OsStr::new("history"));
    let path = if headless { args.next() } else { first }
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    if args.next().is_some() {
        return Err("Use git-good --help for usage".into());
    }
    if headless {
        let history = gg_engine::dispatch(&gg_git::Git, &HistoryRequest { path })?;
        println!("{}", serde_json::to_string_pretty(&history)?);
        return Ok(());
    }
    eframe::run_native(
        "gg · good git",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1120.0, 740.0])
                .with_min_inner_size([720.0, 480.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            let mut app = Browser::new(path);
            app.load(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )?;
    Ok(())
}

struct Browser {
    path: String,
    history: Option<History>,
    pending: Option<mpsc::Receiver<Result<History, String>>>,
    error: Option<String>,
    query: String,
    selected: Option<String>,
    #[cfg(debug_assertions)]
    captured: bool,
}

impl Browser {
    fn new(path: PathBuf) -> Self {
        Self {
            path: path.to_string_lossy().into_owned(),
            history: None,
            pending: None,
            error: None,
            query: String::new(),
            selected: None,
            #[cfg(debug_assertions)]
            captured: false,
        }
    }

    fn load(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() {
            return;
        }
        let request = HistoryRequest {
            path: PathBuf::from(&self.path),
        };
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        self.error = None;
        self.pending = Some(rx);
        if let Err(error) = std::thread::Builder::new()
            .name("gg-history".into())
            .spawn(move || {
                let result = gg_engine::dispatch(&gg_git::Git, &request);
                let _ = tx.send(result);
                ctx.request_repaint();
            })
        {
            self.pending = None;
            self.error = Some(format!("Could not start history reader: {error}"));
        }
    }

    fn draw(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    match result {
                        Ok(history) => {
                            if !history
                                .commits
                                .iter()
                                .any(|c| Some(&c.id) == self.selected.as_ref())
                            {
                                self.selected = history.commits.first().map(|c| c.id.clone());
                            }
                            self.history = Some(history);
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.error =
                        Some("History reader stopped. Open the repository again to retry.".into());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.heading("gg");
                ui.label("good git");
                ui.separator();
                let input = ui.add(
                    egui::TextEdit::singleline(&mut self.path)
                        .desired_width(520.0)
                        .hint_text("Repository path"),
                );
                let enter = input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui
                    .add_enabled(self.pending.is_none(), egui::Button::new("Open / Refresh"))
                    .clicked()
                    || enter
                {
                    self.load(ctx);
                }
                if self.pending.is_some() {
                    ui.spinner();
                }
            });
            ui.add_space(12.0);
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("HISTORY");
                if let Some(history) = &self.history {
                    ui.separator();
                    ui.label(&history.head);
                    ui.separator();
                    ui.label(format!(
                        "{} commits{}",
                        history.commits.len(),
                        if history.truncated {
                            " · latest 500"
                        } else {
                            ""
                        }
                    ));
                    ui.separator();
                    ui.label(history.path.display().to_string());
                }
            });
        });
        if let Some(error) = &self.error {
            egui::TopBottomPanel::top("error").show(ctx, |ui| {
                ui.colored_label(ui.visuals().error_fg_color, error);
            });
        }
        egui::SidePanel::right("details")
            .default_width(360.0)
            .width_range(250.0..=600.0)
            .show(ctx, |ui| {
                ui.add_space(16.0);
                ui.heading("Commit details");
                ui.add_space(12.0);
                let commit = self.history.as_ref().and_then(|h| {
                    h.commits
                        .iter()
                        .find(|c| Some(&c.id) == self.selected.as_ref())
                });
                if let Some(commit) = commit {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading(commit.subject());
                        ui.add_space(16.0);
                        ui.label("COMMIT");
                        ui.monospace(&commit.id);
                        if ui.button("Copy commit ID").clicked() {
                            ctx.copy_text(commit.id.clone());
                        }
                        ui.add_space(12.0);
                        ui.label("AUTHOR");
                        ui.label(&commit.author);
                        ui.label(&commit.email);
                        ui.add_space(12.0);
                        ui.label("DATE");
                        ui.label(&commit.time);
                        ui.add_space(12.0);
                        ui.label("PARENTS");
                        if commit.parents.is_empty() {
                            ui.label("Root commit");
                        }
                        for parent in &commit.parents {
                            ui.monospace(parent);
                        }
                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(12.0);
                        ui.label(&commit.message);
                    });
                } else {
                    ui.label("Select a commit to explore its details.");
                }
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(16.0);
            ui.heading("History");
            ui.label("A clear view of how your project arrived here.");
            ui.add_space(12.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Filter loaded commits by message, author, or ID")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(8.0);
            ui.separator();
            if let Some(history) = &self.history {
                if history.commits.is_empty() {
                    ui.add_space(24.0);
                    ui.heading("Ready for the first commit");
                    ui.label("This repository has a branch and an open future.");
                }
                let commits: Vec<_> = history
                    .commits
                    .iter()
                    .filter(|c| c.matches(&self.query))
                    .collect();
                if commits.is_empty() && !history.commits.is_empty() {
                    ui.label("No loaded commits match this filter.");
                }
                egui::ScrollArea::vertical().show_rows(ui, 56.0, commits.len(), |ui, rows| {
                    for row in rows {
                        if let Some(commit) = commits.get(row) {
                            let short: String = commit.id.chars().take(8).collect();
                            let label =
                                format!("{}\n{}   ·   {}", commit.subject(), short, commit.author);
                            if ui
                                .add_sized(
                                    [ui.available_width(), 52.0],
                                    egui::Button::new(label)
                                        .selected(Some(&commit.id) == self.selected.as_ref()),
                                )
                                .clicked()
                            {
                                self.selected = Some(commit.id.clone());
                            }
                        }
                    }
                });
            } else if self.pending.is_some() {
                ui.label("Reading repository history…");
            } else {
                ui.label("Enter a repository path above and choose Open.");
            }
        });
    }
}

impl eframe::App for Browser {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.draw(ctx);
        #[cfg(debug_assertions)]
        if let Some(path) = std::env::var_os("GG_SCREENSHOT") {
            if self.pending.is_none() && !self.captured {
                self.captured = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            let events = ctx.input(|input| input.events.clone());
            {
                for event in &events {
                    if let egui::Event::Screenshot { image, .. } = event {
                        use std::io::Write;
                        let save = || -> std::io::Result<()> {
                            let mut file = std::fs::File::create(&path)?;
                            writeln!(file, "P6\n{} {}\n255", image.width(), image.height())?;
                            for pixel in &image.pixels {
                                file.write_all(&pixel.to_array()[..3])?;
                            }
                            Ok(())
                        };
                        if let Err(error) = save() {
                            eprintln!("Screenshot: {error}");
                        }
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_graphics_backends_are_enabled() {
        assert!(!wgpu::Instance::enabled_backend_features().is_empty());
    }
    #[test]
    fn renders_empty_error_and_filtered_snapshots() {
        let ctx = egui::Context::default();
        let mut app = Browser::new(PathBuf::from("."));
        for state in 0..3 {
            app.history = Some(History {
                path: ".".into(),
                head: "main".into(),
                commits: if state == 2 {
                    vec![gg_engine::Commit {
                        id: "abc123".into(),
                        parents: vec![],
                        author: "Ada".into(),
                        email: "ada@example.org".into(),
                        time: "2026-01-01".into(),
                        message: "First commit".into(),
                    }]
                } else {
                    vec![]
                },
                truncated: false,
            });
            app.error =
                (state == 1).then(|| "Could not read repository. Open again to retry.".into());
            app.selected = Some("abc123".into());
            app.query = "missing".into();
            let output = ctx.run(egui::RawInput::default(), |ctx| app.draw(ctx));
            assert!(!output.shapes.is_empty());
        }
    }
}
