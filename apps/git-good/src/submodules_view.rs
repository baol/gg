//! Submodules as the inspect proposal draws them: Repository › Submodules…
//! lists every gitlink in a 22 px table with a status pill and 8-character
//! IDs (Enter opens the details), and Submodule details shows a label grid
//! with copy buttons and "same as …" hints, the status pill in the header,
//! and the last result or why Update is unavailable at the footer's left.
use crate::inspect_ui::{self, Tone};
use eframe::egui;
use gg_engine::submodule::Details;
use std::{path::PathBuf, sync::mpsc};

pub struct SubmoduleWindow {
    pub request: crate::diff_view::SubmoduleRequest,
    pub details: Option<Details>,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub busy: Option<String>,
    pub confirm_update: bool,
    pub refresh_parent: bool,
}

impl SubmoduleWindow {
    pub fn new(request: crate::diff_view::SubmoduleRequest) -> Self {
        Self {
            request,
            details: None,
            error: None,
            notice: None,
            busy: None,
            confirm_update: false,
            refresh_parent: false,
        }
    }
}

/// What the details window asks for this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailsAction {
    Close,
    OpenNested,
    AskUpdate,
    Update,
    CancelUpdate,
}

pub fn tone(details: &Details) -> Tone {
    match details.tone() {
        gg_engine::submodule::Tone::Neutral => Tone::Neutral,
        gg_engine::submodule::Tone::Differs => Tone::Amber,
        gg_engine::submodule::Tone::Broken => Tone::Red,
        gg_engine::submodule::Tone::Clean => Tone::Green,
    }
}

/// The primary's label: what Update would do.
pub fn update_label(details: &Details) -> String {
    if !details.initialized {
        "Initialize and update…".into()
    } else if details.dirty == Some(true) {
        "Update…".into()
    } else {
        match details.target() {
            Some(target) => format!("Update to {}…", inspect_ui::short(target)),
            None => "Update…".into(),
        }
    }
}

/// `same as After`, `same as Before · 14 commits behind After`.
fn hint(details: &Details, id: &str, sides: &[(&str, Option<&str>)], behind: bool) -> String {
    let mut parts = Vec::new();
    if let Some((name, _)) = sides.iter().find(|(_, side)| *side == Some(id)) {
        parts.push(format!("same as {name}"));
    }
    if behind && let Some(count) = details.behind.filter(|count| *count > 0) {
        let target = if details.recorded.is_some() {
            "After"
        } else {
            "Index"
        };
        parts.push(format!(
            "{} behind {target}",
            crate::consequence::counted(count as usize, "commit", "commits")
        ));
    }
    parts.join(" · ")
}

fn id_row(ui: &mut egui::Ui, label: &str, details: &Details, id: &str, hint: String) {
    inspect_ui::row(ui, label, |ui| {
        ui.label(inspect_ui::mono(inspect_ui::short(id)));
        if let Some(subject) = details.subjects.get(id) {
            ui.add(egui::Label::new(subject).truncate());
        }
        inspect_ui::copy_button(ui, id, &format!("{} commit ID", label.to_lowercase()));
        if !hint.is_empty() {
            ui.weak(hint);
        }
    });
}

