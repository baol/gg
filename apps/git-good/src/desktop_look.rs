//! The desktop's look: its accent color, its interface and monospace fonts,
//! and its text size, so Git Good looks like the other apps on KDE, GNOME,
//! macOS and Windows.
//!
//! Where each comes from:
//! - **Linux**: the accent from the settings portal (`org.freedesktop.appearance`
//!   `accent-color`, GNOME 47+ and KDE Plasma 6), else KDE's `kdeglobals` or
//!   GNOME's `accent-color` setting; fonts from GNOME's `font-name` and
//!   `monospace-font-name` or KDE's `font=` and `fixed=`, found as files with
//!   `fc-match`; the text size from GNOME's `text-scaling-factor` or KDE's
//!   forced font DPI.
//! - **Windows**: the accent and the text size ("Make text bigger") from the
//!   registry; Segoe UI Variable (else Segoe UI) and Cascadia Mono (else
//!   Consolas).
//! - **macOS**: `NSColor.controlAccentColor`; SF Pro and SF Mono.
//!
//! Everything is read once, best effort, on a worker thread; what a desktop
//! does not answer keeps Git Good's own look. Font files are used only if
//! they parse and have the basic Latin letters; the bundled fonts stay
//! behind them for every other character.

use eframe::egui::{self, Color32};
use std::path::{Path, PathBuf};

/// What the desktop says. `None` and a scale of 1 keep Git Good's own.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub accent: Option<Color32>,
    pub ui_font: Option<PathBuf>,
    pub mono_font: Option<PathBuf>,
    /// The desktop's text size over its default (GNOME's text scaling,
    /// KDE's forced font DPI over 96, Windows' "Make text bigger").
    pub text_scale: f32,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            accent: None,
            ui_font: None,
            mono_font: None,
            text_scale: 1.0,
        }
    }
}

/// The text scales Git Good accepts from a desktop; anything else is a
/// misread and keeps 1.
const SCALES: std::ops::RangeInclusive<f32> = 0.75..=3.0;

fn scale(value: f32) -> Option<f32> {
    SCALES.contains(&value).then_some(value)
}

// ---- Parsers: plain text in, values out (tested on every platform). ----

/// The portal's `accent-color`, as `gdbus` prints it:
/// `(<<(0.2078, 0.5176, 0.8941)>>,)`. Components outside 0–1 mean the
/// desktop has no accent.
pub(crate) fn portal_accent(text: &str) -> Option<Color32> {
    let start = text.rfind('(')? + 1;
    let end = start + text.get(start..)?.find(')')?;
    let parts: Vec<f32> = text
        .get(start..end)?
        .split(',')
        .map(|part| part.trim().parse::<f32>())
        .collect::<Result<_, _>>()
        .ok()?;
    let [r, g, b] = parts.as_slice() else {
        return None;
    };
    let unit = |c: f32| (0.0..=1.0).contains(&c).then(|| (c * 255.0).round() as u8);
    Some(Color32::from_rgb(unit(*r)?, unit(*g)?, unit(*b)?))
}

/// GNOME's named accent (`gsettings get org.gnome.desktop.interface
/// accent-color`, `'blue'`), as libadwaita draws it.
pub(crate) fn gnome_accent(name: &str) -> Option<Color32> {
    let name = name.trim().trim_matches('\'');
    Some(match name {
        "blue" => Color32::from_rgb(0x35, 0x84, 0xe4),
        "teal" => Color32::from_rgb(0x21, 0x90, 0xa4),
        "green" => Color32::from_rgb(0x3a, 0x94, 0x4a),
        "yellow" => Color32::from_rgb(0xc8, 0x88, 0x00),
        "orange" => Color32::from_rgb(0xed, 0x5b, 0x00),
        "red" => Color32::from_rgb(0xe6, 0x2d, 0x42),
        "pink" => Color32::from_rgb(0xd5, 0x61, 0x99),
        "purple" => Color32::from_rgb(0x91, 0x41, 0xac),
        "slate" => Color32::from_rgb(0x6f, 0x83, 0x96),
        _ => return None,
    })
}

