//! Fixed text slots with explicit full-text disclosure.
use eframe::egui;

fn disclosure_id() -> egui::Id {
    egui::Id::new("full-text-disclosure")
}

pub fn field(ui: &mut egui::Ui, value: &str, font: egui::FontId, height: f32) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    paint(ui, rect, value, font, ui.visuals().text_color());
}

pub fn paint(
    ui: &egui::Ui,
    rect: egui::Rect,
    value: &str,
    font: egui::FontId,
    color: egui::Color32,
) -> Option<egui::Rect> {
    let single_line = value.replace(['\n', '\r', '\t'], " ");
    let layout = |width: f32| {
        let mut job =
            egui::text::LayoutJob::simple(single_line.clone(), font.clone(), color, width.max(0.0));
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        job.wrap.overflow_character = None;
        ui.fonts_mut(|fonts| fonts.layout_job(job))
    };
    let mut galley = layout(rect.width());
    let truncated = galley.elided || single_line != value;
    let mut text_rect = rect;
    let more = truncated.then(|| {
        let button = egui::Rect::from_min_max(
            egui::pos2((rect.right() - 24.0).max(rect.left()), rect.top()),
            rect.max,
        );
        text_rect.max.x = button.left();
        galley = layout(text_rect.width());
        let response = ui
            .interact(
                button,
                ui.id().with(("overflow", value)),
                egui::Sense::click(),
            )
            .on_hover_text(value);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Show full text")
        });
        ui.painter().with_clip_rect(button).text(
            button.center(),
            egui::Align2::CENTER_CENTER,
            "...",
            font.clone(),
            color,
        );
        if response.clicked() {
            ui.ctx()
                .data_mut(|data| data.insert_temp(disclosure_id(), value.to_owned()));
        }
        button
    });
    ui.interact(
        text_rect,
        ui.id().with(("text-tooltip", value)),
        egui::Sense::hover(),
    )
    .on_hover_text(value);
    ui.painter().with_clip_rect(text_rect).galley(
        egui::pos2(
            text_rect.left(),
            text_rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        color,
    );
    more
}

pub fn show_disclosure(ctx: &egui::Context) {
    let value = ctx.data(|data| data.get_temp::<String>(disclosure_id()));
    if let Some(value) = value {
        let mut open = true;
        egui::Window::new("Full text")
            .id(disclosure_id())
            .open(&mut open)
            .default_width(560.0)
            .default_height(240.0)
            .resizable(true)
            .show(ctx, |ui| {
                if ui.button("Copy full text").clicked() {
                    ui.ctx().copy_text(value.clone());
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add(egui::Label::new(&value).wrap().selectable(true));
                });
            });
        if !open {
            ctx.data_mut(|data| data.remove::<String>(disclosure_id()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_opens_exact_text_without_changing_the_slot() {
        for scale in [1.0, 2.0] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(scale);
            let value = "Long Unicode subject 日本語\nSecond paragraph ".repeat(30);
            let slot = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(240.0, 26.0));
            let mut draw = |ctx: &egui::Context| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let more = paint(
                        ui,
                        slot,
                        &value,
                        egui::FontId::proportional(13.0),
                        ui.visuals().text_color(),
                    );
                    assert!(more.is_some_and(|rect| slot.contains_rect(rect)));
                });
            };
            let input = |events| egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(720.0, 480.0),
                )),
                events,
                ..Default::default()
            };
            let output = ctx.run(input(vec![]), &mut draw);
            for shape in output.shapes {
                if let egui::epaint::Shape::Text(text) = shape.shape {
                    assert_eq!(text.galley.rows.len(), 1);
                    assert!(text.galley.size().x <= shape.clip_rect.width() + 1.0);
                }
            }
            let position = egui::pos2(slot.right() - 12.0, slot.center().y);
            let _ = ctx.run(
                input(vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]),
                &mut draw,
            );
            let _ = ctx.run(
                input(vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }]),
                &mut draw,
            );
            assert_eq!(
                ctx.data(|data| data.get_temp::<String>(disclosure_id())),
                Some(value)
            );
        }
    }
}
