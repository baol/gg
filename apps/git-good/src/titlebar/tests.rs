//! Adapted from photocraft crates/ui-egui/src/panels/titlebar/tests.rs.

use super::*;
use eframe::egui::{Event, RawInput, ViewportId};

const QUIT: u8 = 99;
const IDENTITY: &str = "Git Good — ~/dev/gg";

/// A headless window `width` pt wide showing the menu bar and the resize zones.
struct Win {
    ctx: egui::Context,
    chrome: Chrome,
    width: f32,
    time: f64,
    maximized: bool,
    chosen: Vec<u8>,
}

fn table() -> Vec<menus::Menu<u8>> {
    ["Repository", "Branch", "Commit", "Changes", "View", "Help"]
        .into_iter()
        .enumerate()
        .map(|(i, title)| menus::Menu {
            title,
            entries: vec![menus::item("Item", "", i as u8).entry()],
        })
        .collect()
}

impl Win {
    fn new(width: f32) -> Self {
        Self {
            ctx: egui::Context::default(),
            chrome: Chrome {
                custom: true,
                style: CaptionStyle::default(),
                merged: false,
            },
            width,
            time: 0.0,
            maximized: false,
            chosen: vec![],
        }
    }

    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.time += 0.05;
        let mut input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(self.width, 600.0))),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        if let Some(viewport) = input.viewports.get_mut(&ViewportId::ROOT) {
            viewport.maximized = Some(self.maximized);
        }
        // What `Browser::raw_input_hook` does.
        release_after_os_resize(&self.ctx, &mut input);
        let chrome = self.chrome.clone();
        let menus = table();
        let mut chosen = None;
        let mut out = self.ctx.run_ui(input, |ui| {
            chosen = show(ui, &chrome, 26.0, &menus, IDENTITY, QUIT);
            if chrome.custom {
                resize_zones(ui);
            }
        });
        self.chosen.extend(chosen);
        out.textures_delta.clear();
        out
    }

    /// Frames for a pointer gesture: move to `path[0]`, press, move along
    /// `path`, release. The viewport commands the frames sent.
    fn gesture_by(&mut self, path: &[Pos2], btn: PointerButton) -> Vec<ViewportCommand> {
        let mut events = vec![
            vec![Event::PointerMoved(path[0])],
            vec![button(path[0], true, btn)],
        ];
        events.extend(path[1..].iter().map(|p| vec![Event::PointerMoved(*p)]));
        events.push(vec![button(*path.last().unwrap_or(&path[0]), false, btn)]);
        events
            .into_iter()
            .flat_map(|e| commands(self.frame(e)))
            .collect()
    }

    fn gesture(&mut self, path: &[Pos2]) -> Vec<ViewportCommand> {
        self.gesture_by(path, PointerButton::Primary)
    }

    fn click(&mut self, p: Pos2) -> Vec<ViewportCommand> {
        self.gesture(&[p])
    }

    fn caption(&self, caption: Caption) -> Rect {
        self.ctx
            .read_response(caption_id(caption))
            .map_or(Rect::NOTHING, |r| r.rect)
    }

    /// The bar's clickable widgets other than its drag gap and the caption buttons.
    fn bar_widgets(&self) -> Vec<Rect> {
        let skip = CAPTIONS.map(caption_id);
        self.ctx.viewport(|v| {
            v.prev_pass
                .widgets
                .layers()
                .filter(|(layer, _)| layer.order != Order::Foreground)
                .flat_map(|(_, widgets)| widgets.iter())
                .filter(|w| {
                    w.sense.senses_click()
                        && !w.sense.senses_drag()
                        && w.rect.top() < 26.0
                        && !skip.contains(&w.id)
                })
                .map(|w| w.rect)
                .collect()
        })
    }
}

fn commands(mut out: egui::FullOutput) -> Vec<ViewportCommand> {
    out.viewport_output
        .remove(&ViewportId::ROOT)
        .map(|v| v.commands)
        .unwrap_or_default()
}

fn texts(out: &egui::FullOutput) -> Vec<(String, Rect)> {
    fn walk(shape: &egui::Shape, found: &mut Vec<(String, Rect)>) {
        match shape {
            egui::Shape::Text(text) => found.push((
                text.galley.text().to_string(),
                Rect::from_min_size(text.pos, text.galley.size()),
            )),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, found)),
            _ => {}
        }
    }
    let mut found = vec![];
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut found));
    found
}

