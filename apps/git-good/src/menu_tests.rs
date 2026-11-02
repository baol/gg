// Adapted from photocraft crates/ui-egui/src/menus/menu_keys/harness_tests.rs and menu_nav/tests.rs
//! The menu bar's keyboard and native-menu behaviour, driven with real egui
//! key and pointer events through `menus::show` (photocraft's
//! `menu_keys/harness_tests.rs` and `menu_nav/tests.rs`, adapted).
use crate::menu_keys::Keys;
use crate::menu_nav::Nav;
use crate::menus::{Entry, Menu, item};
use eframe::egui::{self, Event, Key, Modifiers, Pos2, Rect, pos2, vec2};

fn table(long: usize) -> Vec<Menu<i32>> {
    vec![
        Menu {
            title: "Repository",
            entries: vec![
                item("Open…", "Ctrl+O", 1).entry(),
                Entry::Submenu(
                    "Open recent".into(),
                    ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"]
                        .iter()
                        .zip(10..)
                        .map(|(label, command)| item(*label, "", command).entry())
                        .collect(),
                ),
                Entry::Separator,
                item("Close", "", 2).unless(Some("Nothing open")).entry(),
                item("Quit", "Ctrl+Q", 3).entry(),
            ],
        },
        Menu {
            title: "Branch",
            entries: vec![
                item("New branch…", "Ctrl+B", 20).entry(),
                item("Delete branch…", "", 21).entry(),
            ],
        },
        Menu {
            title: "View",
            entries: (0..long)
                .map(|i| item(format!("Row {i}"), "", 100 + i as i32).entry())
                .collect(),
        },
    ]
}

struct Bar {
    ctx: egui::Context,
    time: f64,
    size: egui::Vec2,
    menus: Vec<Menu<i32>>,
    /// What a text field below the bar holds, and whether it has the focus.
    text: String,
    field: bool,
    /// Alt+A ran (the commit actions shortcut, read before the bar).
    alt_a: usize,
    /// Ctrl+B ran (a command shortcut, read before the bar).
    ctrl_b: usize,
    chosen: Vec<i32>,
    /// The last frame's filled rectangles.
    fills: Vec<(Rect, egui::Color32)>,
}

impl Bar {
    fn new(long: usize, size: egui::Vec2) -> Self {
        let mut bar = Self {
            ctx: egui::Context::default(),
            time: 1.0,
            size,
            menus: table(long),
            text: String::new(),
            field: false,
            alt_a: 0,
            ctrl_b: 0,
            chosen: Vec::new(),
            fills: Vec::new(),
        };
        bar.frame(vec![]);
        bar.frame(vec![]);
        bar
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.time += 1.0 / 60.0;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
            events,
            time: Some(self.time),
            predicted_dt: 1.0 / 60.0,
            ..Default::default()
        };
        let menus = &self.menus;
        let mut chosen = None;
        let (text, field, alt_a, ctrl_b) = (
            &mut self.text,
            &mut self.field,
            &mut self.alt_a,
            &mut self.ctrl_b,
        );
        let mut output = self.ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            crate::menu_keys::leave_for_shortcut(&ctx);
            if ctx.input_mut(|i| i.consume_key(Modifiers::ALT, Key::A)) {
                *alt_a += 1;
            }
            if crate::navigation::available(&ctx)
                && ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::B))
            {
                *ctrl_b += 1;
            }
            egui::Panel::top("menu-bar").show(ui, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    chosen = crate::menus::show(ui, menus);
                });
            });
            egui::CentralPanel::default().show(ui, |ui| {
                let r = ui.add(egui::TextEdit::singleline(text).id_salt("field"));
                if *field {
                    r.request_focus();
                    *field = false;
                }
            });
        });
        output.textures_delta.clear();
        self.chosen.extend(chosen);
        self.fills = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::epaint::Shape::Rect(r) if r.fill.a() > 0 => Some((r.rect, r.fill)),
                _ => None,
            })
            .collect();
    }

    fn settle(&mut self) {
        for _ in 0..3 {
            self.frame(vec![]);
        }
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.frame(vec![
            press(key, modifiers, true),
            press(key, modifiers, false),
        ]);
        self.settle();
    }

    fn alt_tap(&mut self) {
        self.frame(vec![press(Key::AltLeft, Modifiers::ALT, true)]);
        self.frame(vec![press(Key::AltLeft, Modifiers::NONE, false)]);
        self.settle();
    }

    fn type_letter(&mut self, key: Key, c: &str) {
        self.frame(vec![
            press(key, Modifiers::NONE, true),
            Event::Text(c.into()),
            press(key, Modifiers::NONE, false),
        ]);
        self.settle();
    }

    fn click(&mut self, p: Pos2) {
        self.frame(vec![Event::PointerMoved(p)]);
        self.frame(vec![
            button(p, true, Modifiers::NONE),
            button(p, false, Modifiers::NONE),
        ]);
        self.settle();
    }

    fn move_to(&mut self, p: Pos2) {
        self.frame(vec![Event::PointerMoved(p)]);
    }

    fn keys(&self) -> Keys {
        Keys::load(&self.ctx)
    }

    fn nav(&self) -> Nav {
        Nav::current(&self.ctx)
    }

    fn rows(&self, level: usize) -> Vec<crate::menu_nav::Row> {
        self.nav().rows.get(level).cloned().unwrap_or_default()
    }

    fn title(&self, i: usize) -> Rect {
        self.nav().titles.get(i).copied().unwrap_or(Rect::NOTHING)
    }
}

