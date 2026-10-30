//! The window's menus as data: one table of commands that the menu bar
//! renders and search lists, so every command has one menu home with one
//! label, shortcut, and disabled reason.

use eframe::egui;

#[derive(Clone, Debug)]
pub struct Item<C> {
    pub label: String,
    /// Shortcut shown at the right edge, if any.
    pub key: &'static str,
    /// Why the item cannot run now; the item stays in place, disabled.
    pub disabled: Option<String>,
    /// Tooltip for an enabled item.
    pub hint: &'static str,
    /// Destructive items are drawn in the error color.
    pub danger: bool,
    /// `Some` for a toggle or a choice: drawn selected when true.
    pub checked: Option<bool>,
    pub command: C,
}

#[derive(Clone, Debug)]
pub enum Entry<C> {
    Item(Item<C>),
    Separator,
    Submenu(String, Vec<Entry<C>>),
}

#[derive(Clone, Debug)]
pub struct Menu<C> {
    pub title: &'static str,
    pub entries: Vec<Entry<C>>,
}

/// An enabled item.
pub fn item<C>(label: impl Into<String>, key: &'static str, command: C) -> Item<C> {
    Item {
        label: label.into(),
        key,
        disabled: None,
        hint: "",
        danger: false,
        checked: None,
        command,
    }
}

impl<C> Item<C> {
    /// Disable with `reason` when it is `Some`.
    pub fn unless(mut self, reason: Option<impl Into<String>>) -> Self {
        self.disabled = reason.map(Into::into);
        self
    }

    pub fn hint(mut self, hint: &'static str) -> Self {
        self.hint = hint;
        self
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn entry(self) -> Entry<C> {
        Entry::Item(self)
    }
}

/// Every item with the titles of the menus and submenus it sits in.
pub fn flatten<C: Clone>(menus: &[Menu<C>]) -> Vec<(String, Item<C>)> {
    fn walk<C: Clone>(path: &str, entries: &[Entry<C>], out: &mut Vec<(String, Item<C>)>) {
        for entry in entries {
            match entry {
                Entry::Item(item) => out.push((path.to_string(), item.clone())),
                Entry::Separator => {}
                Entry::Submenu(title, entries) => walk(&format!("{path} › {title}"), entries, out),
            }
        }
    }
    let mut out = Vec::new();
    for menu in menus {
        walk(menu.title, &menu.entries, &mut out);
    }
    out
}

/// Draw the menus in a menu bar; returns the command chosen this frame.
pub fn show<C: Clone>(ui: &mut egui::Ui, menus: &[Menu<C>]) -> Option<C> {
    let mut chosen = None;
    for menu in menus {
        ui.menu_button(menu.title, |ui| entries(ui, &menu.entries, &mut chosen));
    }
    chosen
}

fn entries<C: Clone>(ui: &mut egui::Ui, list: &[Entry<C>], chosen: &mut Option<C>) {
    ui.set_min_width(240.0);
    for entry in list {
        match entry {
            Entry::Separator => {
                ui.separator();
            }
            Entry::Submenu(title, list) => {
                ui.menu_button(title.as_str(), |ui| entries(ui, list, chosen));
            }
            Entry::Item(item) => {
                let text = if item.danger {
                    egui::RichText::new(&item.label).color(ui.visuals().error_fg_color)
                } else {
                    egui::RichText::new(&item.label)
                };
                let button = match item.checked {
                    Some(on) => egui::Button::selectable(on, text),
                    None => egui::Button::new(text),
                }
                .shortcut_text(item.key);
                let response = ui.add_enabled(item.disabled.is_none(), button);
                let response = match &item.disabled {
                    Some(reason) => response.on_disabled_hover_text(reason),
                    None if !item.hint.is_empty() => response.on_hover_text(item.hint),
                    None => response,
                };
                if response.clicked() {
                    *chosen = Some(item.command.clone());
                    ui.close();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_names_each_item_with_its_menu_path() {
        let menus = vec![Menu {
            title: "View",
            entries: vec![
                item("History", "Ctrl+2", 1).entry(),
                Entry::Separator,
                Entry::Submenu(
                    "Theme".into(),
                    vec![item("Dark", "", 2).checked(true).entry()],
                ),
            ],
        }];
        let flat = flatten(&menus)
            .into_iter()
            .map(|(path, item)| (path, item.label))
            .collect::<Vec<_>>();
        assert_eq!(
            flat,
            [
                ("View".to_string(), "History".to_string()),
                ("View › Theme".to_string(), "Dark".to_string())
            ]
        );
    }
}