/// Submodule details: header with the status pill, the label grid, the
/// footer, and the Update confirmation.
pub fn draw_details(
    ctx: &egui::Context,
    window: &mut SubmoduleWindow,
    can_open: bool,
) -> Option<DetailsAction> {
    let mut action = None;
    let request = window.request.clone();
    let max_width = (ctx.content_rect().width() - 24.0).max(360.0);
    egui::Window::new("Submodule details")
        .id(egui::Id::unique("submodule-details"))
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .default_width(max_width.min(680.0))
        .max_width(max_width)
        .default_pos(ctx.content_rect().center())
        .pivot(egui::Align2::CENTER_CENTER)
        .show(ctx, |ui| {
            let details = window.details.as_ref();
            if inspect_ui::header(ui, "Submodule", Some(&request.path), |ui| {
                if let Some(details) = details {
                    inspect_ui::pill(ui, details.status_label(), tone(details));
                }
            }) {
                action = Some(DetailsAction::Close);
            }
            // The sentence and grid keep their rows while details arrive.
            ui.allocate_ui(egui::vec2(ui.available_width(), inspect_ui::ROW), |ui| {
                ui.set_min_height(inspect_ui::ROW);
                if let Some(details) = details {
                    ui.weak(details.status_sentence());
                }
            });
            let Some(details) = details else {
                ui.add_space(4.0 * inspect_ui::ROW);
                inspect_ui::footer(ui, None, |ui| {
                    ui.add_enabled(false, inspect_ui::primary("Update…", None));
                    ui.add_enabled(false, egui::Button::new("Open nested repository"));
                });
                return;
            };
            let before = request.before.as_deref();
            let after = request.after.as_deref();
            inspect_ui::grid(ui, "submodule-details-grid", |ui| {
                if let Some(before) = before {
                    id_row(ui, "BEFORE", details, before, String::new());
                }
                if let Some(after) = after {
                    id_row(ui, "AFTER", details, after, String::new());
                }
                match details.index.as_deref() {
                    Some(index) => id_row(
                        ui,
                        "INDEX",
                        details,
                        index,
                        hint(
                            details,
                            index,
                            &[("After", after), ("Before", before)],
                            false,
                        ),
                    ),
                    None => inspect_ui::row(ui, "INDEX", |ui| {
                        ui.weak("absent");
                    }),
                }
                match details.checked_out.as_deref() {
                    Some(checked) => id_row(
                        ui,
                        "CHECKED OUT",
                        details,
                        checked,
                        hint(
                            details,
                            checked,
                            &[
                                ("Before", before),
                                ("After", after),
                                ("Index", details.index.as_deref()),
                            ],
                            true,
                        ),
                    ),
                    None => inspect_ui::row(ui, "CHECKED OUT", |ui| {
                        ui.weak("not initialized");
                    }),
                }
                if let Some(url) = &details.url {
                    inspect_ui::row(ui, "URL", |ui| {
                        ui.add(egui::Label::new(url).truncate());
                    });
                }
                if let Some(branch) = &details.branch {
                    inspect_ui::row(ui, "BRANCH", |ui| {
                        ui.label(branch);
                    });
                }
            });
            if let Some(configured) = &details.configured_url
                && Some(configured) != details.url.as_ref()
            {
                ui.colored_label(
                    Tone::Amber.text(),
                    format!("Local config uses a different URL: {configured}"),
                );
            }
            if let Some(note) = &details.note {
                ui.weak(note);
            }
            let update = details.can_update();
            let left = if let Some(error) = &window.error {
                Some((error.as_str(), Tone::Red.text()))
            } else if let Err(reason) = &update {
                Some((reason.as_str(), Tone::Amber.text()))
            } else if let Some(notice) = &window.notice {
                Some((notice.as_str(), Tone::Green.text()))
            } else if !details.initialized {
                Some(("Not initialized yet.", ui.visuals().weak_text_color()))
            } else {
                None
            };
            let ready = update.is_ok() && window.busy.is_none();
            inspect_ui::footer(ui, left, |ui| {
                if ui
                    .add_enabled(ready, inspect_ui::primary(update_label(details), None))
                    .clicked()
                {
                    action = Some(DetailsAction::AskUpdate);
                }
                if ui
                    .add_enabled(can_open, egui::Button::new("Open nested repository"))
                    .on_disabled_hover_text("Initialize the submodule before opening it")
                    .clicked()
                {
                    action = Some(DetailsAction::OpenNested);
                }
            });
        });
    if window.confirm_update
        && let Some(details) = &window.details
    {
        let target = details
            .target()
            .map(|target| inspect_ui::short(target).to_string())
            .unwrap_or_else(|| "the index commit".into());
        let decision = inspect_ui::confirm(
            ctx,
            "update-submodule",
            "Update submodule",
            "Update",
            window
                .busy
                .is_some()
                .then_some("Wait for the current update to finish."),
            false,
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(if details.initialized {
                        "Check out"
                    } else {
                        "Initialize and check out"
                    });
                    ui.label(inspect_ui::mono(&target));
                    ui.label("in");
                    ui.label(format!("{}?", details.path));
                });
                let fetch = details
                    .url
                    .as_deref()
                    .map(|url| format!("Git may fetch from {url}. "))
                    .unwrap_or_default();
                ui.weak(format!("{fetch}The parent index is not changed."));
            },
        );
        match decision {
            Some(inspect_ui::Decision::Confirm) => action = Some(DetailsAction::Update),
            Some(inspect_ui::Decision::Cancel) => action = Some(DetailsAction::CancelUpdate),
            None => {}
        }
    } else if action.is_none()
        && inspect_ui::owns_keys(ctx, egui::Id::unique("submodule-details"))
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        action = Some(DetailsAction::Close);
    }
    action
}

/// Repository › Submodules…: every gitlink in the parent index.
pub struct ListWindow {
    path: PathBuf,
    rows: Option<Result<Vec<Details>, String>>,
    selected: usize,
    reading: Option<mpsc::Receiver<Result<Vec<Details>, String>>>,
    again: bool,
    ctx: egui::Context,
}