/// One `key=value` of an INI section (KDE's `kdeglobals` and `kcmfonts`).
pub(crate) fn ini_value(text: &str, section: &str, key: &str) -> Option<String> {
    let mut inside = false;
    for line in text.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            inside = name == section;
        } else if inside
            && let Some((k, v)) = line.split_once('=')
            && k.trim() == key
        {
            return Some(v.trim().to_string());
        }
    }
    None
}

fn rgb_triplet(value: &str) -> Option<Color32> {
    let parts: Vec<u8> = value
        .split(',')
        .map(|part| part.trim().parse::<u8>())
        .collect::<Result<_, _>>()
        .ok()?;
    match parts.as_slice() {
        [r, g, b] | [r, g, b, _] => Some(Color32::from_rgb(*r, *g, *b)),
        _ => None,
    }
}

/// KDE's accent in `kdeglobals`: `[General] AccentColor=61,174,233`, else
/// the color scheme's selection, `[Colors:Selection] BackgroundNormal`.
pub(crate) fn kde_accent(kdeglobals: &str) -> Option<Color32> {
    ini_value(kdeglobals, "General", "AccentColor")
        .and_then(|value| rgb_triplet(&value))
        .or_else(|| {
            ini_value(kdeglobals, "Colors:Selection", "BackgroundNormal")
                .and_then(|value| rgb_triplet(&value))
        })
}

/// Windows' `AccentColor` (`HKCU\Software\Microsoft\Windows\DWM`), a DWORD
/// `0xAABBGGRR`.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn windows_accent(dword: u32) -> Color32 {
    let [r, g, b, _] = dword.to_le_bytes();
    Color32::from_rgb(r, g, b)
}

/// A `REG_DWORD` in `reg query` output:
/// `    AccentColor    REG_DWORD    0xffd77800`.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn reg_dword(output: &str, value: &str) -> Option<u32> {
    output.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        (fields.next()? == value && fields.next()? == "REG_DWORD")
            .then(|| fields.next())
            .flatten()
            .and_then(|hex| u32::from_str_radix(hex.trim_start_matches("0x"), 16).ok())
    })
}

/// The family of a GNOME font name, `Cantarell 11` or `Noto Sans Bold 10`:
/// everything before the size.
pub(crate) fn gnome_font_family(spec: &str) -> Option<String> {
    let spec = spec.trim().trim_matches('\'').trim();
    let (family, size) = spec.rsplit_once(' ')?;
    size.parse::<f32>().ok()?;
    let family = family.trim();
    (!family.is_empty()).then(|| family.to_string())
}

/// The family of a KDE font value, `Noto Sans,10,-1,5,50,0,0,0,0,0`.
pub(crate) fn kde_font_family(value: &str) -> Option<String> {
    let family = value.split(',').next()?.trim();
    (!family.is_empty()).then(|| family.to_string())
}

/// GNOME's `text-scaling-factor`: `1.25` from `gsettings`, `(<<1.25>>,)`
/// from the portal.
pub(crate) fn gnome_scale(text: &str) -> Option<f32> {
    let number: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    scale(number.parse().ok()?)
}

/// KDE's forced font DPI (`kcmfonts` or `kdeglobals`, `[General]
/// forceFontDPI=120`) over the default 96; 0 means not forced.
pub(crate) fn kde_scale(text: &str) -> Option<f32> {
    let dpi: f32 = ini_value(text, "General", "forceFontDPI")?.parse().ok()?;
    (dpi > 0.0).then(|| dpi / 96.0).and_then(scale)
}

/// Windows' "Make text bigger" (`HKCU\Software\Microsoft\Accessibility`
/// `TextScaleFactor`, 100–225).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn windows_scale(percent: u32) -> Option<f32> {
    scale(percent as f32 / 100.0)
}

