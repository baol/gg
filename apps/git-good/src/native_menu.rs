//! Adapted from photocraft crates/ui-egui/src/menus/native_menu.rs.
//!
//! The macOS menu bar: the same menus as the in-window bar (gg's one command
//! table, [`crate::menus::Menu`]), laid out the way a Mac app's are and
//! handed to a platform [`Backend`] (the muda one in `mac_menu`).
//!
//! Pure and platform-free, so it is tested everywhere; only the backend
//! touches AppKit. What the layout does ([`mac_layout`]):
//! - an **app menu** named Git Good: About Git Good (from Help), Settings… (Repository ›
//!   Preferences, `Cmd+,`), Services, Hide Git Good, Hide Others, Show All and
//!   Quit Git Good (Git Good's own Quit);
//! - a **Window** menu with Minimize, Zoom and Bring All to Front, and the
//!   **Help** menu gets the system search field;
//! - **clashes** with the system's keys (⌘H, ⌘M, ⌘,, ⌘Q) are resolved in
//!   gg's favour, and reported; Hide and Minimize move to ⌃⌘H and ⌃⌘M.
//!
//! **Keys stay with egui.** AppKit runs a menu's key equivalents before the
//! window sees the key, which would skip gg's key rules (text fields, open
//! menus and dialogs keep their keys). So the backend reports a key
//! equivalent as [`Event::Key`], and [`NativeMenu::raw_input`] feeds it to
//! egui as the key press it was: the shortcut runs through gg's key handling
//! exactly as without a native menu. A click is [`Event::Click`] and runs
//! the command like an in-window menu click.

// Only macOS installs a backend; the layout is built and tested everywhere.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::menus;
use eframe::egui::{self, Key, Modifiers};

pub const APP_NAME: &str = "Git Good";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuBar {
    pub menus: Vec<Menu>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Menu {
    pub title: String,
    pub role: MenuRole,
    pub children: Vec<Node>,
}

/// What a top-level menu is for. macOS treats the app, Window and Help menus specially.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MenuRole {
    #[default]
    Normal,
    App,
    Window,
    Help,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Item(Item),
    Separator,
    Submenu {
        label: String,
        children: Vec<Node>,
    },
    /// An item AppKit provides and runs itself (Services, Zoom, …).
    Standard(Standard),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// Where the command sits in the table (`2.5`), or [`HIDE`] / [`MINIMIZE`].
    pub id: String,
    pub label: String,
    /// gg's written form (`Ctrl+Shift+F`, `F2`, `Del`).
    pub shortcut: Option<String>,
    pub enabled: bool,
    /// `Some` for check items.
    pub checked: Option<bool>,
    pub role: Option<ItemRole>,
}

/// Items macOS expects in fixed places.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemRole {
    About,
    Settings,
    Quit,
    Hide,
    Minimize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Standard {
    Services,
    HideOthers,
    ShowAll,
    Minimize,
    Zoom,
    BringAllToFront,
}

/// Hide Git Good: a custom item, as the predefined one always takes ⌘H.
pub const HIDE: &str = "app.hide";
/// Minimize: a custom item, used when gg binds ⌘M itself.
pub const MINIMIZE: &str = "window.minimize";

impl Item {
    fn new(id: &str, label: &str) -> Item {
        Item {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            enabled: true,
            checked: None,
            role: None,
        }
    }
}

impl MenuBar {
    /// Every item, depth first.
    pub fn items(&self) -> Vec<&Item> {
        fn walk<'a>(nodes: &'a [Node], out: &mut Vec<&'a Item>) {
            for node in nodes {
                match node {
                    Node::Item(item) => out.push(item),
                    Node::Submenu { children, .. } => walk(children, out),
                    Node::Separator | Node::Standard(_) => {}
                }
            }
        }
        let mut out = Vec::new();
        for menu in &self.menus {
            walk(&menu.children, &mut out);
        }
        out
    }

    pub fn find(&self, id: &str) -> Option<&Item> {
        self.items().into_iter().find(|item| item.id == id)
    }

    #[cfg(test)]
    pub fn menu(&self, role: MenuRole) -> Option<&Menu> {
        self.menus.iter().find(|menu| menu.role == role)
    }
}