fn press(key: Key, modifiers: Modifiers, pressed: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    }
}

fn bar() -> Bar {
    Bar::new(3, vec2(900.0, 600.0))
}

#[test]
fn alt_focuses_the_bar_arrows_move_and_down_opens_the_lit_menu() {
    let mut h = bar();
    h.alt_tap();
    assert_eq!(h.keys().focus, Some(0));
    assert!(crate::menu_keys::is_focused(&h.ctx));
    assert!(
        !crate::navigation::available(&h.ctx),
        "the bar has the keys"
    );
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.keys().focus, Some(1));
    h.key(Key::ArrowLeft, Modifiers::NONE);
    h.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(h.keys().focus, Some(2), "Left wraps to the last title");
    h.key(Key::ArrowRight, Modifiers::NONE);
    h.key(Key::ArrowRight, Modifiers::NONE);
    h.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(h.nav().top, Some(1), "Branch is open");
    assert_eq!(h.keys().focus, None);
    assert_eq!(h.nav().highlighted(0), Some(0), "its first row is lit");
    assert!(h.nav().underline);
    h.key(Key::Enter, Modifiers::NONE);
    assert_eq!(h.chosen, [20]);
    assert_eq!(h.nav().top, None);
}

#[test]
fn f10_focuses_the_bar_and_enter_opens_the_lit_menu() {
    let mut h = bar();
    h.key(Key::F10, Modifiers::NONE);
    assert_eq!(h.keys().focus, Some(0));
    h.key(Key::Enter, Modifiers::NONE);
    assert_eq!(h.nav().top, Some(0));
    h.key(Key::F10, Modifiers::NONE);
    assert_eq!(h.nav().top, Some(0), "F10 in an open menu does nothing");
}

