// Adapted from photocraft crates/ui-egui/src/menus/menu_keys.rs
//! The menu bar from the keyboard, as on Windows: a tap on Alt (or F10) puts
//! the keyboard on the bar (the first title lit; Left / Right move along it,
//! Down / Enter open the lit menu, Esc, Space or a click leaves), Alt+letter
//! opens the menu with that mnemonic straight away, and in an open menu a
//! letter runs the row with that mnemonic (or opens its submenu).
//!
//! Mnemonics are underlined while Alt is held, the bar is focused or a menu
//! was opened from the keyboard; a menu opened with the mouse shows none until
//! Alt is pressed. They are assigned per menu level from the labels as shown,
//! unique within the level: [`TOP_PREFERRED`] for the top menus, else the
//! first unused letter, word starts first.
//!
//! The state is view state in egui memory, like [`crate::menu_nav::Nav`]. On
//! macOS the native menu bar replaces the in-window one and this never runs.

use eframe::egui::{self, Color32, Event, Id, Key, Modifiers, Stroke, Ui};

use crate::menu_nav::Nav;

/// Mnemonics of the top menus, used where the shown label has the letter.
/// View takes I and nothing takes A or V: Alt+A opens the commit actions and
/// Alt+V pages up in the reading views ([`TOP_RESERVED`]).
pub const TOP_PREFERRED: [(&str, char); 7] = [
    ("Git Good", 'G'),
    ("Repository", 'R'),
    ("Branch", 'B'),
    ("Commit", 'C'),
    ("Changes", 'N'),
    ("View", 'I'),
    ("Help", 'H'),
];

/// Letters no top menu takes: they are Alt shortcuts of their own.
pub const TOP_RESERVED: [char; 2] = ['A', 'V'];

/// Keyboard state of the menu bar between frames.
#[derive(Clone, Debug, Default)]
pub struct Keys {
    /// The title the keyboard is on with no menu open (Alt tap, F10).
    pub focus: Option<usize>,
    /// Underline the mnemonics: the bar is focused or a menu was opened from
    /// the keyboard.
    pub underline: bool,
    /// Alt is down (from its key events, whatever the modifiers report).
    alt_down: bool,
    /// Alt went down and nothing else was pressed since: releasing it is a tap.
    alt_tap: bool,
    /// The mouse buttons that are down, one bit each, from the button events
    /// in order (a whole click within one frame leaves no pointer state).
    buttons_held: u8,
    /// A menu was just opened from the keyboard: light its first row once it
    /// is drawn.
    first_row: bool,
}

fn keys_id() -> Id {
    Id::unique("gg-menu-bar-keys")
}

fn focus_id() -> Id {
    Id::unique("gg-menu-bar-focus")
}

/// Is the keyboard on the menu bar (no menu open yet)? The shortcuts leave the
/// keys to it then, as they do for an open menu.
pub fn is_focused(ctx: &egui::Context) -> bool {
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|d| d.get_temp::<u64>(focus_id()))
        .is_some_and(|seen| seen + 2 >= pass)
}

/// A command shortcut pressed while the keyboard is on the bar with no menu
/// open (Ctrl+B, F5…) takes it off the bar, so the shortcut runs in the same
/// press, as a native menu bar does. Keys the bar uses (arrows, Enter, Esc,
/// letters, Alt+letter, F10) keep driving it. True when the bar was left.
/// Call before the shortcuts are read.
pub fn leave_for_shortcut(ctx: &egui::Context) -> bool {
    if !is_focused(ctx) || crate::menu_nav::is_open(ctx) {
        return false;
    }
    let shortcut = ctx.input(|i| {
        i.events.iter().any(|e| {
            matches!(e, Event::Key { key, pressed: true, modifiers, .. }
                if is_command_key(*key, *modifiers))
        })
    });
    if !shortcut {
        return false;
    }
    let mut keys = Keys::load(ctx);
    keys.focus = None;
    keys.underline = false;
    keys.store(ctx);
    ctx.request_repaint();
    true
}

/// A key press that is a command shortcut rather than a key of the bar: Ctrl
/// or Cmd with a key (not the modifier keys themselves), or a function key
/// other than F10 (which toggles the bar).
fn is_command_key(key: Key, mods: Modifiers) -> bool {
    let modifier_key = matches!(
        key,
        Key::ShiftLeft
            | Key::ShiftRight
            | Key::ControlLeft
            | Key::ControlRight
            | Key::AltLeft
            | Key::AltRight
            | Key::SuperLeft
            | Key::SuperRight
    );
    if modifier_key {
        return false;
    }
    let function_key = key != Key::F10
        && key
            .name()
            .strip_prefix('F')
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    mods.command || mods.ctrl || mods.mac_cmd || function_key
}

