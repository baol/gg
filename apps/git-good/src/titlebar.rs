//! Adapted from photocraft crates/ui-egui/src/panels/titlebar.rs and panels/title_bar.rs.
//!
//! The window's top row: the menus at the left and the window identity in
//! the free gap. On Windows and Linux, unless Preferences › Appearance › Use
//! the system title bar is on, the window has no OS decorations and this row
//! is the title bar ([`Chrome::custom`]): only the free gap between the menus
//! and the caption buttons drags the window, a double-click there maximizes
//! it, caption buttons minimize, maximize/restore and close, and invisible
//! zones along the window's edges resize it.
//!
//! The caption buttons follow the desktop ([`CaptionStyle`], from
//! `caption_style`): Windows' 46 pt buttons at the right end by default; on
//! Linux libadwaita's round buttons (GNOME), Breeze's (KDE) or a plain round
//! set elsewhere, on the side and in the order the desktop's button layout
//! says. The glyphs are drawn here.

use crate::menus;
use eframe::egui::{
    self, Color32, CursorIcon, Id, LayerId, Order, PointerButton, Pos2, Rect, ResizeDirection,
    Sense, Stroke, Ui, ViewportCommand, pos2, vec2,
};

/// One Windows caption button: as wide as the native ones.
pub const BUTTON_WIDTH: f32 = 46.0;
/// Resize zone thickness along an edge, and the side of a corner zone.
const EDGE: f32 = 8.0;
const CORNER: f32 = 12.0;
/// Windows 11's Close hover red, and its glyph.
const WINDOWS_CLOSE: Color32 = Color32::from_rgb(0xc4, 0x2b, 0x1c);
/// Breeze's Close button on hover and press (KDE's Breeze decoration colours).
const BREEZE_CLOSE_HOVER: Color32 = Color32::from_rgb(0xff, 0x98, 0xa2);
const BREEZE_CLOSE_PRESSED: Color32 = Color32::from_rgb(0x6d, 0x22, 0x29);
/// Minimum space kept between the identity and the menus or buttons beside it.
const TITLE_GAP: f32 = 16.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Caption {
    Minimize,
    Maximize,
    Close,
}

/// Windows' order, left to right.
pub const CAPTIONS: [Caption; 3] = [Caption::Minimize, Caption::Maximize, Caption::Close];

/// Which desktop's caption buttons to look like.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CaptionFamily {
    /// Windows 10/11: full-height buttons 46 pt wide; Close turns red on hover.
    #[default]
    Windows,
    /// GNOME (libadwaita): 24 pt round buttons on a faint disc that darkens on hover and press.
    Adwaita,
    /// KDE (Breeze decoration): smaller round buttons, bare at rest, inverted on hover.
    Breeze,
    /// Other Linux desktops: libadwaita's buttons without the disc at rest.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    Generic,
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
impl CaptionFamily {
    /// `windows`, `adwaita`, `breeze` or `generic`, in any case.
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "windows" => Some(Self::Windows),
            "adwaita" => Some(Self::Adwaita),
            "breeze" => Some(Self::Breeze),
            "generic" => Some(Self::Generic),
            _ => None,
        }
    }

    /// The button side, gap on each side of it, and gap between the
    /// outermost button and the window's edge. libadwaita: 24 pt buttons
    /// 13 pt apart, 10 pt from the edge (the gap is split between
    /// neighbours, so the hit areas touch). Breeze: 18 pt buttons half a
    /// button apart, 8 pt from the edge.
    fn metrics(self) -> (f32, f32, f32) {
        match self {
            Self::Windows => (BUTTON_WIDTH, 0.0, 0.0),
            Self::Adwaita | Self::Generic => (24.0, 6.5, 10.0),
            Self::Breeze => (18.0, 4.5, 8.0),
        }
    }
}

