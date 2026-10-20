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

pub fn toolbar(ui: &mut egui::Ui, texture: &mut Option<egui::TextureHandle>) {
    let Ok(icon) = icon() else {
        ui.label("gg");
        return;
    };
    let size = crate::thumbnails::physical_size(30.0, ui.ctx().pixels_per_point());
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
            .fit_to_exact_size(egui::vec2(30.0, 30.0))
            .alt_text("gg · good git"),
    )
    .on_hover_text("gg · good git");
}
