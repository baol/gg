//! The window commands' keys: one table that the menus label, dispatch
//! reads and Help lists, with the user's own keys from Preferences ›
//! Keyboard over the defaults. The navigation keys (Ctrl+1 and Ctrl+2,
//! Ctrl+K, Ctrl+F, Ctrl+\, Space, the list keys, F6, F10) are fixed and
//! cannot be taken. Source: photocraft `crates/ui-egui/src/input/shortcuts.rs`.

use eframe::egui::{self, Key, Modifiers};
use std::collections::BTreeMap;

/// A command whose key can be changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    Open,
    Quit,
    Fetch,
    Pull,
    Push,
    NewBranch,
    SwitchTo,
    Rename,
    Delete,
    Reword,
    StageAll,
    UnstageAll,
    Stash,
    Back,
    Forward,
    Shortcuts,
}

impl Action {
    pub const ALL: [Self; 16] = [
        Self::Open,
        Self::Quit,
        Self::Fetch,
        Self::Pull,
        Self::Push,
        Self::NewBranch,
        Self::SwitchTo,
        Self::Rename,
        Self::Delete,
        Self::Reword,
        Self::StageAll,
        Self::UnstageAll,
        Self::Stash,
        Self::Back,
        Self::Forward,
        Self::Shortcuts,
    ];

    /// The name the settings file keeps the key under.
    pub fn id(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Quit => "quit",
            Self::Fetch => "fetch",
            Self::Pull => "pull",
            Self::Push => "push",
            Self::NewBranch => "new_branch",
            Self::SwitchTo => "switch_to",
            Self::Rename => "rename_branch",
            Self::Delete => "delete_branch",
            Self::Reword => "reword",
            Self::StageAll => "stage_all",
            Self::UnstageAll => "unstage_all",
            Self::Stash => "stash",
            Self::Back => "back",
            Self::Forward => "forward",
            Self::Shortcuts => "shortcuts",
        }
    }

    /// Where the command lives, as Preferences › Keyboard lists it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "Repository › Open…",
            Self::Quit => "Repository › Quit",
            Self::Fetch => "Repository › Fetch",
            Self::Pull => "Repository › Pull",
            Self::Push => "Repository › Push",
            Self::NewBranch => "Branch › New branch…",
            Self::SwitchTo => "Branch › Switch to…",
            Self::Rename => "Branch › Rename…",
            Self::Delete => "Branch › Delete…",
            Self::Reword => "Commit › Reword…",
            Self::StageAll => "Changes › Stage all",
            Self::UnstageAll => "Changes › Unstage all",
            Self::Stash => "Changes › Stash changes…",
            Self::Back => "View › Back",
            Self::Forward => "View › Forward",
            Self::Shortcuts => "Help › Keyboard shortcuts",
        }
    }

    pub fn default_chord(self) -> Chord {
        let ctrl = |key| Chord::new(true, false, key);
        let shift = |key| Chord {
            shift: true,
            ..Chord::new(true, false, key)
        };
        let plain = |key| Chord::new(false, false, key);
        match self {
            Self::Open => ctrl(Key::O),
            Self::Quit => ctrl(Key::Q),
            Self::Fetch => shift(Key::F),
            Self::Pull => shift(Key::L),
            Self::Push => shift(Key::P),
            Self::NewBranch => ctrl(Key::B),
            Self::SwitchTo => shift(Key::B),
            Self::Rename => plain(Key::F2),
            Self::Delete => plain(Key::Delete),
            Self::Reword => ctrl(Key::R),
            Self::StageAll => shift(Key::S),
            Self::UnstageAll => shift(Key::U),
            Self::Stash => shift(Key::Z),
            Self::Back => ctrl(Key::OpenBracket),
            Self::Forward => ctrl(Key::CloseBracket),
            Self::Shortcuts => plain(Key::Questionmark),
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.id() == id)
    }
}