/// The caption buttons' look and placement.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CaptionStyle {
    pub family: CaptionFamily,
    /// The buttons at the bar's left end, left to right.
    pub left: Vec<Caption>,
    /// The buttons at the bar's right end, left to right.
    pub right: Vec<Caption>,
}

impl Default for CaptionStyle {
    fn default() -> Self {
        Self {
            family: CaptionFamily::Windows,
            left: vec![],
            right: CAPTIONS.to_vec(),
        }
    }
}

// Only Linux reads a desktop layout (`caption_style`).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
impl CaptionStyle {
    /// `family` with the buttons a GTK decoration layout names
    /// (`gtk-decoration-layout`, GNOME's `button-layout`, e.g.
    /// `appmenu:minimize,maximize,close`): those before the colon go left,
    /// those after it right; without a colon all go left, as in GTK. Other
    /// names (`appmenu`, `icon`, `spacer`, …) are skipped, and a button
    /// named twice is placed once.
    pub fn from_gtk_layout(family: CaptionFamily, layout: &str) -> Self {
        let (left, right) = layout.split_once(':').unwrap_or((layout, ""));
        let mut seen = vec![];
        let mut side = |names: &str| {
            let mut buttons = vec![];
            for name in names.split(',') {
                let caption = match name.trim() {
                    "minimize" => Caption::Minimize,
                    "maximize" => Caption::Maximize,
                    "close" => Caption::Close,
                    _ => continue,
                };
                if !seen.contains(&caption) {
                    seen.push(caption);
                    buttons.push(caption);
                }
            }
            buttons
        };
        let left = side(left);
        let right = side(right);
        Self {
            family,
            left,
            right,
        }
    }
}

/// Reads the desktop's caption style; called each frame, cheap.
pub type CaptionStyleFn = Box<dyn Fn(&egui::Context) -> CaptionStyle>;

/// How the window's top row is drawn.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Chrome {
    /// The row is the window's title bar (no OS decorations).
    pub custom: bool,
    pub style: CaptionStyle,
}

impl Chrome {
    /// The width the caption buttons take at the bar's (left, right) end:
    /// zero for a side without buttons, and for both without the custom
    /// title bar.
    pub fn reserved(&self) -> (f32, f32) {
        if !self.custom {
            return (0.0, 0.0);
        }
        let width = |side: &[Caption]| {
            slots(self.style.family, side, 0.0, true)
                .last()
                .map_or(0.0, |(_, range, _)| range.1)
        };
        (width(&self.style.left), width(&self.style.right))
    }
}

/// The menu bar `height` tall at the top of the window: the menus, the
/// window identity, and with [`Chrome::custom`] the caption buttons. Returns
/// the command chosen this frame; Close returns `quit`, so it runs gg's own
/// Quit.
pub fn show<C: Clone>(
    ui: &mut Ui,
    chrome: &Chrome,
    height: f32,
    table: &[menus::Menu<C>],
    identity: &str,
    quit: C,
) -> Option<C> {
    let mut chosen = None;
    let (caption_left, caption_right) = chrome.reserved();
    let mut panel = egui::Panel::top("menu-bar").exact_size(height);
    if chrome.custom {
        let mut frame = egui::Frame::side_top_panel(ui.style());
        if caption_left > 0.0 {
            frame.inner_margin.left = 0;
        }
        if caption_right > 0.0 {
            frame.inner_margin.right = 0;
        }
        panel = panel.frame(frame);
    }
    let bar = panel.show(ui, |ui| {
        let full = ui.max_rect();
        // egui's in-window menu bar: frameless titles, hover moves between open menus.
        let menus_end = egui::MenuBar::new()
            .ui(ui, |ui| {
                if caption_left > 0.0 {
                    ui.add_space(caption_left + 4.0);
                }
                // No logo beside the menus (owner): the titles start the bar.
                chosen = menus::show(ui, table);
                let menus_end = ui.cursor().left();
                if !chrome.custom {
                    let width = ui.available_width();
                    ui.allocate_ui_with_layout(
                        egui::vec2(width, ui.available_height()),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.add_space(6.0);
                            ui.add(
                                egui::Label::new(egui::RichText::new(identity).weak()).truncate(),
                            );
                        },
                    );
                }
                menus_end
            })
            .inner;
        if chrome.custom {
            let right = if caption_right > 0.0 {
                full.right() - caption_right - 4.0
            } else {
                full.right()
            };
            free_gap(ui, full, menus_end, right, identity);
        }
    });
    if chrome.custom
        && let Some(Caption::Close) = caption_buttons(ui, bar.response.rect, &chrome.style)
    {
        chosen = Some(quit);
    }
    chosen
}

