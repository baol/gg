//! The footer's **Details…** window: titled by the operation, one
//! plain-language summary line, then Git's raw output in monospace
//! (selectable, scrolling, the window resizable), with the footer's same
//! single recovery action as the filled primary.
use eframe::egui;

/// `Push rejected: origin/main has…` → `Push rejected · details`: the
/// operation is the summary up to its first `: ` or ` · `.
pub fn title(summary: &str) -> String {
    let first = summary.lines().next().unwrap_or_default();
    let cut = [": ", " · "]
        .iter()
        .filter_map(|separator| first.find(separator))
        .min()
        .unwrap_or(first.len());
    let operation = first[..cut].trim().trim_end_matches('.');
    if operation.is_empty() {
        "Operation details".into()
    } else {
        format!("{operation} · details")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Close,
    Recovery,
}

/// Draw the window; `recovery` is the footer's recovery button label.
pub fn show(
    ctx: &egui::Context,
    summary: &str,
    details: &str,
    recovery: Option<&str>,
) -> Option<Act> {
    let mut act = None;
    egui::Window::new(title(summary))
        .id(egui::Id::unique("operation-details"))
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .default_width(600.0)
        .default_height(300.0)
        .min_width(360.0)
        .min_height(160.0)
        .default_pos(ctx.content_rect().center())
        .pivot(egui::Align2::CENTER_CENTER)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(title(summary)).size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("×").on_hover_text("Close (Esc)").clicked() {
                        act = Some(Act::Close);
                    }
                });
            });
            ui.separator();
            ui.add(egui::Label::new(summary).wrap());
            ui.add_space(4.0);
            let footer = 34.0;
            egui::Frame::new()
                .fill(ui.visuals().extreme_bg_color)
                .inner_margin(6.0)
                .show(ui, |ui| {
                    egui::ScrollArea::both()
                        .id_salt("operation-details-output")
                        .max_height((ui.available_height() - footer).max(60.0))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(egui::RichText::new(details).monospace())
                                    .wrap_mode(egui::TextWrapMode::Extend)
                                    .selectable(true),
                            );
                        });
                });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Copy details").clicked() {
                    ui.ctx().copy_text(details.to_owned());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(label) = recovery {
                        let primary = egui::Button::new((
                            egui::RichText::new(label).color(egui::Color32::WHITE),
                            egui::RichText::new("Enter")
                                .small()
                                .color(egui::Color32::from_rgb(184, 212, 230)),
                        ))
                        .fill(crate::desktop_look::primary_fill());
                        if ui.add(primary).clicked() {
                            act = Some(Act::Recovery);
                        }
                    }
                    if ui
                        .button(("Close", egui::RichText::new("Esc").small().weak()))
                        .clicked()
                    {
                        act = Some(Act::Close);
                    }
                });
            });
        });
    if act.is_none() {
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                act = Some(Act::Close);
            } else if recovery.is_some() && i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                act = Some(Act::Recovery);
            }
        });
    }
    act
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_names_the_operation() {
        assert_eq!(
            title("Push rejected: origin/main has 1 commit you do not have"),
            "Push rejected · details"
        );
        assert_eq!(
            title("Rebased onto main · 2 conflicts"),
            "Rebased onto main · details"
        );
        assert_eq!(title("Switched to main"), "Switched to main · details");
        assert_eq!(title(""), "Operation details");
    }
}
