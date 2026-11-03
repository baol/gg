//! Shared navigation keys; text editors and open menus retain their input.
use eframe::egui;
pub const READING_HINT: &str = "Up/Down / j k: line · Shift+Up/Down: range · [ ]: change · Alt+Up/Down: file · H: history · B: blame · PgUp/PgDn: page · Esc: files";
pub const HISTORY_HINT: &str = "Alt+Left/Right: version · [ ]: change · Up/Down: line · Shift+Up/Down: range · B: blame · Esc: close history";
pub fn available(ctx: &egui::Context) -> bool {
    !egui::Popup::is_any_open(ctx)
        && !crate::menu_keys::is_focused(ctx)
        && ctx.memory(|m| m.top_modal_layer()).is_none()
        && ctx
            .memory(|m| m.focused())
            .is_none_or(|id| egui::TextEdit::load_state(ctx, id).is_none())
}
/// Decision dialogs are modal: while one is open, everything behind it is
/// dimmed, ignores the pointer, and gets no navigation keys.
pub trait ModalWindow {
    /// Shows this window on the foreground layer `id` as the modal layer.
    fn modal(self, ctx: &egui::Context, id: egui::Id) -> Self;
}

impl ModalWindow for egui::Window<'_> {
    fn modal(self, ctx: &egui::Context, id: egui::Id) -> Self {
        let layer = egui::LayerId::new(egui::Order::Foreground, id);
        ctx.memory_mut(|m| m.set_modal_layer(layer));
        // The marked layer keeps keyboard navigation away from the window
        // behind; this backdrop below the dialog absorbs its clicks and drags.
        let screen = ctx.content_rect();
        egui::Area::new(egui::Id::unique("modal-backdrop"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                ui.painter()
                    .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(96));
                ui.allocate_rect(screen, egui::Sense::click_and_drag());
            });
        self.order(egui::Order::Foreground)
    }
}

pub fn activation_available(ctx: &egui::Context) -> bool {
    available(ctx) && ctx.memory(|m| m.focused()).is_none()
}
pub fn focus_content(ctx: &egui::Context) {
    if let Some(id) = ctx.memory(|m| m.focused()) {
        ctx.memory_mut(|m| m.surrender_focus(id));
    }
}
/// A step past either end of any list: Home and End go to the first and
/// last row, and every caller clamps to its rows.
pub const END: isize = isize::MAX / 4;

/// The list keys and the step each moves the selection: arrows, `j`/`k`,
/// `Ctrl+P`/`Ctrl+N`, a page, and Home/End with or without Ctrl/Cmd.
pub fn keys(page: isize) -> [(egui::Modifiers, egui::Key, isize); 13] {
    [
        (egui::Modifiers::NONE, egui::Key::ArrowUp, -1),
        (egui::Modifiers::NONE, egui::Key::K, -1),
        (egui::Modifiers::CTRL, egui::Key::P, -1),
        (egui::Modifiers::NONE, egui::Key::ArrowDown, 1),
        (egui::Modifiers::NONE, egui::Key::J, 1),
        (egui::Modifiers::CTRL, egui::Key::N, 1),
        (egui::Modifiers::NONE, egui::Key::PageUp, -page),
        (egui::Modifiers::ALT, egui::Key::V, -page),
        (egui::Modifiers::NONE, egui::Key::PageDown, page),
        (egui::Modifiers::NONE, egui::Key::Home, -END),
        (egui::Modifiers::COMMAND, egui::Key::Home, -END),
        (egui::Modifiers::NONE, egui::Key::End, END),
        (egui::Modifiers::COMMAND, egui::Key::End, END),
    ]
}

pub fn step(ctx: &egui::Context, page: isize) -> isize {
    let mut distance: isize = 0;
    for (mods, key, delta) in keys(page) {
        if ctx.input_mut(|i| i.consume_key(mods, key)) {
            distance = distance.saturating_add(delta);
        }
    }
    if distance != 0 {
        focus_content(ctx);
    }
    distance
}

