//! Adapted from photocraft crates/ui-egui/src/menus/native_menu/tests.rs.

use super::*;
use crate::menus::{Entry, item};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cmd {
    Open,
    Fetch,
    Recent(u8),
    Preferences,
    Quit,
    Inline,
    Shortcuts,
    About,
}

fn role(command: &Cmd) -> Option<ItemRole> {
    match command {
        Cmd::About => Some(ItemRole::About),
        Cmd::Preferences => Some(ItemRole::Settings),
        Cmd::Quit => Some(ItemRole::Quit),
        _ => None,
    }
}

/// A table shaped like gg's on Linux and Windows.
fn table(recent: u8, inline: bool, fetch: bool) -> Vec<menus::Menu<Cmd>> {
    vec![
        menus::Menu {
            title: "Repository",
            entries: vec![
                item("Open…", "Ctrl+O", Cmd::Open).entry(),
                Entry::Submenu(
                    "Open recent".into(),
                    (0..recent)
                        .map(|i| item(format!("repo {i}"), "", Cmd::Recent(i)).entry())
                        .collect(),
                ),
                Entry::Separator,
                item("Fetch", "Ctrl+Shift+F", Cmd::Fetch)
                    .unless((!fetch).then_some("Open a repository first"))
                    .entry(),
                Entry::Separator,
                item("Preferences…", "Ctrl+,", Cmd::Preferences).entry(),
                item("Quit", "Ctrl+Q", Cmd::Quit).entry(),
            ],
        },
        menus::Menu {
            title: "View",
            entries: vec![item("Inline diff", "", Cmd::Inline).checked(inline).entry()],
        },
        menus::Menu {
            title: "Help",
            entries: vec![
                item("Keyboard shortcuts", "?", Cmd::Shortcuts).entry(),
                item("About Git Good", "", Cmd::About).entry(),
            ],
        },
    ]
}

fn layout(table: &[menus::Menu<Cmd>]) -> Layout {
    mac_layout(&from_table(table, role).0)
}

fn ids(nodes: &[Node]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| match n {
            Node::Item(item) => item.label.clone(),
            Node::Separator => "---".into(),
            Node::Submenu { label, .. } => format!("[{label}]"),
            Node::Standard(s) => format!("<{s:?}>"),
        })
        .collect()
}

#[test]
fn gg_gets_a_mac_app_menu_window_menu_and_help() {
    let l = layout(&table(2, true, true));
    let titles: Vec<&str> = l.bar.menus.iter().map(|m| m.title.as_str()).collect();
    assert_eq!(titles, ["Git Good", "Repository", "View", "Window", "Help"]);
    let app = &l.bar.menus[0];
    assert_eq!(app.role, MenuRole::App);
    assert_eq!(
        ids(&app.children),
        [
            "About Git Good",
            "---",
            "Settings…",
            "---",
            "<Services>",
            "---",
            "Hide Git Good",
            "<HideOthers>",
            "<ShowAll>",
            "---",
            "Quit Git Good"
        ]
    );
    let shortcut = |label: &str| {
        l.bar
            .items()
            .into_iter()
            .find(|i| i.label == label)
            .and_then(|i| i.shortcut.clone())
    };
    assert_eq!(shortcut("Settings…").as_deref(), Some("Cmd+,"));
    assert_eq!(shortcut("Quit Git Good").as_deref(), Some("Cmd+Q"));
    assert_eq!(shortcut("Hide Git Good").as_deref(), Some("Cmd+H"));
    assert!(l.clashes.is_empty(), "{:?}", l.clashes);
    let window = l.bar.menu(MenuRole::Window).map(|m| ids(&m.children));
    assert_eq!(
        window.as_deref(),
        Some(&["<Minimize>", "<Zoom>", "---", "<BringAllToFront>"].map(String::from)[..])
    );
    let help = l.bar.menu(MenuRole::Help).map(|m| ids(&m.children));
    assert_eq!(
        help.as_deref(),
        Some(&["Keyboard shortcuts".to_string()][..])
    );
    let repository = ids(&l.bar.menus[1].children);
    assert_eq!(
        repository,
        ["Open…", "[Open recent]", "---", "Fetch"],
        "Preferences and Quit leave Repository, and the separator before them goes too"
    );
}

