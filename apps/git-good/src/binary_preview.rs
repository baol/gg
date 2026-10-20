//! Bounded PNG/JPEG decoding and hexadecimal fallback, performed on the diff worker.
use eframe::egui;
use gg_engine::review::{FileChange, FileSide};
use std::{io::Cursor, path::Path};
const ENCODED_LIMIT: u64 = 16 * 1024 * 1024;
const PIXELS: u64 = 8 * 1024 * 1024;
pub struct Preview {
    pub note: String,
    pub image: Option<egui::ColorImage>,
    pub hex: String,
}
pub struct Pair {
    pub before: Preview,
    pub after: Preview,
}
impl Preview {
    fn note(note: impl Into<String>) -> Self {
        Self {
            note: note.into(),
            image: None,
            hex: String::new(),
        }
    }
}
pub fn decode(bytes: &[u8]) -> Preview {
    let hex = bytes
        .chunks(16)
        .take(16)
        .enumerate()
        .map(|(row, bytes)| {
            let digits = bytes
                .iter()
                .map(|b| format!("{b:02x} "))
                .collect::<String>();
            let ascii: String = bytes
                .iter()
                .map(|b| {
                    if b.is_ascii_graphic() || *b == b' ' {
                        char::from(*b)
                    } else {
                        '.'
                    }
                })
                .collect();
            format!("{:08x}  {digits:48} {ascii}", row * 16)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut result = Preview {
        note: format!(
            "Hexadecimal preview · first {} of {} bytes",
            bytes.len().min(256),
            bytes.len()
        ),
        image: None,
        hex,
    };
    let Ok(format) = image::guess_format(bytes) else {
        return result;
    };
    if !matches!(format, image::ImageFormat::Png | image::ImageFormat::Jpeg) {
        return result;
    }
    let image = || -> Result<_, String> {
        let (width, height) = image::ImageReader::with_format(Cursor::new(bytes), format)
            .into_dimensions()
            .map_err(|e| e.to_string())?;
        if width > 4096 || height > 4096 || u64::from(width) * u64::from(height) > PIXELS {
            return Err("Image exceeds the 8-megapixel/4096-pixel preview budget".into());
        }
        let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(4096);
        limits.max_image_height = Some(4096);
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        let image = reader
            .decode()
            .map_err(|e| e.to_string())?
            .thumbnail(width.min(1024), height.min(1024))
            .into_rgba8();
        let size = [image.width() as usize, image.height() as usize];
        Ok((
            width,
            height,
            egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        ))
    };
    match image() {
        Ok((w, h, image)) => {
            result.note = format!(
                "{} · {w} × {h} pixels · fitted preview",
                if format == image::ImageFormat::Png {
                    "PNG"
                } else {
                    "JPEG"
                }
            );
            result.image = Some(image);
        }
        Err(error) => result.note = format!("{error} · hexadecimal preview"),
    }
    result
}
pub fn read(path: &Path, change: &FileChange, cancelled: &dyn Fn() -> bool) -> Pair {
    let load = |side: &Option<FileSide>| {
        let Some(side) = side else {
            return Preview::note("Absent");
        };
        match gg_git::blob::preview(path, side, ENCODED_LIMIT, cancelled) {
            Ok(Some(bytes)) if !cancelled() => decode(&bytes),
            Ok(Some(_)) => Preview::note("Preview cancelled"),
            Ok(None) => {
                Preview::note("Preview budget: 16 MiB per side · export for complete content")
            }
            Err(error) => Preview::note(error),
        }
    };
    Pair {
        before: load(&change.old),
        after: load(&change.new),
    }
}
#[derive(Default)]
pub struct View {
    textures: [Option<egui::TextureHandle>; 2],
    hex: bool,
}
impl View {
    pub fn draw(&mut self, ui: &mut egui::Ui, pair: &Pair) {
        if pair.before.image.is_some() || pair.after.image.is_some() {
            ui.checkbox(&mut self.hex, "Hexadecimal");
        }
        let previews = [&pair.before, &pair.after];
        for (i, p) in previews.iter().enumerate() {
            if self.textures[i].is_none()
                && let Some(image) = &p.image
            {
                self.textures[i] = Some(ui.ctx().load_texture(
                    format!("binary-{i}"),
                    image.clone(),
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
        egui::ScrollArea::both()
            .id_salt("binary-preview")
            .show(ui, |ui| {
                let side = |ui: &mut egui::Ui, i: usize| {
                    ui.strong(if i == 0 { "Before" } else { "After" });
                    let p = previews[i];
                    crate::bounded_text::field(ui, &p.note, egui::FontId::proportional(12.0), 22.0);
                    if !self.hex
                        && let Some(texture) = &self.textures[i]
                    {
                        let size = texture.size_vec2();
                        let scale = (ui.available_width() / size.x).min(1.0);
                        ui.add(egui::Image::new((texture.id(), size * scale)));
                    } else {
                        ui.add(
                            egui::Label::new(egui::RichText::new(&p.hex).monospace())
                                .wrap_mode(egui::TextWrapMode::Extend),
                        );
                    }
                };
                if ui.available_width() >= 600.0
                    && !self.hex
                    && previews
                        .iter()
                        .all(|p| p.image.is_some() || p.hex.is_empty())
                {
                    ui.columns(2, |cols| {
                        side(&mut cols[0], 0);
                        side(&mut cols[1], 1);
                    });
                } else {
                    side(ui, 0);
                    ui.separator();
                    side(ui, 1);
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hexadecimal_rows_keep_fixed_columns_in_narrow_views() {
        let bytes: Vec<_> = (0..256).map(|n| n as u8).collect();
        let pair = Pair {
            before: decode(&bytes),
            after: decode(&bytes),
        };
        for width in [320.0, 720.0] {
            let ctx = egui::Context::default();
            let mut view = View::default();
            let output = ctx.run(crate::tests::frame_input(width, 740.0, vec![]), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| view.draw(ui, &pair));
            });
            let rows: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape
                        && text.galley.job.text.starts_with("00000000")
                    {
                        Some(text.galley.rows.len())
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(rows, vec![16, 16]);
        }
    }
    #[test]
    fn png_and_jpeg_decode_with_dimensions_and_hex_fallback_is_bounded()
    -> Result<(), Box<dyn std::error::Error>> {
        for format in [image::ImageFormat::Png, image::ImageFormat::Jpeg] {
            let pixels = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
                12,
                8,
                image::Rgb([40, 80, 120]),
            ));
            let mut encoded = Cursor::new(Vec::new());
            pixels.write_to(&mut encoded, format)?;
            let result = decode(encoded.get_ref());
            assert_eq!(result.image.as_ref().map(|i| i.size), Some([12, 8]));
            assert!(result.note.contains("12 × 8"));
        }
        let bytes: Vec<_> = (0..65536).map(|n| n as u8).collect();
        let fallback = decode(&bytes);
        assert!(fallback.image.is_none());
        assert_eq!(fallback.hex.lines().count(), 16);
        assert!(fallback.hex.starts_with("00000000  00 01 02"));
        assert!(fallback.note.contains("first 256 of 65536"));
        let mut corrupt = b"\x89PNG\r\n\x1a\n".to_vec();
        corrupt.extend([0; 32]);
        assert!(decode(&corrupt).image.is_none());
        Ok(())
    }
    #[test]
    fn image_dimensions_are_checked_before_decoding_pixels()
    -> Result<(), Box<dyn std::error::Error>> {
        let pixels = image::DynamicImage::ImageRgb8(image::RgbImage::new(4097, 1));
        let mut encoded = Cursor::new(Vec::new());
        pixels.write_to(&mut encoded, image::ImageFormat::Png)?;
        let result = decode(encoded.get_ref());
        assert!(result.image.is_none());
        assert!(result.note.contains("budget"));
        Ok(())
    }
}
