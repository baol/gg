//! Compact commit details with a fixed portrait and bounded text fields.
use crate::avatars::Avatars;
use eframe::egui;
use gg_engine::Commit;

pub fn show(ui: &mut egui::Ui, commit: Option<&Commit>, loading: bool, avatars: &mut Avatars) {
    let Some(commit) = commit else {
        ui.label("Select a commit to inspect it.");
        return;
    };
    egui::ScrollArea::vertical()
        .id_salt("commit-details")
        .show(ui, |ui| {
            crate::bounded_text::field(
                ui,
                commit.subject(),
                egui::FontId::proportional(16.0),
                22.0,
            );
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
            let body = if loading {
                "Loading message..."
            } else {
                commit
                    .message
                    .split_once('\n')
                    .map_or("", |(_, body)| body.trim())
            };
            crate::bounded_text::field(ui, body, egui::FontId::proportional(13.0), 20.0);
        });
}