/// The in-window "Git Good" menu macOS shows today empties into the app menu.
#[test]
fn a_menu_emptied_by_the_moves_goes_away() {
    let mut menus = table(0, true, true);
    menus.insert(
        0,
        menus::Menu {
            title: "Git Good",
            entries: vec![
                item("Settings…", "Cmd+,", Cmd::Preferences).entry(),
                Entry::Separator,
                item("Quit", "Cmd+Q", Cmd::Quit).entry(),
            ],
        },
    );
    menus[1].entries.truncate(4);
    let l = layout(&menus);
    let titles: Vec<&str> = l.bar.menus.iter().map(|m| m.title.as_str()).collect();
    assert_eq!(titles, ["Git Good", "Repository", "View", "Window", "Help"]);
    assert_eq!(l.bar.menus[0].role, MenuRole::App);
}

#[test]
fn clashes_with_system_keys_are_resolved_in_ggs_favour() {
    let mut menus = table(0, true, true);
    menus[1]
        .entries
        .push(item("Hidden files", "Ctrl+H", Cmd::Inline).entry());
    menus[1]
        .entries
        .push(item("Merge", "Ctrl+M", Cmd::Fetch).entry());
    let l = layout(&menus);
    let clash = |s: &str| {
        l.clashes
            .iter()
            .find(|c| c.shortcut == s)
            .map(|c| c.command.as_str())
    };
    assert_eq!(clash("Cmd+H"), Some("Hidden files"));
    assert_eq!(clash("Cmd+M"), Some("Merge"));
    assert_eq!(
        l.bar.find(HIDE).and_then(|i| i.shortcut.as_deref()),
        Some("Control+Cmd+H")
    );
    assert_eq!(
        l.bar.find(MINIMIZE).and_then(|i| i.shortcut.as_deref()),
        Some("Control+Cmd+M")
    );
}

#[test]
fn labels_checks_and_enablement_change_in_place_while_lists_rebuild() {
    let key = |recent, inline, fetch| structure_key(&layout(&table(recent, inline, fetch)).bar);
    assert_eq!(
        key(2, true, true),
        key(2, false, false),
        "checks and enablement update in place"
    );
    assert_ne!(
        key(2, true, true),
        key(3, true, true),
        "a new recent repository rebuilds"
    );
    let a = layout(&table(2, true, false)).bar;
    let fetch = a
        .items()
        .into_iter()
        .find(|i| i.label == "Fetch")
        .map(|i| i.enabled);
    assert_eq!(fetch, Some(false));
    let inline = a
        .items()
        .into_iter()
        .find(|i| i.label == "Inline diff")
        .map(|i| i.checked);
    assert_eq!(inline, Some(Some(true)));
}

#[test]
fn native_key_equivalents_are_command_and_function_keys_only() {
    let ok = |s: &str| Chord::parse(s).is_some_and(|c| c.native_ok());
    assert!(ok("Ctrl+B") && ok("Cmd+,") && ok("F2") && ok("Ctrl+Shift+Z"));
    assert!(!ok("R") && !ok("Space") && !ok("Del") && !ok("?") && !ok("Enter"));
    assert!(!ok("Ctrl+C") && !ok("Ctrl+V") && !ok("Ctrl+X") && !ok("Ctrl+A") && !ok("Ctrl+Z"));
    assert_eq!(
        Chord::parse("Ctrl+Shift+F")
            .map(|c| c.portable())
            .as_deref(),
        Some("Cmd+Shift+F")
    );
    assert_eq!(
        Chord::parse("Control+Cmd+M")
            .map(|c| c.portable())
            .as_deref(),
        Some("Control+Cmd+M")
    );
    assert_eq!(Chord::parse("Cmd++").map(|c| c.key).as_deref(), Some("+"));
    assert_eq!(Chord::parse("Ctrl+Shift"), None);
    assert_eq!(Chord::parse("Ctrl+A+B"), None);
}