/// A font file Git Good can draw with: it parses, and its first usable face
/// has the basic Latin letters and digits.
pub(crate) fn usable_font(path: &Path) -> bool {
    use skrifa::MetadataProvider as _;
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() || meta.len() > 64 << 20 {
        return false;
    }
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    skrifa::FontRef::from_index(&bytes, 0).is_ok_and(|font| {
        let charmap = font.charmap();
        "Aa0".chars().all(|c| charmap.map(c).is_some())
    })
}

/// The first of `candidates` that [`usable_font`] accepts.
#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
fn first_usable(candidates: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|path| usable_font(path))
}

// ---- Applying the look. ----

/// Relative luminance, for WCAG contrast.
fn luminance(color: Color32) -> f32 {
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
        .sum()
}

pub(crate) fn contrast(a: Color32, b: Color32) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let [ar, ag, ab, _] = a.to_array();
    let [br, bg, bb, _] = b.to_array();
    let m = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgb(m(ar, br), m(ag, bg), m(ab, bb))
}

/// `color` moved toward `toward` until it has at least `ratio` contrast with
/// `against` (WCAG AA is 4.5 for text).
pub(crate) fn with_contrast(
    color: Color32,
    toward: Color32,
    against: Color32,
    ratio: f32,
) -> Color32 {
    (0..=20)
        .map(|step| mix(color, toward, step as f32 / 20.0))
        .find(|candidate| contrast(*candidate, against) >= ratio)
        .unwrap_or(toward)
}

/// The colors an accent gives one theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AccentColors {
    /// Selected buttons, the cursor row, primary buttons: white text on it.
    pub fill: Color32,
    /// List rows that are selected.
    pub row: Color32,
    /// Links, focus rings and selected-text strokes.
    pub link: Color32,
}

/// The colors of `accent` in the dark or light theme, adjusted so text on
/// and next to them stays readable: 4.5:1 for white on the fill and for
/// links, and at least 3:1 for the theme's `text` on a selected row (Git
/// Good's own dark rows give 3.4:1).
pub(crate) fn accent_colors(
    accent: Color32,
    dark: bool,
    panel: Color32,
    text: Color32,
) -> AccentColors {
    let fill = with_contrast(accent, Color32::BLACK, Color32::WHITE, 4.5);
    if dark {
        AccentColors {
            fill,
            // `colors::selection` draws dark rows as the fill at 55 %.
            row: with_contrast(mix(panel, fill, 0.55), panel, text, 3.0),
            link: with_contrast(accent, Color32::WHITE, panel, 4.5),
        }
    } else {
        AccentColors {
            fill,
            row: with_contrast(mix(accent, Color32::WHITE, 0.78), Color32::WHITE, text, 3.0),
            link: with_contrast(accent, Color32::BLACK, panel, 4.5),
        }
    }
}

/// Use `accent` for selection and links in both themes; `None` keeps Git
/// Good's own blue. Primary buttons follow through [`set_primary`].
pub fn set_accent(ctx: &egui::Context, accent: Option<Color32>) {
    let Some(accent) = accent else {
        return;
    };
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        ctx.style_mut_of(theme, |style| {
            let v = &mut style.visuals;
            let colors = accent_colors(accent, v.dark_mode, v.panel_fill, v.text_color());
            v.selection.bg_fill = if v.dark_mode { colors.fill } else { colors.row };
            v.selection.stroke.color = if v.dark_mode {
                Color32::WHITE
            } else {
                colors.link
            };
            v.hyperlink_color = colors.link;
        });
    }
}

/// The fill of primary buttons for `accent`, with white text readable on
/// it; Git Good's own without one.
pub fn primary_for(accent: Option<Color32>) -> Color32 {
    accent.map_or(crate::welcome_view::PRIMARY, |accent| {
        with_contrast(accent, Color32::BLACK, Color32::WHITE, 4.5)
    })
}