/// The free gap from `left` to `right` across the bar `full`: it drags the
/// window and a double-click maximizes or restores it. The identity is
/// centred on the window when it fits in the gap, slid into the gap when it
/// does not, and shortened from its end when even that is too narrow. It is
/// painted, not a label, so a press on it drags the window too.
fn free_gap(ui: &mut Ui, full: Rect, left: f32, right: f32, identity: &str) {
    let gap = Rect::from_x_y_ranges(left.max(full.left())..=right.max(left), full.y_range());
    if gap.width() <= 0.0 {
        return;
    }
    let drag = ui.interact(gap, Id::unique("titlebar-gap"), Sense::click_and_drag());
    if drag.drag_started_by(PointerButton::Primary) {
        start_os_move(ui.ctx());
    }
    if drag.double_clicked_by(PointerButton::Primary) {
        toggle_maximize(ui.ctx());
    }
    let color = ui.visuals().weak_text_color();
    let font = egui::TextStyle::Body.resolve(ui.style());
    let job = |wrap: Option<f32>| {
        let mut job =
            egui::text::LayoutJob::simple_singleline(identity.to_owned(), font.clone(), color);
        if let Some(width) = wrap {
            job.wrap = egui::text::TextWrapping::truncate_at_width(width);
        }
        job
    };
    let galley = ui.painter().layout_job(job(None));
    let y = full.center().y - galley.size().y / 2.0;
    if let Some(x) = title_x(
        ui.ctx().content_rect().center().x,
        left,
        right,
        galley.size().x,
    ) {
        ui.painter().galley(pos2(x, y), galley, color);
    } else if right - left > 2.0 * TITLE_GAP + 40.0 {
        let galley = ui
            .painter()
            .layout_job(job(Some(right - left - 2.0 * TITLE_GAP)));
        ui.painter()
            .galley(pos2(left + TITLE_GAP, y), galley, color);
    }
}

/// Left edge for a title `width` wide: centred on `center` when it fits in
/// the free gap from `left` to `right`, else slid into that gap; `None` when
/// the gap is too small for the whole title.
pub fn title_x(center: f32, left: f32, right: f32, width: f32) -> Option<f32> {
    let (lo, hi) = (left + TITLE_GAP, right - TITLE_GAP - width);
    if !(lo.is_finite() && hi.is_finite() && width.is_finite()) || hi < lo {
        return None;
    }
    if !center.is_finite() {
        return Some(lo);
    }
    Some((center - width / 2.0).clamp(lo, hi))
}

/// Each button's horizontal extent (left, right) and face centre on a side
/// starting at `from`, in order. `outer_left` puts the edge gap on the first
/// button's left (the left end) instead of the last button's right.
fn slots(
    family: CaptionFamily,
    side: &[Caption],
    from: f32,
    outer_left: bool,
) -> Vec<(Caption, (f32, f32), f32)> {
    let (size, gap, end) = family.metrics();
    let mut x = from;
    let count = side.len();
    side.iter()
        .enumerate()
        .map(|(i, &caption)| {
            let before = if outer_left && i == 0 { end } else { gap };
            let after = if !outer_left && i + 1 == count {
                end
            } else {
                gap
            };
            let slot = (x, x + before + size + after);
            let centre = x + before + size / 2.0;
            x = slot.1;
            (caption, slot, centre)
        })
        .collect()
}