/// gg's table as a native menu tree, and each item id's command. Ids are the
/// item's position in the table (`menu.entry[.entry…]`), so they stay put
/// while only labels, enablement or checks change. `role` names the items
/// macOS moves to the app menu.
pub fn from_table<C: Clone>(
    table: &[menus::Menu<C>],
    role: impl Fn(&C) -> Option<ItemRole>,
) -> (MenuBar, HashMap<String, C>) {
    fn level<C: Clone>(
        entries: &[menus::Entry<C>],
        path: &str,
        role: &dyn Fn(&C) -> Option<ItemRole>,
        commands: &mut HashMap<String, C>,
    ) -> Vec<Node> {
        let nodes = entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let id = format!("{path}.{i}");
                match entry {
                    menus::Entry::Separator => Node::Separator,
                    menus::Entry::Submenu(label, children) => Node::Submenu {
                        label: label.clone(),
                        children: level(children, &id, role, commands),
                    },
                    menus::Entry::Item(item) => {
                        commands.insert(id.clone(), item.command.clone());
                        Node::Item(Item {
                            id,
                            label: item.label.clone(),
                            shortcut: (!item.key.is_empty()).then(|| item.key.to_string()),
                            enabled: item.disabled.is_none(),
                            checked: item.checked,
                            role: role(&item.command),
                        })
                    }
                }
            })
            .collect();
        tidy(nodes)
    }
    let mut commands = HashMap::new();
    let menus = table
        .iter()
        .enumerate()
        .map(|(i, menu)| Menu {
            title: menu.title.to_string(),
            role: if menu.title == "Help" {
                MenuRole::Help
            } else {
                MenuRole::Normal
            },
            children: level(&menu.entries, &i.to_string(), &role, &mut commands),
        })
        .collect();
    (MenuBar { menus }, commands)
}

/// Drop separators at the start or end of a level and collapse runs of them, at every depth.
fn tidy(nodes: Vec<Node>) -> Vec<Node> {
    let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
    for node in nodes {
        let node = match node {
            Node::Submenu { label, children } => Node::Submenu {
                label,
                children: tidy(children),
            },
            other => other,
        };
        if matches!(node, Node::Separator) && matches!(out.last(), None | Some(Node::Separator)) {
            continue;
        }
        out.push(node);
    }
    while matches!(out.last(), Some(Node::Separator)) {
        out.pop();
    }
    out
}

/// A hash of everything a native menu can only change by rebuilding:
/// menus, roles, item ids, kinds (plain or check), shortcuts and submenu
/// labels. Labels, enabled and checked are updated in place.
pub fn structure_key(bar: &MenuBar) -> u64 {
    fn nodes(list: &[Node], h: &mut DefaultHasher) {
        for node in list {
            match node {
                Node::Item(item) => (
                    'i',
                    &item.id,
                    item.checked.is_some(),
                    &item.shortcut,
                    item.role,
                )
                    .hash(h),
                Node::Separator => '-'.hash(h),
                Node::Submenu { label, children } => {
                    ('[', label).hash(h);
                    nodes(children, h);
                    ']'.hash(h);
                }
                Node::Standard(standard) => ('s', standard).hash(h),
            }
        }
    }
    let mut h = DefaultHasher::new();
    for menu in &bar.menus {
        (&menu.title, menu.role).hash(&mut h);
        nodes(&menu.children, &mut h);
    }
    h.finish()
}

// ----------------------------------------------------------------------------- shortcuts

/// A shortcut as gg writes it (`Ctrl+Shift+F`). gg's `Ctrl` is the command
/// key (egui's `COMMAND`): ⌘ on macOS. `Cmd` is the same key spelled the
/// Mac way; `Control` is the real Control key, which only the layout's own
/// items (⌃⌘H, ⌃⌘M) use.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    pub cmd: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    /// The key as written: `Z`, `1`, `,`, `F2`, `Del`, `Enter`, `?`.
    pub key: String,
}