fn button(pos: Pos2, pressed: bool, button: PointerButton) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Default::default(),
    }
}

fn starts(cmds: &[ViewportCommand]) -> usize {
    cmds.iter()
        .filter(|c| matches!(c, ViewportCommand::StartDrag))
        .count()
}

#[test]
fn caption_buttons_take_the_right_end_and_nothing_overlaps_them() {
    for width in [720.0, 1120.0, 1440.0] {
        let mut w = Win::new(width);
        w.frame(vec![]);
        w.frame(vec![]);
        let [min, max, close] = CAPTIONS.map(|c| w.caption(c));
        assert_eq!(
            (close.right(), close.top()),
            (width, 0.0),
            "Close sits in the window's corner at {width}"
        );
        assert!([min, max, close].iter().all(|r| r.width() == BUTTON_WIDTH));
        assert_eq!(close.height(), 25.0, "full bar height above its separator");
        assert_eq!((min.right(), max.right()), (max.left(), close.left()));
        let widgets = w.bar_widgets();
        assert!(widgets.len() >= 6, "the six menus: {widgets:?}");
        for rect in &widgets {
            assert!(
                rect.right() <= min.left(),
                "{rect:?} runs into the caption buttons at {width}"
            );
        }
    }
}

#[test]
fn the_identity_is_centred_in_the_free_gap() {
    let mut w = Win::new(1440.0);
    w.frame(vec![]);
    let out = w.frame(vec![]);
    let found = texts(&out);
    let help = found.iter().find(|(t, _)| t == "Help").map(|(_, r)| *r);
    let identity = found.iter().find(|(t, _)| t == IDENTITY).map(|(_, r)| *r);
    let (Some(help), Some(identity)) = (help, identity) else {
        unreachable!("the Help menu and the identity are drawn: {found:?}");
    };
    assert!((identity.center().x - 720.0).abs() < 1.0, "{identity:?}");
    assert!(identity.left() > help.right() + TITLE_GAP);
    assert!(identity.top() >= 0.0 && identity.bottom() <= 26.0);
    // Narrow: slid toward the menus, still clear of the caption buttons.
    let mut w = Win::new(720.0);
    w.frame(vec![]);
    let out = w.frame(vec![]);
    let min = w.caption(Caption::Minimize);
    for (text, rect) in texts(&out) {
        if text.starts_with("Git Good") {
            assert!(rect.right() <= min.left(), "{rect:?} {min:?}");
        }
    }
}

#[test]
fn close_runs_quit_and_the_others_follow_the_window_state() {
    let mut w = Win::new(1120.0);
    w.frame(vec![]);
    let close = w.caption(Caption::Close).center();
    w.click(close);
    assert_eq!(w.chosen, [QUIT], "Close runs Git Good's own Quit");
    let (min, max) = (
        w.caption(Caption::Minimize).center(),
        w.caption(Caption::Maximize).center(),
    );
    assert!(w.click(min).contains(&ViewportCommand::Minimized(true)));
    assert!(w.click(max).contains(&ViewportCommand::Maximized(true)));
    w.maximized = true;
    assert!(
        w.click(max).contains(&ViewportCommand::Maximized(false)),
        "Restore"
    );
}