/// Every caption button's hit rect (full bar height, flush with the
/// window's edge) and face centre, for a bar between `left` and `right`.
fn layout(style: &CaptionStyle, bar: Rect, left: f32, right: f32) -> Vec<(Caption, Rect, Pos2)> {
    let rect = |(caption, (l, r), centre): (Caption, (f32, f32), f32)| {
        (
            caption,
            Rect::from_min_max(pos2(l, bar.top()), pos2(r, bar.bottom() - 1.0)),
            pos2(centre, bar.center().y),
        )
    };
    let lefts = slots(style.family, &style.left, left, true);
    let width = slots(style.family, &style.right, 0.0, false)
        .last()
        .map_or(0.0, |(_, range, _)| range.1);
    let rights = slots(style.family, &style.right, right - width, false);
    lefts.into_iter().chain(rights).map(rect).collect()
}

/// Caption button ids (stable, so tests can find them).
pub fn caption_id(caption: Caption) -> Id {
    Id::unique(("titlebar-caption", caption as u8))
}

fn maximized(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().maximized.unwrap_or(false))
}

fn toggle_maximize(ctx: &egui::Context) {
    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized(ctx)));
}

/// The caption buttons at the ends of `bar` (the title bar's outer rect),
/// flush with the window's edges so a maximized window's corner hits the
/// outermost one. Minimize and maximize act here; Close is returned for the
/// caller to run gg's Quit. On Linux the buttons drag the window too, as
/// GTK's and KDE's do.
fn caption_buttons(ui: &mut Ui, bar: Rect, style: &CaptionStyle) -> Option<Caption> {
    let ctx = ui.ctx().clone();
    let max = maximized(&ctx);
    let content = ctx.content_rect();
    let windows = style.family == CaptionFamily::Windows;
    let visuals = ui.visuals().clone();
    let ink = visuals.text_color();
    let mut clicked = None;
    for (caption, rect, centre) in layout(
        style,
        bar,
        bar.left().max(content.left()),
        bar.right().min(content.right()),
    ) {
        let sense = if windows {
            Sense::click()
        } else {
            Sense::click_and_drag()
        };
        let response = ui.interact(rect, caption_id(caption), sense);
        if response.drag_started_by(PointerButton::Primary) {
            start_os_move(&ctx);
        }
        let down = response.is_pointer_button_down_on();
        let hovered = response.hovered();
        if windows {
            let close = caption == Caption::Close;
            let fill = match (down, hovered) {
                (true, _) if close => WINDOWS_CLOSE.gamma_multiply(0.8),
                (_, true) if close => WINDOWS_CLOSE,
                (true, _) => visuals.widgets.active.weak_bg_fill,
                (_, true) => visuals.widgets.hovered.weak_bg_fill,
                _ => Color32::TRANSPARENT,
            };
            ui.painter().rect_filled(rect, 0.0, fill);
            let glyph = if close && hovered {
                Color32::WHITE
            } else {
                ink
            };
            paint_windows_glyph(ui, caption, max, rect.center(), Stroke::new(1.0, glyph));
        } else {
            let round = Round {
                family: style.family,
                caption,
                maximized: max,
                hovered,
                pressed: down && hovered,
                ink,
                chrome: visuals.panel_fill,
            };
            paint_round(ui, centre, round);
        }
        let tip = match caption {
            Caption::Minimize => "Minimize",
            Caption::Maximize if max => "Restore",
            Caption::Maximize => "Maximize",
            Caption::Close => "Close",
        };
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tip));
        if response.on_hover_text(tip).clicked() {
            clicked = Some(caption);
        }
    }
    match clicked {
        Some(Caption::Minimize) => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
        Some(Caption::Maximize) => toggle_maximize(&ctx),
        Some(Caption::Close) | None => {}
    }
    clicked
}

