//! Shared navigation keys; text editors and open menus retain their input.
use eframe::egui;
pub const READING_HINT: &str = "Up/Down / j k: line · Shift+Up/Down: range · [ ]: change · Alt+Up/Down: file · H: history · PgUp/PgDn: page · Esc: files";
pub const HISTORY_HINT: &str = "Alt+Left/Right: version · [ ]: change · Up/Down: line · Shift+Up/Down: range · Esc: close history";
pub fn available(ctx: &egui::Context) -> bool {
    !egui::Popup::is_any_open(ctx)
        && ctx
            .memory(|m| m.focused())
            .is_none_or(|id| egui::TextEdit::load_state(ctx, id).is_none())
}
pub fn activation_available(ctx: &egui::Context) -> bool {
    available(ctx) && ctx.memory(|m| m.focused()).is_none()
}
pub fn focus_content(ctx: &egui::Context) {
    if let Some(id) = ctx.memory(|m| m.focused()) {
        ctx.memory_mut(|m| m.surrender_focus(id));
    }
}
pub fn step(ctx: &egui::Context, page: isize) -> isize {
    let mut distance = 0;
    for (mods, key, delta) in [
        (egui::Modifiers::NONE, egui::Key::ArrowUp, -1),
        (egui::Modifiers::NONE, egui::Key::K, -1),
        (egui::Modifiers::CTRL, egui::Key::P, -1),
        (egui::Modifiers::NONE, egui::Key::ArrowDown, 1),
        (egui::Modifiers::NONE, egui::Key::J, 1),
        (egui::Modifiers::CTRL, egui::Key::N, 1),
        (egui::Modifiers::NONE, egui::Key::PageUp, -page),
        (egui::Modifiers::ALT, egui::Key::V, -page),
        (egui::Modifiers::NONE, egui::Key::PageDown, page),
    ] {
        if ctx.input_mut(|i| i.consume_key(mods, key)) {
            distance += delta;
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