/// What the list asks for this frame.
#[derive(Clone, Debug)]
pub enum ListAction {
    Close,
    Details(crate::diff_view::SubmoduleRequest),
    OpenNested(Vec<u8>),
}

impl ListWindow {
    pub fn new(path: PathBuf, ctx: &egui::Context) -> Self {
        let mut window = Self {
            path,
            rows: None,
            selected: 0,
            reading: None,
            again: false,
            ctx: ctx.clone(),
        };
        window.reread();
        window
    }

    /// Re-read the gitlinks in the background.
    pub fn reread(&mut self) {
        if self.reading.is_some() {
            self.again = true;
            return;
        }
        let (send, receive) = mpsc::channel();
        let path = self.path.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let _ = send.send(gg_git::submodule::list(&path));
            ctx.request_repaint();
        });
        self.reading = Some(receive);
    }

    #[cfg(test)]
    pub fn wait(&mut self) {
        while let Some(reading) = self.reading.take() {
            if let Ok(rows) = reading.recv() {
                self.rows = Some(rows);
            }
            if std::mem::take(&mut self.again) {
                self.reread();
            }
        }
    }

    fn poll(&mut self) {
        if let Some(rows) = self.reading.as_ref().and_then(|read| read.try_recv().ok()) {
            self.reading = None;
            self.rows = Some(rows);
            if std::mem::take(&mut self.again) {
                self.reread();
            }
        }
    }

    pub fn draw(&mut self, ctx: &egui::Context) -> Option<ListAction> {
        self.poll();
        let mut action = None;
        let id = egui::Id::unique("submodules");
        let keys = inspect_ui::owns_keys(ctx, id);
        let max_width = (ctx.content_rect().width() - 24.0).max(360.0);
        let rows: &[Details] = match &self.rows {
            Some(Ok(rows)) => rows,
            _ => &[],
        };
        if self.selected >= rows.len() {
            self.selected = rows.len().saturating_sub(1);
        }
        let mut selected = self.selected;
        let list_height = (ctx.content_rect().height() - 220.0)
            .clamp(3.0 * inspect_ui::ROW, 10.0 * inspect_ui::ROW);
        egui::Window::new("Submodules")
            .id(id)
            .title_bar(false)
            .collapsible(false)
            .resizable(true)
            .default_width(max_width.min(640.0))
            .max_width(max_width)
            .default_pos(ctx.content_rect().center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                let count = rows.len().to_string();
                let close = inspect_ui::header(ui, "Submodules", None, |ui| {
                    if self.rows.is_some() {
                        inspect_ui::pill(ui, &count, Tone::Neutral);
                    }
                });
                if close {
                    action = Some(ListAction::Close);
                }
                let columns = [
                    ui.available_width() - 16.0 - 150.0 - 74.0 - 74.0 - 30.0,
                    150.0,
                    74.0,
                    0.0,
                ];
                inspect_ui::table_header(
                    ui,
                    &columns,
                    &["PATH", "STATUS", "RECORDED", "CHECKED OUT"],
                );
                let width = ui.available_width();
                ui.allocate_ui_with_layout(
                    egui::vec2(width, list_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_size(egui::vec2(width, list_height));
                        match &self.rows {
                            Some(Err(error)) => {
                                ui.colored_label(Tone::Red.text(), error);
                            }
                            Some(Ok(rows)) if rows.is_empty() => {
                                ui.add_space(list_height / 2.0 - 22.0);
                                ui.vertical_centered(|ui| {
                                    ui.label("No submodules in this repository.");
                                    ui.weak("Gitlinks in the parent index appear here.");
                                });
                            }
                            _ => {}
                        }
                        egui::ScrollArea::vertical()
                            .id_salt("submodule-rows")
                            .max_height(list_height)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for (index, row) in rows.iter().enumerate() {
                                    let response = inspect_ui::table_row(
                                        ui,
                                        index == selected,
                                        &columns,
                                        |cells| {
                                            cells[0].add(
                                                egui::Label::new(inspect_ui::mono(&row.path))
                                                    .truncate(),
                                            );
                                            inspect_ui::pill(
                                                &mut cells[1],
                                                row.status_label(),
                                                tone(row),
                                            );
                                            match row.index.as_deref() {
                                                Some(index) => cells[2].label(inspect_ui::mono(
                                                    inspect_ui::short(index),
                                                )),
                                                None => cells[2].weak("—"),
                                            };
                                            match row.checked_out.as_deref() {
                                                Some(checked) => cells[3].label(inspect_ui::mono(
                                                    inspect_ui::short(checked),
                                                )),
                                                None => cells[3].weak("—"),
                                            };
                                        },
                                    );
                                    if response.clicked() {
                                        selected = index;
                                    }
                                    if response.double_clicked() {
                                        action = Some(ListAction::Details(request(row)));
                                    }
                                }
                            });
                    },
                );
                let current = rows.get(selected);
                let can_open = current.is_some_and(|row| row.initialized);
                inspect_ui::footer(
                    ui,
                    Some((
                        "Recorded = commit staged in the parent index.",
                        ui.visuals().weak_text_color(),
                    )),
                    |ui| {
                        if ui
                            .add_enabled(
                                current.is_some(),
                                inspect_ui::secondary("Details", "Enter"),
                            )
                            .clicked()
                            && let Some(row) = current
                        {
                            action = Some(ListAction::Details(request(row)));
                        }
                        if ui
                            .add_enabled(can_open, egui::Button::new("Open nested repository"))
                            .on_disabled_hover_text("Initialize the submodule before opening it")
                            .clicked()
                            && let Some(row) = current
                        {
                            action = Some(ListAction::OpenNested(row.path_bytes.clone()));
                        }
                    },
                );
            });
        if keys && action.is_none() {
            let step = crate::navigation::step(ctx, 8);
            if step != 0 && !rows.is_empty() {
                selected = (selected as isize + step).clamp(0, rows.len() as isize - 1) as usize;
            }
            let (enter, escape) = ctx.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                )
            });
            if enter && let Some(row) = rows.get(selected) {
                action = Some(ListAction::Details(request(row)));
            } else if escape {
                action = Some(ListAction::Close);
            }
        }
        self.selected = selected;
        action
    }
}

