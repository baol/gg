//! Adapted from photocraft apps/photocraft/src/mac_menu.rs.
//!
//! The macOS menu bar, through muda (a standalone native menu crate, not
//! Tauri: no webview, just AppKit menus behind a safe API). The menus
//! themselves, their Mac layout and how their keys reach gg are in
//! `native_menu`; this file only builds and updates the `NSMenu`s.
//!
//! - Item ids are the items' places in gg's table (`native_menu::from_table`).
//! - A structure change (a new Open recent entry, the Theme submenu's
//!   label) rebuilds the menu; anything else (labels, enabled, checked) is
//!   updated in place.
//! - A chosen item is a click or a key equivalent: AppKit's current event
//!   says which. Key equivalents go back to egui as the key press that was
//!   made (the event's modifiers, not the item's), so gg's key rules apply.
//! - Hide is a custom item calling `NSApplication hide:`, as the predefined
//!   one always takes ⌘H.

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, channel};

use crate::native_menu::{self, Backend, Chord, Event, MenuBar, MenuRole, Node, Standard};
use eframe::egui;
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSEventType};

enum Handle {
    Plain(MenuItem),
    Check(CheckMenuItem),
}

/// One native item.
struct Entry {
    handle: Handle,
    chord: Option<Chord>,
    label: String,
    enabled: bool,
}

/// How an item was chosen, read from AppKit's current event while it calls the handler.
struct Chosen {
    id: String,
    /// A key press (not a mouse click), and the modifiers held: ⌘, ⌃, ⌥, ⇧.
    key: bool,
    mods: [bool; 4],
}

pub struct MacMenu {
    menu: Option<Menu>,
    /// By item id.
    entries: HashMap<String, Entry>,
    rx: Receiver<Chosen>,
    structure: u64,
}

impl MacMenu {
    fn new(ctx: &egui::Context) -> MacMenu {
        let (tx, rx) = channel();
        let repaint = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            // AppKit calls this on the main thread, from the event that chose
            // the item: a key equivalent is a key-down, a click a mouse-up.
            let current = MainThreadMarker::new()
                .and_then(|mtm| NSApplication::sharedApplication(mtm).currentEvent());
            let key = current
                .as_ref()
                .is_some_and(|ev| ev.r#type() == NSEventType::KeyDown);
            let flags = current
                .as_ref()
                .map_or(NSEventModifierFlags::empty(), |ev| ev.modifierFlags());
            let mods = [
                NSEventModifierFlags::Command,
                NSEventModifierFlags::Control,
                NSEventModifierFlags::Option,
                NSEventModifierFlags::Shift,
            ]
            .map(|m| flags.contains(m));
            let _ = tx.send(Chosen {
                id: event.id.0,
                key,
                mods,
            });
            // Wake egui, so the item runs now rather than on the next mouse move.
            repaint.request_repaint();
        }));
        MacMenu {
            menu: None,
            entries: HashMap::new(),
            rx,
            structure: 0,
        }
    }

    fn build(&mut self, bar: &MenuBar) {
        if let Some(old) = self.menu.take() {
            old.remove_for_nsapp();
        }
        self.entries.clear();
        let menu = Menu::new();
        let mut special = Vec::new();
        for m in &bar.menus {
            let sub = Submenu::new(escape(&m.title), true);
            self.append(&sub, &m.children);
            let _ = menu.append(&sub);
            if matches!(m.role, MenuRole::Window | MenuRole::Help) {
                special.push((m.role, sub));
            }
        }
        menu.init_for_nsapp();
        // Only once the menu is NSApp's: muda looks the submenus up in the installed main menu.
        for (role, sub) in special {
            match role {
                MenuRole::Window => sub.set_as_windows_menu_for_nsapp(),
                _ => sub.set_as_help_menu_for_nsapp(),
            }
        }
        self.menu = Some(menu);
        self.structure = native_menu::structure_key(bar);
    }

    fn append(&mut self, sub: &Submenu, nodes: &[Node]) {
        for node in nodes {
            match node {
                Node::Separator => {
                    let _ = sub.append(&PredefinedMenuItem::separator());
                }
                Node::Submenu { label, children } => {
                    let child = Submenu::new(escape(label), !children.is_empty());
                    self.append(&child, children);
                    let _ = sub.append(&child);
                }
                Node::Standard(item) => {
                    let predefined = match item {
                        Standard::Services => PredefinedMenuItem::services(None),
                        Standard::HideOthers => PredefinedMenuItem::hide_others(None),
                        Standard::ShowAll => PredefinedMenuItem::show_all(None),
                        Standard::Minimize => PredefinedMenuItem::minimize(None),
                        Standard::Zoom => PredefinedMenuItem::maximize(Some("Zoom")),
                        Standard::BringAllToFront => PredefinedMenuItem::bring_all_to_front(None),
                    };
                    let _ = sub.append(&predefined);
                }
                Node::Item(item) => {
                    let chord = item.shortcut.as_deref().and_then(Chord::parse);
                    let accel = chord
                        .as_ref()
                        .filter(|c| c.native_ok())
                        .and_then(accelerator);
                    let handle = match item.checked {
                        Some(checked) => {
                            let h = CheckMenuItem::with_id(
                                item.id.clone(),
                                escape(&item.label),
                                item.enabled,
                                checked,
                                accel,
                            );
                            let _ = sub.append(&h);
                            Handle::Check(h)
                        }
                        None => {
                            let h = MenuItem::with_id(
                                item.id.clone(),
                                escape(&item.label),
                                item.enabled,
                                accel,
                            );
                            let _ = sub.append(&h);
                            Handle::Plain(h)
                        }
                    };
                    self.entries.insert(
                        item.id.clone(),
                        Entry {
                            handle,
                            chord,
                            label: item.label.clone(),
                            enabled: item.enabled,
                        },
                    );
                }
            }
        }
    }
}