#[test]
fn empty_bar_space_drags_and_double_click_maximizes() {
    let mut w = Win::new(1440.0);
    w.frame(vec![]);
    let empty = pos2(1100.0, 13.0);
    assert!(w.bar_widgets().iter().all(|r| !r.contains(empty)));
    let cmds = w.gesture(&[empty, empty + vec2(12.0, 4.0), empty + vec2(30.0, 8.0)]);
    assert_eq!(starts(&cmds), 1, "{cmds:?}");
    let mut cmds = w.click(empty);
    cmds.extend(w.click(empty));
    assert!(cmds.contains(&ViewportCommand::Maximized(true)), "{cmds:?}");
    w.maximized = true;
    w.time += 1.0;
    let mut cmds = w.click(empty);
    cmds.extend(w.click(empty));
    assert!(
        cmds.contains(&ViewportCommand::Maximized(false)),
        "{cmds:?}"
    );
    // The identity drags too: it is painted, not a selectable label.
    let cmds = w.gesture(&[pos2(720.0, 13.0), pos2(740.0, 20.0)]);
    assert_eq!(starts(&cmds), 1, "{cmds:?}");
    // Menu titles keep their presses.
    let out = w.frame(vec![]);
    let branch = texts(&out)
        .into_iter()
        .find(|(t, _)| t == "Branch")
        .map(|(_, r)| r.center());
    let Some(branch) = branch else {
        unreachable!("the Branch menu title is drawn");
    };
    let cmds = w.gesture(&[branch, branch + vec2(12.0, 4.0)]);
    assert_eq!(starts(&cmds), 0, "{cmds:?}");
    // Only the primary button moves the window.
    let cmds = w.gesture_by(&[empty, empty + vec2(30.0, 8.0)], PointerButton::Middle);
    assert_eq!(starts(&cmds), 0, "{cmds:?}");
}

#[test]
fn the_system_title_bar_keeps_the_identity_at_the_right_and_no_buttons() {
    let mut w = Win::new(1120.0);
    w.chrome.custom = false;
    w.frame(vec![]);
    let out = w.frame(vec![]);
    assert_eq!(w.chrome.reserved(), (0.0, 0.0));
    assert!(CAPTIONS.iter().all(|c| w.caption(*c) == Rect::NOTHING));
    let identity = texts(&out)
        .into_iter()
        .find(|(t, _)| t == IDENTITY)
        .map(|(_, r)| r);
    let Some(identity) = identity else {
        unreachable!("the identity is drawn");
    };
    assert!(identity.right() > 1100.0, "right-aligned: {identity:?}");
    let cmds = w.gesture(&[pos2(800.0, 13.0), pos2(830.0, 20.0)]);
    assert_eq!(
        starts(&cmds),
        0,
        "the OS title bar moves the window: {cmds:?}"
    );
    let cmds = w.gesture(&[pos2(3.0, 300.0), pos2(9.0, 300.0)]);
    assert!(
        !cmds
            .iter()
            .any(|c| matches!(c, ViewportCommand::BeginResize(_))),
        "{cmds:?}"
    );
}

/// KDE Wayland keeps the button release of an OS resize: end the press, and
/// re-send the resize cursor the compositor replaced.
#[test]
fn after_an_os_resize_the_edge_hovers_and_shows_its_cursor_again() {
    let mut w = Win::new(1000.0);
    w.frame(vec![]);
    let p = pos2(3.0, 300.0);
    let mut cmds = vec![];
    for events in [
        vec![Event::PointerMoved(p)],
        vec![button(p, true, PointerButton::Primary)],
        vec![Event::PointerMoved(p + vec2(6.0, 0.0))],
    ] {
        cmds.extend(commands(w.frame(events)));
    }
    assert!(
        cmds.contains(&ViewportCommand::BeginResize(ResizeDirection::West)),
        "{cmds:?}"
    );
    w.frame(vec![]);
    assert!(
        !w.ctx.input(|i| i.pointer.primary_down()),
        "the press ends with the hand-off"
    );
    let out = w.frame(vec![Event::PointerMoved(p)]);
    assert_eq!(out.platform_output.cursor_icon, CursorIcon::Default);
    let out = w.frame(vec![Event::PointerMoved(p + vec2(1.0, 0.0))]);
    assert_eq!(out.platform_output.cursor_icon, CursorIcon::ResizeWest);
    assert!(
        w.gesture(&[p, p + vec2(6.0, 0.0)])
            .contains(&ViewportCommand::BeginResize(ResizeDirection::West))
    );
}

/// Where the OS does send the release (Windows), no second one is added.
#[test]
fn a_release_the_os_sends_is_not_doubled() {
    let mut w = Win::new(1000.0);
    w.frame(vec![]);
    let p = pos2(3.0, 300.0);
    w.frame(vec![Event::PointerMoved(p)]);
    w.frame(vec![button(p, true, PointerButton::Primary)]);
    w.frame(vec![Event::PointerMoved(p + vec2(6.0, 0.0))]);
    let mut raw = RawInput {
        events: vec![button(p + vec2(6.0, 0.0), false, PointerButton::Primary)],
        ..Default::default()
    };
    release_after_os_resize(&w.ctx, &mut raw);
    assert_eq!(raw.events.len(), 1, "{:?}", raw.events);
}