impl Chord {
    pub fn parse(s: &str) -> Option<Chord> {
        let mut chord = Chord {
            cmd: false,
            ctrl: false,
            alt: false,
            shift: false,
            key: String::new(),
        };
        for part in s.split('+') {
            match part {
                "Ctrl" | "Cmd" => chord.cmd = true,
                "Control" => chord.ctrl = true,
                "Alt" => chord.alt = true,
                "Shift" => chord.shift = true,
                // `Cmd++` splits into an empty last part.
                "" => chord.key = "+".into(),
                key if chord.key.is_empty() => chord.key = key.into(),
                _ => return None,
            }
        }
        (!chord.key.is_empty()).then_some(chord)
    }

    /// The written form, modifiers in a fixed order.
    pub fn portable(&self) -> String {
        let mut parts = Vec::new();
        for (on, name) in [
            (self.ctrl, "Control"),
            (self.cmd, "Cmd"),
            (self.alt, "Alt"),
            (self.shift, "Shift"),
        ] {
            if on {
                parts.push(name);
            }
        }
        parts.push(&self.key);
        parts.join("+")
    }

    pub fn is_function_key(&self) -> bool {
        self.key
            .strip_prefix('F')
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=24).contains(&n))
    }

    /// May the native menu show this as a key equivalent? Only with ⌘ or ⌃,
    /// or a function key: bare keys are list keys (`R`, `Space`, `Del`) that
    /// AppKit would take from a text field. ⌘C, ⌘X, ⌘V, ⌘A and ⌘Z stay the
    /// window's: egui turns them into clipboard and undo events a text field
    /// needs, which a key equivalent never reaches.
    pub fn native_ok(&self) -> bool {
        let editing = self.cmd
            && !self.ctrl
            && !self.alt
            && !self.shift
            && matches!(self.key.as_str(), "C" | "X" | "V" | "A" | "Z");
        (self.cmd || self.ctrl || self.is_function_key()) && !editing
    }

    /// The key press egui would have received for this chord.
    pub fn key_event(&self, pressed: bool) -> Option<egui::Event> {
        let key = match self.key.as_str() {
            "+" => Key::Plus,
            "'" => Key::Quote,
            ";" => Key::Semicolon,
            "=" => Key::Equals,
            "-" => Key::Minus,
            "[" => Key::OpenBracket,
            "]" => Key::CloseBracket,
            "\\" => Key::Backslash,
            "/" => Key::Slash,
            "," => Key::Comma,
            "." => Key::Period,
            "?" => Key::Questionmark,
            "Del" => Key::Delete,
            k => Key::from_name(k)?,
        };
        let modifiers = Modifiers {
            alt: self.alt,
            ctrl: self.ctrl,
            shift: self.shift,
            mac_cmd: self.cmd,
            command: self.cmd,
        };
        Some(egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers,
        })
    }
}

// ----------------------------------------------------------------------------- macOS layout

/// A system key equivalent gg already uses for something else.
#[derive(Clone, Debug, PartialEq)]
pub struct Clash {
    pub shortcut: &'static str,
    /// What macOS uses it for.
    pub system: &'static str,
    /// The label of the gg command bound to it.
    pub command: String,
    /// What the layout did about it.
    pub resolution: &'static str,
}

pub struct Layout {
    pub bar: MenuBar,
    pub clashes: Vec<Clash>,
}