/// The 10 pt line glyphs of the Windows caption buttons.
fn paint_windows_glyph(ui: &Ui, caption: Caption, maximized: bool, at: Pos2, stroke: Stroke) {
    let painter = ui.painter();
    let g = Rect::from_center_size(at, vec2(10.0, 10.0));
    match caption {
        Caption::Minimize => {
            painter.line_segment([g.left_center(), g.right_center()], stroke);
        }
        // Restore: a front square with the corner of the one behind it.
        Caption::Maximize if maximized => {
            let front = Rect::from_min_max(g.min + vec2(0.0, 2.0), g.max - vec2(2.0, 0.0));
            painter.rect_stroke(front, 0.0, stroke, egui::StrokeKind::Middle);
            painter.line(
                vec![
                    g.min + vec2(2.0, 2.0),
                    g.min + vec2(2.0, 0.0),
                    g.right_top(),
                    g.max - vec2(0.0, 2.0),
                    g.max - vec2(2.0, 2.0),
                ],
                stroke,
            );
        }
        Caption::Maximize => {
            painter.rect_stroke(g, 0.0, stroke, egui::StrokeKind::Middle);
        }
        Caption::Close => {
            painter.line_segment([g.left_top(), g.right_bottom()], stroke);
            painter.line_segment([g.right_top(), g.left_bottom()], stroke);
        }
    }
}

/// What a round Linux caption button shows.
struct Round {
    family: CaptionFamily,
    caption: Caption,
    maximized: bool,
    hovered: bool,
    pressed: bool,
    /// The text colour, and the bar's fill (Breeze inverts on hover).
    ink: Color32,
    chrome: Color32,
}

/// A round Linux caption button centred at `centre`: its disc, then its
/// glyph on a 16 pt grid. The disc is the ink at a fraction of its opacity
/// (libadwaita's `alpha(currentColor)`), so it suits light and dark themes;
/// Breeze fills it with the ink on hover and draws the glyph in the bar's
/// colour.
fn paint_round(ui: &Ui, centre: Pos2, button: Round) {
    let Round {
        family,
        caption,
        maximized,
        hovered,
        pressed,
        ink,
        chrome,
    } = button;
    let (size, _, _) = family.metrics();
    // Whole pixels, so the glyphs' edges stay sharp.
    let ppp = ui.ctx().pixels_per_point();
    let centre = pos2(
        (centre.x * ppp).round() / ppp,
        (centre.y * ppp).round() / ppp,
    );
    let close = caption == Caption::Close;
    let (fill, ink) = if family == CaptionFamily::Breeze {
        match (hovered, pressed) {
            (_, true) if close => (BREEZE_CLOSE_PRESSED, chrome),
            (true, _) if close => (BREEZE_CLOSE_HOVER, chrome),
            (_, true) => (ink.gamma_multiply(0.5), chrome),
            (true, _) => (ink, chrome),
            _ => (Color32::TRANSPARENT, ink),
        }
    } else {
        let rest = if family == CaptionFamily::Adwaita {
            0.10
        } else {
            0.0
        };
        let target = if pressed {
            0.30
        } else if hovered {
            0.15
        } else {
            rest
        };
        // libadwaita eases the disc over 0.2 s.
        let alpha = ui
            .ctx()
            .animate_value_with_time(caption_id(caption).with("disc"), target, 0.2);
        (ink.gamma_multiply(alpha), ink)
    };
    let painter = ui.painter();
    if fill.a() > 0 {
        painter.circle_filled(centre, size / 2.0, fill);
    }
    // The glyph grid: 16 pt, or 2 pt less than a smaller button.
    let k = (size - 2.0).min(16.0) / 16.0;
    let at = |x: f32, y: f32| centre + vec2(x - 8.0, y - 8.0) * k;
    if family == CaptionFamily::Breeze {
        // Breeze: 1 pt chevrons (down to minimize, up to maximize), a diamond to restore, an X.
        let stroke = Stroke::new(1.0, ink);
        match caption {
            Caption::Minimize => {
                painter.line(vec![at(2.5, 5.5), at(8.0, 11.0), at(13.5, 5.5)], stroke);
            }
            Caption::Maximize if maximized => {
                painter.add(egui::Shape::closed_line(
                    vec![at(8.0, 3.0), at(13.0, 8.0), at(8.0, 13.0), at(3.0, 8.0)],
                    stroke,
                ));
            }
            Caption::Maximize => {
                painter.line(vec![at(2.5, 10.5), at(8.0, 5.0), at(13.5, 10.5)], stroke);
            }
            Caption::Close => {
                let stroke = Stroke::new(1.2, ink);
                painter.line_segment([at(3.6, 3.6), at(12.4, 12.4)], stroke);
                painter.line_segment([at(12.4, 3.6), at(3.6, 12.4)], stroke);
            }
        }
    } else {
        // libadwaita: a 2 pt bar low down, a 2 pt square outline (smaller to restore), an X.
        let frame = |a: f32, b: f32| Rect::from_min_max(at(a, a), at(b, b));
        match caption {
            Caption::Minimize => {
                painter.rect_filled(Rect::from_min_max(at(4.0, 10.0), at(12.0, 12.0)), 0.0, ink);
            }
            Caption::Maximize => {
                let square = if maximized {
                    frame(5.0, 11.0)
                } else {
                    frame(4.0, 12.0)
                };
                painter.rect_stroke(
                    square,
                    0.0,
                    Stroke::new(2.0 * k, ink),
                    egui::StrokeKind::Inside,
                );
            }
            Caption::Close => {
                let stroke = Stroke::new(1.5 * k, ink);
                painter.line_segment([at(4.5, 4.5), at(11.5, 11.5)], stroke);
                painter.line_segment([at(11.5, 4.5), at(4.5, 11.5)], stroke);
            }
        }
    }
}