/// A key with its modifiers. Ctrl stands for Command on macOS, as in the
/// menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    pub command: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: Key,
}

impl Chord {
    const fn new(command: bool, alt: bool, key: Key) -> Self {
        Self {
            command,
            shift: false,
            alt,
            key,
        }
    }

    /// Read `Ctrl+Shift+F`, `F2`, `Del`, `?`, `Ctrl+[`.
    pub fn parse(text: &str) -> Option<Self> {
        let mut chord = Self::new(false, false, Key::A);
        let mut key = None;
        // `Ctrl++` would name the plus key; no default uses it.
        for part in text.split('+') {
            match part {
                "Ctrl" | "Cmd" => chord.command = true,
                "Shift" => chord.shift = true,
                "Alt" | "Option" => chord.alt = true,
                "Del" => key = Some(Key::Delete),
                name => key = Some(Key::from_name(name)?),
            }
        }
        chord.key = key?;
        Some(chord)
    }

    /// As the menus show it: `Ctrl+Shift+F`, `F2`, `Del`.
    pub fn label(&self) -> String {
        let mut label = String::new();
        if self.command {
            label.push_str("Ctrl+");
        }
        if self.alt {
            label.push_str("Alt+");
        }
        if self.shift {
            label.push_str("Shift+");
        }
        label.push_str(match self.key {
            Key::Delete => "Del",
            key => key.symbol_or_name(),
        });
        label
    }

    pub fn modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::NONE;
        if self.command {
            modifiers |= Modifiers::COMMAND;
        }
        if self.shift {
            modifiers |= Modifiers::SHIFT;
        }
        if self.alt {
            modifiers |= Modifiers::ALT;
        }
        modifiers
    }

    /// A chord without Ctrl or Alt: it acts only while no field is typed in.
    pub fn plain(&self) -> bool {
        !self.command && !self.alt
    }

    /// The chord of a key press, with its modifiers.
    pub fn of(key: Key, modifiers: Modifiers) -> Self {
        Self {
            command: modifiers.command || modifiers.ctrl || modifiers.mac_cmd,
            shift: modifiers.shift,
            alt: modifiers.alt,
            key,
        }
    }

    /// Whether this chord was pressed in `events`, modifiers exactly. A
    /// symbol typed with Shift on most layouts (`?`, `+`, `!`) matches with
    /// or without it.
    pub fn pressed_in(&self, events: &[egui::Event]) -> bool {
        events.iter().any(|event| {
            let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            else {
                return false;
            };
            let mut pressed = Self::of(*key, *modifiers);
            if !self.shift && shifted_symbol(*key) {
                pressed.shift = false;
            }
            pressed == *self
        })
    }
}

/// Symbols most layouts type with Shift.
fn shifted_symbol(key: Key) -> bool {
    matches!(
        key,
        Key::Questionmark
            | Key::Exclamationmark
            | Key::Plus
            | Key::Colon
            | Key::Pipe
            | Key::OpenCurlyBracket
            | Key::CloseCurlyBracket
    )
}

/// Keys the window keeps for itself, and what for.
const FIXED: &[(&str, &str)] = &[
    ("Ctrl+1", "View › Local Changes"),
    ("Ctrl+2", "View › History tree"),
    ("Ctrl+K", "Search"),
    ("Ctrl+F", "Find"),
    ("Ctrl+\\", "View › Refs sidebar"),
    ("Ctrl+,", "Preferences"),
    ("Ctrl+C", "Copy"),
    ("Ctrl+V", "Paste"),
    ("Ctrl+X", "Cut"),
    ("Ctrl+A", "Select all"),
    ("Ctrl+Z", "Undo typing"),
    ("Ctrl+Enter", "Run the dialog"),
    ("Ctrl+S", "Stage the resolution"),
    ("Alt+A", "Commit actions"),
    ("Shift+F10", "Context menu"),
    ("F6", "Next region"),
    ("Shift+F6", "Previous region"),
    ("F10", "Menu bar"),
];