#[test]
fn alt_with_a_key_a_wheel_a_click_or_lost_focus_is_no_tap() {
    let mut h = bar();
    // Alt+A runs the commit actions and leaves the bar alone.
    h.frame(vec![press(Key::AltLeft, Modifiers::ALT, true)]);
    h.frame(vec![
        press(Key::A, Modifiers::ALT, true),
        press(Key::A, Modifiers::ALT, false),
    ]);
    h.frame(vec![press(Key::AltLeft, Modifiers::NONE, false)]);
    h.settle();
    assert_eq!(h.alt_a, 1);
    assert_eq!(h.keys().focus, None);
    assert_eq!(h.nav().top, None);
    // Alt+wheel.
    h.frame(vec![press(Key::AltLeft, Modifiers::ALT, true)]);
    h.frame(vec![Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, 10.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: Modifiers::ALT,
    }]);
    h.frame(vec![press(Key::AltLeft, Modifiers::NONE, false)]);
    h.settle();
    assert_eq!(h.keys().focus, None);
    // Alt held through a click (a whole gesture in one frame).
    let p = pos2(450.0, 400.0);
    h.frame(vec![
        press(Key::AltLeft, Modifiers::ALT, true),
        button(p, true, Modifiers::ALT),
        button(p, false, Modifiers::ALT),
        press(Key::AltLeft, Modifiers::NONE, false),
    ]);
    h.settle();
    assert_eq!(h.keys().focus, None);
    // Alt+Tab to another window.
    h.frame(vec![press(Key::AltLeft, Modifiers::ALT, true)]);
    h.frame(vec![Event::WindowFocused(false)]);
    h.frame(vec![
        Event::WindowFocused(true),
        press(Key::AltLeft, Modifiers::NONE, false),
    ]);
    h.settle();
    assert_eq!(h.keys().focus, None);
}

#[test]
fn a_held_alt_repeating_is_still_one_tap() {
    let mut h = bar();
    h.frame(vec![press(Key::AltLeft, Modifiers::ALT, true)]);
    h.frame(vec![Event::Key {
        key: Key::AltLeft,
        physical_key: None,
        pressed: true,
        repeat: true,
        modifiers: Modifiers::ALT,
    }]);
    h.frame(vec![press(Key::AltLeft, Modifiers::NONE, false)]);
    h.settle();
    assert_eq!(h.keys().focus, Some(0));
}

#[test]
fn a_second_alt_esc_space_or_a_click_takes_the_keyboard_off_the_bar() {
    let mut h = bar();
    h.alt_tap();
    h.alt_tap();
    assert_eq!(h.keys().focus, None);
    h.alt_tap();
    h.key(Key::Escape, Modifiers::NONE);
    assert_eq!(h.keys().focus, None);
    h.alt_tap();
    h.key(Key::Space, Modifiers::NONE);
    assert_eq!(h.keys().focus, None);
    assert_eq!(h.nav().top, None, "Space opens nothing");
    h.alt_tap();
    h.click(pos2(450.0, 400.0));
    assert_eq!(h.keys().focus, None);
    assert!(crate::navigation::available(&h.ctx));
}

#[test]
fn alt_with_a_letter_opens_that_menu_but_never_takes_alt_shortcuts() {
    let mut h = bar();
    assert_eq!(
        crate::menu_keys::top_mnemonics(&["Repository", "Branch", "View"]),
        [Some('R'), Some('B'), Some('I')]
    );
    h.key(Key::B, Modifiers::ALT);
    assert_eq!(h.nav().top, Some(1));
    assert_eq!(h.nav().highlighted(0), Some(0));
    // Alt+letter switches an open menu.
    h.key(Key::I, Modifiers::ALT);
    assert_eq!(h.nav().top, Some(2));
    h.key(Key::Escape, Modifiers::NONE);
    // Alt+V pages up in the reading views: View is Alt+I.
    h.key(Key::V, Modifiers::ALT);
    assert_eq!(h.nav().top, None);
    assert_eq!(h.keys().focus, None);
}

#[test]
fn a_letter_in_an_open_menu_runs_the_row_with_that_mnemonic_or_opens_its_submenu() {
    let mut h = bar();
    h.key(Key::R, Modifiers::ALT);
    let mnemonics: Vec<Option<char>> = h.rows(0).iter().map(|r| r.mnemonic).collect();
    assert_eq!(mnemonics, [Some('O'), Some('R'), Some('C'), Some('Q')]);
    h.type_letter(Key::R, "r");
    assert_eq!(h.nav().level, 1, "Open recent's submenu has the keys");
    assert_eq!(h.rows(1).len(), 6);
    assert_eq!(h.nav().highlighted(1), Some(0));
    h.type_letter(Key::B, "b");
    assert_eq!(h.chosen, [11]);
    assert_eq!(h.nav().top, None);
    // A disabled row's letter does nothing.
    h.key(Key::R, Modifiers::ALT);
    h.type_letter(Key::C, "c");
    assert!(h.chosen.len() == 1 && h.nav().top == Some(0));
    h.type_letter(Key::Q, "q");
    assert_eq!(h.chosen, [11, 3]);
}

