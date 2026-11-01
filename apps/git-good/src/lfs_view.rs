//! Git LFS as the inspect proposal draws it: LFS details with 8-character
//! OIDs and copy buttons (size and availability on the same row), the
//! status pill in the header, a labelled bounded preview, the result at the
//! footer's left and **Export original…**; and Repository › LFS…, the
//! pointers in HEAD (missing objects by default) with checkboxes and one
//! batch fetch whose button names the count and size.
use crate::inspect_ui::{self, Tone};
use eframe::egui;
use gg_engine::lfs::{Details, HeadPointer, HeadPointers};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::mpsc,
};

pub struct LfsWindow {
    pub request: crate::diff_view::LfsRequest,
    pub details: Option<Details>,
    pub content: Option<crate::binary_preview::Preview>,
    pub preview_note: Option<String>,
    pub texture: Option<egui::TextureHandle>,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub busy: Option<String>,
    pub confirm_fetch: bool,
    export: Option<mpsc::Receiver<Result<String, String>>>,
}

impl LfsWindow {
    pub fn new(request: crate::diff_view::LfsRequest) -> Self {
        Self {
            request,
            details: None,
            content: None,
            preview_note: None,
            texture: None,
            error: None,
            notice: None,
            busy: None,
            confirm_fetch: false,
            export: None,
        }
    }

    /// Copy the complete object from the LFS store to `destination`.
    fn export(&mut self, ctx: &egui::Context, repo: PathBuf, oid: String, destination: PathBuf) {
        let (send, receive) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = send.send(export_object(&repo, &oid, &destination));
            ctx.request_repaint();
        });
        self.export = Some(receive);
    }
}

fn export_object(repo: &Path, oid: &str, destination: &Path) -> Result<String, String> {
    let source = gg_git::lfs::object_path(repo, oid)
        .filter(|path| path.is_file())
        .ok_or("The LFS object is not in the local store.")?;
    if destination.exists() {
        return Err("Destination already exists; choose a new file.".into());
    }
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    let mut input = std::fs::File::open(&source).map_err(|error| error.to_string())?;
    let size = std::io::copy(&mut input, file.as_file_mut()).map_err(|error| error.to_string())?;
    file.as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    file.persist_noclobber(destination)
        .map_err(|error| error.error.to_string())?;
    Ok(format!(
        "Exported {} to {}.",
        inspect_ui::bytes(size),
        destination.display()
    ))
}

/// What the details window asks for this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailsAction {
    Close,
    AskFetch,
    Fetch,
    CancelFetch,
}

pub fn tone(details: &Details) -> Tone {
    match details.status_label() {
        "Object available" => Tone::Green,
        "Object missing" => Tone::Red,
        "LFS unavailable" => Tone::Amber,
        _ => Tone::Neutral,
    }
}

fn side_row(ui: &mut egui::Ui, label: &str, pointer: &gg_engine::lfs::SidePointer) {
    inspect_ui::row(ui, label, |ui| {
        ui.label(inspect_ui::mono(inspect_ui::short(&pointer.oid)));
        ui.label(inspect_ui::bytes(pointer.size));
        if pointer.available {
            ui.colored_label(Tone::Green.text(), "available");
        } else {
            ui.colored_label(Tone::Red.text(), "missing");
        }
        inspect_ui::copy_button(ui, &pointer.oid, &format!("{} OID", label.to_lowercase()));
    });
}

/// Which version the target is, for sentences: `new` (After) or `old`.
fn version(details: &Details) -> &'static str {
    if details.after.is_some() {
        "new"
    } else {
        "old"
    }
}

