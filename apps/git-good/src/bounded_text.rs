//! Fixed text slots with explicit full-text disclosure.
use eframe::egui;

fn disclosure_id() -> egui::Id {
    egui::Id::unique("full-text-disclosure")
}

/// What the full-text window shows: its title names the source field
/// (`Commit subject`, `Path`, `Parents`, `Error`); paths and IDs are mono.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Disclosure {
    title: String,
    value: String,
    mono: bool,
}

pub fn field(ui: &mut egui::Ui, value: &str, font: egui::FontId, height: f32) {
    field_named(ui, value, font, height, "Full text");
}

/// [`field`] whose "…" opens a window titled `title`.
pub fn field_named(ui: &mut egui::Ui, value: &str, font: egui::FontId, height: f32, title: &str) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    paint_named(ui, rect, value, font, ui.visuals().text_color(), title);
}

pub fn paint(
    ui: &egui::Ui,
    rect: egui::Rect,
    value: &str,
    font: egui::FontId,
    color: egui::Color32,
) -> Option<egui::Rect> {
    paint_weighted(ui, rect, value, font, color, false, "Full text")
}

/// [`paint`] whose "…" opens a window titled `title`.
pub fn paint_named(
    ui: &egui::Ui,
    rect: egui::Rect,
    value: &str,
    font: egui::FontId,
    color: egui::Color32,
    title: &str,
) -> Option<egui::Rect> {
    paint_weighted(ui, rect, value, font, color, false, title)
}

/// Paint one-line text with a faux-bold weight on the selected row.
pub fn paint_bold(
    ui: &egui::Ui,
    rect: egui::Rect,
    value: &str,
    font: egui::FontId,
    color: egui::Color32,
) -> Option<egui::Rect> {
    paint_weighted(ui, rect, value, font, color, true, "Full text")
}

fn paint_weighted(
    ui: &egui::Ui,
    rect: egui::Rect,
    value: &str,
    font: egui::FontId,
    color: egui::Color32,
    bold: bool,
    title: &str,
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
                ui.scope_id().with(("overflow", value)),
                egui::Sense::click(),
            )
            .on_hover_text(value);
        // A real button named for its field, for screen readers.
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::Role::Button,
                ui.is_enabled(),
                format!("Show full {}", title.to_lowercase()),
            )
        });
        let painter = ui.painter().with_clip_rect(button);
        let at = button.center();
        painter.text(at, egui::Align2::CENTER_CENTER, "...", font.clone(), color);
        if bold {
            painter.text(
                at + egui::vec2(0.4, 0.0),
                egui::Align2::CENTER_CENTER,
                "...",
                font.clone(),
                color,
            );
        }
        if response.clicked() {
            let disclosure = Disclosure {
                title: title.to_owned(),
                value: value.to_owned(),
                mono: font.family == egui::FontFamily::Monospace,
            };
            ui.ctx()
                .data_mut(|data| data.insert_temp(disclosure_id(), disclosure));
        }
        button
    });
    ui.interact(
        text_rect,
        ui.scope_id().with(("text-tooltip", value)),
        egui::Sense::hover(),
    )
    .on_hover_text(value);
    let painter = ui.painter().with_clip_rect(text_rect);
    let at = egui::pos2(
        text_rect.left(),
        text_rect.center().y - galley.size().y / 2.0,
    );
    painter.galley(at, galley.clone(), color);
    if bold {
        painter.galley(at + egui::vec2(0.4, 0.0), galley, color);
    }
    more
}

/// Paint a path on one line, eliding its start so the file name stays
/// visible. Returns whether the path was shortened; the caller names the full
/// path on hover.
pub fn paint_path(
    ui: &egui::Ui,
    rect: egui::Rect,
    value: &str,
    font: egui::FontId,
    color: egui::Color32,
) -> bool {
    let single_line = value.replace(['\n', '\r', '\t'], " ");
    let width = |text: &str| {
        ui.fonts_mut(|fonts| fonts.layout_no_wrap(text.to_owned(), font.clone(), color))
            .size()
            .x
    };
    let shown = if width(&single_line) <= rect.width() {
        single_line
    } else {
        // The shortest suffix start whose "…" + suffix still fits.
        let starts: Vec<usize> = single_line.char_indices().map(|(at, _)| at).collect();
        let (mut low, mut high) = (0, starts.len());
        while low < high {
            let middle = (low + high) / 2;
            if width(&format!("…{}", &single_line[starts[middle]..])) <= rect.width() {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        format!("…{}", starts.get(low).map_or("", |at| &single_line[*at..]))
    };
    let galley = ui.fonts_mut(|fonts| fonts.layout_no_wrap(shown.clone(), font, color));
    let at = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
    ui.painter().with_clip_rect(rect).galley(at, galley, color);
    shown != value
}

/// Keys for an open full-text window, read before any view: `Esc` closes
/// it and `Ctrl+C` copies its text.
pub fn disclosure_keys(ctx: &egui::Context) {
    let Some(shown) = ctx.data(|data| data.get_temp::<Disclosure>(disclosure_id())) else {
        return;
    };
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        ctx.data_mut(|data| data.remove::<Disclosure>(disclosure_id()));
    } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::C)) {
        ctx.copy_text(shown.value);
    }
}