#[test]
fn mnemonics_are_underlined_only_with_alt_or_a_keyboard_opened_menu() {
    let mut h = bar();
    let title = h.title(0).center();
    h.click(title);
    assert_eq!(h.nav().top, Some(0));
    assert!(
        !h.nav().underline,
        "a menu opened with the mouse shows none"
    );
    h.frame(vec![press(Key::AltLeft, Modifiers::ALT, true)]);
    h.frame(vec![]);
    assert!(h.nav().underline, "Alt held shows them");
    h.frame(vec![press(Key::AltLeft, Modifiers::NONE, false)]);
    h.settle();
    assert_eq!(h.nav().top, None, "an Alt tap closes the open menu");
    assert!(!h.nav().underline);
    h.key(Key::R, Modifiers::ALT);
    assert!(h.nav().underline);
}

#[test]
fn a_command_shortcut_leaves_the_bar_and_runs() {
    let mut h = bar();
    h.alt_tap();
    h.key(Key::B, Modifiers::COMMAND);
    assert_eq!(h.ctrl_b, 1);
    assert_eq!(h.keys().focus, None);
    assert_eq!(h.nav().top, None);
}

#[test]
fn a_focused_text_field_keeps_its_text_while_the_bar_or_a_menu_has_the_keys() {
    let mut h = bar();
    h.field = true;
    h.settle();
    h.type_letter(Key::X, "x");
    assert_eq!(h.text, "x", "the field types");
    h.alt_tap();
    assert_eq!(h.keys().focus, Some(0));
    h.type_letter(Key::Z, "z");
    assert_eq!(h.text, "x", "the bar took the letter");
    h.type_letter(Key::B, "b");
    assert_eq!(h.nav().top, Some(1), "B opens Branch");
    assert_eq!(h.text, "x");
    h.key(Key::Escape, Modifiers::NONE);
    assert_eq!(h.nav().top, None);
}

#[test]
fn arrows_skip_disabled_rows_wrap_and_enter_runs_the_lit_row() {
    let mut h = bar();
    h.click(h.title(0).center());
    assert_eq!(h.nav().highlighted(0), None);
    h.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(h.nav().highlighted(0), Some(0));
    h.key(Key::ArrowDown, Modifiers::NONE);
    h.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(
        h.nav().highlighted(0),
        Some(3),
        "Close is disabled: skipped"
    );
    h.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(h.nav().highlighted(0), Some(0), "wraps");
    h.key(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(h.nav().highlighted(0), Some(3));
    h.key(Key::Enter, Modifiers::NONE);
    assert_eq!(h.chosen, [3]);
    assert_eq!(h.nav().top, None);
}

#[test]
fn right_and_left_open_and_close_submenus_and_switch_menus() {
    let mut h = bar();
    h.key(Key::R, Modifiers::ALT);
    // Right on a command switches to the next menu.
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.nav().top, Some(1));
    h.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(h.nav().top, Some(0));
    h.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(h.nav().top, Some(2), "Left wraps to the last menu");
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.nav().top, Some(0));
    h.key(Key::ArrowDown, Modifiers::NONE);
    h.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(h.nav().highlighted(0), Some(1));
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.nav().level, 1, "Right opens Open recent");
    assert_eq!(h.rows(1).len(), 6);
    assert_eq!(h.nav().highlighted(1), Some(0));
    h.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(h.nav().level, 0);
    assert_eq!(h.nav().top, Some(0));
    assert!(h.rows(1).is_empty(), "the submenu closed");
    // Enter opens a submenu too, Space runs a command, Esc closes all.
    h.key(Key::Enter, Modifiers::NONE);
    assert_eq!(h.nav().level, 1);
    h.key(Key::ArrowDown, Modifiers::NONE);
    h.key(Key::Space, Modifiers::NONE);
    assert_eq!(h.chosen, [11]);
    h.key(Key::R, Modifiers::ALT);
    h.key(Key::Escape, Modifiers::NONE);
    assert_eq!(h.nav().top, None);
}

