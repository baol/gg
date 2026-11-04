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
    .map_err(|error| format!("Load embedded Git Good icon: {error}"))
}

/// The gg logo at `points` square, cached in `texture` per physical size.
pub fn logo(ui: &mut egui::Ui, texture: &mut Option<egui::TextureHandle>, points: f32) {
    let Ok(icon) = icon() else {
        ui.label("Git Good");
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
            "Git Good icon",
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
            .alt_text("Git Good"),
    )
    .on_hover_text("Git Good");
}

/// Version, license, and authors for About Git Good. Authors are the people who
/// wrote gg; tools used along the way are named in the note instead.
pub struct About {
    pub version: &'static str,
    /// The commit gg was built from (8 characters), from `build.rs`.
    pub commit: &'static str,
    /// The build date, `YYYY-MM-DD` (UTC).
    pub built: &'static str,
    /// The Rust target triple.
    pub target: &'static str,
    pub license: &'static str,
    pub authors: Vec<&'static str>,
    pub note: &'static str,
}

pub fn about() -> About {
    About {
        version: env!("CARGO_PKG_VERSION"),
        commit: env!("GG_BUILD_COMMIT"),
        built: env!("GG_BUILD_DATE"),
        target: env!("GG_BUILD_TARGET"),
        license: env!("CARGO_PKG_LICENSE"),
        authors: env!("CARGO_PKG_AUTHORS")
            .split(':')
            .filter(|name| !name.is_empty())
            .collect(),
        note: "Parts of Git Good were written with AI coding assistants. Git Good comes with no warranty; see the license.",
    }
}

/// Help › About Git Good in the dialog frame: title and × in the first row, the
/// logo, version and a label grid with the build facts, then the footer
/// with **Copy version info** and **Close**. Returns true when it closes.
pub fn show_about(ctx: &egui::Context, texture: &mut Option<egui::TextureHandle>) -> bool {
    let about = about();
    let mut close = false;
    egui::Window::new("About Git Good")
        .id(egui::Id::unique("about-gg"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_pos(ctx.content_rect().center())
        .pivot(egui::Align2::CENTER_CENTER)
        .show(ctx, |ui| {
            ui.set_width(360.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("About Git Good").size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close |= ui.button("×").on_hover_text("Close (Esc)").clicked();
                });
            });
            ui.separator();
            ui.vertical_centered(|ui| {
                logo(ui, texture, 96.0);
                ui.heading("Git Good");
                ui.weak(format!("Version {}", about.version));
            });
            ui.add_space(8.0);
            egui::Grid::new("about-gg-facts")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    ui.weak("Commit");
                    ui.monospace(about.commit);
                    ui.end_row();
                    ui.weak("Built");
                    ui.label(format!("{} · {}", about.built, about.target));
                    ui.end_row();
                    ui.weak("Author");
                    ui.label(about.authors.join(", "));
                    ui.end_row();
                    ui.weak("License");
                    ui.hyperlink_to(about.license, "https://www.apache.org/licenses/LICENSE-2.0");
                    ui.end_row();
                    ui.weak("Source");
                    ui.hyperlink_to("github.com/baol/gg", "https://github.com/baol/gg");
                    ui.end_row();
                });
            ui.add_space(8.0);
            ui.add(egui::Label::new(egui::RichText::new(about.note).weak()).wrap());
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .button("Copy version info")
                    .on_hover_text("Copy the four build lines as plain text for bug reports")
                    .clicked()
                {
                    ui.ctx().copy_text(about.version_info());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close |= ui
                        .button(("Close", egui::RichText::new("Esc").small().weak()))
                        .clicked();
                });
            });
        });
    close || ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
}

impl About {
    /// What **Copy version info** puts on the clipboard, as plain text for
    /// bug reports: the four build lines, then Git's version (once read)
    /// and the operating system.
    pub fn version_info(&self) -> String {
        format!(
            "Git Good {}\ncommit {}\nbuilt {}\ntarget {}\ngit {}\nos {}",
            self.version,
            self.commit,
            self.built,
            self.target,
            crate::diagnostics::known_git_version()
                .map(|version| version.trim_start_matches("git version "))
                .unwrap_or("unknown"),
            crate::diagnostics::os()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_info_is_plain_build_git_and_os_lines() {
        let info = about().version_info();
        let lines = info.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 6, "{info}");
        assert_eq!(lines[0], format!("Git Good {}", env!("CARGO_PKG_VERSION")));
        assert!(lines[1].starts_with("commit ") && lines[1].len() > "commit ".len());
        let date = lines[2].trim_start_matches("built ");
        assert_eq!(date.len(), 10, "{date}");
        assert_eq!(date.as_bytes()[4], b'-');
        assert!(lines[3].starts_with("target ") && lines[3].contains('-'));
        assert!(lines[4].starts_with("git "), "{info}");
        assert!(
            lines[5].starts_with(&format!("os {}", std::env::consts::OS)),
            "{info}"
        );
    }

    #[test]
    fn about_names_the_license_and_the_human_authors() {
        let about = about();
        assert_eq!(about.license, "Apache-2.0");
        assert_eq!(about.authors, ["Mirko Maischberger"]);
        assert!(about.note.contains("AI"));
    }
}