impl Backend for MacMenu {
    fn sync(&mut self, bar: &MenuBar) {
        if self.menu.is_none() || native_menu::structure_key(bar) != self.structure {
            self.build(bar);
            return;
        }
        let items: HashMap<&str, &native_menu::Item> = bar
            .items()
            .into_iter()
            .map(|item| (item.id.as_str(), item))
            .collect();
        for (id, entry) in &mut self.entries {
            let Some(item) = items.get(id.as_str()) else {
                continue;
            };
            if entry.label != item.label {
                let text = escape(&item.label);
                match &entry.handle {
                    Handle::Plain(h) => h.set_text(text),
                    Handle::Check(h) => h.set_text(text),
                }
                entry.label = item.label.clone();
            }
            if entry.enabled != item.enabled {
                match &entry.handle {
                    Handle::Plain(h) => h.set_enabled(item.enabled),
                    Handle::Check(h) => h.set_enabled(item.enabled),
                }
                entry.enabled = item.enabled;
            }
            // Compare with AppKit's own state: muda toggles a check item when it's clicked.
            if let (Handle::Check(h), Some(checked)) = (&entry.handle, item.checked)
                && h.is_checked() != checked
            {
                h.set_checked(checked);
            }
        }
    }

    fn drain(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        while let Ok(chosen) = self.rx.try_recv() {
            // Ids that aren't ours are AppKit's own items (Services, Zoom, …), already handled.
            let Some(entry) = self.entries.get(&chosen.id) else {
                continue;
            };
            if chosen.id == native_menu::HIDE {
                if let Some(mtm) = MainThreadMarker::new() {
                    NSApplication::sharedApplication(mtm).hide(None);
                }
                continue;
            }
            // A key-down chose it: its key equivalent, unless it was ↩ or
            // Space inside the open menu (no ⌘ or ⌃, and the item's shortcut
            // needs one). The key is the item's; the modifiers are the ones held.
            let [cmd, ctrl, alt, shift] = chosen.mods;
            let key = entry
                .chord
                .as_ref()
                .filter(|c| chosen.key && (cmd || ctrl || !(c.cmd || c.ctrl)))
                .map(|c| Chord {
                    cmd,
                    ctrl,
                    alt,
                    shift,
                    key: c.key.clone(),
                });
            out.push(match key {
                Some(chord) if chosen.id != native_menu::MINIMIZE => Event::Key(chord),
                _ => Event::Click(chosen.id),
            });
        }
        out
    }
}