#[test]
fn the_lit_row_and_the_focused_title_are_filled_like_hovered_ones() {
    let mut h = bar();
    let fill_at = |h: &Bar, rect: Rect| {
        h.fills
            .iter()
            .rev()
            .find(|(r, _)| r.contains(rect.center()) && r.height() <= rect.height() + 2.0)
            .map(|(_, fill)| *fill)
    };
    // What the pointer draws on a row and on a title.
    h.click(h.title(0).center());
    let quit = h.rows(0)[3].rect;
    h.move_to(quit.center());
    h.settle();
    h.settle();
    h.settle();
    let hovered_row = fill_at(&h, quit);
    assert!(hovered_row.is_some());
    h.key(Key::Escape, Modifiers::NONE);
    h.move_to(h.title(1).center());
    h.settle();
    h.settle();
    h.settle();
    let hovered_title = fill_at(&h, h.title(1));
    assert!(hovered_title.is_some());
    h.move_to(pos2(450.0, 400.0));
    h.settle();
    assert_eq!(fill_at(&h, h.title(1)), None);
    // The keyboard draws the same.
    h.alt_tap();
    assert_eq!(fill_at(&h, h.title(0)), hovered_title, "the focused title");
    assert_eq!(fill_at(&h, h.title(1)), None);
    h.key(Key::ArrowDown, Modifiers::NONE);
    // Past the popup's fade-in.
    for _ in 0..30 {
        h.frame(vec![]);
    }
    let rows = h.rows(0);
    assert_eq!(fill_at(&h, rows[0].rect), hovered_row, "the keyboard's row");
    assert_eq!(fill_at(&h, rows[3].rect), None);
}

#[test]
fn the_pointer_and_the_keys_share_one_highlight() {
    let mut h = bar();
    h.click(h.title(0).center());
    let quit = h.rows(0)[3].rect.center();
    h.move_to(quit);
    h.settle();
    assert_eq!(h.nav().highlighted(0), Some(3));
    h.key(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(h.nav().highlighted(0), Some(1), "Up from Quit skips Close");
    // The pointer resting on Quit doesn't take the highlight back.
    assert_eq!(h.nav().highlighted(0), Some(1));
}

#[test]
fn a_menu_taller_than_the_window_scrolls_by_keys_wheel_and_arrows() {
    let mut h = Bar::new(60, vec2(900.0, 400.0));
    h.key(Key::I, Modifiers::ALT);
    h.settle();
    let nav = h.nav();
    let (view, content) = (nav.views[0], nav.contents[0]);
    assert!(
        content > view.height() + 100.0,
        "{content} rows in {view:?}"
    );
    assert!(view.bottom() <= 400.0 - 6.0, "the menu stays in the window");
    assert!(view.top() >= h.title(0).bottom(), "and below the bar");
    let visible = |h: &Bar, i: usize| {
        let nav = h.nav();
        nav.views[0].contains_rect(nav.rows[0][i].rect.shrink(1.0))
    };
    assert!(visible(&h, 0) && !visible(&h, 59));
    // Up from the first row wraps to the last, scrolled into view.
    h.key(Key::ArrowUp, Modifiers::NONE);
    for _ in 0..60 {
        h.frame(vec![]);
    }
    assert_eq!(h.nav().highlighted(0), Some(59));
    assert!(visible(&h, 59), "the lit row is in view");
    assert!(!visible(&h, 0));
    // The wheel scrolls back up.
    let inside = h.nav().views[0].center();
    h.move_to(inside);
    for _ in 0..40 {
        h.frame(vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, 200.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: Modifiers::NONE,
        }]);
    }
    h.settle();
    assert!(visible(&h, 0), "the wheel scrolled to the top");
    // Resting on the bottom arrow scrolls down.
    let view = h.nav().views[0];
    let arrow = pos2(
        view.center().x,
        view.bottom() + crate::menu_nav::ARROW / 2.0,
    );
    h.move_to(arrow);
    // 600 points a second: the 1440-point menu in two seconds.
    for _ in 0..150 {
        h.frame(vec![]);
    }
    assert!(visible(&h, 59), "resting on the down arrow reached the end");
}

