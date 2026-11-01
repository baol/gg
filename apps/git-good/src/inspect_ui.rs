//! The parts the inspect windows share (Submodules, LFS, Signature,
//! Recovery), as the inspect proposal draws them: a title header with a
//! status pill and ×, label grids of 22 px rows with 8-character IDs and
//! copy buttons, status pills and banners in fixed tones, a footer with the
//! result or the disabled reason at its left and the filled primary
//! rightmost, a 22 px status strip, and confirmations in the modal frame.
use crate::phosphor;
use eframe::egui::{self, Color32, RichText};

/// Height of a list or label-grid row.
pub const ROW: f32 = 22.0;
/// Width of a label-grid's label column.
pub const LABEL_WIDTH: f32 = 110.0;

pub const COPY: &str = "\u{E1CA}";
pub const CHECK_CIRCLE: &str = "\u{E184}";
pub const X_CIRCLE: &str = "\u{E4F6}";
pub const WARNING_CIRCLE: &str = "\u{E4E2}";
pub const MINUS_CIRCLE: &str = "\u{E32A}";
pub const CIRCLE_NOTCH: &str = "\u{EB44}";

/// Glyphs these windows draw, checked against the vendored font.
#[cfg(test)]
pub const GLYPHS: [&str; 6] = [
    COPY,
    CHECK_CIRCLE,
    X_CIRCLE,
    WARNING_CIRCLE,
    MINUS_CIRCLE,
    CIRCLE_NOTCH,
];

/// A fixed colour family: amber differs or is untrusted, red is broken,
/// green is clean or good, grey is everything else; blue marks a branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Amber,
    Red,
    Green,
    Blue,
}

impl Tone {
    /// Pill fill and text.
    pub fn pill(self) -> (Color32, Color32) {
        match self {
            Self::Neutral => (
                Color32::from_rgb(46, 46, 46),
                Color32::from_rgb(207, 207, 207),
            ),
            Self::Amber => (
                Color32::from_rgb(77, 58, 20),
                Color32::from_rgb(240, 195, 90),
            ),
            Self::Red => (
                Color32::from_rgb(58, 35, 35),
                Color32::from_rgb(245, 160, 160),
            ),
            Self::Green => (
                Color32::from_rgb(31, 54, 38),
                Color32::from_rgb(150, 220, 170),
            ),
            Self::Blue => (
                Color32::from_rgb(29, 58, 87),
                Color32::from_rgb(156, 202, 245),
            ),
        }
    }

    /// Text colour for a word in this tone (`available`, a receipt).
    pub fn text(self) -> Color32 {
        match self {
            Self::Neutral => Color32::from_rgb(168, 168, 168),
            tone => tone.pill().1,
        }
    }

    /// Banner fill, border, title, and detail colours.
    fn banner(self) -> (Color32, Color32, Color32, Color32) {
        match self {
            Self::Green => (
                Color32::from_rgb(31, 54, 38),
                Color32::from_rgb(44, 82, 54),
                Color32::from_rgb(150, 220, 170),
                Color32::from_rgb(207, 232, 214),
            ),
            Self::Red => (
                Color32::from_rgb(58, 35, 35),
                Color32::from_rgb(90, 48, 48),
                Color32::from_rgb(245, 160, 160),
                Color32::from_rgb(236, 208, 208),
            ),
            Self::Amber => (
                Color32::from_rgb(58, 48, 32),
                Color32::from_rgb(90, 74, 38),
                Color32::from_rgb(240, 195, 90),
                Color32::from_rgb(236, 223, 192),
            ),
            Self::Neutral | Self::Blue => (
                Color32::from_rgb(42, 42, 42),
                Color32::from_rgb(58, 58, 58),
                Color32::from_rgb(230, 230, 230),
                Color32::from_rgb(168, 168, 168),
            ),
        }
    }
}

