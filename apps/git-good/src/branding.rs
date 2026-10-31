use eframe::egui;
use std::sync::OnceLock;

pub fn icon() -> Result<&'static egui::IconData, String> {
    static ICON: OnceLock<Result<egui::IconData, image::ImageError>> = OnceLock::new();
    ICON.get_or_init(|| {
        let image = image::load_from_memory(include_bytes!("../../../gg.png"))?
            .resize(256, 256, image::imageops::FilterType::Lanczos3)
            .into_rgba8();
        Ok(egui::IconData {
            width: image.width(),
            height: image.height(),
            rgba: image.into_raw(),
        })
    })
    .as_ref()
    .map_err(|error| format!("Load embedded gg icon: {error}"))
}

/// The gg logo at `points` square, cached in `texture` per physical size.
pub fn logo(ui: &mut egui::Ui, texture: &mut Option<egui::TextureHandle>, points: f32) {
    let Ok(icon) = icon() else {
        ui.label("gg");
        return;
    };
    let size = crate::thumbnails::physical_size(points, ui.ctx().pixels_per_point());
    if texture
        .as_ref()
        .is_none_or(|texture| texture.size() != [size, size])
    {
        let source = egui::ColorImage::from_rgba_unmultiplied(
            [icon.width as usize, icon.height as usize],
            &icon.rgba,
        );
        *texture = Some(ui.ctx().load_texture(
            "gg icon",
            crate::thumbnails::resize(&source, size),
            egui::TextureOptions::LINEAR,
        ));
    }
    let Some(texture) = texture.as_ref() else {
        return;
    };
    ui.add(
        egui::Image::new(texture)
            .fit_to_exact_size(egui::vec2(points, points))
            .alt_text("gg · good git"),
    )
    .on_hover_text("gg · good git");
}

/// Version, license, and authors for About gg. Authors are the people who
/// wrote gg; tools used along the way are named in the note instead.
pub struct About {
    pub version: &'static str,
    pub license: &'static str,
    pub authors: Vec<&'static str>,
    pub note: &'static str,
}

pub fn about() -> About {
    About {
        version: env!("CARGO_PKG_VERSION"),
        license: env!("CARGO_PKG_LICENSE"),
        authors: env!("CARGO_PKG_AUTHORS")
            .split(':')
            .filter(|name| !name.is_empty())
            .collect(),
        note: "Parts of gg were written with AI coding assistants. gg comes with no warranty; see the license.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_names_the_license_and_the_human_authors() {
        let about = about();
        assert_eq!(about.license, "Apache-2.0");
        assert_eq!(about.authors, ["Mirko Maischberger"]);
        assert!(about.note.contains("AI"));
    }
}