/// The primary fill the window uses: `0` is Git Good's own, else
/// `0x01RRGGBB`. Only the running app sets it; tests keep the default.
static PRIMARY: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Primary buttons follow `accent` from now on.
pub fn set_primary(accent: Option<Color32>) {
    let value = accent.map_or(0, |c| {
        let [r, g, b, _] = c.to_array();
        0x0100_0000 | u32::from_be_bytes([0, r, g, b])
    });
    PRIMARY.store(value, std::sync::atomic::Ordering::Relaxed);
}

/// The fill of primary buttons now.
pub fn primary_fill() -> Color32 {
    let value = PRIMARY.load(std::sync::atomic::Ordering::Relaxed);
    let accent = (value != 0).then(|| {
        let [_, r, g, b] = value.to_be_bytes();
        Color32::from_rgb(r, g, b)
    });
    primary_for(accent)
}

/// Put the desktop's fonts ahead of the bundled ones: the interface font
/// for proportional text, the monospace one for code. The bundled fonts,
/// Phosphor icons and CJK fallbacks stay behind them.
pub fn install_fonts(ctx: &egui::Context, look: &Look) {
    use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
    for (name, path, family) in [
        ("desktop-ui", &look.ui_font, egui::FontFamily::Proportional),
        ("desktop-mono", &look.mono_font, egui::FontFamily::Monospace),
    ] {
        let Some(bytes) = path.as_ref().and_then(|path| std::fs::read(path).ok()) else {
            continue;
        };
        ctx.add_font(FontInsert {
            name: name.into(),
            data: egui::FontData::from_owned(bytes),
            families: vec![InsertFontFamily {
                family,
                priority: FontPriority::Highest,
            }],
        });
    }
}

// ---- Reading the desktop. ----

/// Read the desktop's look on a worker thread. The receiver gets one
/// [`Look`]; a desktop that answers nothing sends the default.
pub fn read_in_background() -> std::sync::mpsc::Receiver<Look> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("gg-desktop-look".into())
        .spawn(move || {
            let _ = tx.send(read());
        });
    rx
}

/// What this desktop says, now.
pub fn read() -> Look {
    platform::read()
}

/// Linux: whether the session, from its environment `var`, is KDE Plasma.
pub(crate) fn kde_session(var: &dyn Fn(&str) -> Option<String>) -> bool {
    [
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_DESKTOP",
        "DESKTOP_SESSION",
    ]
    .iter()
    .filter_map(|key| var(key))
    .any(|value| {
        let value = value.to_ascii_lowercase();
        value.contains("kde") || value.contains("plasma")
    }) || var("KDE_FULL_SESSION").is_some()
}