/// LFS details and its Fetch confirmation.
pub fn draw_details(
    ctx: &egui::Context,
    window: &mut LfsWindow,
    repo: Option<&Path>,
) -> Option<DetailsAction> {
    if let Some(result) = window.export.as_ref().and_then(|job| job.try_recv().ok()) {
        window.export = None;
        match result {
            Ok(notice) => {
                window.notice = Some(notice);
                window.error = None;
            }
            Err(error) => window.error = Some(error),
        }
    }
    let mut action = None;
    let mut export = None;
    let path = window.request.path().to_string();
    let max_width = (ctx.content_rect().width() - 24.0).max(360.0);
    let id = egui::Id::unique("lfs-details");
    egui::Window::new("LFS pointer")
        .id(id)
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .default_width(max_width.min(680.0))
        .max_width(max_width)
        .default_pos(ctx.content_rect().center())
        .pivot(egui::Align2::CENTER_CENTER)
        .show(ctx, |ui| {
            if inspect_ui::header(ui, "LFS", Some(&path), |ui| {
                if let Some(details) = &window.details {
                    inspect_ui::pill(ui, details.status_label(), tone(details));
                }
            }) {
                action = Some(DetailsAction::Close);
            }
            let Some(details) = &window.details else {
                // Rows stay reserved until the pointer arrives.
                ui.add_space(3.0 * inspect_ui::ROW);
                inspect_ui::footer(ui, None, |ui| {
                    ui.add_enabled(false, inspect_ui::primary("Fetch object…", None));
                });
                return;
            };
            inspect_ui::grid(ui, "lfs-details-grid", |ui| {
                if let Some(before) = &details.before {
                    side_row(ui, "BEFORE", before);
                }
                if let Some(after) = &details.after {
                    side_row(ui, "AFTER", after);
                }
                if details.target().is_some_and(|target| !target.available) {
                    inspect_ui::row(ui, "GIT LFS", |ui| {
                        if details.lfs_installed {
                            ui.label("available");
                            if let Some(remote) = &details.remote {
                                ui.label("· remote");
                                inspect_ui::pill(ui, remote, Tone::Neutral);
                            }
                        } else {
                            ui.colored_label(Tone::Amber.text(), "not installed");
                        }
                    });
                }
            });
            let target = details.target();
            if target.is_some_and(|target| !target.available) {
                ui.weak(format!(
                    "The {} version\u{2019}s content is not in the local LFS store, so the diff shows only the pointer.",
                    version(details)
                ));
            } else if let Some(note) = &details.note {
                ui.weak(note);
            }
            if let Some(content) = &window.content {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(inspect_ui::caption(&format!(
                        "CONTENT PREVIEW · {}",
                        if details.after.is_some() { "AFTER" } else { "BEFORE" }
                    )));
                    if let Some(dimensions) = crate::binary_preview::dimensions_label(content) {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(inspect_ui::caption(&dimensions));
                        });
                    }
                });
                crate::binary_preview::show_single(ui, content, &mut window.texture);
            } else if let Some(note) = &window.preview_note
                && target.is_some_and(|target| target.available)
            {
                ui.weak(note);
            }
            let available = target.is_some_and(|target| target.available);
            let fetch = details.can_fetch();
            let left = if let Some(error) = &window.error {
                Some((error.as_str(), Tone::Red.text()))
            } else if let Some(notice) = &window.notice {
                Some((notice.as_str(), Tone::Green.text()))
            } else if !available
                && let Err(reason) = &fetch
                && target.is_some()
            {
                Some((reason.as_str(), Tone::Amber.text()))
            } else if !available && target.is_some() {
                Some(("Fetching does not change working files.", ui.visuals().weak_text_color()))
            } else {
                None
            };
            let busy = window.busy.is_some() || window.export.is_some();
            inspect_ui::footer(ui, left, |ui| {
                if available {
                    if ui
                        .add_enabled(!busy && repo.is_some(), egui::Button::new("Export original…"))
                        .on_hover_text("Save the complete original bytes to a new file")
                        .clicked()
                        && let Some(target) = target
                    {
                        export = Some(target.oid.clone());
                    }
                } else if ui
                    .add_enabled(
                        fetch.is_ok() && !busy,
                        inspect_ui::primary("Fetch object…", None),
                    )
                    .clicked()
                {
                    action = Some(DetailsAction::AskFetch);
                }
            });
        });
    if let Some(oid) = export
        && let Some(repo) = repo
    {
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        if let Some(destination) = rfd::FileDialog::new()
            .set_title("Export original")
            .set_file_name(&name)
            .save_file()
        {
            window.export(ctx, repo.to_path_buf(), oid, destination);
        }
    }
    if window.confirm_fetch
        && let Some(details) = &window.details
        && let Some(target) = details.target()
    {
        let remote = details
            .remote
            .as_deref()
            .map(|remote| format!(" from {remote}"))
            .unwrap_or_default();
        match inspect_ui::confirm(
            ctx,
            "fetch-lfs",
            "Fetch LFS object",
            "Fetch",
            window
                .busy
                .is_some()
                .then_some("Wait for the current fetch to finish."),
            false,
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("Download the {} version of", version(details)));
                    ui.label(inspect_ui::mono(&path));
                    ui.label("?");
                });
                ui.weak(format!(
                    "{}{remote}. Working files are not changed.",
                    inspect_ui::bytes(target.size)
                ));
            },
        ) {
            Some(inspect_ui::Decision::Confirm) => action = Some(DetailsAction::Fetch),
            Some(inspect_ui::Decision::Cancel) => action = Some(DetailsAction::CancelFetch),
            None => {}
        }
    } else if action.is_none()
        && inspect_ui::owns_keys(ctx, id)
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        action = Some(DetailsAction::Close);
    }
    action
}