/// Keys that can stand alone (no Ctrl or Alt): the others are list keys.
fn may_stand_alone(key: Key) -> bool {
    matches!(
        key,
        Key::F1
            | Key::F2
            | Key::F3
            | Key::F4
            | Key::F5
            | Key::F7
            | Key::F8
            | Key::F9
            | Key::F11
            | Key::F12
            | Key::Delete
            | Key::Insert
            | Key::Questionmark
    )
}

/// The keys in use: the defaults with the user's own over them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keymap {
    chords: BTreeMap<Action, Chord>,
}

impl Default for Keymap {
    fn default() -> Self {
        Self::from_saved(&BTreeMap::new())
    }
}

impl Keymap {
    /// The defaults with `saved` (action id → chord) over them; an entry
    /// that does not read, or clashes, keeps the default.
    pub fn from_saved(saved: &BTreeMap<String, String>) -> Self {
        let mut keymap = Self {
            chords: Action::ALL
                .into_iter()
                .map(|action| (action, action.default_chord()))
                .collect(),
        };
        for (id, text) in saved {
            if let (Some(action), Some(chord)) = (Action::from_id(id), Chord::parse(text)) {
                let _ = keymap.bind(action, chord);
            }
        }
        keymap
    }

    pub fn chord(&self, action: Action) -> Chord {
        self.chords
            .get(&action)
            .copied()
            .unwrap_or_else(|| action.default_chord())
    }

    /// The menu label of `action`'s key.
    pub fn label(&self, action: Action) -> String {
        self.chord(action).label()
    }

    /// Give `action` the key `chord`, unless the window keeps it, another
    /// command has it, or it is a list key on its own.
    pub fn bind(&mut self, action: Action, chord: Chord) -> Result<(), String> {
        let label = chord.label();
        if let Some((_, what)) = FIXED.iter().find(|(fixed, _)| *fixed == label) {
            return Err(format!("{label} is kept for {what}."));
        }
        if chord.plain() && !may_stand_alone(chord.key) {
            return Err(format!(
                "{label} on its own is a list key: add Ctrl or Alt, or choose a function key."
            ));
        }
        if let Some((other, _)) = self
            .chords
            .iter()
            .find(|(other, bound)| **other != action && **bound == chord)
        {
            return Err(format!("{label} is already {}.", other.label()));
        }
        self.chords.insert(action, chord);
        Ok(())
    }

    pub fn reset(&mut self, action: Action) {
        self.chords.insert(action, action.default_chord());
    }

    /// The keys that differ from the defaults, to save.
    pub fn saved(&self) -> BTreeMap<String, String> {
        self.chords
            .iter()
            .filter(|(action, chord)| action.default_chord() != **chord)
            .map(|(action, chord)| (action.id().to_owned(), chord.label()))
            .collect()
    }

    /// The command whose key was pressed in `events`, if any: exact
    /// modifiers, so Ctrl+F never runs a command bound to Ctrl+Shift+F.
    /// Plain keys count only when `typing` is false.
    pub fn pressed(&self, events: &[egui::Event], typing: bool) -> Option<Action> {
        Action::ALL.into_iter().find(|action| {
            let chord = self.chord(*action);
            (!chord.plain() || !typing) && chord.pressed_in(events)
        })
    }
}

thread_local! {
    /// Whether key labels are drawn the macOS way (`⇧⌘S`) instead of
    /// `Ctrl+Shift+S`. The UI draws on one thread; a test sets its own.
    static MAC_KEYS: std::cell::Cell<bool> = const { std::cell::Cell::new(cfg!(target_os = "macos")) };
}

/// Tests: draw key labels as on macOS, or as elsewhere, on this thread.
#[cfg(test)]
pub fn set_mac_keys(mac: bool) {
    MAC_KEYS.with(|cell| cell.set(mac));
}