/// Install the menu bar now (in eframe's creator closure, so winit's
/// default menu doesn't stay). `None` keeps the in-window menus.
pub fn install<C: Clone>(ctx: &egui::Context) -> Option<native_menu::NativeMenu<C>> {
    // Never crash for a menu: if AppKit refuses, keep the in-window menus.
    let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| MacMenu::new(ctx)));
    match built {
        Ok(menu) => Some(native_menu::NativeMenu::new(Box::new(menu))),
        Err(_) => {
            eprintln!("couldn't install the macOS menu bar; using the in-window menus");
            None
        }
    }
}

/// `&` marks a mnemonic in muda labels.
fn escape(s: &str) -> String {
    s.replace('&', "&&")
}

/// A key equivalent for AppKit. `cmd` is ⌘ (`META`) and `ctrl` is ⌃: never folded together.
fn accelerator(chord: &Chord) -> Option<Accelerator> {
    let mut mods = Modifiers::empty();
    for (on, m) in [
        (chord.cmd, Modifiers::META),
        (chord.ctrl, Modifiers::CONTROL),
        (chord.alt, Modifiers::ALT),
        (chord.shift, Modifiers::SHIFT),
    ] {
        if on {
            mods |= m;
        }
    }
    const LETTERS: [Code; 26] = [
        Code::KeyA,
        Code::KeyB,
        Code::KeyC,
        Code::KeyD,
        Code::KeyE,
        Code::KeyF,
        Code::KeyG,
        Code::KeyH,
        Code::KeyI,
        Code::KeyJ,
        Code::KeyK,
        Code::KeyL,
        Code::KeyM,
        Code::KeyN,
        Code::KeyO,
        Code::KeyP,
        Code::KeyQ,
        Code::KeyR,
        Code::KeyS,
        Code::KeyT,
        Code::KeyU,
        Code::KeyV,
        Code::KeyW,
        Code::KeyX,
        Code::KeyY,
        Code::KeyZ,
    ];
    const DIGITS: [Code; 10] = [
        Code::Digit0,
        Code::Digit1,
        Code::Digit2,
        Code::Digit3,
        Code::Digit4,
        Code::Digit5,
        Code::Digit6,
        Code::Digit7,
        Code::Digit8,
        Code::Digit9,
    ];
    const F: [Code; 24] = [
        Code::F1,
        Code::F2,
        Code::F3,
        Code::F4,
        Code::F5,
        Code::F6,
        Code::F7,
        Code::F8,
        Code::F9,
        Code::F10,
        Code::F11,
        Code::F12,
        Code::F13,
        Code::F14,
        Code::F15,
        Code::F16,
        Code::F17,
        Code::F18,
        Code::F19,
        Code::F20,
        Code::F21,
        Code::F22,
        Code::F23,
        Code::F24,
    ];
    let key = chord.key.as_str();
    let mut chars = key.chars();
    let code = match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => {
            *LETTERS.get((c.to_ascii_uppercase() as usize).checked_sub('A' as usize)?)?
        }
        (Some(c), None) if c.is_ascii_digit() => {
            *DIGITS.get((c as usize).checked_sub('0' as usize)?)?
        }
        _ if chord.is_function_key() => {
            *F.get(key.get(1..)?.parse::<usize>().ok()?.checked_sub(1)?)?
        }
        _ => match key {
            "[" => Code::BracketLeft,
            "]" => Code::BracketRight,
            "\\" => Code::Backslash,
            "/" => Code::Slash,
            "=" => Code::Equal,
            "-" => Code::Minus,
            "'" => Code::Quote,
            ";" => Code::Semicolon,
            "," => Code::Comma,
            "." => Code::Period,
            "`" => Code::Backquote,
            "Del" => Code::Delete,
            "Enter" if !mods.is_empty() => Code::Enter,
            "Tab" if !mods.is_empty() => Code::Tab,
            "Space" if !mods.is_empty() => Code::Space,
            _ => return None,
        },
    };
    Some(Accelerator::new(mods, code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_and_command_stay_distinct() {
        let a = Chord::parse("Cmd+M").and_then(|c| accelerator(&c));
        let b = Chord::parse("Control+Cmd+M").and_then(|c| accelerator(&c));
        assert!(a.is_some() && b.is_some());
        assert_ne!(a, b);
    }
}
