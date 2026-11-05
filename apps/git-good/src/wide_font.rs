//! A system font for CJK text. The bundled fonts have no CJK glyphs, so
//! such text drew as boxes. The first time egui finds no glyph for a wide
//! character (ideographs, kana, Hangul, fullwidth forms), one system font
//! that has them is read on a background thread, checked, and appended to
//! both font families after every bundled font: Latin text keeps its fonts.
//! At most one read per run; when no font is found, the boxes stay. Source:
//! wordcraft `crates/fonts/src/ui_fallback.rs`, `ui-egui/src/app/fonts.rs`.

use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// The name the font is installed under.
const NAME: &str = "system-cjk";

/// A character every CJK font has: 漢 (Han, U+6F22).
const PROBE: char = '\u{6F22}';

/// Install the hook that notices the first missing wide glyph.
pub fn install(ctx: &egui::Context) {
    ctx.add_font_provider(Arc::new(Watch {
        ctx: ctx.clone(),
        started: AtomicBool::new(false),
    }));
}

/// Asked by egui for each character no installed font has. It answers
/// nothing itself: on the first wide character it starts the read, whose
/// font then replaces the boxes.
struct Watch {
    ctx: egui::Context,
    started: AtomicBool,
}

impl egui::FontProvider for Watch {
    fn font_for(&self, request: &egui::FallbackRequest<'_>) -> Option<egui::FontInsert> {
        let first = request.cluster.chars().next()?;
        if crate::wrap_rows::char_columns(first) == 2 && !self.started.swap(true, Ordering::Relaxed)
        {
            let ctx = self.ctx.clone();
            let _ = std::thread::Builder::new()
                .name("gg-cjk-font".into())
                .spawn(move || {
                    if let Some(font) = find(&candidates()) {
                        ctx.add_font(insert(font));
                        ctx.request_repaint();
                    }
                });
        }
        None
    }
}

/// A font file read and checked: its bytes and the face that has CJK.
struct Found {
    bytes: Vec<u8>,
    index: u32,
}

fn insert(font: Found) -> egui::FontInsert {
    let mut data = egui::FontData::from_owned(font.bytes);
    data.index = font.index;
    egui::FontInsert {
        name: NAME.into(),
        data,
        families: [egui::FontFamily::Monospace, egui::FontFamily::Proportional]
            .into_iter()
            .map(|family| egui::epaint::text::InsertFontFamily {
                family,
                priority: egui::epaint::text::FontPriority::Lowest,
            })
            .collect(),
    }
}

/// The first candidate that parses and draws [`PROBE`]: egui stops on a
/// font it cannot parse, so nothing else reaches it.
fn find(candidates: &[PathBuf]) -> Option<Found> {
    candidates.iter().find_map(|path| check(path))
}

fn check(path: &Path) -> Option<Found> {
    use skrifa::MetadataProvider as _;
    let bytes = std::fs::read(path).ok()?;
    let faces = skrifa::raw::FileRef::new(&bytes).ok()?.fonts().count();
    let index = (0..faces).find(|index| {
        skrifa::FontRef::from_index(&bytes, *index as u32)
            .is_ok_and(|font| font.charmap().map(PROBE).is_some())
    })?;
    Some(Found {
        bytes,
        index: index as u32,
    })
}

/// Font files to try, best first: monospace CJK faces suit code.
fn candidates() -> Vec<PathBuf> {
    let mut found = Vec::new();
    if cfg!(target_os = "macos") {
        for name in [
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/AppleSDGothicNeo.ttc",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            "/Library/Fonts/Arial Unicode.ttf",
        ] {
            found.push(PathBuf::from(name));
        }
    } else if cfg!(windows) {
        let fonts = std::env::var_os("WINDIR")
            .map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from)
            .join("Fonts");
        for name in [
            "msyh.ttc",
            "YuGothR.ttc",
            "meiryo.ttc",
            "msgothic.ttc",
            "malgun.ttf",
            "msjh.ttc",
            "simsun.ttc",
        ] {
            found.push(fonts.join(name));
        }
    } else {
        let mut dirs = vec![
            PathBuf::from("/usr/share/fonts"),
            PathBuf::from("/usr/local/share/fonts"),
        ];
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            dirs.push(home.join(".local/share/fonts"));
            dirs.push(home.join(".fonts"));
        }
        let mut files = Vec::new();
        for dir in dirs {
            font_files(&dir, 5, &mut files);
        }
        found.extend(ranked(files));
    }
    found
}