/// A batch fetch's re-read pointers, object count, and bytes.
type Fetched = (HeadPointers, usize, u64);

/// Repository › LFS…: the pointers committed in HEAD.
pub struct ListWindow {
    path: PathBuf,
    pointers: Option<Result<HeadPointers, String>>,
    /// Show every pointer, not only the missing ones.
    all: bool,
    /// Paths chosen for the batch fetch; missing objects start chosen.
    chosen: BTreeSet<String>,
    /// Paths already seen, so a re-read keeps the user's unchecks.
    seen: BTreeSet<String>,
    selected: Option<String>,
    result: Option<(Tone, String)>,
    reading: Option<mpsc::Receiver<Result<HeadPointers, String>>>,
    fetching: Option<mpsc::Receiver<Result<Fetched, String>>>,
    again: bool,
    ctx: egui::Context,
}

/// What the list asks for this frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListAction {
    Close,
    Details(HeadPointer),
}

impl ListWindow {
    pub fn new(path: PathBuf, ctx: &egui::Context) -> Self {
        let mut window = Self {
            path,
            pointers: None,
            all: false,
            chosen: BTreeSet::new(),
            seen: BTreeSet::new(),
            selected: None,
            result: None,
            reading: None,
            fetching: None,
            again: false,
            ctx: ctx.clone(),
        };
        window.reread();
        window
    }

