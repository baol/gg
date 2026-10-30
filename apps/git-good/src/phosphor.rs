//! Shared Phosphor icon font setup for toolbar and status actions.
//!
//! The font asset is vendored so the native app can follow egui releases
//! independently of the icon crate's egui compatibility window. See
//! `assets/Phosphor-LICENSE.txt` for attribution.

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, RichText};
use std::sync::Arc;

pub const ARROW_UP: &str = "\u{E08E}";
pub const ARROW_DOWN: &str = "\u{E03E}";
pub const CHECK: &str = "\u{E182}";
pub const ARCHIVE: &str = "\u{E00C}";
pub const ARROW_LINE_DOWN: &str = "\u{E05C}";
pub const ARROW_LINE_UP: &str = "\u{E066}";
pub const CLOCK_COUNTER_CLOCKWISE: &str = "\u{E1A0}";
pub const DOWNLOAD_SIMPLE: &str = "\u{E20C}";
pub const GIT_BRANCH: &str = "\u{E278}";
pub const GIT_MERGE: &str = "\u{E280}";
pub const LIST_NUMBERS: &str = "\u{E2F6}";
pub const SIDEBAR_SIMPLE: &str = "\u{EC24}";
pub const STACK: &str = "\u{E466}";
pub const TAG: &str = "\u{E478}";

/// Glyphs used by the window toolbar, checked against the vendored font.
#[cfg(test)]
pub const TOOLBAR: [&str; 11] = [
    ARCHIVE,
    ARROW_LINE_DOWN,
    ARROW_LINE_UP,
    CLOCK_COUNTER_CLOCKWISE,
    DOWNLOAD_SIMPLE,
    GIT_BRANCH,
    GIT_MERGE,
    LIST_NUMBERS,
    SIDEBAR_SIMPLE,
    STACK,
    TAG,
];

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
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("phosphor-icons".to_owned());
    }
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
        for glyph in [ARROW_UP, ARROW_DOWN, CHECK].into_iter().chain(TOOLBAR) {
            assert!(
                ctx.fonts_mut(|fonts| fonts.has_glyphs(&font, glyph)),
                "missing glyph {glyph:?}"
            );
        }
    }

    #[test]
    fn default_families_fall_back_to_phosphor_icons() {
        let ctx = egui::Context::default();
        install(&ctx);
        let _ = crate::tests::run_ui(&ctx, egui::RawInput::default(), |_ui| {});
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            let font = egui::FontId::new(11.0, family);
            for glyph in [ARROW_UP, ARROW_DOWN, CHECK] {
                assert!(
                    ctx.fonts_mut(|fonts| fonts.has_glyphs(&font, glyph)),
                    "missing fallback glyph {glyph:?}"
                );
            }
        }
    }

    #[test]
    fn tracking_label_uses_phosphor_arrows() {
        let tracking = gg_engine::refs::Tracking {
            branch: "main".into(),
            full_name: "refs/heads/main".into(),
            upstream: "origin/main".into(),
            upstream_full_name: "refs/remotes/origin/main".into(),
            remote: "origin".into(),
            remote_branch: "main".into(),
            ahead: Some(1),
            behind: Some(2),
        };
        let label = crate::refs_view::tracking_label(&tracking);
        assert!(label.contains(ARROW_UP), "{label:?}");
        assert!(label.contains(ARROW_DOWN), "{label:?}");
    }
}
