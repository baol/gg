//! Semantic colors shared by history, changed files, and diffs.
use eframe::egui::{Color32, Visuals};
use gg_engine::{diff::RowKind, review::ChangeKind};

pub fn diff(visuals: &Visuals, kind: RowKind) -> (&'static str, Color32, Color32) {
    if !visuals.dark_mode {
        return match kind {
            RowKind::Added => (
                "+",
                Color32::from_rgb(20, 96, 45),
                Color32::from_rgb(225, 242, 229),
            ),
            RowKind::Deleted => (
                "-",
                Color32::from_rgb(155, 32, 38),
                Color32::from_rgb(252, 229, 229),
            ),
            RowKind::Hunk => (
                "",
                Color32::from_rgb(25, 75, 120),
                Color32::from_rgb(226, 236, 246),
            ),
            _ => ("", visuals.strong_text_color(), Color32::TRANSPARENT),
        };
    }
    match kind {
        RowKind::Added => (
            "+",
            Color32::from_rgb(150, 220, 170),
            Color32::from_rgba_unmultiplied(35, 95, 55, 65),
        ),
        RowKind::Deleted => (
            "-",
            Color32::from_rgb(245, 160, 160),
            Color32::from_rgba_unmultiplied(115, 40, 40, 65),
        ),
        RowKind::Hunk => ("", Color32::from_rgb(135, 185, 230), Color32::from_gray(35)),
        _ => ("", visuals.strong_text_color(), Color32::TRANSPARENT),
    }
}
pub fn change(visuals: &Visuals, kind: &ChangeKind) -> (&'static str, Color32) {
    if !visuals.dark_mode {
        return match kind {
            ChangeKind::Added => ("+ Added", Color32::from_rgb(20, 100, 45)),
            ChangeKind::Deleted => ("− Deleted", Color32::from_rgb(160, 35, 40)),
            ChangeKind::Modified => ("~ Modified", Color32::from_rgb(119, 76, 0)),
            ChangeKind::TypeChanged => ("~ Type", Color32::from_rgb(105, 58, 160)),
        };
    }
    match kind {
        ChangeKind::Added => ("+ Added", Color32::from_rgb(120, 200, 150)),
        ChangeKind::Deleted => ("− Deleted", Color32::from_rgb(240, 140, 140)),
        ChangeKind::Modified => ("~ Modified", Color32::from_rgb(220, 195, 125)),
        ChangeKind::TypeChanged => ("~ Type", Color32::from_rgb(170, 160, 230)),
    }
}
/// One-character change marker; the full kind from [`change`] belongs in the tooltip.
pub fn change_symbol(kind: &ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Added => "+",
        ChangeKind::Deleted => "−",
        ChangeKind::Modified => "~",
        ChangeKind::TypeChanged => "T",
    }
}

pub fn selection(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        visuals.selection.bg_fill.gamma_multiply(0.55)
    } else {
        Color32::from_rgb(211, 230, 244)
    }
}

pub fn apply(ctx: &eframe::egui::Context) {
    for theme in [eframe::egui::Theme::Dark, eframe::egui::Theme::Light] {
        let style = ctx.style_of(theme);
        let visuals = &style.visuals;
        if visuals.dark_mode || visuals.weak_text_color == Some(Color32::from_gray(90)) {
            continue;
        }
        ctx.style_mut_of(theme, |style| {
            let v = &mut style.visuals;
            v.weak_text_color = Some(Color32::from_gray(90));
            v.hyperlink_color = Color32::from_rgb(25, 85, 135);
            v.warn_fg_color = Color32::from_rgb(119, 76, 0);
            v.error_fg_color = Color32::from_rgb(160, 35, 40);
            v.selection.bg_fill = selection(v);
            v.selection.stroke.color = Color32::from_rgb(25, 75, 120);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_change_markers_match_their_labels() {
        for kind in [
            ChangeKind::Added,
            ChangeKind::Deleted,
            ChangeKind::Modified,
            ChangeKind::TypeChanged,
        ] {
            let symbol = change_symbol(&kind);
            assert_eq!(symbol.chars().count(), 1);
            let (label, _) = change(&Visuals::dark(), &kind);
            assert!(label.ends_with(match kind {
                ChangeKind::Added => "Added",
                ChangeKind::Deleted => "Deleted",
                ChangeKind::Modified => "Modified",
                ChangeKind::TypeChanged => "Type",
            }));
        }
    }

    fn contrast(foreground: Color32, background: Color32) -> f32 {
        let luminance = |color: Color32| {
            let [r, g, b, _] = color.to_array();
            [r, g, b]
                .into_iter()
                .zip([0.2126, 0.7152, 0.0722])
                .map(|(c, w)| {
                    let v = f32::from(c) / 255.0;
                    w * if v <= 0.04045 {
                        v / 12.92
                    } else {
                        ((v + 0.055) / 1.055).powf(2.4)
                    }
                })
                .sum::<f32>()
        };
        let a = luminance(foreground);
        let b = luminance(background);
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
    #[test]
    fn light_semantic_text_has_contrast_on_plain_selected_and_diff_rows() {
        let visuals = Visuals::light();
        for kind in [RowKind::Added, RowKind::Deleted, RowKind::Hunk] {
            let (_, text, fill) = diff(&visuals, kind);
            assert!(contrast(text, fill) >= 4.5, "{kind:?} diff contrast");
        }
        for kind in [
            ChangeKind::Added,
            ChangeKind::Deleted,
            ChangeKind::Modified,
            ChangeKind::TypeChanged,
        ] {
            let (_, text) = change(&visuals, &kind);
            for fill in [visuals.panel_fill, selection(&visuals)] {
                assert!(contrast(text, fill) >= 4.5, "{kind:?} status contrast");
            }
        }
    }

    #[test]
    fn light_accents_are_readable_and_dark_visuals_are_preserved() {
        let ctx = eframe::egui::Context::default();
        let dark = ctx.style_of(eframe::egui::Theme::Dark).visuals.clone();
        apply(&ctx);
        assert_eq!(ctx.style_of(eframe::egui::Theme::Dark).visuals, dark);
        let light = ctx.style_of(eframe::egui::Theme::Light).visuals.clone();
        for text in [
            light.weak_text_color(),
            light.hyperlink_color,
            light.error_fg_color,
            light.warn_fg_color,
            light.selection.stroke.color,
        ] {
            for background in [light.panel_fill, light.selection.bg_fill] {
                assert!(contrast(text, background) >= 4.5);
            }
        }
        // Re-applying preserves the established dark palette.
        apply(&ctx);
        assert_eq!(ctx.style_of(eframe::egui::Theme::Dark).visuals, dark);
    }
}