/// Linux: what a desktop session says, from `var` (its environment), `run`
/// (a command's trimmed output, `None` when it fails) and `config` (the
/// configuration folder). Pure apart from reading font files, so a test can
/// stand in for KDE or GNOME.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn read_session(
    var: &dyn Fn(&str) -> Option<String>,
    run: &dyn Fn(&str, &[&str]) -> Option<String>,
    config: Option<&Path>,
) -> Look {
    let portal = |namespace: &str, key: &str| {
        run(
            "gdbus",
            &[
                "call",
                "--session",
                "--timeout",
                "1",
                "--dest",
                "org.freedesktop.portal.Desktop",
                "--object-path",
                "/org/freedesktop/portal/desktop",
                "--method",
                "org.freedesktop.portal.Settings.Read",
                namespace,
                key,
            ],
        )
    };
    let gsettings = |schema: &str, key: &str| run("gsettings", &["get", schema, key]);
    // The file `fc-match` picks for `family`, if it is that family's own
    // (not a fallback) and usable.
    let font_file = |family: String| -> Option<PathBuf> {
        let found = run("fc-match", &["-f", "%{file}\n%{family}", &family])?;
        let (path, names) = found.split_once('\n')?;
        let same = names
            .split(',')
            .any(|name| name.trim().eq_ignore_ascii_case(family.trim()));
        let path = PathBuf::from(path.trim());
        (same && usable_font(&path)).then_some(path)
    };
    let file = |name: &str| {
        config
            .and_then(|dir| std::fs::read_to_string(dir.join(name)).ok())
            .unwrap_or_default()
    };
    let kde = kde_session(var);
    let kdeglobals = file("kdeglobals");
    let accent = portal("org.freedesktop.appearance", "accent-color")
        .and_then(|text| portal_accent(&text))
        .or_else(|| kde.then(|| kde_accent(&kdeglobals)).flatten())
        .or_else(|| {
            gsettings("org.gnome.desktop.interface", "accent-color")
                .and_then(|name| gnome_accent(&name))
        });
    let (ui_family, mono_family, text_scale) = if kde {
        (
            ini_value(&kdeglobals, "General", "font").and_then(|v| kde_font_family(&v)),
            ini_value(&kdeglobals, "General", "fixed").and_then(|v| kde_font_family(&v)),
            kde_scale(&file("kcmfonts")).or_else(|| kde_scale(&kdeglobals)),
        )
    } else {
        let interface = |key: &str| {
            portal("org.gnome.desktop.interface", key)
                .or_else(|| gsettings("org.gnome.desktop.interface", key))
        };
        (
            interface("font-name").and_then(|v| gnome_font_family(&v)),
            interface("monospace-font-name").and_then(|v| gnome_font_family(&v)),
            interface("text-scaling-factor").and_then(|v| gnome_scale(&v)),
        )
    };
    Look {
        accent,
        ui_font: ui_family.and_then(font_file),
        mono_font: mono_family.and_then(font_file),
        text_scale: text_scale.unwrap_or(1.0),
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::process::{Command, Stdio};

    pub fn read() -> Look {
        let var = |key: &str| std::env::var(key).ok();
        let run = |program: &str, args: &[&str]| {
            Command::new(program)
                .args(args)
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .ok()
                .filter(|out| out.status.success())
                .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
                .filter(|text| !text.is_empty())
        };
        let config = var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
            .or_else(|| var("HOME").map(|home| PathBuf::from(home).join(".config")));
        read_session(&var, &run, config.as_deref())
    }
}

#[cfg(windows)]
mod platform {
    use super::*;

    /// A DWORD under `HKEY_CURRENT_USER`, read with `reg query`: Git Good
    /// forbids unsafe code, which the registry API needs.
    fn dword(key: &str, value: &str) -> Option<u32> {
        use std::os::windows::process::CommandExt as _;
        // CREATE_NO_WINDOW: no console flashes up.
        let output = std::process::Command::new("reg")
            .args(["query", &format!(r"HKCU\{key}"), "/v", value])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(0x0800_0000)
            .output()
            .ok()
            .filter(|out| out.status.success())?;
        reg_dword(&String::from_utf8_lossy(&output.stdout), value)
    }

    pub fn read() -> Look {
        let fonts = std::env::var_os("WINDIR")
            .map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from)
            .join("Fonts");
        Look {
            accent: dword(r"Software\Microsoft\Windows\DWM", "AccentColor").map(windows_accent),
            ui_font: first_usable(["SegUIVar.ttf", "segoeui.ttf"].map(|name| fonts.join(name))),
            mono_font: first_usable(
                ["CascadiaMono.ttf", "consola.ttf"].map(|name| fonts.join(name)),
            ),
            text_scale: dword(r"Software\Microsoft\Accessibility", "TextScaleFactor")
                .and_then(windows_scale)
                .unwrap_or(1.0),
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;

    fn accent() -> Option<Color32> {
        use objc2_app_kit::{NSColor, NSColorSpace};
        let color = NSColor::controlAccentColor();
        let srgb = color.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
        let unit = |c: f64| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        Some(Color32::from_rgb(
            unit(srgb.redComponent()),
            unit(srgb.greenComponent()),
            unit(srgb.blueComponent()),
        ))
    }

    pub fn read() -> Look {
        let fonts = Path::new("/System/Library/Fonts");
        Look {
            accent: accent(),
            ui_font: first_usable(["SFNS.ttf", "SFNSText.ttf"].map(|name| fonts.join(name))),
            mono_font: first_usable(["SFNSMono.ttf", "Menlo.ttc"].map(|name| fonts.join(name))),
            text_scale: 1.0,
        }
    }
}

#[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
mod platform {
    use super::*;

    pub fn read() -> Look {
        Look::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_portal_accent_reads_gdbus_output_and_rejects_unset() {
        assert_eq!(
            portal_accent("(<<(0.2078, 0.5176, 0.8941)>>,)"),
            Some(Color32::from_rgb(53, 132, 228))
        );
        assert_eq!(
            portal_accent("(<<(-1.0, -1.0, -1.0)>>,)"),
            None,
            "no accent set"
        );
        assert_eq!(portal_accent("Error: no such key"), None);
    }

    #[test]
    fn gnome_and_kde_accents_read_their_settings() {
        assert_eq!(
            gnome_accent("'blue'"),
            Some(Color32::from_rgb(0x35, 0x84, 0xe4))
        );
        assert_eq!(
            gnome_accent("'slate'"),
            Some(Color32::from_rgb(0x6f, 0x83, 0x96))
        );
        assert_eq!(gnome_accent("'chartreuse'"), None);
        let kdeglobals =
            "[General]\nAccentColor=61,174,233\n\n[Colors:Selection]\nBackgroundNormal=1,2,3\n";
        assert_eq!(
            kde_accent(kdeglobals),
            Some(Color32::from_rgb(61, 174, 233))
        );
        assert_eq!(
            kde_accent("[Colors:Selection]\nBackgroundNormal=231,76,60\n"),
            Some(Color32::from_rgb(231, 76, 60)),
            "the color scheme's selection when no accent is set"
        );
        assert_eq!(kde_accent("[General]\nfont=Noto Sans,10\n"), None);
    }

    #[test]
    fn the_windows_accent_dword_is_abgr() {
        assert_eq!(
            windows_accent(0xffd77800),
            Color32::from_rgb(0x00, 0x78, 0xd7)
        );
    }

    #[test]
    fn reg_query_output_gives_its_dword() {
        let output = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\DWM\r\n    AccentColor    REG_DWORD    0xffd77800\r\n\r\n";
        assert_eq!(reg_dword(output, "AccentColor"), Some(0xffd77800));
        assert_eq!(
            reg_dword(
                "    TextScaleFactor    REG_DWORD    0x7d\r\n",
                "TextScaleFactor"
            ),
            Some(125)
        );
        assert_eq!(reg_dword(output, "TextScaleFactor"), None);
        assert_eq!(
            reg_dword("    AccentColor    REG_SZ    blue\r\n", "AccentColor"),
            None
        );
    }

    #[test]
    fn font_settings_name_their_family() {
        assert_eq!(
            gnome_font_family("'Cantarell 11'").as_deref(),
            Some("Cantarell")
        );
        assert_eq!(
            gnome_font_family("'Noto Sans Mono 10.5'").as_deref(),
            Some("Noto Sans Mono")
        );
        assert_eq!(
            gnome_font_family("'Cantarell'"),
            None,
            "no size: not a font name"
        );
        assert_eq!(
            kde_font_family("Noto Sans,10,-1,5,50,0,0,0,0,0").as_deref(),
            Some("Noto Sans")
        );
        assert_eq!(kde_font_family(",10"), None);
    }

    #[test]
    fn text_scales_read_each_desktop_and_reject_misreads() {
        assert_eq!(gnome_scale("1.25"), Some(1.25));
        assert_eq!(gnome_scale("(<<1.5>>,)"), Some(1.5));
        assert_eq!(gnome_scale("12"), None, "out of range");
        assert_eq!(kde_scale("[General]\nforceFontDPI=120\n"), Some(1.25));
        assert_eq!(
            kde_scale("[General]\nforceFontDPI=0\n"),
            None,
            "0: not forced"
        );
        assert_eq!(windows_scale(150), Some(1.5));
        assert_eq!(windows_scale(100), Some(1.0));
    }

    #[test]
    fn only_font_files_that_parse_with_latin_letters_are_used() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let good = dir.path().join("good.ttf");
        let defaults = egui::FontDefinitions::default();
        let bytes = defaults
            .font_data
            .values()
            .map(|data| (*data.font).as_ref().to_vec())
            .find(|bytes| skrifa::FontRef::from_index(bytes, 0).is_ok())
            .unwrap_or_default();
        std::fs::write(&good, bytes)?;
        let junk = dir.path().join("junk.ttf");
        std::fs::write(&junk, b"not a font")?;
        let missing = dir.path().join("missing.ttf");
        assert!(usable_font(&good));
        assert!(!usable_font(&junk));
        assert_eq!(
            first_usable([missing, junk, good.clone()]),
            Some(good),
            "the first usable candidate"
        );
        Ok(())
    }

    /// A font file Git Good accepts, written to `dir`.
    fn fixture_font(dir: &Path, name: &str) -> std::io::Result<PathBuf> {
        let bytes = egui::FontDefinitions::default()
            .font_data
            .values()
            .map(|data| (*data.font).as_ref().to_vec())
            .find(|bytes| skrifa::FontRef::from_index(bytes, 0).is_ok())
            .unwrap_or_default();
        let path = dir.join(name);
        std::fs::write(&path, bytes)?;
        Ok(path)
    }

    #[test]
    fn a_kde_session_gives_its_accent_fonts_and_forced_dpi() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let noto = fixture_font(dir.path(), "NotoSans-Regular.ttf")?;
        let hack = fixture_font(dir.path(), "Hack-Regular.ttf")?;
        std::fs::write(
            dir.path().join("kdeglobals"),
            "[General]\nAccentColor=61,174,233\nfixed=Hack,10,-1,5,50,0,0,0,0,0\nfont=Noto Sans,10,-1,5,50,0,0,0,0,0\n",
        )?;
        std::fs::write(dir.path().join("kcmfonts"), "[General]\nforceFontDPI=120\n")?;
        let var = |key: &str| (key == "XDG_CURRENT_DESKTOP").then(|| "KDE".to_string());
        let (noto_path, hack_path) = (noto.clone(), hack.clone());
        let run = move |program: &str, args: &[&str]| -> Option<String> {
            match (program, args.last().copied()) {
                // Plasma 5: no portal accent.
                ("gdbus", _) | ("gsettings", _) => None,
                ("fc-match", Some("Noto Sans")) => {
                    Some(format!("{}\nNoto Sans", noto_path.display()))
                }
                ("fc-match", Some("Hack")) => Some(format!("{}\nHack", hack_path.display())),
                _ => None,
            }
        };
        let look = read_session(&var, &run, Some(dir.path()));
        assert_eq!(
            look,
            Look {
                accent: Some(Color32::from_rgb(61, 174, 233)),
                ui_font: Some(noto),
                mono_font: Some(hack),
                text_scale: 1.25,
            }
        );
        Ok(())
    }

    #[test]
    fn a_gnome_session_reads_the_portal_then_gsettings() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let cantarell = fixture_font(dir.path(), "Cantarell-VF.otf")?;
        let var = |key: &str| (key == "XDG_CURRENT_DESKTOP").then(|| "ubuntu:GNOME".to_string());
        let path = cantarell.clone();
        let run = move |program: &str, args: &[&str]| -> Option<String> {
            let key = args.last().copied();
            match (program, key) {
                ("gdbus", Some("accent-color")) => Some("(<<(0.5686, 0.2549, 0.6745)>>,)".into()),
                ("gdbus", Some("text-scaling-factor")) => Some("(<<1.5>>,)".into()),
                ("gdbus", _) => None,
                ("gsettings", Some("font-name")) => Some("'Cantarell 11'".into()),
                // fc-match falls back to another family: not used.
                ("gsettings", Some("monospace-font-name")) => Some("'Source Code Pro 10'".into()),
                ("fc-match", Some("Cantarell")) => Some(format!("{}\nCantarell", path.display())),
                ("fc-match", Some(_)) => {
                    Some("/usr/share/fonts/DejaVuSansMono.ttf\nDejaVu Sans Mono".into())
                }
                _ => None,
            }
        };
        let look = read_session(&var, &run, Some(dir.path()));
        assert_eq!(
            look.accent,
            Some(Color32::from_rgb(145, 65, 172)),
            "GNOME purple"
        );
        assert_eq!(look.ui_font, Some(cantarell));
        assert_eq!(
            look.mono_font, None,
            "a fallback family is not the desktop's font"
        );
        assert_eq!(look.text_scale, 1.5);
        Ok(())
    }

    #[test]
    fn a_desktop_that_answers_nothing_keeps_git_goods_look() {
        let look = read_session(&|_| None, &|_, _| None, None);
        assert_eq!(look, Look::default());
    }

    #[test]
    fn every_accent_keeps_text_readable_in_both_themes() {
        let accents = [
            Color32::from_rgb(0x35, 0x84, 0xe4), // GNOME blue
            Color32::from_rgb(0xc8, 0x88, 0x00), // GNOME yellow
            Color32::from_rgb(0x3a, 0x94, 0x4a), // GNOME green
            Color32::from_rgb(61, 174, 233),     // Breeze
            Color32::from_rgb(0x00, 0x78, 0xd7), // Windows
            Color32::from_rgb(255, 255, 0),      // a bright yellow
            Color32::from_rgb(20, 20, 20),       // near black
        ];
        for visuals in [egui::Visuals::dark(), egui::Visuals::light()] {
            let text = visuals.text_color();
            for accent in accents {
                let colors = accent_colors(accent, visuals.dark_mode, visuals.panel_fill, text);
                assert!(
                    contrast(Color32::WHITE, colors.fill) >= 4.5,
                    "{accent:?}: white on the fill"
                );
                assert!(
                    contrast(colors.link, visuals.panel_fill) >= 4.5,
                    "{accent:?}: links on the panel"
                );
                assert!(
                    contrast(text, colors.row) >= 3.0,
                    "{accent:?} dark={}: text on a selected row",
                    visuals.dark_mode
                );
            }
        }
    }

    #[test]
    fn the_accent_reaches_selection_links_and_primary_buttons() {
        let ctx = egui::Context::default();
        let breeze = Color32::from_rgb(61, 174, 233);
        assert_eq!(primary_for(None), crate::welcome_view::PRIMARY);
        set_accent(&ctx, Some(breeze));
        let light = ctx.style_of(egui::Theme::Light);
        let colors = accent_colors(
            breeze,
            false,
            light.visuals.panel_fill,
            light.visuals.text_color(),
        );
        assert_eq!(light.visuals.selection.bg_fill, colors.row);
        assert_eq!(light.visuals.hyperlink_color, colors.link);
        let dark = ctx.style_of(egui::Theme::Dark);
        assert_eq!(
            dark.visuals.selection.bg_fill,
            accent_colors(
                breeze,
                true,
                dark.visuals.panel_fill,
                dark.visuals.text_color()
            )
            .fill
        );
        assert_ne!(primary_for(Some(breeze)), crate::welcome_view::PRIMARY);
        assert!(contrast(Color32::WHITE, primary_for(Some(breeze))) >= 4.5);
    }
}