/// Apply navigation inside an ordinary scroll area.
pub fn scroll(ui: &egui::Ui, page_height: f32) {
    if available(ui.ctx()) {
        let step = step(ui.ctx(), (page_height / 20.0).floor().max(2.0) as isize - 1);
        if step != 0 {
            ui.scroll_with_delta(egui::vec2(0.0, -20.0 * step as f32));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_and_end_step_past_either_end_and_page_keys_by_a_page() {
        let ctx = egui::Context::default();
        let press = |key, modifiers| {
            let mut input = egui::RawInput::default();
            input.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            });
            let mut step = 0;
            let _ = crate::tests::run_ui(&ctx, input, |ui| step = super::step(ui.ctx(), 9));
            step
        };
        assert_eq!(press(egui::Key::Home, egui::Modifiers::NONE), -END);
        assert_eq!(press(egui::Key::End, egui::Modifiers::NONE), END);
        assert_eq!(press(egui::Key::End, egui::Modifiers::COMMAND), END);
        assert_eq!(press(egui::Key::PageDown, egui::Modifiers::NONE), 9);
        // Every caller clamps with plain arithmetic: no overflow at the ends.
        assert_eq!((5isize + END).clamp(0, 41), 41);
        assert_eq!(5usize.saturating_add_signed(-END), 0);
    }
    #[test]
    fn navigation_and_change_labels_have_glyphs_in_bundled_fonts() {
        let ctx = egui::Context::default();
        let _ = crate::tests::run_ui(&ctx, Default::default(), |ui| {
            let ctx = ui.ctx();
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                let font = egui::FontId::new(12.0, family);
                for text in [
                    READING_HINT,
                    HISTORY_HINT,
                    crate::colors::change(
                        &egui::Visuals::dark(),
                        &gg_engine::review::ChangeKind::TypeChanged,
                    )
                    .0,
                ] {
                    assert!(
                        ctx.fonts_mut(|fonts| fonts.has_glyphs(&font, text)),
                        "Missing glyph in {text}"
                    );
                }
            }
        });
    }
    #[test]
    fn focused_buttons_allow_navigation_and_text_editors_keep_keys() {
        let ctx = egui::Context::default();
        let mut text = String::new();
        let _ = crate::tests::run_ui(&ctx, Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let button = ui.button("Action");
                button.request_focus();
                assert!(available(ui.ctx()));
                assert!(!activation_available(ui.ctx()));
                focus_content(ui.ctx());
                assert!(activation_available(ui.ctx()));
                let edit = ui.text_edit_singleline(&mut text);
                edit.request_focus();
                assert!(!available(ui.ctx()));
            });
        });
    }
}

#[cfg(test)]
mod modal_tests {
    use super::*;

    #[test]
    fn a_modal_window_blocks_clicks_and_navigation_behind_it() {
        let ctx = egui::Context::default();
        let mut clicked = false;
        let mut rect = egui::Rect::NOTHING;
        let input = |events: Vec<egui::Event>| egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            events,
            ..Default::default()
        };
        let mut navigation = true;
        for frame in 0..5 {
            let events = if frame == 4 {
                let pos = rect.center();
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]
            } else {
                vec![]
            };
            let _ = crate::tests::run_ui(&ctx, input(events), |ctx| {
                navigation = available(ctx);
                egui::CentralPanel::default().show(ctx, |ui| {
                    let response = ui.button("behind");
                    rect = response.rect;
                    clicked |= response.clicked();
                });
                egui::Window::new("dialog")
                    .id(egui::Id::unique("modal-test-dialog"))
                    .modal(ctx, egui::Id::unique("modal-test-dialog"))
                    .default_pos(egui::pos2(400.0, 300.0))
                    .show(ctx, |ui| ui.label("decide"));
            });
        }
        assert!(!clicked, "the backdrop absorbs clicks behind the dialog");
        assert!(!navigation, "navigation keys stay with the dialog");
    }
}