/// Hovering Open recent opens its submenu; heading for it across Close and
/// Quit, on a diagonal with a short pause, keeps it open. Resting on Quit
/// then lets Quit take over.
#[test]
fn an_open_submenu_stays_open_while_the_pointer_heads_for_it() {
    let mut h = bar();
    h.click(h.title(0).center());
    let recent = h.rows(0)[1].rect;
    let start = pos2(recent.right() - 60.0, recent.center().y);
    h.move_to(start);
    h.settle();
    assert_eq!(h.rows(1).len(), 6, "hovering opens the submenu");
    let sub = h.nav().views[1];
    assert!(
        sub.left() >= recent.right() - 1.0,
        "{sub:?} right of {recent:?}"
    );
    // Head down-right toward the submenu's lower part, crossing Quit.
    let quit = h.rows(0)[3].rect;
    let end = pos2(recent.right() - 2.0, quit.center().y);
    for step in 1..=10 {
        let t = step as f32 / 10.0;
        h.move_to(start + (end - start) * t);
        if step == 5 {
            // A short pause on the way (under the rest limit).
            for _ in 0..6 {
                h.frame(vec![]);
            }
        }
        assert_eq!(h.rows(1).len(), 6, "still open at step {step}");
        assert_eq!(h.nav().highlighted(0), Some(1), "Open recent stays lit");
    }
    // Resting on Quit, the submenu closes and Quit lights.
    for _ in 0..40 {
        h.frame(vec![]);
    }
    assert!(
        h.rows(1).is_empty(),
        "resting lets the crossed row take over"
    );
    assert_eq!(h.nav().highlighted(0), Some(3));
}

#[test]
fn context_menus_and_choice_lists_keep_egui_keyboard_navigation() {
    // Popups that list `menus::entries` (the commit actions) rely on egui's
    // own menu focus: Down lights the first row and Enter runs it.
    let ctx = egui::Context::default();
    let list = vec![item("First", "", 1).entry(), item("Second", "", 2).entry()];
    let mut chosen = None;
    let mut time = 1.0;
    let popup = egui::Id::unique("test-popup");
    let mut run = |events: Vec<Event>, chosen: &mut Option<i32>| {
        time += 1.0 / 60.0;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0))),
            events,
            time: Some(time),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            let r = ui.button("anchor");
            egui::Popup::menu(&r)
                .id(popup)
                .show(|ui| crate::menus::entries(ui, &list, chosen));
        });
        output.textures_delta.clear();
    };
    run(vec![], &mut chosen);
    egui::Popup::open_id(&ctx, popup);
    for _ in 0..3 {
        run(vec![], &mut chosen);
    }
    for k in [Key::ArrowDown, Key::ArrowDown, Key::Enter] {
        run(vec![press(k, Modifiers::NONE, true)], &mut chosen);
        run(vec![press(k, Modifiers::NONE, false)], &mut chosen);
        run(vec![], &mut chosen);
    }
    assert_eq!(chosen, Some(2));
    assert!(!egui::Popup::is_id_open(&ctx, popup));
}

#[test]
fn moving_onto_another_title_switches_the_open_menu_and_a_click_closes_it() {
    let mut h = bar();
    h.click(h.title(0).center());
    assert_eq!(h.nav().top, Some(0));
    h.move_to(h.title(1).center());
    h.settle();
    assert_eq!(h.nav().top, Some(1), "Branch opens without a second click");
    // A pointer resting on Branch doesn't undo the keys.
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.nav().top, Some(2));
    h.click(h.title(2).center());
    assert_eq!(
        h.nav().top,
        None,
        "clicking the open menu's title closes it"
    );
}
