//! Help › Keyboard shortcuts: one filterable key | action table generated
//! from the command table (every menu item with a key, grouped by its menu)
//! plus the list scopes whose single keys act on a focused list. The window
//! is not modal; Esc or × closes it and **Edit keys…** opens Preferences ›
//! Keyboard.
use crate::menus;
use eframe::egui;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub key: String,
    pub action: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub title: String,
    pub rows: Vec<Row>,
}

/// Keys that act on a focused list rather than through a menu item.
const SCOPES: &[(&str, &[(&str, &str)])] = &[
    (
        "Window",
        &[(
            "F6  Shift+F6",
            "Next / previous region: search, sidebar, list",
        )],
    ),
    (
        "History tree",
        &[
            ("j k  ↑ ↓", "Move selection"),
            ("PgUp PgDn", "Page"),
            ("Home End", "First / last commit"),
            ("Ctrl+K", "Search commits, branches, files, actions"),
            ("Ctrl+F", "Filter the History tree"),
            ("Alt+A  Shift+F10", "Commit actions"),
            ("Esc", "Back · clear filter"),
        ],
    ),
    (
        "Local Changes list",
        &[
            ("S  U", "Stage / unstage selection"),
            ("[  ]", "Previous / next hunk"),
        ],
    ),
    (
        "Review",
        &[
            ("↑ ↓", "Files"),
            ("Alt+↑ ↓", "Next / previous file while reading"),
            ("[  ]", "Previous / next change"),
            ("Alt+← →", "Versions"),
            ("B", "Blame"),
            ("H", "File history"),
        ],
    ),
    (
        "Rebase list",
        &[(
            "P R E S F D",
            "Pick · reword · reopen · squash · fixup · drop",
        )],
    ),
    (
        "Conflicts",
        &[
            ("[  ]", "Previous / next conflict"),
            ("1–4", "Ours · theirs · ours + theirs · theirs + ours"),
            ("Ctrl+S", "Stage resolution"),
        ],
    ),
    ("Stashes", &[("Enter", "Apply"), ("Del", "Drop…")]),
];

/// The table: each menu's items that have a key, in menu order, then the
/// list scopes. Menus without keyed items are left out.
pub fn groups<C: Clone>(table: &[menus::Menu<C>]) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    for (path, item) in menus::flatten(table) {
        if item.key.is_empty() {
            continue;
        }
        let title = path.split(" › ").next().unwrap_or(&path).to_string();
        let row = Row {
            key: item.key.to_string(),
            action: item.label.clone(),
        };
        match groups.iter_mut().find(|group| group.title == title) {
            Some(group) => group.rows.push(row),
            None => groups.push(Group {
                title,
                rows: vec![row],
            }),
        }
    }
    groups.push(menu_bar_group(table));
    groups.extend(SCOPES.iter().map(|(title, rows)| {
        Group {
            title: (*title).to_string(),
            rows: rows
                .iter()
                .map(|(key, action)| Row {
                    key: (*key).to_string(),
                    action: (*action).to_string(),
                })
                .collect(),
        }
    }));
    groups
}

/// The in-window menu bar's keys ([`crate::menu_keys`], [`crate::menu_nav`]),
/// with each menu's Alt+letter from its title.
fn menu_bar_group<C>(table: &[menus::Menu<C>]) -> Group {
    let titles: Vec<&str> = table.iter().map(|menu| menu.title).collect();
    let row = |key: &str, action: &str| Row {
        key: key.to_string(),
        action: action.to_string(),
    };
    let mut rows = vec![row(
        "Alt  F10",
        "Keyboard to the menu bar · ← → choose · ↓ open",
    )];
    for (title, key) in titles.iter().zip(crate::menu_keys::top_mnemonics(&titles)) {
        if let Some(key) = key {
            rows.push(row(&format!("Alt+{key}"), &format!("Open {title} menu")));
        }
    }
    rows.extend([
        row("↑ ↓", "Move in an open menu"),
        row("← →", "Close or open a submenu · previous or next menu"),
        row("Enter  Space", "Run the highlighted item"),
        row("Letter", "Run the item with that underlined letter"),
        row("Esc", "Close the menu"),
    ]);
    Group {
        title: "Menu bar".to_string(),
        rows,
    }
}