#[test]
fn gg_shortcuts_become_the_key_presses_egui_sees() {
    let press = |s: &str| match Chord::parse(s).and_then(|c| c.key_event(true)) {
        Some(egui::Event::Key { key, modifiers, .. }) => Some((key, modifiers)),
        _ => None,
    };
    let command_shift = Modifiers::COMMAND | Modifiers::SHIFT | Modifiers::MAC_CMD;
    assert_eq!(press("Ctrl+Shift+F"), Some((Key::F, command_shift)));
    assert_eq!(
        press("Ctrl+,"),
        Some((Key::Comma, Modifiers::COMMAND | Modifiers::MAC_CMD))
    );
    assert_eq!(
        press("Ctrl+\\"),
        Some((Key::Backslash, Modifiers::COMMAND | Modifiers::MAC_CMD))
    );
    assert_eq!(
        press("Ctrl+1"),
        Some((Key::Num1, Modifiers::COMMAND | Modifiers::MAC_CMD))
    );
    assert_eq!(
        press("Ctrl+Enter"),
        Some((Key::Enter, Modifiers::COMMAND | Modifiers::MAC_CMD))
    );
    assert_eq!(press("F2"), Some((Key::F2, Modifiers::NONE)));
    assert_eq!(press("Del"), Some((Key::Delete, Modifiers::NONE)));
    assert_eq!(press("?"), Some((Key::Questionmark, Modifiers::NONE)));
    // A key press consumed with gg's COMMAND matches on every platform.
    let Some((key, modifiers)) = press("Ctrl+Shift+F") else {
        unreachable!("parsed above");
    };
    assert!(
        modifiers.matches_logically(Modifiers::COMMAND | Modifiers::SHIFT),
        "{key:?}"
    );
}

/// A backend that records what it was given and reports scripted events.
#[derive(Default)]
struct Fake {
    synced: Vec<MenuBar>,
    events: Vec<Event>,
}

struct Shared(Rc<RefCell<Fake>>);

impl Backend for Shared {
    fn sync(&mut self, bar: &MenuBar) {
        self.0.borrow_mut().synced.push(bar.clone());
    }
    fn drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.0.borrow_mut().events)
    }
}

#[test]
fn key_equivalents_reach_egui_as_key_presses_and_clicks_run_commands() {
    let fake = Rc::new(RefCell::new(Fake::default()));
    let mut menu = NativeMenu::new(Box::new(Shared(Rc::clone(&fake))));
    menu.sync(&table(1, true, true), role);
    menu.sync(&table(1, true, true), role);
    assert_eq!(
        fake.borrow().synced.len(),
        1,
        "an unchanged bar is not sent again"
    );
    menu.sync(&table(1, false, true), role);
    assert_eq!(fake.borrow().synced.len(), 2, "a check changed");

    let fetch = fake.borrow().synced[1]
        .items()
        .into_iter()
        .find(|i| i.label == "Fetch")
        .map(|i| i.id.clone());
    let Some(fetch) = fetch else {
        unreachable!("Fetch is in the bar");
    };
    let shortcut = Chord::parse("Ctrl+Shift+F");
    let Some(shortcut) = shortcut else {
        unreachable!("parses");
    };
    fake.borrow_mut().events = vec![Event::Key(shortcut), Event::Click(fetch.clone())];
    let release = egui::Event::Key {
        key: Key::F,
        physical_key: Some(Key::F),
        pressed: false,
        repeat: false,
        modifiers: Modifiers::COMMAND | Modifiers::SHIFT,
    };
    let mut raw = egui::RawInput {
        events: vec![release.clone()],
        ..Default::default()
    };
    menu.raw_input(&mut raw);
    assert!(
        matches!(&raw.events[..], [egui::Event::Key { key: Key::F, pressed: true, .. }, r] if *r == release),
        "the press goes ahead of AppKit's key-up: {:?}",
        raw.events
    );
    let ctx = egui::Context::default();
    assert_eq!(menu.chosen(&ctx), [Cmd::Fetch]);
    assert!(menu.chosen(&ctx).is_empty(), "each click runs once");
    // A click on an item that became disabled is dropped.
    menu.sync(&table(1, false, false), role);
    fake.borrow_mut().events = vec![Event::Click(fetch)];
    menu.raw_input(&mut egui::RawInput::default());
    assert!(menu.chosen(&ctx).is_empty());
}
