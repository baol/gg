use eframe::egui;

pub fn physical_size(points: f32, pixels_per_point: f32) -> usize {
    (points * pixels_per_point).round().clamp(1.0, 512.0) as usize
}

pub fn resize(source: &egui::ColorImage, size: usize) -> egui::ColorImage {
    // Filter premultiplied channels together to preserve transparent edges.
    let pixels = image::RgbaImage::from_fn(source.size[0] as u32, source.size[1] as u32, |x, y| {
        image::Rgba(source[(x as usize, y as usize)].to_array())
    });
    let resized = image::imageops::resize(
        &pixels,
        size as u32,
        size as u32,
        image::imageops::FilterType::Lanczos3,
    );
    egui::ColorImage::from_rgba_premultiplied([size, size], resized.as_raw())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnails_integrate_fine_detail_at_each_display_scale() {
        let mut source = egui::ColorImage::filled([128, 128], egui::Color32::BLACK);
        for y in 0..128 {
            for x in 0..128 {
                if (x + y) % 2 == 0 {
                    source[(x, y)] = egui::Color32::WHITE;
                }
            }
        }
        for scale in [1.0, 1.25, 2.0] {
            for points in [18.0, 30.0, 64.0] {
                let size = physical_size(points, scale);
                let thumbnail = resize(&source, size);
                assert_eq!(thumbnail.size, [size, size]);
                if size < 128 {
                    for pixel in &thumbnail.pixels {
                        assert!((110..=145).contains(&pixel.r()));
                    }
                }
            }
        }
    }
}
