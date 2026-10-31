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

/// Menu row height, as in the design's menus.
pub const ROW_HEIGHT: f32 = 24.0;
const CHECK_WIDTH: f32 = 14.0;

/// A menu row as the design draws it: a check column, then the label. A
/// checked row shows a checkmark in the column; no row gets a selected fill.
pub fn row<'a>(checked: bool, text: impl egui::IntoAtoms<'a>) -> egui::Button<'a> {
    let check = egui::Atom::paint(egui::vec2(CHECK_WIDTH, CHECK_WIDTH), move |ui, args| {
        if checked {
            let r = args.rect.shrink(2.0);
            let points = vec![
                egui::pos2(r.left(), r.center().y),
                egui::pos2(r.left() + r.width() * 0.38, r.bottom() - r.height() * 0.15),
                egui::pos2(r.right(), r.top() + r.height() * 0.15),
            ];
            let color = ui.visuals().hyperlink_color;
            ui.painter()
                .add(egui::Shape::line(points, egui::Stroke::new(1.6, color)));
        }
    });
    egui::Button::new((check, text))
        .frame_when_inactive(false)
        .gap(8.0)
        .min_size(egui::vec2(0.0, ROW_HEIGHT))
}

/// One choice in a dropdown: a menu row that reports its checked state.
pub fn choice(
    ui: &mut egui::Ui,
    checked: bool,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    let text = text.into();
    let label = text.text().to_owned();
    let response = ui.add(row(checked, text));
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::accesskit::Role::MenuItemRadio,
            ui.is_enabled(),
            checked,
            &label,
        )
    });
    response
}

/// [`choice`] bound to a value, like `selectable_value`.
pub fn choose<V: PartialEq>(
    ui: &mut egui::Ui,
    current: &mut V,
    value: V,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    let mut response = choice(ui, *current == value, text);
    if response.clicked() && *current != value {
        *current = value;
        response.mark_changed();
    }
    response
}

/// Draw a list of entries (a menu's body, or a popup that mirrors a menu).
pub fn entries<C: Clone>(ui: &mut egui::Ui, list: &[Entry<C>], chosen: &mut Option<C>) {
    ui.set_min_width(240.0);
    for entry in list {
        match entry {
            Entry::Separator => {
                ui.separator();
            }
            Entry::Submenu(title, list) => {
                use egui::containers::menu::SubMenuButton;
                let button = row(false, title.as_str()).right_text(SubMenuButton::arrow_atom(None));
                SubMenuButton::from_button(button).ui(ui, |ui| entries(ui, list, chosen));
            }
            Entry::Item(item) => {
                let text = if item.danger {
                    egui::RichText::new(&item.label).color(ui.visuals().error_fg_color)
                } else {
                    egui::RichText::new(&item.label)
                };
                let button = row(item.checked == Some(true), text).shortcut_text(item.key);
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
    fn checked_rows_show_a_checkmark_and_no_selected_fill() {
        let ctx = egui::Context::default();
        let mut fill = None;
        let mut current = 1;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            fill = Some(ui.visuals().selection.bg_fill);
            let list = vec![
                item("Inline diff", "", 1).checked(true).entry(),
                item("Side-by-side diff", "", 2).checked(false).entry(),
            ];
            entries(ui, &list, &mut None);
            choose(ui, &mut current, 1, "Compact");
        });
        output.textures_delta.clear();
        let Some(fill) = fill else {
            unreachable!("the frame ran");
        };
        let mut rects = 0;
        let mut lines = 0;
        for shape in output.shapes.iter().map(|clipped| &clipped.shape) {
            match shape {
                egui::Shape::Rect(rect) if rect.fill == fill => rects += 1,
                egui::Shape::Path(path) if path.points.len() == 3 => lines += 1,
                _ => {}
            }
        }
        assert_eq!(rects, 0, "no row is filled as selected");
        assert_eq!(lines, 2, "one checkmark per checked row");
    }

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