    pub fn reread(&mut self) {
        if self.reading.is_some() || self.fetching.is_some() {
            self.again = true;
            return;
        }
        let (send, receive) = mpsc::channel();
        let path = self.path.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let _ = send.send(gg_git::lfs::head_pointers(&path));
            ctx.request_repaint();
        });
        self.reading = Some(receive);
    }

    fn accept(&mut self, pointers: Result<HeadPointers, String>) {
        if let Ok(list) = &pointers {
            for pointer in &list.pointers {
                if self.seen.insert(pointer.path.clone()) && !pointer.available {
                    self.chosen.insert(pointer.path.clone());
                }
                if pointer.available {
                    self.chosen.remove(&pointer.path);
                }
            }
        }
        self.pointers = Some(pointers);
    }

    #[cfg(test)]
    pub fn wait(&mut self) {
        loop {
            if let Some(fetching) = self.fetching.take() {
                if let Ok(result) = fetching.recv() {
                    self.fetched(result);
                }
            } else if let Some(reading) = self.reading.take() {
                if let Ok(pointers) = reading.recv() {
                    self.accept(pointers);
                }
            } else if std::mem::take(&mut self.again) {
                self.reread();
            } else {
                return;
            }
        }
    }

    fn fetched(&mut self, result: Result<Fetched, String>) {
        match result {
            Ok((pointers, count, size)) => {
                self.result = Some((
                    Tone::Green,
                    format!(
                        "Fetched {} ({}).",
                        crate::consequence::counted(count, "object", "objects"),
                        inspect_ui::bytes(size)
                    ),
                ));
                self.accept(Ok(pointers));
            }
            Err(error) => {
                self.result = Some((
                    Tone::Red,
                    error.lines().next().unwrap_or_default().to_string(),
                ));
            }
        }
    }

    fn poll(&mut self) {
        if let Some(result) = self.fetching.as_ref().and_then(|job| job.try_recv().ok()) {
            self.fetching = None;
            self.fetched(result);
        }
        if let Some(pointers) = self.reading.as_ref().and_then(|read| read.try_recv().ok()) {
            self.reading = None;
            self.accept(pointers);
        }
        if self.reading.is_none() && self.fetching.is_none() && std::mem::take(&mut self.again) {
            self.reread();
        }
    }

    fn fetch(&mut self, paths: Vec<String>, size: u64) {
        let (send, receive) = mpsc::channel();
        let path = self.path.clone();
        let ctx = self.ctx.clone();
        let count = paths.len();
        std::thread::spawn(move || {
            let result =
                gg_git::lfs::fetch_paths(&path, &paths).map(|pointers| (pointers, count, size));
            let _ = send.send(result);
            ctx.request_repaint();
        });
        self.result = None;
        self.fetching = Some(receive);
    }

    pub fn draw(&mut self, ctx: &egui::Context) -> Option<ListAction> {
        self.poll();
        let mut action = None;
        let id = egui::Id::unique("lfs-objects");
        let keys = inspect_ui::owns_keys(ctx, id);
        let max_width = (ctx.content_rect().width() - 24.0).max(360.0);
        let list_height = (ctx.content_rect().height() - 220.0)
            .clamp(3.0 * inspect_ui::ROW, 10.0 * inspect_ui::ROW);
        let (pointers, installed, remote) = match &self.pointers {
            Some(Ok(list)) => (
                list.pointers.clone(),
                list.lfs_installed,
                list.remote.clone(),
            ),
            _ => (Vec::new(), true, None),
        };
        let missing = pointers.iter().filter(|pointer| !pointer.available).count();
        let shown = pointers
            .iter()
            .filter(|pointer| self.all || !pointer.available)
            .cloned()
            .collect::<Vec<_>>();
        let batch = pointers
            .iter()
            .filter(|pointer| !pointer.available && self.chosen.contains(&pointer.path))
            .collect::<Vec<_>>();
        let batch_size = batch.iter().map(|pointer| pointer.size).sum::<u64>();
        let batch_paths = batch
            .iter()
            .map(|pointer| pointer.path.clone())
            .collect::<Vec<_>>();
        let mut fetch = false;
        egui::Window::new("LFS objects")
            .id(id)
            .title_bar(false)
            .collapsible(false)
            .resizable(true)
            .default_width(max_width.min(640.0))
            .max_width(max_width)
            .default_pos(ctx.content_rect().center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                let mut all = self.all;
                if inspect_ui::header(ui, "LFS objects", None, |ui| {
                    if let Some(choice) = inspect_ui::segmented(
                        ui,
                        all,
                        &[
                            (false, format!("Missing {missing}")),
                            (true, format!("All {}", pointers.len())),
                        ],
                    ) {
                        all = choice;
                    }
                }) {
                    action = Some(ListAction::Close);
                }
                self.all = all;
                let columns = [
                    16.0,
                    ui.available_width() - 16.0 - 16.0 - 74.0 - 70.0 - 40.0,
                    74.0,
                    0.0,
                ];
                inspect_ui::table_header(ui, &columns, &["", "PATH", "OBJECT", "SIZE"]);
                let width = ui.available_width();
                ui.allocate_ui_with_layout(
                    egui::vec2(width, list_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_size(egui::vec2(width, list_height));
                        match &self.pointers {
                            Some(Err(error)) => {
                                ui.colored_label(Tone::Red.text(), error);
                                return;
                            }
                            Some(Ok(_)) if shown.is_empty() => {
                                ui.add_space(list_height / 2.0 - 22.0);
                                ui.vertical_centered(|ui| {
                                    if pointers.is_empty() {
                                        ui.label("No LFS pointers in HEAD.");
                                        ui.weak("Files tracked with git lfs track appear here.");
                                    } else {
                                        ui.label("No missing objects.");
                                        ui.weak(format!(
                                            "All {} in HEAD are in the local LFS store.",
                                            crate::consequence::counted(
                                                pointers.len(),
                                                "pointer",
                                                "pointers"
                                            )
                                        ));
                                    }
                                });
                                return;
                            }
                            _ => {}
                        }
                        egui::ScrollArea::vertical()
                            .id_salt("lfs-object-rows")
                            .max_height(list_height)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for pointer in &shown {
                                    let on =
                                        self.selected.as_deref() == Some(pointer.path.as_str());
                                    let mut checked =
                                        !pointer.available && self.chosen.contains(&pointer.path);
                                    let mut toggled = false;
                                    let response =
                                        inspect_ui::table_row(ui, on, &columns, |cells| {
                                            toggled = cells[0]
                                                .add_enabled(
                                                    !pointer.available,
                                                    egui::Checkbox::without_text(&mut checked),
                                                )
                                                .on_disabled_hover_text(
                                                    "Already in the local store",
                                                )
                                                .changed();
                                            cells[1].add(
                                                egui::Label::new(inspect_ui::mono(&pointer.path))
                                                    .truncate(),
                                            );
                                            cells[2].label(
                                                inspect_ui::mono(inspect_ui::short(&pointer.oid))
                                                    .weak(),
                                            );
                                            cells[3].with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(inspect_ui::bytes(pointer.size));
                                                },
                                            );
                                        });
                                    if toggled {
                                        if checked {
                                            self.chosen.insert(pointer.path.clone());
                                        } else {
                                            self.chosen.remove(&pointer.path);
                                        }
                                    }
                                    if response.clicked() {
                                        self.selected = Some(pointer.path.clone());
                                    }
                                    if response.double_clicked() {
                                        action = Some(ListAction::Details(pointer.clone()));
                                    }
                                }
                            });
                    },
                );
                let reason = if !installed {
                    Some("git-lfs is not installed.".to_string())
                } else if self.fetching.is_some() {
                    Some(format!(
                        "Fetching {}…",
                        crate::consequence::counted(batch_paths.len(), "object", "objects")
                    ))
                } else {
                    None
                };
                let summary = match &reason {
                    Some(reason) => (reason.clone(), Tone::Amber.text()),
                    None => match &self.result {
                        Some((tone, text)) => (text.clone(), tone.text()),
                        None => (
                            format!(
                                "{} selected · {}{}",
                                batch_paths.len(),
                                inspect_ui::bytes(batch_size),
                                remote
                                    .as_deref()
                                    .map(|remote| format!(" from {remote}"))
                                    .unwrap_or_default()
                            ),
                            ui.visuals().weak_text_color(),
                        ),
                    },
                };
                let ready = reason.is_none() && !batch_paths.is_empty();
                inspect_ui::footer(ui, Some((&summary.0, summary.1)), |ui| {
                    let label = format!(
                        "Fetch {}",
                        crate::consequence::counted(batch_paths.len(), "object", "objects")
                    );
                    if ui
                        .add_enabled(ready, inspect_ui::primary(label, Some("Ctrl+Enter")))
                        .clicked()
                    {
                        fetch = true;
                    }
                    let selected = self
                        .selected
                        .as_ref()
                        .and_then(|path| pointers.iter().find(|pointer| &pointer.path == path));
                    if ui
                        .add_enabled(selected.is_some(), egui::Button::new("LFS details"))
                        .clicked()
                        && let Some(pointer) = selected
                    {
                        action = Some(ListAction::Details(pointer.clone()));
                    }
                });
                if ready
                    && keys
                    && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter))
                {
                    fetch = true;
                }
            });
        if fetch {
            self.fetch(batch_paths, batch_size);
        }
        if action.is_none()
            && keys
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            action = Some(ListAction::Close);
        }
        action
    }
}