/// The edge and corner zones of a window with content rect `w`: (zone,
/// direction, cursor). Corners come last so they win where they overlap the
/// edges. Edges use the one-sided cursors, which every cursor theme has.
fn zones(w: Rect) -> [(Rect, ResizeDirection, CursorIcon); 8] {
    use ResizeDirection::{East, North, NorthEast, NorthWest, South, SouthEast, SouthWest, West};
    let (e, c) = (EDGE, CORNER);
    [
        (
            Rect::from_min_max(w.left_top(), pos2(w.right(), w.top() + e)),
            North,
            CursorIcon::ResizeNorth,
        ),
        (
            Rect::from_min_max(pos2(w.left(), w.bottom() - e), w.right_bottom()),
            South,
            CursorIcon::ResizeSouth,
        ),
        (
            Rect::from_min_max(w.left_top(), pos2(w.left() + e, w.bottom())),
            West,
            CursorIcon::ResizeWest,
        ),
        (
            Rect::from_min_max(pos2(w.right() - e, w.top()), w.right_bottom()),
            East,
            CursorIcon::ResizeEast,
        ),
        (
            Rect::from_min_size(w.left_top(), vec2(c, c)),
            NorthWest,
            CursorIcon::ResizeNwSe,
        ),
        (
            Rect::from_min_size(w.right_top() - vec2(c, 0.0), vec2(c, c)),
            NorthEast,
            CursorIcon::ResizeNeSw,
        ),
        (
            Rect::from_min_size(w.left_bottom() - vec2(0.0, c), vec2(c, c)),
            SouthWest,
            CursorIcon::ResizeNeSw,
        ),
        (
            Rect::from_min_size(w.right_bottom() - vec2(c, c), vec2(c, c)),
            SouthEast,
            CursorIcon::ResizeNwSe,
        ),
    ]
}

fn handed_off_id() -> Id {
    Id::unique("titlebar-resize-handed-off")
}