/// The full-text window: titled by its field, the exact text selectable
/// (mono for paths and IDs), then the character count and **Close** (`Esc`)
/// beside the primary **Copy** (`Ctrl+C`). It is not modal.
pub fn show_disclosure(ctx: &egui::Context) {
    let Some(shown) = ctx.data(|data| data.get_temp::<Disclosure>(disclosure_id())) else {
        return;
    };
    let mut close = false;
    let mut copy = false;
    egui::Window::new(&shown.title)
        .id(disclosure_id())
        .title_bar(false)
        .collapsible(false)
        .default_width(560.0)
        .default_height(240.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&shown.title).size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close |= ui.button("×").on_hover_text("Close (Esc)").clicked();
                });
            });
            ui.separator();
            let footer = 34.0;
            egui::ScrollArea::vertical()
                .max_height((ui.available_height() - footer).max(60.0))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    let text = if shown.mono {
                        egui::RichText::new(&shown.value).monospace()
                    } else {
                        egui::RichText::new(&shown.value)
                    };
                    ui.add(egui::Label::new(text).wrap().selectable(true));
                });
            ui.separator();
            ui.horizontal(|ui| {
                ui.weak(format!(
                    "{} characters · text is selectable",
                    shown.value.chars().count()
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let primary = egui::Button::new((
                        egui::RichText::new("Copy").color(egui::Color32::WHITE),
                        egui::RichText::new("Ctrl+C")
                            .small()
                            .color(egui::Color32::from_rgb(184, 212, 230)),
                    ))
                    .fill(crate::welcome_view::PRIMARY);
                    copy |= ui.add(primary).clicked();
                    close |= ui
                        .button(("Close", egui::RichText::new("Esc").small().weak()))
                        .clicked();
                });
            });
        });
    ctx.input_mut(|i| {
        close |= i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
        copy |= i.consume_key(egui::Modifiers::COMMAND, egui::Key::C);
    });
    if copy {
        ctx.copy_text(shown.value.clone());
    }
    if close {
        ctx.data_mut(|data| data.remove::<Disclosure>(disclosure_id()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_paths_keep_their_file_name_and_elide_the_start() {
        for scale in [1.0, 2.0] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(scale);
            let slot = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(160.0, 22.0));
            let long = "apps/very/deeply/nested/folder/structure/with/日本語/stage_view.rs";
            let mut elided = [None, None];
            let output = crate::tests::run_ui(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(720.0, 480.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        let font = egui::FontId::proportional(12.0);
                        let color = ui.visuals().text_color();
                        elided[0] = Some(paint_path(ui, slot, long, font.clone(), color));
                        let below = slot.translate(egui::vec2(0.0, 30.0));
                        elided[1] = Some(paint_path(ui, below, "src/a.rs", font, color));
                    });
                },
            );
            assert_eq!(elided, [Some(true), Some(false)]);
            let texts: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.clone()),
                    _ => None,
                })
                .collect();
            let shortened = texts
                .iter()
                .find(|text| text.galley.job.text.starts_with('…'))
                .map(|text| (text.galley.job.text.clone(), text.galley.size().x));
            let (text, width) = shortened.unwrap_or_default();
            assert!(text.ends_with("/stage_view.rs"), "{text}");
            assert!(width <= slot.width() + 0.5, "{width}");
            assert!(texts.iter().any(|text| text.galley.job.text == "src/a.rs"));
        }
    }

    #[test]
    fn the_full_text_window_is_titled_by_its_field_with_copy_primary() {
        let ctx = egui::Context::default();
        ctx.data_mut(|data| {
            data.insert_temp(
                disclosure_id(),
                Disclosure {
                    title: "Commit subject".into(),
                    value: "Keep the tracking branch visible".into(),
                    mono: false,
                },
            )
        });
        let input = |events| crate::tests::frame_input(720.0, 480.0, events);
        let _ = crate::tests::run_ui(&ctx, input(vec![]), |ui| show_disclosure(ui.ctx()));
        let output = crate::tests::run_ui(&ctx, input(vec![]), |ui| show_disclosure(ui.ctx()));
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for expected in ["Commit subject", "Copy", "Ctrl+C", "Close", "Esc"] {
            assert!(
                text.iter().any(|t| t == expected),
                "missing {expected}: {text:?}"
            );
        }
        assert!(!text.iter().any(|t| t == "Copy full text"));
        let escape = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = crate::tests::run_ui(&ctx, input(vec![escape]), |ui| show_disclosure(ui.ctx()));
        assert!(
            ctx.data(|data| data.get_temp::<Disclosure>(disclosure_id()))
                .is_none(),
            "Esc closes it"
        );
    }

    #[test]
    fn overflow_opens_exact_text_without_changing_the_slot() {
        for scale in [1.0, 2.0] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(scale);
            let value = "Long Unicode subject 日本語\nSecond paragraph ".repeat(30);
            let slot = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(240.0, 26.0));
            let mut draw = |ui: &mut egui::Ui| {
                egui::CentralPanel::default().show(ui, |ui| {
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
            let output = crate::tests::run_ui(&ctx, input(vec![]), &mut draw);
            for shape in output.shapes {
                if let egui::epaint::Shape::Text(text) = shape.shape {
                    assert_eq!(text.galley.rows.len(), 1);
                    assert!(text.galley.size().x <= shape.clip_rect.width() + 1.0);
                }
            }
            let position = egui::pos2(slot.right() - 12.0, slot.center().y);
            let _ = crate::tests::run_ui(
                &ctx,
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
            let _ = crate::tests::run_ui(
                &ctx,
                input(vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }]),
                &mut draw,
            );
            assert_eq!(
                ctx.data(|data| data.get_temp::<Disclosure>(disclosure_id()))
                    .map(|shown| shown.value),
                Some(value)
            );
        }
    }
}