/// The mnemonic a key press stands for: the letter or digit of a plain key.
fn key_char(key: Key) -> Option<char> {
    let name = key.name();
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphanumeric() => Some(c.to_ascii_uppercase()),
        _ => None,
    }
}

/// Mnemonics for `labels`, unique within the group: the preferred letter when
/// the label has it, else the first unused ASCII letter or digit, trying the
/// first character of each word before the others. Letters in `avoid` are
/// never taken. Compared case-insensitively; the returned letter is upper case.
pub fn assign(labels: &[&str], preferred: &[Option<char>], avoid: &[char]) -> Vec<Option<char>> {
    let mut used: Vec<char> = avoid.to_vec();
    let mut out = Vec::with_capacity(labels.len());
    for (i, label) in labels.iter().enumerate() {
        let upper: Vec<char> = label.chars().map(|c| c.to_ascii_uppercase()).collect();
        let has = |c: char| upper.contains(&c);
        let mut pick = preferred
            .get(i)
            .copied()
            .flatten()
            .map(|c| c.to_ascii_uppercase())
            .filter(|&c| has(c) && !used.contains(&c));
        if pick.is_none() {
            let starts = label
                .split(|c: char| !c.is_alphanumeric())
                .filter_map(|w| w.chars().next());
            pick = starts
                .chain(label.chars())
                .filter(char::is_ascii_alphanumeric)
                .map(|c| c.to_ascii_uppercase())
                .find(|c| !used.contains(c));
        }
        if let Some(c) = pick {
            used.push(c);
        }
        out.push(pick);
    }
    out
}

/// The top menus' mnemonics, in order.
pub fn top_mnemonics(titles: &[&str]) -> Vec<Option<char>> {
    let preferred: Vec<Option<char>> = titles
        .iter()
        .map(|t| {
            TOP_PREFERRED
                .iter()
                .find(|(name, _)| name == t)
                .map(|(_, c)| *c)
        })
        .collect();
    assign(titles, &preferred, &TOP_RESERVED)
}

/// `label` as text with the mnemonic's first occurrence underlined (plain
/// text when `mnemonic` is None). `color` is the text color; None takes the
/// widget's.
pub fn mnemonic_text(
    ui: &Ui,
    label: &str,
    mnemonic: Option<char>,
    color: Option<Color32>,
) -> egui::WidgetText {
    let at = mnemonic.and_then(|m| {
        label
            .char_indices()
            .find(|(_, c)| c.to_ascii_uppercase() == m)
            .map(|(i, c)| (i, c.len_utf8()))
    });
    let Some((i, len)) = at else {
        let text = egui::RichText::new(label);
        return match color {
            Some(color) => text.color(color),
            None => text,
        }
        .into();
    };
    let color = color.unwrap_or(Color32::PLACEHOLDER);
    let font = egui::TextStyle::Button.resolve(ui.style());
    let plain = egui::TextFormat {
        font_id: font,
        color,
        ..Default::default()
    };
    let underlined = egui::TextFormat {
        underline: Stroke::new(1.0, color),
        ..plain.clone()
    };
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    for (part, format) in [
        (label.get(..i), &plain),
        (label.get(i..i + len), &underlined),
        (label.get(i + len..), &plain),
    ] {
        if let Some(part) = part.filter(|p| !p.is_empty()) {
            job.append(part, 0.0, format.clone());
        }
    }
    job.into()
}

/// Drop this frame's typed text: the bar or an open menu took the keys, so a
/// text field behind them must not type them too.
fn drop_text(ctx: &egui::Context) {
    ctx.input_mut(|i| i.events.retain(|e| !matches!(e, Event::Text(_))));
}

impl Keys {
    /// Last frame's state.
    pub fn load(ctx: &egui::Context) -> Keys {
        ctx.data(|d| d.get_temp(keys_id())).unwrap_or_default()
    }

    pub fn store(self, ctx: &egui::Context) {
        let pass = ctx.cumulative_pass_nr();
        ctx.data_mut(|d| {
            if self.focus.is_some() {
                d.insert_temp(focus_id(), pass);
            } else {
                d.remove::<u64>(focus_id());
            }
            d.insert_temp(keys_id(), self);
        });
    }

    /// Underline the mnemonics this frame?
    pub fn show_underlines(&self, ctx: &egui::Context) -> bool {
        self.underline || self.focus.is_some() || self.alt_down || ctx.input(|i| i.modifiers.alt)
    }

    /// A menu just opened from the keyboard: light its first row (once, when
    /// it is drawn).
    pub fn take_first_row(&mut self, nav: &mut Nav) {
        if self.first_row && nav.top.is_some() {
            nav.highlight_first(0);
            self.first_row = false;
        }
    }