/// The first 8 characters of an object ID.
pub fn short(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

/// An 18 px status pill.
pub fn pill(ui: &mut egui::Ui, text: &str, tone: Tone) -> egui::Response {
    let (fill, color) = tone.pill();
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_owned(), egui::FontId::proportional(11.5), color);
    let size = egui::vec2(galley.size().x + 14.0, 18.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect_filled(rect, 3.0, fill);
    ui.painter().galley(
        egui::pos2(rect.left() + 7.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Label, true, text));
    response
}

/// A frameless copy button that puts `full` on the clipboard.
pub fn copy_button(ui: &mut egui::Ui, full: &str, what: &str) -> egui::Response {
    let response = ui
        .add(
            egui::Button::new(
                phosphor::icon(COPY)
                    .size(12.0)
                    .color(ui.visuals().weak_text_color()),
            )
            .frame_when_inactive(false)
            .min_size(egui::vec2(20.0, 18.0)),
        )
        .on_hover_text(format!("Copy {what}"));
    if response.clicked() {
        ui.ctx().copy_text(full.to_owned());
    }
    response
}

/// Monospace text at the grid's ID size.
pub fn mono(text: impl Into<String>) -> RichText {
    RichText::new(text).monospace().size(11.5)
}

/// A small upper-case column or grid label.
pub fn caption(text: &str) -> RichText {
    RichText::new(text)
        .size(11.0)
        .color(Color32::from_rgb(140, 140, 140))
}

/// A two-column label grid with 22 px rows; `rows` calls [`row`].
pub fn grid(ui: &mut egui::Ui, id: &str, rows: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id)
        .num_columns(2)
        .min_col_width(LABEL_WIDTH)
        .min_row_height(ROW)
        .spacing([12.0, 0.0])
        .show(ui, rows);
}

/// One label-grid row: the label, then whatever `value` adds on one line.
pub fn row(ui: &mut egui::Ui, label: &str, value: impl FnOnce(&mut egui::Ui)) {
    ui.label(caption(label));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        value(ui);
    });
    ui.end_row();
}

/// The window header: a 15 px title, an optional monospace subject, an
/// optional status pill, then ×. Returns true when × is clicked.
pub fn header(
    ui: &mut egui::Ui,
    title: &str,
    subject: Option<&str>,
    right: impl FnOnce(&mut egui::Ui),
) -> bool {
    let mut close = false;
    ui.horizontal(|ui| {
        ui.set_min_height(28.0);
        ui.label(RichText::new(title).size(15.0));
        if let Some(subject) = subject {
            ui.add(egui::Label::new(mono(subject).size(12.5)).truncate());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            close = ui
                .add(egui::Button::new("×").frame_when_inactive(false))
                .on_hover_text("Close (Esc)")
                .clicked();
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), right);
        });
    });
    ui.separator();
    close
}

/// A filled primary button with its key hint.
pub fn primary<'a>(label: impl Into<String>, key: Option<&'a str>) -> egui::Button<'a> {
    let text = RichText::new(label.into()).color(Color32::WHITE);
    match key {
        Some(key) => egui::Button::new((
            text,
            RichText::new(key)
                .small()
                .color(Color32::from_rgb(184, 212, 230)),
        )),
        None => egui::Button::new(text),
    }
    .fill(crate::welcome_view::PRIMARY)
}

/// A secondary button with a dimmed key hint.
pub fn secondary<'a>(label: impl Into<String>, key: &'a str) -> egui::Button<'a> {
    egui::Button::new((label.into(), RichText::new(key).small().weak()))
}

/// The action footer: `left` (a result or a disabled reason, one
/// truncated line) at the left, then `buttons` laid out right to left, so
/// add the primary first.
pub fn footer(
    ui: &mut egui::Ui,
    left: Option<(&str, Color32)>,
    buttons: impl FnOnce(&mut egui::Ui),
) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.set_min_height(26.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            buttons(ui);
            if let Some((text, color)) = left {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(egui::Label::new(RichText::new(text).color(color)).truncate())
                        .on_hover_text(text);
                });
            }
        });
    });
}

/// The window's 22 px status strip under its footer.
pub fn status_strip(ui: &mut egui::Ui, text: &str, color: Color32) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, ROW), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, Color32::from_rgb(30, 30, 30));
    let galley = ui.painter().layout(
        text.to_owned(),
        egui::FontId::proportional(11.0),
        color,
        width - 20.0,
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 10.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
}

/// A coloured banner with an icon, a title, and a detail line. Returns the
/// link's response when `link` is given.
pub fn banner(
    ui: &mut egui::Ui,
    tone: Tone,
    icon: &str,
    title: &str,
    detail: Option<&str>,
    link: Option<&str>,
) -> Option<egui::Response> {
    let (fill, stroke, title_color, detail_color) = tone.banner();
    let mut clicked = None;
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // Two lines always, so the grid below never moves.
            ui.set_min_height(32.0);
            ui.horizontal(|ui| {
                ui.label(phosphor::icon(icon).size(18.0).color(title_color));
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.label(RichText::new(title).color(title_color));
                    ui.horizontal_wrapped(|ui| {
                        if let Some(detail) = detail {
                            ui.label(RichText::new(detail).size(11.5).color(detail_color));
                        }
                        if let Some(link) = link {
                            clicked = Some(ui.link(RichText::new(link).size(11.5)));
                        }
                    });
                });
            });
        });
    clicked
}

