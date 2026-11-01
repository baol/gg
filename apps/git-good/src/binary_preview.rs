//! Bounded PNG/JPEG decoding, image comparison, and hexadecimal fallback.
use eframe::egui;
use gg_engine::review::{FileChange, FileSide};
use std::{io::Cursor, path::Path};
const ENCODED_LIMIT: u64 = 16 * 1024 * 1024;
const PIXELS: u64 = 8 * 1024 * 1024;
const NARROW: f32 = 800.0;

pub struct Preview {
    pub note: String,
    pub image: Option<egui::ColorImage>,
    pub hex: String,
    pub id: Option<String>,
    pub bytes: u64,
    pub dimensions: Option<(u32, u32)>,
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
            id: None,
            bytes: 0,
            dimensions: None,
        }
    }

    fn metadata(&self, label: &str) -> String {
        let Some(id) = self.id.as_ref() else {
            return format!("{label} absent");
        };
        let id = id.chars().take(8).collect::<String>();
        let dimensions = self
            .dimensions
            .map(|(width, height)| format!("{width} × {height}"))
            .unwrap_or_else(|| "dimensions unavailable".into());
        format!("{label} {id} · {dimensions} · {}", bytes(self.bytes))
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
        id: None,
        bytes: bytes.len() as u64,
        dimensions: None,
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
            result.dimensions = Some((w, h));
        }
        Err(error) => result.note = format!("{error} · hexadecimal preview"),
    }
    result
}

pub fn read(path: &Path, change: &FileChange, cancelled: &dyn Fn() -> bool) -> Pair {
    let load = |label: &str, side: &Option<FileSide>| {
        let Some(side) = side else {
            return Preview::note(format!("{label} absent"));
        };
        let size = gg_git::blob::size(path, side).unwrap_or_default();
        if side.mode == 0o160000 {
            let mut preview = Preview::note("Gitlink · referenced commit");
            preview.id = Some(side.id.clone());
            return preview;
        }
        match gg_git::blob::preview(path, side, ENCODED_LIMIT, cancelled) {
            Ok(Some(bytes)) if !cancelled() => {
                let mut preview = decode(&bytes);
                preview.id = Some(side.id.clone());
                preview.bytes = bytes.len() as u64;
                preview
            }
            Ok(Some(_)) => {
                let mut preview = Preview::note("Preview cancelled");
                preview.id = Some(side.id.clone());
                preview.bytes = size;
                preview
            }
            Ok(None) => {
                let mut preview =
                    Preview::note("Preview budget: 16 MiB per side · export for complete content");
                preview.id = Some(side.id.clone());
                preview.bytes = size;
                preview
            }
            Err(error) => {
                let mut preview = Preview::note(error);
                preview.id = Some(side.id.clone());
                preview.bytes = size;
                preview
            }
        }
    };
    Pair {
        before: load("Before", &change.old),
        after: load("After", &change.new),
    }
}

fn bytes(value: u64) -> String {
    match value {
        0 => "0 B".into(),
        value if value < 1024 => format!("{value} B"),
        value if value < 1024 * 1024 => format!("{:.1} KiB", value as f64 / 1024.0),
        value => format!("{:.1} MiB", value as f64 / (1024.0 * 1024.0)),
    }
}