/// A label or hint with keys in it, as this platform names them. Labels are
/// written `Ctrl+Shift+S`, the form the menus and settings parse; on macOS,
/// where egui's Command modifier is ⌘, they are drawn `⇧⌘S`.
pub fn shown(text: &str) -> String {
    if MAC_KEYS.with(std::cell::Cell::get) {
        mac_style(text)
    } else {
        text.to_owned()
    }
}

/// `Ctrl+Shift+Enter` → `⇧⌘↩`, `Alt+Up/Down` → `⌥Up/Down`; other text stays.
/// Modifiers come in Apple's order: ⌃ ⌥ ⇧ ⌘.
pub fn mac_style(text: &str) -> String {
    const MODIFIERS: [(&str, usize); 5] = [
        ("Ctrl+", 3),
        ("Cmd+", 3),
        ("Alt+", 1),
        ("Option+", 1),
        ("Shift+", 2),
    ];
    const SYMBOLS: [&str; 4] = ["⌃", "⌥", "⇧", "⌘"];
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        // A run of modifiers starts a chord only at a word's start.
        let at_word_start = out.chars().next_back().is_none_or(|c| !c.is_alphanumeric());
        let mut held = [false; 4];
        let mut chord = rest;
        if at_word_start {
            while let Some((prefix, slot)) = MODIFIERS
                .iter()
                .find(|(prefix, _)| chord.starts_with(prefix))
            {
                held[*slot] = true;
                chord = &chord[prefix.len()..];
            }
        }
        if chord.len() < rest.len() && !chord.is_empty() {
            for (symbol, on) in SYMBOLS.iter().zip(held) {
                if on {
                    out.push_str(symbol);
                }
            }
            let enter = chord.starts_with("Enter")
                && !chord[5..].starts_with(|c: char| c.is_alphanumeric());
            if enter {
                out.push('↩');
                rest = &chord[5..];
            } else {
                rest = chord;
            }
            continue;
        }
        let mut chars = rest.chars();
        if let Some(c) = chars.next() {
            out.push(c);
        }
        rest = chars.as_str();
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn mac_labels_use_the_command_symbols() {
        assert_eq!(mac_style("Ctrl+Enter"), "⌘↩");
        assert_eq!(mac_style("Ctrl+Shift+S"), "⇧⌘S");
        assert_eq!(mac_style("Ctrl+Alt+G"), "⌥⌘G");
        assert_eq!(mac_style("Alt+A  Shift+F10"), "⌥A  ⇧F10");
        assert_eq!(
            mac_style("S / U: stage / unstage · Ctrl+Enter: commit"),
            "S / U: stage / unstage · ⌘↩: commit"
        );
        assert_eq!(mac_style("Alt+Up/Down: file"), "⌥Up/Down: file");
        assert_eq!(mac_style("Enter: read file"), "Enter: read file");
        assert_eq!(mac_style("Copy ID · Ctrl+C"), "Copy ID · ⌘C");
        assert_eq!(mac_style("Ctrl+Enterprise"), "⌘Enterprise");
    }

    #[test]
    fn labels_follow_the_platform_this_thread_is_set_to() {
        set_mac_keys(false);
        assert_eq!(shown("Ctrl+Enter"), "Ctrl+Enter");
        set_mac_keys(true);
        assert_eq!(shown("Ctrl+Enter"), "⌘↩");
    }

    use super::*;

    fn press(key: Key, modifiers: Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn chords_read_and_write_as_the_menus_show_them() {
        for label in [
            "Ctrl+Shift+F",
            "Ctrl+O",
            "F2",
            "Del",
            "?",
            "Ctrl+[",
            "Ctrl+]",
            "Ctrl+Alt+G",
        ] {
            assert_eq!(
                Chord::parse(label).map(|chord| chord.label()).as_deref(),
                Some(label)
            );
        }
        assert_eq!(Chord::parse("Ctrl+Nope"), None);
        assert_eq!(Chord::parse("Ctrl"), None);
        // Every default writes back as the menu label it replaces.
        for action in Action::ALL {
            let chord = action.default_chord();
            assert_eq!(Chord::parse(&chord.label()), Some(chord), "{action:?}");
        }
    }

    #[test]
    fn a_key_is_refused_when_fixed_taken_or_a_bare_list_key() {
        let mut keymap = Keymap::default();
        let chord = |text| Chord::parse(text).unwrap_or(Chord::new(false, false, Key::A));
        assert_eq!(
            keymap.bind(Action::Fetch, chord("Ctrl+K")),
            Err("Ctrl+K is kept for Search.".into())
        );
        assert_eq!(
            keymap.bind(Action::Fetch, chord("Ctrl+Shift+P")),
            Err("Ctrl+Shift+P is already Repository › Push.".into())
        );
        assert!(
            keymap.bind(Action::Fetch, chord("J")).is_err(),
            "a list key"
        );
        assert_eq!(keymap.bind(Action::Fetch, chord("Ctrl+Shift+G")), Ok(()));
        assert_eq!(keymap.label(Action::Fetch), "Ctrl+Shift+G");
        // Its old key is free again.
        assert_eq!(keymap.bind(Action::Pull, chord("Ctrl+Shift+F")), Ok(()));
        assert_eq!(
            keymap.saved(),
            BTreeMap::from([
                ("fetch".to_owned(), "Ctrl+Shift+G".to_owned()),
                ("pull".to_owned(), "Ctrl+Shift+F".to_owned()),
            ])
        );
        keymap.reset(Action::Pull);
        assert_eq!(keymap.label(Action::Pull), "Ctrl+Shift+L");
        // Saved keys come back; a broken or clashing entry keeps the default.
        let loaded = Keymap::from_saved(&BTreeMap::from([
            ("fetch".to_owned(), "Ctrl+Shift+G".to_owned()),
            ("push".to_owned(), "Ctrl+Shift+G".to_owned()),
            ("quit".to_owned(), "nonsense".to_owned()),
            ("gone".to_owned(), "Ctrl+J".to_owned()),
        ]));
        assert_eq!(loaded.label(Action::Fetch), "Ctrl+Shift+G");
        assert_eq!(loaded.label(Action::Push), "Ctrl+Shift+P");
        assert_eq!(loaded.label(Action::Quit), "Ctrl+Q");
    }

    #[test]
    fn a_press_runs_only_the_chord_with_exactly_its_modifiers() {
        let keymap = Keymap::default();
        let shift = Modifiers::COMMAND | Modifiers::SHIFT;
        assert_eq!(
            keymap.pressed(&[press(Key::F, shift)], false),
            Some(Action::Fetch)
        );
        assert_eq!(
            keymap.pressed(&[press(Key::F, Modifiers::COMMAND)], false),
            None,
            "Ctrl+F is Find"
        );
        assert_eq!(
            keymap.pressed(&[press(Key::B, Modifiers::COMMAND)], false),
            Some(Action::NewBranch)
        );
        assert_eq!(
            keymap.pressed(&[press(Key::B, shift)], false),
            Some(Action::SwitchTo)
        );
        assert_eq!(
            keymap.pressed(&[press(Key::F2, Modifiers::NONE)], false),
            Some(Action::Rename)
        );
        assert_eq!(
            keymap.pressed(&[press(Key::F2, Modifiers::NONE)], true),
            None,
            "typing"
        );
        assert_eq!(
            keymap.pressed(&[press(Key::Questionmark, Modifiers::SHIFT)], false),
            Some(Action::Shortcuts),
            "? is typed with Shift"
        );
        assert_eq!(
            keymap.pressed(&[press(Key::O, Modifiers::COMMAND)], true),
            Some(Action::Open),
            "a Ctrl chord works while typing"
        );
    }
}