/// The details request for one pointer in HEAD.
pub fn request(pointer: &HeadPointer) -> crate::diff_view::LfsRequest {
    crate::diff_view::LfsRequest::Change(gg_engine::review::FileChange {
        path: pointer.path.clone(),
        path_bytes: pointer.path_bytes.clone(),
        kind: gg_engine::review::ChangeKind::Modified,
        old: None,
        new: Some(gg_engine::review::FileSide {
            id: pointer.blob.clone(),
            mode: 0o100644,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const OID: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

    fn text(output: &egui::FullOutput) -> Vec<String> {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect()
    }

    fn input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1120.0, 740.0),
            )),
            events,
            ..Default::default()
        }
    }

    fn pointer(path: &str, oid: &str, size: u64, available: bool) -> HeadPointer {
        HeadPointer {
            path: path.into(),
            path_bytes: path.as_bytes().to_vec(),
            oid: oid.into(),
            size,
            available,
            blob: "b".repeat(40),
        }
    }

    #[test]
    fn details_shorten_oids_and_name_the_file_and_size_in_the_fetch_question() {
        let mut window = LfsWindow::new(request(&pointer(
            "assets/splash.png",
            OID,
            1_468_006,
            false,
        )));
        window.details = Some(Details {
            path: "assets/splash.png".into(),
            path_bytes: b"assets/splash.png".to_vec(),
            before: Some(gg_engine::lfs::SidePointer {
                oid: format!("3a7bd3e2{}", "0".repeat(56)),
                size: 1_258_291,
                available: true,
            }),
            after: Some(gg_engine::lfs::SidePointer {
                oid: OID.into(),
                size: 1_468_006,
                available: false,
            }),
            lfs_installed: true,
            remote: Some("origin".into()),
            note: None,
        });
        let ctx = egui::Context::default();
        let _ = crate::tests::run_ui(&ctx, input(vec![]), |ui| {
            draw_details(ui.ctx(), &mut window, None);
        });
        let output = crate::tests::run_ui(&ctx, input(vec![]), |ui| {
            draw_details(ui.ctx(), &mut window, None);
        });
        let shown = text(&output);
        for expected in [
            "LFS",
            "assets/splash.png",
            "Object missing",
            "BEFORE",
            "3a7bd3e2",
            "1.2 MiB",
            "available",
            "AFTER",
            "9f86d081",
            "1.4 MiB",
            "missing",
            "GIT LFS",
            "origin",
            "The new version\u{2019}s content is not in the local LFS store, so the diff shows only the pointer.",
            "Fetching does not change working files.",
            "Fetch object…",
        ] {
            assert!(
                shown.iter().any(|line| line.contains(expected)),
                "missing {expected}: {shown:?}"
            );
        }
        assert!(
            !shown.iter().any(|line| line.contains(OID)),
            "no 64-character OIDs: {shown:?}"
        );

        window.confirm_fetch = true;
        let _ = crate::tests::run_ui(&ctx, input(vec![]), |ui| {
            draw_details(ui.ctx(), &mut window, None);
        });
        let output = crate::tests::run_ui(&ctx, input(vec![]), |ui| {
            draw_details(ui.ctx(), &mut window, None);
        });
        let shown = text(&output);
        for expected in [
            "Fetch LFS object",
            "Download the new version of",
            "1.4 MiB from origin. Working files are not changed.",
            "Cancel",
            "Fetch",
        ] {
            assert!(
                shown.iter().any(|line| line.contains(expected)),
                "missing {expected}: {shown:?}"
            );
        }
    }

    #[test]
    fn the_list_shows_missing_objects_checked_and_states_the_batch()
    -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir()?;
        let mut list = ListWindow::new(dir.path().into(), &ctx);
        list.wait();
        list.seen.clear();
        list.accept(Ok(HeadPointers {
            pointers: vec![
                pointer("assets/splash.png", OID, 1_468_006, false),
                pointer(
                    "assets/icons/app-256.png",
                    &"c2b9e04a".repeat(8),
                    86_016,
                    false,
                ),
                pointer(
                    "assets/fonts/Ubuntu-Light.ttf",
                    &"5e10f7d3".repeat(8),
                    296_960,
                    false,
                ),
                pointer("assets/ok.png", &"aaaaaaaa".repeat(8), 10, true),
            ],
            lfs_installed: true,
            remote: Some("origin".into()),
        }));
        let _ = crate::tests::run_ui(&ctx, input(vec![]), |ui| {
            list.draw(ui.ctx());
        });
        let output = crate::tests::run_ui(&ctx, input(vec![]), |ui| {
            list.draw(ui.ctx());
        });
        let shown = text(&output);
        for expected in [
            "LFS objects",
            "Missing 3",
            "All 4",
            "PATH",
            "OBJECT",
            "SIZE",
            "assets/splash.png",
            "9f86d081",
            "84 KiB",
            "3 selected · 1.8 MiB from origin",
            "LFS details",
            "Fetch 3 objects",
            "Ctrl+Enter",
        ] {
            assert!(
                shown.iter().any(|line| line.contains(expected)),
                "missing {expected}: {shown:?}"
            );
        }
        assert!(
            !shown.iter().any(|line| line.contains("assets/ok.png")),
            "missing objects by default"
        );
        list.chosen.remove("assets/splash.png");
        let output = crate::tests::run_ui(&ctx, input(vec![]), |ui| {
            list.draw(ui.ctx());
        });
        let shown = text(&output);
        assert!(
            shown.iter().any(|line| line.contains("Fetch 2 objects")),
            "{shown:?}"
        );
        list.all = true;
        let _ = crate::tests::run_ui(&ctx, input(vec![]), |ui| {
            list.draw(ui.ctx());
        });
        let output = crate::tests::run_ui(&ctx, input(vec![]), |ui| {
            list.draw(ui.ctx());
        });
        assert!(
            text(&output)
                .iter()
                .any(|line| line.contains("assets/ok.png"))
        );
        Ok(())
    }
}
