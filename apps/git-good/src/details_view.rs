//! Compact commit details with a fixed portrait and bounded text fields.
use crate::CommitCommand;
use crate::avatars::Avatars;
use eframe::egui;
use gg_engine::Commit;

/// Stable popup identity shared by the visible button and the Alt+A route.
pub fn commit_actions_popup_id() -> egui::Id {
    egui::Id::unique("selected-commit-actions")
}

pub fn show(
    ui: &mut egui::Ui,
    commit: Option<&Commit>,
    loading: bool,
    avatars: &mut Avatars,
    actions: impl FnOnce(&mut egui::Ui) -> Option<CommitCommand>,
) -> Option<CommitCommand> {
    let Some(commit) = commit else {
        ui.label("Select a commit to inspect it.");
        return None;
    };
    let mut command = None;
    let page_height = ui.available_height();
    egui::ScrollArea::vertical()
        .id_salt("commit-details")
        .show(ui, |ui| {
            crate::navigation::scroll(ui, page_height);
            ui.horizontal(|ui| {
                let button_width = 122.0;
                let subject_width = (ui.available_width() - button_width - 6.0).max(80.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(subject_width, 22.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_width(subject_width);
                        crate::bounded_text::field(
                            ui,
                            commit.subject(),
                            egui::FontId::proportional(16.0),
                            22.0,
                        );
                    },
                );
                let response = ui.add(egui::Button::new("Commit actions"));
                egui::Popup::menu(&response)
                    .id(commit_actions_popup_id())
                    .show(|ui| {
                        ui.set_min_width(250.0);
                        command = actions(ui);
                        if command.is_some() {
                            ui.close();
                        }
                    });
            });
            ui.add_space(12.0);
            ui.horizontal_top(|ui| {
                avatars.show(ui, &commit.email, &commit.author, 64.0);
                let width = ui.available_width();
                ui.allocate_ui_with_layout(
                    egui::vec2(width, 64.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        crate::bounded_text::field(
                            ui,
                            &commit.author,
                            egui::FontId::proportional(13.0),
                            18.0,
                        );
                        crate::bounded_text::field(
                            ui,
                            &commit.email,
                            egui::FontId::proportional(12.0),
                            18.0,
                        );
                        crate::bounded_text::field(
                            ui,
                            &commit.time,
                            egui::FontId::proportional(11.0),
                            16.0,
                        );
                    },
                );
            });
            ui.add_space(10.0);
            egui::Grid::new("commit-metadata")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.weak("Commit");
                    let short: String = commit.id.chars().take(12).collect();
                    if ui
                        .button(egui::RichText::new(short).monospace())
                        .on_hover_text(format!("{}\nClick to copy full ID", commit.id))
                        .clicked()
                    {
                        ui.ctx().copy_text(commit.id.clone());
                    }
                    ui.end_row();
                    ui.weak(if commit.parents.len() == 1 {
                        "Parent"
                    } else {
                        "Parents"
                    });
                    let parents = if commit.parents.is_empty() {
                        "Root commit".to_owned()
                    } else {
                        commit.parents.join("\n")
                    };
                    crate::bounded_text::field(ui, &parents, egui::FontId::monospace(11.0), 20.0);
                    ui.end_row();
                });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);
            // While the full message loads its slot stays empty.
            let body = if loading {
                ""
            } else {
                commit
                    .message
                    .split_once('\n')
                    .map_or("", |(_, body)| body.trim())
            };
            crate::bounded_text::field(ui, body, egui::FontId::proportional(13.0), 20.0);
        });
    command
}

/// Height of the bottom commit inspector in History.
pub const INSPECTOR_HEIGHT: f32 = 58.0;

/// The bottom commit inspector from the History mockup: avatar, subject,
/// ID pill, **Review** (`Enter`) and **Actions** (`Alt+A`) on the first line;
/// author, date, and parent on the second. The rows keep their slots while a
/// commit loads, so nothing moves.
pub fn inspector(
    ui: &mut egui::Ui,
    commit: Option<&Commit>,
    avatars: &mut Avatars,
    actions: impl FnOnce(&mut egui::Ui) -> Option<CommitCommand>,
) -> Option<CommitCommand> {
    let mut command = None;
    let Some(commit) = commit else {
        ui.weak("Select a commit to inspect it.");
        return None;
    };
    ui.horizontal(|ui| {
        avatars.show(ui, &commit.email, &commit.author, 22.0);
        let buttons = 190.0;
        let pill = 84.0;
        let subject_width = (ui.available_width() - buttons - pill).max(80.0);
        ui.allocate_ui_with_layout(
            egui::vec2(subject_width, 22.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_max_width(subject_width);
                ui.add(
                    egui::Label::new(egui::RichText::new(commit.subject()).strong().size(13.5))
                        .truncate(),
                )
                .on_hover_text(commit.message.trim());
            },
        );
        let short: String = commit.id.chars().take(8).collect();
        if ui
            .add(
                egui::Button::new(egui::RichText::new(short).monospace().size(11.0))
                    .fill(ui.visuals().faint_bg_color),
            )
            .on_hover_text(format!("{}\nClick to copy the full ID", commit.id))
            .clicked()
        {
            ui.ctx().copy_text(commit.id.clone());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let response = ui.add(egui::Button::new((
                "Actions",
                egui::RichText::new("Alt+A").weak().small(),
            )));
            egui::Popup::menu(&response)
                .id(commit_actions_popup_id())
                .show(|ui| {
                    ui.set_min_width(250.0);
                    command = actions(ui);
                    if command.is_some() {
                        ui.close();
                    }
                });
            if ui
                .add(egui::Button::new((
                    "Review",
                    egui::RichText::new("Enter").weak().small(),
                )))
                .clicked()
            {
                command = Some(CommitCommand::Menu(crate::FrameCommand::ReviewSelected));
            }
        });
    });
    let parents = match commit.parents.as_slice() {
        [] => "root commit".to_string(),
        [parent] => format!("parent {}", &parent[..parent.len().min(8)]),
        parents => format!(
            "parents {}",
            parents
                .iter()
                .map(|parent| &parent[..parent.len().min(8)])
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    ui.add(
        egui::Label::new(
            egui::RichText::new(format!("{} · {} · {parents}", commit.author, commit.time))
                .small()
                .weak(),
        )
        .truncate(),
    );
    command
}
