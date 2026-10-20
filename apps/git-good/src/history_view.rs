//! Shared column geometry for headers, rows, and overlap assertions.
use crate::avatars::Avatars;
use eframe::egui;
use gg_engine::Commit;

pub const ROW_HEIGHT: f32 = 26.0;
pub const MIN_WIDTH: f32 = 660.0;

pub fn columns(rect: egui::Rect) -> [egui::Rect; 5] {
    let widths = [
        24.0,
        (rect.width() - 24.0 - 168.0 - 76.0 - 132.0).max(0.0),
        168.0,
        76.0,
        132.0,
    ];
    let mut x = rect.left();
    widths.map(|width| {
        let cell =
            egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(width, rect.height()));
        x += width;
        cell
    })
}

fn text(ui: &egui::Ui, rect: egui::Rect, value: &str, color: egui::Color32, mono: bool) {
    let rect = rect.shrink2(egui::vec2(6.0, 0.0));
    if rect.width() <= 0.0 {
        return;
    }
    crate::bounded_text::paint(
        ui,
        rect,
        value,
        if mono {
            egui::FontId::monospace(11.0)
        } else {
            egui::FontId::proportional(12.0)
        },
        color,
    );
}

pub fn header(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Sense::hover(),
    );
    for (cell, label) in columns(rect)
        .into_iter()
        .zip(["", "Commit", "Author", "Hash", "Date"])
    {
        text(ui, cell, label, ui.visuals().text_color(), false);
    }
    ui.painter().hline(
        rect.x_range(),
        rect.bottom(),
        ui.visuals().widgets.noninteractive.bg_stroke,
    );
}

pub fn row(
    ui: &mut egui::Ui,
    commit: &Commit,
    selected: bool,
    avatars: &mut Avatars,
) -> egui::Response {
    let response = ui.add_sized(
        [ui.available_width(), ROW_HEIGHT],
        egui::Button::new("")
            .fill(if selected {
                crate::colors::selection(ui.visuals())
            } else {
                egui::Color32::TRANSPARENT
            })
            .stroke(egui::Stroke::NONE)
            .corner_radius(0.0)
            .selected(selected),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            commit.subject(),
        )
    });
    let [dot, subject, author, hash, date] = columns(response.rect);
    let color = if selected {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().strong_text_color().gamma_multiply(0.8)
    };
    let muted = if selected {
        color
    } else {
        ui.visuals().text_color()
    };
    ui.painter()
        .circle_filled(dot.center(), 3.0, ui.visuals().selection.stroke.color);
    if commit.parents.len() > 1 {
        ui.painter().circle_stroke(
            dot.center(),
            6.0,
            egui::Stroke::new(1.0_f32, ui.visuals().selection.stroke.color),
        );
    }
    text(ui, subject, commit.subject(), color, false);
    let avatar = egui::Rect::from_center_size(
        egui::pos2(author.left() + 14.0, author.center().y),
        egui::vec2(18.0, 18.0),
    );
    avatars.paint(ui, &commit.email, &commit.author, avatar);
    text(
        ui,
        egui::Rect::from_min_max(egui::pos2(author.left() + 25.0, author.top()), author.max),
        &commit.author,
        muted,
        false,
    );
    text(
        ui,
        hash,
        &commit.id.chars().take(8).collect::<String>(),
        muted,
        true,
    );
    let short_date = commit
        .time
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ");
    text(ui, date, &short_date, muted, false);
    response.on_hover_text(format!(
        "{}\n{}\n{}\n{}",
        commit.subject(),
        commit.author,
        commit.id,
        commit.time
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cells_are_disjoint_and_contained_at_all_supported_widths() {
        for width in [MIN_WIDTH, 720.0, 1120.0, 2560.0] {
            let rect =
                egui::Rect::from_min_size(egui::pos2(17.0, 30.0), egui::vec2(width, ROW_HEIGHT));
            let cells = columns(rect);
            for cell in cells {
                assert!(rect.contains_rect(cell));
                assert_eq!(cell.height(), ROW_HEIGHT);
            }
            for pair in cells.windows(2) {
                assert!(pair[0].right() <= pair[1].left());
            }
            assert!(cells[1].width() >= 200.0);
        }
    }
    #[test]
    fn long_fields_stay_single_line_at_supported_scales() {
        for scale in [1.0, 2.0] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(scale);
            let commit = Commit {
                id: "a".repeat(40),
                parents: vec![],
                author: "Long author\nname ".repeat(20),
                email: "test@example.org".into(),
                time: "2026-10-17 12:30:00 +0200".into(),
                message: "Long subject ".repeat(100),
            };
            let mut avatars = Avatars::default();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(720.0, 480.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let response = row(ui, &commit, true, &mut avatars);
                        assert_eq!(response.rect.height(), ROW_HEIGHT);
                    });
                },
            );
            let mut count = 0;
            for shape in &output.shapes {
                if let egui::epaint::Shape::Text(text) = &shape.shape {
                    if text.galley.job.text.is_empty() {
                        continue;
                    }
                    assert_eq!(text.galley.rows.len(), 1);
                    assert!(text.galley.size().x <= shape.clip_rect.width() + 1.0);
                    count += 1;
                }
            }
            assert!(count >= 5); // Fields, avatar fallback, and overflow controls.
        }
    }
}