/// Rows whose key or action contains `query` (case-insensitive); groups with
/// no matching row are hidden.
pub fn filter(groups: &[Group], query: &str) -> Vec<Group> {
    let query = query.trim().to_lowercase();
    groups
        .iter()
        .filter_map(|group| {
            let rows = group
                .rows
                .iter()
                .filter(|row| {
                    query.is_empty()
                        || row.key.to_lowercase().contains(&query)
                        || row.action.to_lowercase().contains(&query)
                })
                .cloned()
                .collect::<Vec<_>>();
            (!rows.is_empty()).then(|| Group {
                title: group.title.clone(),
                rows,
            })
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Close,
    EditKeys,
}

const KEY_WIDTH: f32 = 132.0;

/// Draw the window; `query` is the filter text, kept while it is open.
pub fn show(ctx: &egui::Context, query: &mut String, groups: &[Group]) -> Option<Act> {
    let mut act = None;
    let shown = filter(groups, query);
    egui::Window::new("Keyboard shortcuts")
        .id(egui::Id::unique("keyboard-shortcuts"))
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .default_width(560.0)
        .default_height(560.0)
        .default_pos(ctx.content_rect().center())
        .pivot(egui::Align2::CENTER_CENTER)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Keyboard shortcuts").size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("×").on_hover_text("Close (Esc)").clicked() {
                        act = Some(Act::Close);
                    }
                });
            });
            ui.add(
                egui::TextEdit::singleline(query)
                    .id(egui::Id::unique("keyboard-shortcuts-filter"))
                    .hint_text("Filter shortcuts")
                    .desired_width(f32::INFINITY),
            );
            ui.separator();
            let footer = 34.0;
            egui::ScrollArea::vertical()
                .id_salt("keyboard-shortcuts-table")
                .max_height((ui.available_height() - footer).max(80.0))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if shown.is_empty() {
                        ui.weak(format!("No shortcut matches “{}”.", query.trim()));
                    }
                    for group in &shown {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(group.title.to_uppercase())
                                    .small()
                                    .weak(),
                            );
                            ui.label(
                                egui::RichText::new(group.rows.len().to_string())
                                    .small()
                                    .weak(),
                            );
                        });
                        for row in &group.rows {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 22.0),
                                egui::Sense::hover(),
                            );
                            let key = egui::Rect::from_min_size(
                                rect.min,
                                egui::vec2(KEY_WIDTH, rect.height()),
                            );
                            let action =
                                egui::Rect::from_min_max(egui::pos2(key.right(), rect.top()), rect.max);
                            crate::bounded_text::paint(
                                ui,
                                key,
                                &crate::keymap::shown(&row.key),
                                egui::FontId::monospace(12.0),
                                ui.visuals().strong_text_color(),
                            );
                            crate::bounded_text::paint(
                                ui,
                                action,
                                &row.action,
                                egui::FontId::proportional(13.0),
                                ui.visuals().text_color(),
                            );
                        }
                    }
                });
            ui.separator();
            ui.horizontal(|ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(
                            "Generated from the command table: every row is a menu or list command.",
                        )
                        .small()
                        .weak(),
                    )
                    .truncate(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(("Close", egui::RichText::new("Esc").small().weak()))
                        .clicked()
                    {
                        act = Some(Act::Close);
                    }
                    if ui.button("Edit keys…").clicked() {
                        act = Some(Act::EditKeys);
                    }
                });
            });
        });
    if act.is_none() && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        act = Some(Act::Close);
    }
    act
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::{Entry, Menu, item};

    fn table() -> Vec<Menu<u8>> {
        vec![
            Menu {
                title: "Repository",
                entries: vec![
                    item("Open…", "Ctrl+O", 1).entry(),
                    item("Clone…", "", 2).entry(),
                    Entry::Separator,
                    item("Fetch", "Ctrl+Shift+F", 3).entry(),
                ],
            },
            Menu {
                title: "Help",
                entries: vec![item("About Git Good", "", 4).entry()],
            },
        ]
    }

    #[test]
    fn groups_follow_the_menus_then_the_list_scopes() {
        let groups = groups(&table());
        let titles = groups
            .iter()
            .map(|group| group.title.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            titles,
            [
                "Repository",
                "Menu bar",
                "Window",
                "History tree",
                "Local Changes list",
                "Review",
                "Rebase list",
                "Conflicts",
                "Stashes"
            ],
            "a menu without keyed items is left out"
        );
        let bar = &groups[1].rows;
        assert!(
            bar.iter()
                .any(|row| row.key == "Alt+R" && row.action == "Open Repository menu")
        );
        assert!(
            bar.iter()
                .any(|row| row.key == "Alt+H" && row.action == "Open Help menu")
        );
        assert!(bar.iter().any(|row| row.key == "Alt  F10"));
        assert_eq!(
            groups[0].rows,
            [
                Row {
                    key: "Ctrl+O".into(),
                    action: "Open…".into()
                },
                Row {
                    key: "Ctrl+Shift+F".into(),
                    action: "Fetch".into()
                }
            ]
        );
    }

    #[test]
    fn the_filter_matches_keys_or_actions_and_hides_empty_groups() {
        let groups = groups(&table());
        let by_key = filter(&groups, "ctrl+shift");
        assert_eq!(by_key.len(), 1);
        assert_eq!(by_key[0].rows[0].action, "Fetch");
        let by_action = filter(&groups, "blame");
        assert_eq!(by_action.len(), 1);
        assert_eq!(by_action[0].title, "Review");
        assert!(filter(&groups, "zzqq").is_empty());
        assert_eq!(filter(&groups, "  ").len(), groups.len());
    }

    #[test]
    fn no_row_offers_refresh() {
        assert!(
            groups(&table())
                .iter()
                .flat_map(|group| &group.rows)
                .all(|row| !row.action.to_lowercase().contains("refresh"))
        );
    }
}