    /// Handle this frame's keys, after the menus were drawn. `tops` are the
    /// titles' popup ids and `mnemonics` their letters, both in screen order;
    /// `nav` is the open menu's state. The command index a mnemonic ran lands
    /// in `clicked`.
    pub fn keys(
        &mut self,
        ctx: &egui::Context,
        tops: &[Id],
        mnemonics: &[Option<char>],
        nav: &mut Nav,
        clicked: &mut Option<usize>,
    ) {
        if tops.is_empty() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        let mut tap = false;
        // The tap is judged on every event of the frame, also those another
        // handler consumed before the bar's turn: Alt+A (commit actions) runs
        // first and leaves no trace in `events`, yet must not count as a tap.
        let raw = ctx.input(|i| i.raw.events.clone());
        for e in &raw {
            match e {
                // A held Alt auto-repeats: only its first press starts a tap.
                Event::Key {
                    key: Key::AltLeft | Key::AltRight,
                    pressed: true,
                    repeat: true,
                    ..
                } => self.alt_down = true,
                Event::Key {
                    key: Key::AltLeft | Key::AltRight,
                    pressed: true,
                    ..
                } => {
                    self.alt_down = true;
                    // Alt pressed during a drag modifies that drag.
                    self.alt_tap = self.buttons_held == 0;
                }
                Event::Key {
                    key: Key::AltLeft | Key::AltRight,
                    pressed: false,
                    ..
                } => {
                    self.alt_down = false;
                    tap |= std::mem::take(&mut self.alt_tap);
                }
                // A click or the end of a drag while Alt is held is no tap,
                // even when the whole gesture arrives in one frame.
                Event::PointerButton {
                    button, pressed, ..
                } => {
                    let bit = 1u8 << (*button as u8);
                    if *pressed {
                        self.buttons_held |= bit;
                    } else {
                        self.buttons_held &= !bit;
                    }
                    self.alt_tap = false;
                    tap = false;
                }
                // Alt going to the desktop (Alt+Tab) is no tap.
                Event::WindowFocused(false) => {
                    self.alt_tap = false;
                    self.alt_down = false;
                    self.buttons_held = 0;
                }
                // Alt+key, Alt+typed text and Alt+wheel are no tap either.
                Event::Key { pressed: true, .. } | Event::Text(_) | Event::MouseWheel { .. } => {
                    self.alt_tap = false
                }
                _ => {}
            }
        }
        if events
            .iter()
            .any(|e| matches!(e, Event::PointerButton { pressed: true, .. }))
        {
            self.focus = None;
            self.underline = false;
        }
        let open = nav.top.is_some();
        let n = tops.len();
        let open_top = |i: usize, keys: &mut Keys| {
            if let Some(&id) = tops.get(i) {
                egui::Popup::open_id(ctx, id);
                keys.focus = None;
                keys.underline = true;
                keys.first_row = true;
                ctx.request_repaint();
            }
        };
        let consume = |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_key(m, k));
        // Alt+letter opens that menu, open or not (an open one switches).
        let alt_letter = events.iter().find_map(|e| match e {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if modifiers.alt && !modifiers.command && !modifiers.ctrl => {
                key_char(*key).map(|c| (*key, c))
            }
            _ => None,
        });
        if let Some((key, c)) = alt_letter
            && let Some(i) = mnemonics.iter().position(|m| *m == Some(c))
        {
            consume(Modifiers::ALT, key);
            drop_text(ctx);
            self.alt_tap = false;
            open_top(i, self);
            return;
        }
        if open {
            // The menu has the keyboard: nothing types behind it.
            drop_text(ctx);
            let letter = events.iter().find_map(|e| match e {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if !modifiers.command && !modifiers.ctrl && !modifiers.alt => {
                    key_char(*key).map(|c| (*key, c))
                }
                _ => None,
            });
            if let Some((key, c)) = letter
                && let Some(row) = nav.row_with_mnemonic(c)
            {
                consume(Modifiers::NONE, key);
                self.underline = true;
                if let Some(cmd) = row.command {
                    *clicked = Some(cmd);
                    egui::Popup::close_all(ctx);
                } else {
                    nav.open_sub_row(ctx, row.widget);
                }
                ctx.request_repaint();
            }
            if tap {
                egui::Popup::close_all(ctx);
                self.underline = false;
                ctx.request_repaint();
            }
            return;
        }
        // No menu open: Alt (tapped) or F10 puts the keyboard on the bar or
        // takes it off again.
        // F10 alone: egui's match would also take Shift+F10, the context
        // menu key (a11y.rs).
        let f10 = ctx.input(|i| {
            i.events.iter().any(|event| {
                matches!(event, Event::Key { key: Key::F10, pressed: true, modifiers, .. }
                    if modifiers.is_none())
            })
        }) && consume(Modifiers::NONE, Key::F10);
        if tap || f10 {
            self.focus = if self.focus.is_some() { None } else { Some(0) };
            self.underline = false;
            ctx.request_repaint();
            return;
        }
        let Some(focus) = self.focus else { return };
        // The bar has the keyboard: nothing types behind it.
        drop_text(ctx);
        let press = |k: Key| consume(Modifiers::NONE, k);
        if press(Key::Escape) {
            self.focus = None;
        } else if press(Key::ArrowRight) {
            self.focus = Some((focus + 1) % n);
        } else if press(Key::ArrowLeft) {
            self.focus = Some((focus + n - 1) % n);
        } else if press(Key::ArrowDown) || press(Key::Enter) {
            open_top(focus, self);
        } else if press(Key::Space) {
            // Space leaves the bar and opens nothing.
            self.focus = None;
        } else if let Some((key, c)) = events.iter().find_map(|e| match e {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if modifiers.is_none() => key_char(*key).map(|c| (*key, c)),
            _ => None,
        }) {
            if let Some(i) = mnemonics.iter().position(|m| *m == Some(c)) {
                consume(Modifiers::NONE, key);
                open_top(i, self);
            }
        } else {
            return;
        }
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_letters_are_unique_conventional_and_avoid_alt_shortcuts() {
        let titles = ["Repository", "Branch", "Commit", "Changes", "View", "Help"];
        assert_eq!(
            top_mnemonics(&titles),
            "RBCNIH".chars().map(Some).collect::<Vec<_>>()
        );
        assert_eq!(
            top_mnemonics(&["Git Good", "Repository"]),
            [Some('G'), Some('R')]
        );
        // Without preferences the reserved letters are still skipped.
        assert_eq!(
            assign(&["Apply", "View"], &[], &TOP_RESERVED),
            [Some('P'), Some('I')]
        );
    }