/// A segmented switch; returns the chosen option when it changes.
pub fn segmented<V: Copy + PartialEq>(
    ui: &mut egui::Ui,
    current: V,
    options: &[(V, String)],
) -> Option<V> {
    let mut chosen = None;
    egui::Frame::new()
        .fill(Color32::from_rgb(30, 30, 30))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(51, 51, 51)))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(1))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for (value, label) in options {
                    let on = *value == current;
                    let response = ui.add(
                        egui::Button::new(RichText::new(label).color(if on {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(207, 207, 207)
                        }))
                        .fill(if on {
                            Color32::from_rgb(58, 58, 58)
                        } else {
                            Color32::TRANSPARENT
                        })
                        .frame_when_inactive(on)
                        .selected(false),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::accesskit::Role::Button,
                            ui.is_enabled(),
                            on,
                            label,
                        )
                    });
                    if response.clicked() && !on {
                        chosen = Some(*value);
                    }
                }
            });
        });
    chosen
}

/// A 22 px selectable table row: paints the selection, then lays the cells
/// out left to right at `columns` widths (the last one takes the rest).
pub fn table_row(
    ui: &mut egui::Ui,
    selected: bool,
    columns: &[f32],
    cells: impl FnOnce(&mut [egui::Ui]),
) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, ROW), egui::Sense::click());
    if selected {
        ui.painter()
            .rect_filled(rect, 3.0, Color32::from_rgb(14, 58, 79));
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, 3.0, Color32::from_rgb(44, 44, 44));
    }
    let mut x = rect.left() + 8.0;
    let mut uis = Vec::with_capacity(columns.len() + 1);
    for (index, column) in columns.iter().enumerate() {
        let last = index + 1 == columns.len();
        let right = if last {
            rect.right() - 8.0
        } else {
            (x + column).min(rect.right() - 8.0)
        };
        let cell =
            egui::Rect::from_min_max(egui::pos2(x, rect.top()), egui::pos2(right, rect.bottom()));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(cell)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.set_clip_rect(cell.intersect(ui.clip_rect()));
        uis.push(child);
        x = right + 10.0;
    }
    cells(&mut uis);
    response
}

/// The column header above a [`table_row`] list.
pub fn table_header(ui: &mut egui::Ui, columns: &[f32], titles: &[&str]) {
    table_row_static(ui, columns, |cells| {
        for (cell, title) in cells.iter_mut().zip(titles) {
            cell.label(caption(title));
        }
    });
    ui.separator();
}

fn table_row_static(ui: &mut egui::Ui, columns: &[f32], cells: impl FnOnce(&mut [egui::Ui])) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, ROW), egui::Sense::hover());
    let mut x = rect.left() + 8.0;
    let mut uis = Vec::new();
    for (index, column) in columns.iter().enumerate() {
        let right = if index + 1 == columns.len() {
            rect.right() - 8.0
        } else {
            (x + column).min(rect.right() - 8.0)
        };
        let cell =
            egui::Rect::from_min_max(egui::pos2(x, rect.top()), egui::pos2(right, rect.bottom()));
        uis.push(
            ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(cell)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            ),
        );
        x = right + 10.0;
    }
    cells(&mut uis);
}

/// What a confirmation decided this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Confirm,
    Cancel,
}

/// A confirmation in the modal frame: title and ×, `body`, then a footer
/// with `reason` (why the primary is disabled) at the left and Cancel Esc /
/// the filled primary Enter side by side.
pub fn confirm(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    primary_label: &str,
    reason: Option<&str>,
    danger: bool,
    body: impl FnOnce(&mut egui::Ui),
) -> Option<Decision> {
    let mut decision = None;
    let enabled = reason.is_none();
    let modal = egui::Modal::new(egui::Id::unique(("inspect-confirm", id)))
        .backdrop_color(Color32::from_black_alpha(140))
        .show(ctx, |ui| {
            ui.set_width(440.0);
            if header(ui, title, None, |_| {}) {
                decision = Some(Decision::Cancel);
            }
            ui.add_space(4.0);
            body(ui);
            ui.add_space(4.0);
            let warn = Tone::Amber.text();
            footer(ui, reason.map(|reason| (reason, warn)), |ui| {
                let button = if danger {
                    crate::consequence::danger_button(primary_label, enabled)
                } else if enabled {
                    primary(primary_label, Some("Enter"))
                } else {
                    egui::Button::new(primary_label)
                };
                if ui.add_enabled(enabled, button).clicked() {
                    decision = Some(Decision::Confirm);
                }
                if ui.add(secondary("Cancel", "Esc")).clicked() {
                    decision = Some(Decision::Cancel);
                }
            });
            if enabled && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                decision = Some(Decision::Confirm);
            }
        });
    if modal.should_close() && decision.is_none() {
        decision = Some(Decision::Cancel);
    }
    decision
}

