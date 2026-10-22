//! Shared Phosphor icon font setup for toolbar and status actions.
//!
//! The font asset is vendored so the native app can follow egui releases
//! independently of the icon crate's egui compatibility window. See
//! `assets/Phosphor-LICENSE.txt` for attribution.

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, RichText};
use std::sync::Arc;

pub const ARROW_UP: &str = "\u{E08E}";
pub const ARROW_DOWN: &str = "\u{E03E}";
pub const X: &str = "\u{E4F6}";

const FAMILY: &str = "phosphor-regular";

pub fn icon(glyph: &str) -> RichText {
    RichText::new(glyph).family(FontFamily::Name(FAMILY.into()))
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "phosphor-icons".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/Phosphor.ttf"
        ))),
    );
    fonts.families.insert(
        FontFamily::Name(FAMILY.into()),
        vec!["phosphor-icons".to_owned()],
    );
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_are_available_in_the_vendored_family() {
        let ctx = egui::Context::default();
        install(&ctx);
        let font = egui::FontId::new(10.0, FontFamily::Name(FAMILY.into()));
        let _ = crate::tests::run_ui(&ctx, egui::RawInput::default(), |_ui| {});
        for glyph in [ARROW_UP, ARROW_DOWN, X] {
            assert!(
                ctx.fonts_mut(|fonts| fonts.has_glyphs(&font, glyph)),
                "missing glyph {glyph:?}"
            );
        }
    }
}