    #[test]
    fn item_letters_take_word_starts_first_then_any_free_letter() {
        assert_eq!(
            assign(
                &["Open…", "Open recent", "Close", "Clone…", "Close all"],
                &[],
                &[]
            ),
            [Some('O'), Some('R'), Some('C'), Some('L'), Some('A')]
        );
        // A preferred letter the label lacks falls back; a taken one too.
        assert_eq!(
            assign(&["Datei", "Bearbeiten"], &[Some('F'), Some('E')], &[]),
            [Some('D'), Some('E')]
        );
        assert_eq!(
            assign(&["Aa", "Ab", "Ba"], &[Some('A'), Some('A'), Some('A')], &[]),
            [Some('A'), Some('B'), None]
        );
        // Only ASCII letters and digits are keys.
        assert_eq!(assign(&["→", "Zoom 100%"], &[], &[]), [None, Some('Z')]);
    }

    #[test]
    fn key_chars_are_plain_letters_and_digits() {
        assert_eq!(key_char(Key::A), Some('A'));
        assert_eq!(key_char(Key::Num7), Some('7'));
        assert_eq!(key_char(Key::F10), None);
        assert_eq!(key_char(Key::ArrowDown), None);
        assert_eq!(key_char(Key::Space), None);
    }

    #[test]
    fn command_keys_are_command_combos_and_function_keys_but_f10() {
        assert!(is_command_key(Key::B, Modifiers::COMMAND));
        assert!(is_command_key(Key::F5, Modifiers::NONE));
        assert!(
            !is_command_key(Key::F10, Modifiers::NONE),
            "F10 toggles the bar"
        );
        assert!(!is_command_key(Key::ControlLeft, Modifiers::CTRL));
        assert!(
            !is_command_key(Key::E, Modifiers::NONE),
            "a letter is a mnemonic"
        );
        assert!(
            !is_command_key(Key::E, Modifiers::ALT),
            "Alt+letter opens a menu"
        );
        assert!(!is_command_key(Key::ArrowRight, Modifiers::NONE));
        assert!(!is_command_key(Key::Escape, Modifiers::NONE));
    }

    #[test]
    fn underlined_text_marks_the_mnemonic_once() {
        let ctx = egui::Context::default();
        let mut job = None;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            job = Some(mnemonic_text(ui, "Open recent", Some('R'), None));
        });
        output.textures_delta.clear();
        let Some(egui::WidgetText::LayoutJob(job)) = job else {
            unreachable!("an underlined label is a layout job");
        };
        let underlined: Vec<&str> = job
            .sections
            .iter()
            .filter(|s| s.format.underline.width > 0.0)
            .map(|s| &job.text[s.byte_range.start.0..s.byte_range.end.0])
            .collect();
        assert_eq!(underlined, ["r"]);
        assert_eq!(job.text, "Open recent");
    }
}
