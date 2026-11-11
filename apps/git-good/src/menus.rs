//! The window's menus as data: one table of commands that the menu bar
//! renders and search lists, so every command has one menu home with one
//! label, shortcut, and disabled reason.

use eframe::egui;

#[derive(Clone, Debug)]
pub struct Item<C> {
    pub label: String,
    /// Shortcut shown at the right edge, if any.
    pub key: String,
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
pub fn item<C>(label: impl Into<String>, key: impl Into<String>, command: C) -> Item<C> {
    Item {
        label: label.into(),
        key: key.into(),
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
///
/// The menus behave like native ones ([`crate::menu_nav`]: arrows, Enter,
/// Esc, scrolling, hover intent) and work from the keyboard
/// ([`crate::menu_keys`]: Alt / F10 focus, Alt+letter, mnemonics).
pub fn show<C: Clone>(ui: &mut egui::Ui, menus: &[Menu<C>]) -> Option<C> {
    use crate::{menu_keys, menu_nav};
    let ctx = ui.ctx().clone();
    let mut nav = menu_nav::Nav::load(&ctx);
    let mut keys = menu_keys::Keys::load(&ctx);
    keys.take_first_row(&mut nav);
    nav.underline = keys.show_underlines(&ctx);
    if nav.top.is_some() || keys.focus.is_some() {
        // egui's own arrow-key focus in menus would move a second highlight.
        ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
    }
    nav.bar_bottom = Some(ui.max_rect().bottom());
    let titles: Vec<&str> = menus.iter().map(|m| m.title).collect();
    let mnemonics = menu_keys::top_mnemonics(&titles);
    let mut commands: Vec<C> = Vec::new();
    let mut chosen: Option<usize> = None;
    let mut buttons = Vec::with_capacity(menus.len());
    for (k, menu) in menus.iter().enumerate() {
        let mnemonic = mnemonics.get(k).copied().flatten();
        let label =
            menu_keys::mnemonic_text(ui, menu.title, mnemonic.filter(|_| nav.underline), None);
        let mut button = egui::Button::new(label);
        if keys.focus == Some(k) {
            // The title the keyboard is on (Alt / F10) is lit like a hovered one.
            button = button.fill(ui.visuals().widgets.hovered.weak_bg_fill);
        }
        let title = ui.add(button);
        // The bar's close behaviour and style, as a menu (not bar) config so
        // submenus inside render as submenus.
        let bar = egui::containers::menu::MenuConfig::find(ui);
        let config = egui::containers::menu::MenuConfig::new()
            .close_behavior(bar.close_behavior)
            .style(bar.style.clone());
        egui::Popup::menu(&title)
            .close_behavior(config.close_behavior)
            .style(config.style.clone())
            .info(
                egui::UiStackInfo::new(egui::UiKind::Menu)
                    .with_tag_value(egui::containers::menu::MenuConfig::MENU_CONFIG_TAG, config),
            )
            .show(|ui| {
                ui.set_min_width(240.0);
                menu_nav::level(ui, 1, &mut nav, |ui, nav| {
                    nav_entries(ui, &menu.entries, 1, nav, &mut commands, &mut chosen);
                });
            });
        buttons.push(title);
    }
    switch_on_hover(&ctx, &buttons);
    let tops: Vec<egui::Id> = buttons
        .iter()
        .map(egui::Popup::default_response_id)
        .collect();
    nav.keys(&ctx, &tops, &mut chosen);
    keys.keys(&ctx, &tops, &mnemonics, &mut nav, &mut chosen);
    nav.titles = buttons.iter().map(|b| b.rect).collect();
    keys.store(&ctx);
    nav.store(&ctx);
    chosen.and_then(|i| commands.get(i).cloned())
}

/// Like a native menu bar: while one menu is open, moving the pointer onto
/// another title opens that menu instead. A pointer resting on a title does
/// not undo Left / Right.
fn switch_on_hover(ctx: &egui::Context, buttons: &[egui::Response]) {
    let ids: Vec<egui::Id> = buttons
        .iter()
        .map(egui::Popup::default_response_id)
        .collect();
    let Some(open) = ids.iter().position(|id| egui::Popup::is_id_open(ctx, *id)) else {
        return;
    };
    let Some(p) = ctx.pointer_hover_pos() else {
        return;
    };
    if ctx.input(|i| i.pointer.delta() == egui::Vec2::ZERO) {
        return;
    }
    let reaches =
        |b: &egui::Response| b.interact_rect.contains(p) && ctx.layer_id_at(p) == Some(b.layer_id);
    if let Some(i) = buttons.iter().position(reaches)
        && i != open
        && let Some(&id) = ids.get(i)
    {
        egui::Popup::open_id(ctx, id);
        ctx.request_repaint();
    }
}

/// One level of a menu bar menu: [`entries`] with the keyboard highlight,
/// mnemonics and scrolling of [`crate::menu_nav`]. `depth` is 1 for a
/// top-level menu. A chosen item's index into `commands` lands in `chosen`.
fn nav_entries<C: Clone>(
    ui: &mut egui::Ui,
    list: &[Entry<C>],
    depth: usize,
    nav: &mut crate::menu_nav::Nav,
    commands: &mut Vec<C>,
    chosen: &mut Option<usize>,
) {
    use crate::menu_keys::{assign, mnemonic_text};
    let labels: Vec<&str> = list
        .iter()
        .filter_map(|entry| match entry {
            Entry::Item(item) => Some(item.label.as_str()),
            Entry::Submenu(title, _) => Some(title.as_str()),
            Entry::Separator => None,
        })
        .collect();
    let mut mnemonics = assign(&labels, &[], &[]).into_iter();
    let level = depth - 1;
    for entry in list {
        match entry {
            Entry::Separator => {
                ui.separator();
            }
            Entry::Submenu(title, list) => {
                use egui::containers::menu::SubMenuButton;
                let mnemonic = mnemonics.next().flatten();
                let text = mnemonic_text(ui, title, mnemonic.filter(|_| nav.underline), None);
                nav.row(ui, level, true, None, mnemonic, |ui, nav, lit| {
                    let button = row(false, text)
                        .frame_when_inactive(lit)
                        .right_text(SubMenuButton::arrow_atom(None));
                    let (response, inner) = SubMenuButton::from_button(button).ui(ui, |ui| {
                        crate::menu_nav::level(ui, depth + 1, nav, |ui, nav| {
                            nav_entries(ui, list, depth + 1, nav, commands, chosen);
                        });
                    });
                    if inner.is_some() {
                        // Where the submenu hangs from, to keep it below the bar.
                        nav.set_anchor(depth, response.rect);
                    }
                    (response, ())
                });
            }
            Entry::Item(item) => {
                let mnemonic = mnemonics.next().flatten();
                let index = commands.len();
                commands.push(item.command.clone());
                let enabled = item.disabled.is_none();
                let underline = mnemonic.filter(|_| nav.underline);
                let clicked = nav.row(ui, level, enabled, Some(index), mnemonic, |ui, _, lit| {
                    let response = item_response(ui, item, true, underline, lit);
                    let clicked = response.clicked();
                    (response, clicked)
                });
                if clicked {
                    *chosen = Some(index);
                    ui.close();
                }
            }
        }
    }
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
                if item_row(ui, item, true) {
                    *chosen = Some(item.command.clone());
                    ui.close();
                }
            }
        }
    }
}

/// One item as a menu row; true when it was clicked. A disabled item names
/// its reason on hover unless `reason` is false (a band above says it).
pub fn item_row<C>(ui: &mut egui::Ui, item: &Item<C>, reason: bool) -> bool {
    item_response(ui, item, reason, None, false).clicked()
}

/// [`item_row`]'s response, with `mnemonic` underlined and the row `lit`
/// (highlighted from the keyboard) when asked.
fn item_response<C>(
    ui: &mut egui::Ui,
    item: &Item<C>,
    reason: bool,
    mnemonic: Option<char>,
    lit: bool,
) -> egui::Response {
    let color = (item.danger && item.disabled.is_none()).then(|| ui.visuals().error_fg_color);
    let text = crate::menu_keys::mnemonic_text(ui, &item.label, mnemonic, color);
    let mut button =
        row(item.checked == Some(true), text).shortcut_text(crate::keymap::shown(&item.key));
    if lit {
        button = button.frame_when_inactive(true);
    }
    let response = ui.add_enabled(item.disabled.is_none(), button);
    match &item.disabled {
        Some(why) if reason => response.on_disabled_hover_text(why),
        Some(_) => response,
        None if !item.hint.is_empty() => response.on_hover_text(item.hint),
        None => response,
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