/// Names of CJK font files, best first.
const LINUX_NAMES: [&str; 11] = [
    "notosansmonocjk",
    "notosanscjk",
    "sourcehansans",
    "notoserifcjk",
    "notosanssc",
    "notosansjp",
    "notosanskr",
    "notosanstc",
    "wqy-microhei",
    "wqy-zenhei",
    "droidsansfallback",
];

/// The files whose name is a known CJK font, best first; regular weights
/// before others.
fn ranked(files: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut known: Vec<(usize, bool, PathBuf)> = files
        .into_iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?.to_ascii_lowercase();
            let rank = LINUX_NAMES
                .iter()
                .position(|known| name.starts_with(known))?;
            let regular = name.contains("regular") || !name.contains('-') || rank >= 8;
            Some((rank, !regular, path))
        })
        .collect();
    known.sort();
    known.into_iter().map(|(_, _, path)| path).collect()
}

/// Font files under `dir`, `depth` folders down at most.
fn font_files(dir: &Path, depth: usize, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if depth > 0 {
                font_files(&path, depth - 1, files);
            }
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "ttf" | "ttc" | "otf" | "otc"
                )
            })
        {
            files.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_cjk_files_rank_monospace_and_regular_first() {
        let files = [
            "/f/wqy/wqy-zenhei.ttc",
            "/f/DejaVuSans.ttf",
            "/f/noto/NotoSansCJK-Bold.ttc",
            "/f/noto/NotoSansCJK-Regular.ttc",
            "/f/noto/NotoSansMonoCJKsc-Regular.otf",
        ]
        .map(PathBuf::from);
        assert_eq!(
            ranked(files.to_vec()),
            [
                "/f/noto/NotoSansMonoCJKsc-Regular.otf",
                "/f/noto/NotoSansCJK-Regular.ttc",
                "/f/noto/NotoSansCJK-Bold.ttc",
                "/f/wqy/wqy-zenhei.ttc",
            ]
            .map(PathBuf::from)
        );
    }

    #[test]
    fn a_file_that_is_not_a_cjk_font_is_never_installed() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        let junk = dir.path().join("NotoSansCJK-Regular.ttc");
        std::fs::write(&junk, b"not a font")?;
        // The bundled Latin font parses but has no ideographs.
        let latin = dir.path().join("latin.ttf");
        std::fs::write(&latin, include_bytes!("../assets/Phosphor.ttf"))?;
        assert!(find(&[junk, latin]).is_none());
        Ok(())
    }

    /// With a CJK font on this machine, the first ideograph laid out loads
    /// it, and the ideograph then draws from it instead of a box.
    #[test]
    fn the_first_ideograph_loads_a_system_font() {
        let Some(path) = candidates().into_iter().find(|path| check(path).is_some()) else {
            eprintln!("skipped: no CJK font on this machine");
            return;
        };
        let ctx = egui::Context::default();
        install(&ctx);
        let font = egui::FontId::monospace(13.0);
        // Ours, not some other fallback, draws the ideograph.
        let has = |ctx: &egui::Context| {
            ctx.fonts_mut(|fonts| {
                fonts.definitions().font_data.contains_key(NAME) && fonts.has_glyph(&font, PROBE)
            })
        };
        let frame = |ctx: &egui::Context| {
            let _ = crate::tests::run_ui(
                ctx,
                crate::tests::frame_input(320.0, 120.0, vec![]),
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.label(egui::RichText::new("漢字 ok").font(font.clone()));
                    });
                },
            );
        };
        frame(&ctx);
        let mut loaded = false;
        for _ in 0..500 {
            frame(&ctx);
            if has(&ctx) {
                loaded = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(loaded, "no glyph from {}", path.display());
    }
}