/// Whether keys belong to the window at `id`: it is the topmost window and
/// nothing is typing or has a popup open.
pub fn owns_keys(ctx: &egui::Context, id: egui::Id) -> bool {
    crate::navigation::activation_available(ctx)
        && ctx.memory(|memory| memory.areas().top_layer_id(egui::Order::Middle))
            == Some(egui::LayerId::new(egui::Order::Middle, id))
}

/// `1.4 MiB`, `84 KiB`, `12 B`.
pub fn bytes(value: u64) -> String {
    if value < 1024 {
        format!("{value} B")
    } else if value < 1024 * 1024 {
        format!("{} KiB", (value as f64 / 1024.0).round() as u64)
    } else if value < 1024 * 1024 * 1024 {
        format!("{:.1} MiB", value as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GiB", value as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// `just now`, `12m ago`, `2h ago`, `yesterday`, `3d ago`.
pub fn relative(seconds: i64, now: i64) -> String {
    let delta = (now - seconds).max(0);
    if delta < 60 {
        "just now".into()
    } else if delta < 3_600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86_400 {
        format!("{}h ago", delta / 3_600)
    } else if delta < 2 * 86_400 {
        "yesterday".into()
    } else {
        format!("{}d ago", delta / 86_400)
    }
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

/// `30 Oct 2026, 16:43` for Unix `seconds` at `offset` minutes east of UTC.
pub fn calendar(seconds: i64, offset: i32) -> String {
    let local = seconds + i64::from(offset) * 60;
    let days = local.div_euclid(86_400);
    let minutes = local.rem_euclid(86_400) / 60;
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!(
        "{day} {} {year}, {:02}:{:02}",
        MONTHS[(month - 1) as usize],
        minutes / 60,
        minutes % 60
    )
}

/// The local UTC offset in minutes, as Git formats it (`+0100` → 60).
pub fn parse_offset(text: &str) -> Option<i32> {
    let text = text.trim();
    let (sign, digits) = match text.as_bytes().first()? {
        b'+' => (1, &text[1..]),
        b'-' => (-1, &text[1..]),
        _ => return None,
    };
    if digits.len() != 4 {
        return None;
    }
    let hours: i32 = digits[..2].parse().ok()?;
    let minutes: i32 = digits[2..].parse().ok()?;
    Some(sign * (hours * 60 + minutes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_sizes_and_ids_read_as_the_board_writes_them() {
        assert_eq!(calendar(1_793_374_980, 60), "30 Oct 2026, 16:43");
        assert_eq!(calendar(0, 0), "1 Jan 1970, 00:00");
        assert_eq!(parse_offset("+0100"), Some(60));
        assert_eq!(parse_offset("-0530"), Some(-330));
        assert_eq!(parse_offset("UTC"), None);
        let now = 1_000_000;
        assert_eq!(relative(now - 7_200, now), "2h ago");
        assert_eq!(relative(now - 90_000, now), "yesterday");
        assert_eq!(relative(now - 3 * 86_400, now), "3d ago");
        assert_eq!(bytes(1_468_006), "1.4 MiB");
        assert_eq!(bytes(86_016), "84 KiB");
        assert_eq!(short("9b1e6d4c0123456789"), "9b1e6d4c");
        assert_eq!(short("abc"), "abc");
    }

    #[test]
    fn inspect_glyphs_are_in_the_bundled_icon_font() {
        let ctx = egui::Context::default();
        phosphor::install(&ctx);
        let _ = crate::tests::run_ui(&ctx, Default::default(), |_ui| {});
        let font = egui::FontId::new(14.0, egui::FontFamily::Name("phosphor-regular".into()));
        for glyph in GLYPHS {
            assert!(
                ctx.fonts_mut(|fonts| fonts.has_glyphs(&font, glyph)),
                "missing {glyph:?}"
            );
        }
    }
}