/// An OS resize or move started at `pass`; `released` is the pass whose
/// input ended the press ([`release_after_os_resize`]).
#[derive(Clone, Copy, Debug)]
struct HandOff {
    pass: u64,
    released: Option<u64>,
}

/// The OS took the press at this pass: [`release_after_os_resize`] ends it on the next input.
fn hand_off(ctx: &egui::Context) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        d.insert_temp(
            handed_off_id(),
            HandOff {
                pass,
                released: None,
            },
        );
    });
}

/// Move the window from this press. The window manager keeps the release of
/// an OS move as of a resize, so the same hand-off ends the press.
fn start_os_move(ctx: &egui::Context) {
    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    hand_off(ctx);
}

/// The compositor takes the button release of an OS resize or move (KDE
/// Wayland, KWin on X11), so egui kept the edge pressed and every other
/// resize or move failed. End the press on the input after the hand-off,
/// only while egui still has it down and this input brings no release of its
/// own: where the OS does send it (Windows), nothing is added. Call from
/// `raw_input_hook`.
pub fn release_after_os_resize(ctx: &egui::Context, raw: &mut egui::RawInput) {
    let Some(handed) = ctx.data(|d| d.get_temp::<HandOff>(handed_off_id())) else {
        return;
    };
    if handed.released.is_some() || handed.pass >= ctx.cumulative_pass_nr() {
        return;
    }
    let os_released = raw.events.iter().any(|e| {
        matches!(
            e,
            egui::Event::PointerButton {
                button: PointerButton::Primary,
                pressed: false,
                ..
            }
        )
    });
    if !os_released
        && ctx.input(|i| i.pointer.primary_down())
        && let Some(pos) = ctx.input(|i| i.pointer.latest_pos())
    {
        raw.events.push(egui::Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: ctx.input(|i| i.modifiers),
        });
    }
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        d.insert_temp(
            handed_off_id(),
            HandOff {
                released: Some(pass),
                ..handed
            },
        );
    });
}

/// egui-winit only re-sends a cursor that changed, so after an OS resize the
/// window kept the compositor's arrow. On the first pointer motion after the
/// hand-off, show the default cursor for one frame so the resize cursor is
/// sent again on the next.
fn reshow_cursor_after_os_resize(ctx: &egui::Context) {
    let Some(HandOff {
        released: Some(pass),
        ..
    }) = ctx.data(|d| d.get_temp::<HandOff>(handed_off_id()))
    else {
        return;
    };
    if pass < ctx.cumulative_pass_nr()
        && ctx.input(|i| {
            i.events
                .iter()
                .any(|e| matches!(e, egui::Event::PointerMoved(_)))
        })
    {
        ctx.data_mut(|d| d.remove::<HandOff>(handed_off_id()));
        ctx.set_cursor_icon(CursorIcon::Default);
        ctx.request_repaint();
    }
}

/// Invisible resize zones along the window's edges, above everything else;
/// a press in one starts an OS resize in that direction. None while the
/// window is maximized or full screen.
pub fn resize_zones(ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    if maximized(&ctx) || ctx.input(|i| i.viewport().fullscreen.unwrap_or(false)) {
        reshow_cursor_after_os_resize(&ctx);
        return;
    }
    let w = ctx.content_rect();
    let layer = LayerId::new(Order::Foreground, Id::unique("titlebar-resize"));
    let zones_ui = ui.new_child(egui::UiBuilder::new().layer_id(layer).max_rect(w));
    for (i, (rect, direction, cursor)) in zones(w).into_iter().enumerate() {
        let response = zones_ui
            .interact(rect, Id::unique(("titlebar-resize", i)), Sense::drag())
            .on_hover_cursor(cursor);
        if response.drag_started_by(PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
            hand_off(&ctx);
        }
    }
    // After the zones, so its one-frame cursor wins over theirs.
    reshow_cursor_after_os_resize(&ctx);
}

#[cfg(test)]
mod tests;