#[test]
fn after_an_os_move_the_bar_and_the_edges_take_the_next_press() {
    let mut w = Win::new(1440.0);
    w.frame(vec![]);
    let empty = pos2(1100.0, 13.0);
    let drag = [empty, empty + vec2(12.0, 4.0), empty + vec2(30.0, 8.0)];
    // The window manager takes over: no release ever arrives.
    let mut events = vec![
        vec![Event::PointerMoved(drag[0])],
        vec![button(drag[0], true, PointerButton::Primary)],
    ];
    events.extend(drag[1..].iter().map(|p| vec![Event::PointerMoved(*p)]));
    events.push(vec![]);
    let cmds: Vec<_> = events
        .into_iter()
        .flat_map(|e| commands(w.frame(e)))
        .collect();
    assert_eq!(starts(&cmds), 1);
    assert!(!w.ctx.input(|i| i.pointer.primary_down()));
    assert_eq!(starts(&w.gesture(&drag)), 1, "the next move");
    let edge = pos2(3.0, 300.0);
    assert!(
        w.gesture(&[edge, edge + vec2(6.0, 0.0)])
            .contains(&ViewportCommand::BeginResize(ResizeDirection::West)),
        "a resize after a move"
    );
}

#[test]
fn edge_zones_resize_unless_maximized() {
    use ResizeDirection::{East, North, NorthEast, NorthWest, South, SouthEast, SouthWest, West};
    let mut w = Win::new(1000.0);
    w.frame(vec![]);
    for (p, direction) in [
        (pos2(1.0, 300.0), West),
        (pos2(998.0, 300.0), East),
        (pos2(500.0, 1.0), North),
        (pos2(500.0, 598.0), South),
        (pos2(2.0, 2.0), NorthWest),
        (pos2(997.0, 3.0), NorthEast),
        (pos2(3.0, 597.0), SouthWest),
        (pos2(998.0, 598.0), SouthEast),
    ] {
        let cmds = w.gesture(&[p, p + vec2(6.0, 6.0)]);
        assert_eq!(
            cmds,
            vec![ViewportCommand::BeginResize(direction)],
            "at {p:?}"
        );
    }
    w.frame(vec![Event::PointerMoved(pos2(500.0, 300.0))]);
    for (p, cursor) in [
        (pos2(7.0, 300.0), CursorIcon::ResizeWest),
        (pos2(993.0, 300.0), CursorIcon::ResizeEast),
        (pos2(500.0, 7.0), CursorIcon::ResizeNorth),
        (pos2(500.0, 593.0), CursorIcon::ResizeSouth),
        (pos2(2.0, 2.0), CursorIcon::ResizeNwSe),
        (pos2(997.0, 3.0), CursorIcon::ResizeNeSw),
    ] {
        let out = w.frame(vec![Event::PointerMoved(p)]);
        assert_eq!(out.platform_output.cursor_icon, cursor, "at {p:?}");
    }
    let cmds = w.gesture(&[pos2(20.0, 300.0), pos2(26.0, 306.0)]);
    assert!(
        !cmds
            .iter()
            .any(|c| matches!(c, ViewportCommand::BeginResize(_))),
        "{cmds:?}"
    );
    // Maximized: no zones, and the window's top-right corner is Close.
    w.maximized = true;
    let cmds = w.gesture(&[pos2(1.0, 300.0), pos2(8.0, 300.0)]);
    assert!(
        !cmds
            .iter()
            .any(|c| matches!(c, ViewportCommand::BeginResize(_))),
        "{cmds:?}"
    );
    w.click(pos2(999.5, 0.5));
    assert_eq!(w.chosen, [QUIT]);
}