/// Turn gg's menus (tagged with roles) into a Mac menu bar. See the module docs.
pub fn mac_layout(bar: &MenuBar) -> Layout {
    let mut bar = bar.clone();
    let mut clashes = Vec::new();
    let bound: HashMap<String, (String, String)> = bar
        .items()
        .iter()
        .filter_map(|item| {
            let chord = Chord::parse(item.shortcut.as_deref()?)?;
            Some((chord.portable(), (item.id.clone(), item.label.clone())))
        })
        .collect();
    let taken = |shortcut: &str, own: Option<&str>| {
        bound
            .get(shortcut)
            .filter(|(id, _)| Some(id.as_str()) != own)
            .map(|(_, label)| label.clone())
    };

    let about = take(&mut bar, ItemRole::About);
    let settings = take(&mut bar, ItemRole::Settings);
    let quit = take(&mut bar, ItemRole::Quit);

    let mut menu = Vec::new();
    if let Some(mut about) = about {
        about.label = format!("About {APP_NAME}");
        menu.extend([Node::Item(about), Node::Separator]);
    }
    if let Some(mut settings) = settings {
        settings.label = "Settings…".into();
        match taken("Cmd+,", Some(&settings.id)) {
            Some(command) => {
                clashes.push(Clash {
                    shortcut: "Cmd+,",
                    system: "Settings",
                    command,
                    resolution: "Settings has no shortcut",
                });
                settings.shortcut = None;
            }
            None => settings.shortcut = Some("Cmd+,".into()),
        }
        menu.extend([Node::Item(settings), Node::Separator]);
    }
    menu.extend([Node::Standard(Standard::Services), Node::Separator]);
    let mut hide = Item::new(HIDE, &format!("Hide {APP_NAME}"));
    hide.role = Some(ItemRole::Hide);
    match taken("Cmd+H", None) {
        Some(command) => {
            let free = taken("Control+Cmd+H", None).is_none();
            if free {
                hide.shortcut = Some("Control+Cmd+H".into());
            }
            clashes.push(Clash {
                shortcut: "Cmd+H",
                system: "Hide",
                command,
                resolution: if free {
                    "Hide moves to ⌃⌘H"
                } else {
                    "Hide has no shortcut"
                },
            });
        }
        None => hide.shortcut = Some("Cmd+H".into()),
    }
    menu.push(Node::Item(hide));
    if let Some(command) = taken("Cmd+Alt+H", None) {
        clashes.push(Clash {
            shortcut: "Cmd+Alt+H",
            system: "Hide Others",
            command,
            resolution: "AppKit's Hide Others keeps ⌥⌘H",
        });
    }
    menu.extend([
        Node::Standard(Standard::HideOthers),
        Node::Standard(Standard::ShowAll),
        Node::Separator,
    ]);
    if let Some(mut quit) = quit {
        quit.label = format!("Quit {APP_NAME}");
        match taken("Cmd+Q", Some(&quit.id)) {
            Some(command) => {
                clashes.push(Clash {
                    shortcut: "Cmd+Q",
                    system: "Quit",
                    command,
                    resolution: "Quit has no shortcut",
                });
                quit.shortcut = None;
            }
            None => quit.shortcut = Some("Cmd+Q".into()),
        }
        menu.push(Node::Item(quit));
    }

    // A menu emptied by the moves goes away (gg's in-window "Git Good" menu on
    // macOS); Help stays, it has the search field.
    for m in &mut bar.menus {
        m.children = tidy(std::mem::take(&mut m.children));
    }
    bar.menus
        .retain(|m| !m.children.is_empty() || m.role == MenuRole::Help);
    bar.menus.insert(
        0,
        Menu {
            title: APP_NAME.into(),
            role: MenuRole::App,
            children: tidy(menu),
        },
    );

    let minimize = match taken("Cmd+M", None) {
        Some(command) => {
            let mut item = Item::new(MINIMIZE, "Minimize");
            item.role = Some(ItemRole::Minimize);
            let free = taken("Control+Cmd+M", None).is_none();
            if free {
                item.shortcut = Some("Control+Cmd+M".into());
            }
            clashes.push(Clash {
                shortcut: "Cmd+M",
                system: "Minimize",
                command,
                resolution: if free {
                    "Minimize moves to ⌃⌘M"
                } else {
                    "Minimize has no shortcut"
                },
            });
            Node::Item(item)
        }
        None => Node::Standard(Standard::Minimize),
    };
    let at = bar
        .menus
        .iter()
        .position(|m| m.role == MenuRole::Help)
        .unwrap_or(bar.menus.len());
    bar.menus.insert(
        at,
        Menu {
            title: "Window".into(),
            role: MenuRole::Window,
            children: vec![
                minimize,
                Node::Standard(Standard::Zoom),
                Node::Separator,
                Node::Standard(Standard::BringAllToFront),
            ],
        },
    );
    Layout { bar, clashes }
}