fn request(row: &Details) -> crate::diff_view::SubmoduleRequest {
    crate::diff_view::SubmoduleRequest {
        path: row.path.clone(),
        path_bytes: row.path_bytes.clone(),
        before: None,
        after: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details() -> Details {
        Details {
            path: "vendor/egui".into(),
            path_bytes: b"vendor/egui".to_vec(),
            recorded: Some(format!("9b1e6d4c{}", "0".repeat(32))),
            before: Some(format!("4f2a91c0{}", "0".repeat(32))),
            index: Some(format!("9b1e6d4c{}", "0".repeat(32))),
            checked_out: Some(format!("4f2a91c0{}", "0".repeat(32))),
            initialized: true,
            dirty: Some(false),
            url: Some("https://github.com/emilk/egui.git".into()),
            branch: Some("main".into()),
            configured_url: Some("git@github.com:emilk/egui.git".into()),
            recorded_available: Some(true),
            status: Some("+".into()),
            note: None,
            subjects: [
                (
                    format!("9b1e6d4c{}", "0".repeat(32)),
                    "Release 0.30.0".to_string(),
                ),
                (
                    format!("4f2a91c0{}", "0".repeat(32)),
                    "Release 0.29.1".to_string(),
                ),
            ]
            .into_iter()
            .collect(),
            behind: Some(14),
        }
    }

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

    fn input(width: f32, events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 740.0),
            )),
            events,
            ..Default::default()
        }
    }

    #[test]
    fn details_show_a_label_grid_with_short_ids_and_same_as_hints() {
        let details = details();
        let mut window = SubmoduleWindow::new(crate::diff_view::SubmoduleRequest {
            path: "vendor/egui".into(),
            path_bytes: b"vendor/egui".to_vec(),
            before: details.before.clone(),
            after: details.recorded.clone(),
        });
        window.details = Some(details);
        window.notice = Some("Updated vendor/egui to 9b1e6d4c.".into());
        for width in [720.0, 1120.0] {
            let ctx = egui::Context::default();
            let _ = crate::tests::run_ui(&ctx, input(width, vec![]), |ui| {
                draw_details(ui.ctx(), &mut window, true);
            });
            let output = crate::tests::run_ui(&ctx, input(width, vec![]), |ui| {
                draw_details(ui.ctx(), &mut window, true);
            });
            let text = text(&output);
            for expected in [
                "Submodule",
                "vendor/egui",
                "Different commit",
                "The checked-out commit is not the one the parent records.",
                "BEFORE",
                "4f2a91c0",
                "Release 0.29.1",
                "AFTER",
                "9b1e6d4c",
                "INDEX",
                "same as After",
                "CHECKED OUT",
                "same as Before · 14 commits behind After",
                "URL",
                "BRANCH",
                "Local config uses a different URL: git@github.com:emilk/egui.git",
                "Updated vendor/egui to 9b1e6d4c.",
                "Open nested repository",
                "Update to 9b1e6d4c…",
            ] {
                assert!(
                    text.iter().any(|line| line.contains(expected)),
                    "missing {expected} at {width}: {text:?}"
                );
            }
            assert!(
                !text.iter().any(|line| line.contains(&"0".repeat(32))),
                "no 40-character hashes: {text:?}"
            );
            let rect = ctx.memory(|memory| memory.area_rect(egui::Id::unique("submodule-details")));
            assert!(rect.is_some_and(|rect| rect.min.x >= 0.0 && rect.max.x <= width));
        }
    }

    #[test]
    fn a_disabled_update_says_why_at_the_footer_left_and_the_confirm_sits_side_by_side() {
        let mut dirty = details();
        dirty.dirty = Some(true);
        let mut window = SubmoduleWindow::new(request(&dirty));
        window.details = Some(dirty);
        let ctx = egui::Context::default();
        let _ = crate::tests::run_ui(&ctx, input(1120.0, vec![]), |ui| {
            draw_details(ui.ctx(), &mut window, true);
        });
        let output = crate::tests::run_ui(&ctx, input(1120.0, vec![]), |ui| {
            draw_details(ui.ctx(), &mut window, true);
        });
        let shown = text(&output);
        assert!(
            shown
                .iter()
                .any(|line| line
                    == "vendor/egui has local changes. Open it to inspect or clean them."),
            "{shown:?}"
        );
        assert!(shown.iter().any(|line| line == "Update…"), "{shown:?}");

        let mut clean = details();
        clean.status = Some(" ".into());
        clean.checked_out = clean.index.clone();
        window.details = Some(clean);
        window.confirm_update = true;
        let mut actions = Vec::new();
        let _ = crate::tests::run_ui(&ctx, input(1120.0, vec![]), |ui| {
            actions.extend(draw_details(ui.ctx(), &mut window, true));
        });
        let output = crate::tests::run_ui(&ctx, input(1120.0, vec![]), |ui| {
            actions.extend(draw_details(ui.ctx(), &mut window, true));
        });
        let shown = text(&output);
        for expected in [
            "Update submodule",
            "Check out",
            "9b1e6d4c",
            "vendor/egui?",
            "Git may fetch from https://github.com/emilk/egui.git. The parent index is not changed.",
            "Cancel",
            "Esc",
            "Update",
            "Enter",
        ] {
            assert!(
                shown.iter().any(|line| line.contains(expected)),
                "missing {expected}: {shown:?}"
            );
        }
        let enter = egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = crate::tests::run_ui(&ctx, input(1120.0, vec![enter]), |ui| {
            actions.extend(draw_details(ui.ctx(), &mut window, true));
        });
        assert_eq!(actions, vec![DetailsAction::Update]);
    }

    #[test]
    fn the_list_has_status_pills_short_ids_and_enter_opens_details()
    -> Result<(), Box<dyn std::error::Error>> {
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir()?;
        let mut list = ListWindow::new(dir.path().into(), &ctx);
        list.wait();
        let mut second = details();
        second.path = "third_party/libgit2".into();
        second.path_bytes = b"third_party/libgit2".to_vec();
        second.initialized = false;
        second.checked_out = None;
        second.dirty = None;
        list.rows = Some(Ok(vec![details(), second]));
        let _ = crate::tests::run_ui(&ctx, input(1120.0, vec![]), |ui| {
            list.draw(ui.ctx());
        });
        let output = crate::tests::run_ui(&ctx, input(1120.0, vec![]), |ui| {
            list.draw(ui.ctx());
        });
        let shown = text(&output);
        for expected in [
            "Submodules",
            "PATH",
            "STATUS",
            "RECORDED",
            "CHECKED OUT",
            "vendor/egui",
            "Different commit",
            "third_party/libgit2",
            "Uninitialized",
            "9b1e6d4c",
            "4f2a91c0",
            "—",
            "Recorded = commit staged in the parent index.",
            "Open nested repository",
            "Details",
        ] {
            assert!(
                shown.iter().any(|line| line.contains(expected)),
                "missing {expected}: {shown:?}"
            );
        }
        let down = egui::Event::Key {
            key: egui::Key::ArrowDown,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = crate::tests::run_ui(&ctx, input(1120.0, vec![down]), |ui| {
            list.draw(ui.ctx());
        });
        let enter = egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let mut action = None;
        let _ = crate::tests::run_ui(&ctx, input(1120.0, vec![enter]), |ui| {
            action = list.draw(ui.ctx());
        });
        match action {
            Some(ListAction::Details(request)) => assert_eq!(request.path, "third_party/libgit2"),
            other => return Err(format!("Enter did not open details: {other:?}").into()),
        }
        Ok(())
    }
}