#[test]
fn gtk_layouts_pick_the_buttons_and_their_sides() {
    use Caption::{Close, Maximize, Minimize};
    let s =
        CaptionStyle::from_gtk_layout(CaptionFamily::Adwaita, "appmenu:minimize,maximize,close");
    assert_eq!((s.left, s.right), (vec![], vec![Minimize, Maximize, Close]));
    // Unknown names are skipped, spaces trimmed, a repeat placed once, a second colon is a name.
    let s = CaptionStyle::from_gtk_layout(
        CaptionFamily::Adwaita,
        "close,close , minimize:icon,maximize:close",
    );
    assert_eq!((s.left, s.right), (vec![Close, Minimize], vec![]));
    let s = CaptionStyle::from_gtk_layout(CaptionFamily::Breeze, "minimize,close");
    assert_eq!(
        (s.left, s.right),
        (vec![Minimize, Close], vec![]),
        "no colon: GTK puts them all at the start"
    );
    let s = CaptionStyle::from_gtk_layout(CaptionFamily::Breeze, "");
    assert!(s.left.is_empty() && s.right.is_empty());
    assert_eq!(
        CaptionFamily::parse(" Breeze "),
        Some(CaptionFamily::Breeze)
    );
    assert_eq!(CaptionFamily::parse("aqua"), None);
}

#[test]
fn linux_buttons_sit_where_the_layout_says_and_drag_the_window() {
    use Caption::{Close, Maximize, Minimize};
    for (family, size, gap, end) in [
        (CaptionFamily::Adwaita, 24.0, 6.5, 10.0),
        (CaptionFamily::Breeze, 18.0, 4.5, 8.0),
    ] {
        let width = 1120.0;
        let mut w = Win::new(width);
        w.chrome.style = CaptionStyle {
            family,
            left: vec![Close],
            right: vec![Minimize, Maximize],
        };
        w.frame(vec![]);
        w.frame(vec![]);
        let [min, max, close] = CAPTIONS.map(|c| w.caption(c));
        assert_eq!(
            (close.left(), close.width()),
            (0.0, end + size + gap),
            "{family:?}"
        );
        assert_eq!(
            (max.right(), max.width()),
            (width, gap + size + end),
            "{family:?}"
        );
        assert_eq!(
            (min.right(), min.width()),
            (max.left(), 2.0 * gap + size),
            "{family:?}"
        );
        for rect in w.bar_widgets() {
            assert!(
                rect.left() >= close.right() && rect.right() <= min.left(),
                "{rect:?} runs into the caption buttons ({family:?})"
            );
        }
        assert!(
            w.click(max.center())
                .contains(&ViewportCommand::Maximized(true))
        );
        w.click(close.center());
        assert_eq!(w.chosen, [QUIT]);
        // A drag that starts on a button moves the window instead of pressing it.
        let at = min.center();
        let cmds = w.gesture(&[at, at + vec2(12.0, 4.0), at + vec2(30.0, 8.0)]);
        assert_eq!(starts(&cmds), 1, "{cmds:?}");
        assert!(
            !cmds.contains(&ViewportCommand::Minimized(true)),
            "{cmds:?}"
        );
    }
}

/// A side without buttons reserves nothing; without the custom title bar, neither does any.
#[test]
fn only_sides_with_buttons_take_room() {
    let mut chrome = Chrome {
        custom: true,
        style: CaptionStyle::from_gtk_layout(CaptionFamily::Adwaita, "appmenu:close"),
        merged: false,
    };
    assert_eq!(chrome.reserved(), (0.0, 6.5 + 24.0 + 10.0));
    chrome.style = CaptionStyle::default();
    assert_eq!(chrome.reserved(), (0.0, 3.0 * BUTTON_WIDTH));
    chrome.custom = false;
    assert_eq!(chrome.reserved(), (0.0, 0.0));
}

#[test]
fn the_title_is_centred_or_slid_into_the_gap() {
    assert_eq!(title_x(800.0, 420.0, 1300.0, 60.0), Some(770.0));
    assert_eq!(title_x(400.0, 420.0, 1300.0, 60.0), Some(436.0));
    assert_eq!(title_x(1280.0, 420.0, 1300.0, 60.0), Some(1224.0));
    assert_eq!(title_x(400.0, 420.0, 480.0, 60.0), None);
    assert_eq!(title_x(f32::NAN, 420.0, 1300.0, 60.0), Some(436.0));
    assert_eq!(title_x(400.0, f32::INFINITY, 1300.0, 60.0), None);
}