/// Remove and return the first item with `role`, at any depth.
fn take(bar: &mut MenuBar, role: ItemRole) -> Option<Item> {
    fn walk(nodes: &mut Vec<Node>, role: ItemRole) -> Option<Item> {
        if let Some(i) = nodes
            .iter()
            .position(|n| matches!(n, Node::Item(item) if item.role == Some(role)))
            && let Node::Item(item) = nodes.remove(i)
        {
            return Some(item);
        }
        nodes.iter_mut().find_map(|n| match n {
            Node::Submenu { children, .. } => walk(children, role),
            _ => None,
        })
    }
    bar.menus
        .iter_mut()
        .find_map(|m| walk(&mut m.children, role))
}

// ----------------------------------------------------------------------------- the app's side

/// What the native menu reports.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// An item was chosen with the mouse (or the keyboard inside an open menu).
    Click(String),
    /// A key equivalent was pressed: the shortcut, to be handled as the key press it was.
    Key(Chord),
}

/// A platform's native menu bar (macOS: muda, in `mac_menu`).
pub trait Backend {
    /// Bring the menu in step with `bar`: update labels, enabled and checked
    /// in place, or rebuild when its structure changed.
    fn sync(&mut self, bar: &MenuBar);
    /// Events since the last call. Platform items (Hide, Services, Zoom, …)
    /// are handled by the backend and not reported.
    fn drain(&mut self) -> Vec<Event>;
}

/// The native menu bar, while gg uses one (the in-window menus are hidden then).
pub struct NativeMenu<C> {
    backend: Box<dyn Backend>,
    /// The bar last handed to the backend, and each item id's command.
    shown: Option<MenuBar>,
    commands: HashMap<String, C>,
    clicks: Vec<String>,
    /// AppKit toggled a check item itself, or chose one: re-sync.
    dirty: bool,
    logged: bool,
}

impl<C: Clone> NativeMenu<C> {
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self {
            backend,
            shown: None,
            commands: HashMap::new(),
            clicks: Vec::new(),
            dirty: true,
            logged: false,
        }
    }

    /// Before egui sees this frame's input: key equivalents become the key
    /// presses they were, and clicks wait for [`NativeMenu::chosen`].
    ///
    /// AppKit takes only the key-down: winit still delivers the key-up. So a
    /// key equivalent adds just its press, ahead of its key-up when that is
    /// already queued.
    pub fn raw_input(&mut self, raw: &mut egui::RawInput) {
        for event in self.backend.drain() {
            match event {
                Event::Key(chord) => {
                    if let Some(press @ egui::Event::Key { key: pressed, .. }) =
                        chord.key_event(true)
                    {
                        let at = raw
                            .events
                            .iter()
                            .position(|e| {
                                matches!(e, egui::Event::Key { key, pressed: false, .. } if *key == pressed)
                            })
                            .unwrap_or(raw.events.len());
                        raw.events.insert(at, press);
                    }
                }
                Event::Click(id) => self.clicks.push(id),
            }
            self.dirty = true;
        }
    }

    /// The commands clicked since the last frame, to run like in-window
    /// menu clicks. Minimize (a custom item when gg binds ⌘M) runs here. A
    /// click that raced a change disabling its item is dropped.
    pub fn chosen(&mut self, ctx: &egui::Context) -> Vec<C> {
        let mut out = Vec::new();
        for id in std::mem::take(&mut self.clicks) {
            if id == MINIMIZE {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                continue;
            }
            let enabled = self
                .shown
                .as_ref()
                .and_then(|bar| bar.find(&id))
                .is_some_and(|item| item.enabled);
            if enabled && let Some(command) = self.commands.get(&id) {
                out.push(command.clone());
            }
        }
        out
    }

    /// Bring the native menu in step with this frame's table. The layout is
    /// cheap; the backend only hears of it when something it shows changed.
    pub fn sync(&mut self, table: &[menus::Menu<C>], role: impl Fn(&C) -> Option<ItemRole>) {
        let (bar, commands) = from_table(table, role);
        let layout = mac_layout(&bar);
        if !self.logged {
            for clash in &layout.clashes {
                eprintln!(
                    "menu: {} is {} on macOS and {} in Git Good; {}",
                    clash.shortcut, clash.system, clash.command, clash.resolution
                );
            }
            self.logged = true;
        }
        self.commands = commands;
        if !self.dirty && self.shown.as_ref() == Some(&layout.bar) {
            return;
        }
        self.backend.sync(&layout.bar);
        self.shown = Some(layout.bar);
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests;