fn opacity_alpha(opacity: f32) -> u8 {
    (opacity.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn fit_size(natural: egui::Vec2, bounds: egui::Vec2) -> egui::Vec2 {
    let scale = (bounds.x / natural.x).min(bounds.y / natural.y).min(1.0);
    natural * scale
}

fn checkerboard(painter: &egui::Painter, rect: egui::Rect, visuals: &egui::Visuals) {
    const CELL: f32 = 16.0;
    let clip = rect.intersect(painter.clip_rect());
    if !clip.is_positive() {
        return;
    }
    let (light, dark) = if visuals.dark_mode {
        (egui::Color32::from_gray(58), egui::Color32::from_gray(42))
    } else {
        (egui::Color32::from_gray(215), egui::Color32::from_gray(190))
    };
    let mut y = rect.top();
    let mut row = 0;
    while y < clip.bottom() {
        let mut x = rect.left();
        let mut column = 0;
        while x < clip.right() {
            let cell = egui::Rect::from_min_max(
                egui::pos2(x, y),
                egui::pos2((x + CELL).min(rect.right()), (y + CELL).min(rect.bottom())),
            );
            if cell.intersects(clip) {
                painter.rect_filled(
                    cell,
                    0.0,
                    if (row + column) % 2 == 0 { light } else { dark },
                );
            }
            x += CELL;
            column += 1;
        }
        y += CELL;
        row += 1;
    }
}

/// Draw one bounded LFS content preview with the same fit and checkerboard rules as image differences.
pub fn show_single(
    ui: &mut egui::Ui,
    preview: &Preview,
    texture: &mut Option<egui::TextureHandle>,
) {
    if let Some(image) = &preview.image {
        if texture.is_none() {
            *texture = Some(ui.ctx().load_texture(
                "lfs-content",
                image.clone(),
                egui::TextureOptions::LINEAR,
            ));
        }
        if let Some(texture) = texture {
            let natural = texture.size_vec2();
            let bounds = egui::vec2(ui.available_width().max(40.0), 240.0);
            let size = fit_size(natural, bounds);
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            checkerboard(ui.painter(), rect, ui.visuals());
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
    } else if !preview.hex.is_empty() {
        egui::ScrollArea::both()
            .id_salt("lfs-content-hex")
            .max_height(180.0)
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(&preview.hex).monospace())
                        .wrap_mode(egui::TextWrapMode::Extend),
                );
            });
    }
    ui.weak(&preview.note);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layout {
    SideBySide,
    Swipe,
    Onion,
    /// Hexadecimal rows of both sides: the fourth layout.
    Hex,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Size {
    Fit,
    Actual,
}

/// The image reader: its layout and size controls sit on the reader's one
/// header row ([`View::header`]); the canvas, with a checkerboard behind
/// transparency and a slider strip under it, fills the rest
/// ([`View::draw`]).
pub struct View {
    textures: [Option<egui::TextureHandle>; 2],
    layout: Layout,
    /// Whether `layout` was taken from the remembered choice yet.
    seeded: bool,
    size: Size,
    split: f32,
    opacity: f32,
}

impl Layout {
    const ALL: [(Self, &'static str); 4] = [
        (Self::SideBySide, "Side by side"),
        (Self::Swipe, "Swipe"),
        (Self::Onion, "Onion skin"),
        (Self::Hex, "Hex"),
    ];

    fn from_setting(layout: crate::settings::ImageLayout) -> Self {
        match layout {
            crate::settings::ImageLayout::SideBySide => Self::SideBySide,
            crate::settings::ImageLayout::Swipe => Self::Swipe,
            crate::settings::ImageLayout::Onion => Self::Onion,
            crate::settings::ImageLayout::Hex => Self::Hex,
        }
    }

    fn setting(self) -> crate::settings::ImageLayout {
        match self {
            Self::SideBySide => crate::settings::ImageLayout::SideBySide,
            Self::Swipe => crate::settings::ImageLayout::Swipe,
            Self::Onion => crate::settings::ImageLayout::Onion,
            Self::Hex => crate::settings::ImageLayout::Hex,
        }
    }
}

impl Default for View {
    fn default() -> Self {
        Self {
            textures: [None, None],
            layout: Layout::SideBySide,
            seeded: false,
            size: Size::Fit,
            split: 0.5,
            opacity: 0.5,
        }
    }
}

impl View {
    fn seed(&mut self, ctx: &egui::Context) {
        if !self.seeded {
            self.seeded = true;
            self.layout = Layout::from_setting(crate::settings::image_layout(ctx));
        }
    }

    /// The layout actually drawn: Hex when neither side decoded as an image.
    fn shown(&self, pair: &Pair) -> Layout {
        if pair.before.image.is_none() && pair.after.image.is_none() {
            Layout::Hex
        } else {
            self.layout
        }
    }

    /// The reader-header controls: Side by side | Swipe | Onion skin | Hex
    /// in the layout slot, then the Fit | 1:1 segment.
    pub fn header(&mut self, ui: &mut egui::Ui, pair: &Pair) {
        self.seed(ui.ctx());
        let images = pair.before.image.is_some() || pair.after.image.is_some();
        let both = pair.before.image.is_some() && pair.after.image.is_some();
        let shown = self.shown(pair);
        for (layout, label) in Layout::ALL {
            let available = match layout {
                Layout::Hex => true,
                Layout::SideBySide => images,
                Layout::Swipe | Layout::Onion => both,
            };
            let response = ui
                .add_enabled(
                    available,
                    egui::Button::new(label).selected(shown == layout),
                )
                .on_disabled_hover_text(if images {
                    "Needs both Before and After images"
                } else {
                    "Neither side is a PNG or JPEG image"
                });
            if response.clicked() && self.layout != layout {
                self.layout = layout;
                // Remember the choice for the next comparison.
                crate::settings::set_image_layout(ui.ctx(), layout.setting());
            }
        }
        ui.separator();
        let scaled = shown != Layout::Hex;
        for (size, label, hint) in [
            (Size::Fit, "Fit", "Fit the image to the reader"),
            (Size::Actual, "1:1", "Actual size"),
        ] {
            if ui
                .add_enabled(scaled, egui::Button::new(label).selected(self.size == size))
                .on_hover_text(hint)
                .on_disabled_hover_text("Hex rows have no zoom")
                .clicked()
            {
                self.size = size;
            }
        }
    }

    pub fn draw(&mut self, ui: &mut egui::Ui, pair: &Pair, keyboard: bool) {
        self.seed(ui.ctx());
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
        let layout = self.shown(pair);
        let both_images = pair.before.image.is_some() && pair.after.image.is_some();
        let narrow = ui.available_width() < NARROW;
        // Sliders sit in a strip under the canvas, so the header stays one row.
        let strip = matches!(layout, Layout::Swipe | Layout::Onion) && both_images;
        let page_height = ui.available_height();
        let canvas = egui::vec2(
            ui.available_width(),
            (ui.available_height() - if strip { 30.0 } else { 0.0 }).max(0.0),
        );
        ui.allocate_ui_with_layout(canvas, egui::Layout::top_down(egui::Align::Min), |ui| {
            egui::ScrollArea::both()
                .id_salt("binary-preview")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if keyboard {
                        crate::navigation::scroll(ui, page_height);
                    }
                    match layout {
                        Layout::Onion if both_images => self.onion(ui, pair),
                        Layout::Swipe if both_images => self.swipe(ui, pair),
                        Layout::SideBySide if !narrow => {
                            ui.columns(2, |cols| {
                                self.side(&mut cols[0], pair, 0, layout);
                                self.side(&mut cols[1], pair, 1, layout);
                            });
                        }
                        _ => {
                            // Narrow Side by side and Hex stack Before over After.
                            self.side(ui, pair, 0, layout);
                            ui.separator();
                            self.side(ui, pair, 1, layout);
                        }
                    }
                });
        });
        if strip {
            ui.horizontal(|ui| {
                ui.set_min_height(28.0);
                let (value, label) = if layout == Layout::Swipe {
                    (&mut self.split, "Boundary")
                } else {
                    (&mut self.opacity, "After opacity")
                };
                ui.weak(label);
                ui.add(egui::Slider::new(value, 0.0..=1.0).show_value(false))
                    .on_hover_text("Drag, or focus the slider and use Left/Right");
                ui.weak(format!("{:.0}%", *value * 100.0));
            });
        }
    }

    fn side(&mut self, ui: &mut egui::Ui, pair: &Pair, index: usize, layout: Layout) {
        let preview = if index == 0 {
            &pair.before
        } else {
            &pair.after
        };
        let label = if index == 0 { "Before" } else { "After" };
        ui.strong(label);
        let meta = preview.metadata(label);
        let response = ui.label(egui::RichText::new(meta).monospace().size(11.0));
        if let Some(id) = &preview.id {
            response.on_hover_text(id);
        }
        crate::bounded_text::field(ui, &preview.note, egui::FontId::proportional(12.0), 22.0);
        if layout != Layout::Hex
            && let Some(texture) = &self.textures[index]
        {
            let natural = texture.size_vec2();
            let bounds = egui::vec2(
                ui.available_width().max(40.0),
                (ui.available_height() - 40.0).max(120.0),
            );
            let size = match self.size {
                Size::Fit => fit_size(natural, bounds),
                Size::Actual => natural,
            };
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            checkerboard(ui.painter(), rect, ui.visuals());
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            ui.add(
                egui::Label::new(egui::RichText::new(&preview.hex).monospace())
                    .wrap_mode(egui::TextWrapMode::Extend),
            );
        }
    }

    fn onion(&mut self, ui: &mut egui::Ui, pair: &Pair) {
        let Some(before) = &self.textures[0] else {
            return;
        };
        let Some(after) = &self.textures[1] else {
            return;
        };
        let bounds = egui::vec2(
            ui.available_width().max(40.0),
            (ui.available_height() - 40.0).max(160.0),
        );
        let before_size = match self.size {
            Size::Fit => fit_size(before.size_vec2(), bounds),
            Size::Actual => before.size_vec2(),
        };
        let after_size = match self.size {
            Size::Fit => fit_size(after.size_vec2(), bounds),
            Size::Actual => after.size_vec2(),
        };
        let canvas = egui::vec2(
            before_size.x.max(after_size.x),
            before_size.y.max(after_size.y),
        );
        let (rect, _) = ui.allocate_exact_size(canvas, egui::Sense::hover());
        checkerboard(ui.painter(), rect, ui.visuals());
        let place = |size: egui::Vec2| egui::Rect::from_center_size(rect.center(), size);
        let before_rect = place(before_size);
        ui.painter().image(
            before.id(),
            before_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        let after_rect = place(after_size);
        ui.painter().image(
            after.id(),
            after_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::from_white_alpha(opacity_alpha(self.opacity)),
        );
        let percent = format!("{:.0}%", self.opacity * 100.0);
        ui.painter().text(
            before_rect.left_top() + egui::vec2(8.0, 8.0),
            egui::Align2::LEFT_TOP,
            "Before",
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        ui.painter().text(
            after_rect.right_top() + egui::vec2(-8.0, 8.0),
            egui::Align2::RIGHT_TOP,
            format!("After · {percent}"),
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        let before_meta = pair.before.metadata("Before");
        let after_meta = pair.after.metadata("After");
        ui.horizontal_wrapped(|ui| {
            ui.weak(before_meta);
            ui.separator();
            ui.weak(format!("{after_meta} · overlay {percent}"));
        });
    }

    fn swipe(&mut self, ui: &mut egui::Ui, pair: &Pair) {
        let Some(before) = &self.textures[0] else {
            return;
        };
        let Some(after) = &self.textures[1] else {
            return;
        };
        let bounds = egui::vec2(
            ui.available_width().max(40.0),
            (ui.available_height() - 40.0).max(160.0),
        );
        let before_size = match self.size {
            Size::Fit => fit_size(before.size_vec2(), bounds),
            Size::Actual => before.size_vec2(),
        };
        let after_size = match self.size {
            Size::Fit => fit_size(after.size_vec2(), bounds),
            Size::Actual => after.size_vec2(),
        };
        let canvas = egui::vec2(
            before_size.x.max(after_size.x),
            before_size.y.max(after_size.y),
        );
        let (rect, response) = ui.allocate_exact_size(canvas, egui::Sense::click_and_drag());
        if response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            self.split = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        }
        checkerboard(ui.painter(), rect, ui.visuals());
        let place = |size: egui::Vec2| egui::Rect::from_center_size(rect.center(), size);
        let before_rect = place(before_size);
        ui.painter().image(
            before.id(),
            before_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        let after_rect = place(after_size);
        let boundary = rect.left() + rect.width() * self.split;
        let clip =
            egui::Rect::from_min_max(egui::pos2(boundary, rect.top()), rect.max).intersect(rect);
        ui.painter().with_clip_rect(clip).image(
            after.id(),
            after_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        ui.painter().line_segment(
            [
                egui::pos2(boundary, rect.top()),
                egui::pos2(boundary, rect.bottom()),
            ],
            egui::Stroke::new(1.0, egui::Color32::WHITE),
        );
        ui.painter().text(
            rect.left_top() + egui::vec2(8.0, 8.0),
            egui::Align2::LEFT_TOP,
            "Before",
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        ui.painter().text(
            rect.right_top() + egui::vec2(-8.0, 8.0),
            egui::Align2::RIGHT_TOP,
            "After",
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        let before_meta = pair.before.metadata("Before");
        let after_meta = pair.after.metadata("After");
        ui.horizontal_wrapped(|ui| {
            ui.weak(before_meta);
            ui.separator();
            ui.weak(after_meta);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The header row, then the canvas, as the reader draws them.
    fn both(view: &mut View, ui: &mut egui::Ui, pair: &Pair) {
        ui.horizontal(|ui| view.header(ui, pair));
        view.draw(ui, pair, false);
    }
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
            let output = crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(width, 740.0, vec![]),
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| view.draw(ui, &pair, false));
                },
            );
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
            assert_eq!(result.dimensions, Some((12, 8)));
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
    #[test]
    fn the_layout_starts_from_and_remembers_the_last_choice() {
        let ctx = egui::Context::default();
        crate::settings::set_image_layout(&ctx, crate::settings::ImageLayout::Swipe);
        let image = || {
            let mut preview = Preview::note("image");
            preview.image = Some(egui::ColorImage::filled([2, 2], egui::Color32::RED));
            preview
        };
        let pair = Pair {
            before: image(),
            after: image(),
        };
        let mut view = View::default();
        let output = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(1120.0, 740.0, vec![]),
            |ui| both(&mut view, ui, &pair),
        );
        assert!(
            view.layout == Layout::Swipe,
            "starts from the remembered layout"
        );
        let onion = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.job.text == "Onion skin" => {
                Some(text.visual_bounding_rect().center())
            }
            _ => None,
        });
        let Some(pos) = onion else {
            unreachable!("the Onion skin choice is drawn at wide widths")
        };
        let click = vec![
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
        ];
        let _ = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(1120.0, 740.0, click),
            |ui| both(&mut view, ui, &pair),
        );
        assert_eq!(
            crate::settings::image_layout(&ctx),
            crate::settings::ImageLayout::Onion,
            "the choice is remembered for the next comparison"
        );
        let mut next = View::default();
        let _ = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(1120.0, 740.0, vec![]),
            |ui| both(&mut next, ui, &pair),
        );
        assert!(next.layout == Layout::Onion);
    }

    #[test]
    fn single_content_preview_draws_images_and_hex_within_a_bounded_height()
    -> Result<(), Box<dyn std::error::Error>> {
        let pixels = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            12,
            8,
            image::Rgba([1, 2, 3, 255]),
        ));
        let mut encoded = Cursor::new(Vec::new());
        pixels.write_to(&mut encoded, image::ImageFormat::Png)?;
        for (bytes, expected) in [
            (encoded.get_ref().to_vec(), "PNG"),
            (b"plain content".to_vec(), "Hexadecimal preview"),
        ] {
            let preview = decode(&bytes);
            let ctx = egui::Context::default();
            let mut texture = None;
            let output = crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(720.0, 480.0, vec![]),
                |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| show_single(ui, &preview, &mut texture));
                },
            );
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(text.iter().any(|line| line.contains(expected)), "{text:?}");
        }
        Ok(())
    }

    #[test]
    fn onion_skin_overlays_after_and_requires_both_images() -> Result<(), Box<dyn std::error::Error>>
    {
        let pixels = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            12,
            8,
            image::Rgba([40, 80, 120, 128]),
        ));
        let mut encoded = Cursor::new(Vec::new());
        pixels.write_to(&mut encoded, image::ImageFormat::Png)?;
        let mut before = decode(encoded.get_ref());
        before.id = Some("a".repeat(40));
        before.bytes = encoded.get_ref().len() as u64;
        let mut after = decode(encoded.get_ref());
        after.id = Some("b".repeat(40));
        after.bytes = encoded.get_ref().len() as u64;
        let pair = Pair { before, after };
        let ctx = egui::Context::default();
        let mut view = View {
            layout: Layout::Onion,
            seeded: true,
            opacity: 0.25,
            ..Default::default()
        };
        let output = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(1120.0, 740.0, vec![]),
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| both(&mut view, ui, &pair));
            },
        );
        // The slider strip sits under the canvas, not in the header.
        let y = |needle: &str| {
            output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == needle => {
                    Some(text.pos.y)
                }
                _ => None,
            })
        };
        assert!(
            y("After opacity") > y("Hex"),
            "the slider is below the header row"
        );
        assert!(
            y("After opacity").is_some_and(|y| y > 740.0 - 60.0),
            "the strip is at the bottom"
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for expected in [
            "Onion skin",
            "After opacity",
            "25%",
            "Before",
            "After · 25%",
        ] {
            assert!(
                text.iter().any(|line| line.contains(expected)),
                "missing {expected}: {text:?}"
            );
        }
        assert!(
            text.iter()
                .any(|line| line.contains("overlay 25%") && line.contains("12 × 8")),
            "{text:?}"
        );
        assert_eq!(opacity_alpha(0.5), 128);
        assert_eq!(opacity_alpha(-1.0), 0);
        assert_eq!(opacity_alpha(2.0), 255);

        let mut before = decode(encoded.get_ref());
        before.id = Some("a".repeat(40));
        before.bytes = encoded.get_ref().len() as u64;
        let one_sided = Pair {
            before,
            after: Preview::note("After absent"),
        };
        let output = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(1120.0, 740.0, vec![]),
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| view.draw(ui, &one_sided, false));
            },
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        // One image cannot overlay: the sides stack and the strip goes.
        assert!(
            !text.iter().any(|line| line.contains("After opacity")),
            "{text:?}"
        );
        assert!(
            text.iter().any(|line| line.contains("After absent")),
            "{text:?}"
        );
        Ok(())
    }

    #[test]
    fn fit_preserves_aspect_and_never_upscales() {
        assert_eq!(
            fit_size(egui::vec2(800.0, 400.0), egui::vec2(600.0, 600.0)),
            egui::vec2(600.0, 300.0)
        );
        assert_eq!(
            fit_size(egui::vec2(100.0, 100.0), egui::vec2(600.0, 600.0)),
            egui::vec2(100.0, 100.0)
        );
        assert_eq!(
            fit_size(egui::vec2(100.0, 200.0), egui::vec2(600.0, 300.0)),
            egui::vec2(100.0, 200.0)
        );
    }
    #[test]
    fn presentation_controls_metadata_and_swipe_are_rendered()
    -> Result<(), Box<dyn std::error::Error>> {
        let pixels = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            12,
            8,
            image::Rgba([40, 80, 120, 0]),
        ));
        let mut encoded = Cursor::new(Vec::new());
        pixels.write_to(&mut encoded, image::ImageFormat::Png)?;
        let mut decoded = decode(encoded.get_ref());
        assert_eq!(
            decoded
                .image
                .as_ref()
                .and_then(|image| image.pixels.first())
                .map(|p| p.a()),
            Some(0)
        );
        decoded.id = Some("a".repeat(40));
        decoded.bytes = encoded.get_ref().len() as u64;
        let mut after = decode(encoded.get_ref());
        after.id = Some("b".repeat(40));
        after.bytes = encoded.get_ref().len() as u64;
        let pair = Pair {
            before: decoded,
            after,
        };
        for width in [1120.0, 720.0] {
            let ctx = egui::Context::default();
            let mut view = View::default();
            let output = crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(width, 740.0, vec![]),
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| both(&mut view, ui, &pair));
                },
            );
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            // Every width keeps the four layouts and the Fit | 1:1 segment.
            for label in ["Side by side", "Swipe", "Onion skin", "Hex", "Fit", "1:1"] {
                assert!(
                    text.contains(&label),
                    "missing {label} at {width}: {text:?}"
                );
            }
            assert!(text.iter().any(|line| line.contains("12 × 8")), "{text:?}");
            assert!(
                !text.iter().any(|line| line.contains("Hexadecimal")),
                "no checkbox"
            );
        }
        let ctx = egui::Context::default();
        let mut view = View {
            layout: Layout::Swipe,
            seeded: true,
            ..Default::default()
        };
        let output = crate::tests::run_ui(
            &ctx,
            crate::tests::frame_input(1120.0, 740.0, vec![]),
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| both(&mut view, ui, &pair));
            },
        );
        // The slider strip sits under the canvas, not in the header.
        let y = |needle: &str| {
            output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == needle => {
                    Some(text.pos.y)
                }
                _ => None,
            })
        };
        assert!(
            y("Boundary") > y("Hex"),
            "the slider is below the header row"
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            text.iter().any(|line| line.contains("Boundary")),
            "{text:?}"
        );
        assert!(text.iter().any(|line| line.contains("Before")), "{text:?}");
        assert!(text.iter().any(|line| line.contains("After")), "{text:?}");
        Ok(())
    }
}
