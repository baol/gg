//! Fork-style branch, remote, and tag navigation.
use eframe::egui;
use gg_engine::refs::{RefItem, RefKind, Snapshot};
use std::collections::BTreeMap;

const ROW_HEIGHT: f32 = 22.0;
const ICON_SIZE: f32 = 12.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Switch(String),
    Jump(String),
    NewBranch,
}

#[derive(Default)]
struct Tree<'a> {
    item: Option<&'a RefItem>,
    children: BTreeMap<String, Tree<'a>>,
}

impl<'a> Tree<'a> {
    fn insert(&mut self, item: &'a RefItem) {
        let mut node = self;
        for segment in item.name.split('/').filter(|segment| !segment.is_empty()) {
            node = node.children.entry(segment.to_string()).or_default();
        }
        node.item = Some(item);
    }
}

pub fn show(ui: &mut egui::Ui, refs: &Snapshot, filter: &mut String, busy: bool) -> Option<Action> {
    let mut action = None;
    ui.add_space(4.0);
    ui.strong("Refs");
    ui.add(
        egui::TextEdit::singleline(filter)
            .hint_text("Filter refs")
            .desired_width(f32::INFINITY),
    );
    if ui
        .add_enabled(
            !busy && refs.head.as_ref().is_some_and(|head| head.commit.is_some()),
            egui::Button::new("+  New branch").min_size(egui::vec2(ui.available_width(), 24.0)),
        )
        .on_hover_text("Create a branch from HEAD")
        .clicked()
    {
        action = Some(Action::NewBranch);
    }
    ui.add_space(2.0);

    let query = filter.trim().to_lowercase();
    let head_name = refs.head.as_ref().and_then(|head| head.name.as_deref());
    let matching = |kind: RefKind| {
        refs.items
            .iter()
            .filter(move |item| item.kind == kind)
            .filter(|item| query.is_empty() || item.name.to_lowercase().contains(&query))
            .collect::<Vec<_>>()
    };

    let branches = matching(RefKind::Branch);
    let remotes = matching(RefKind::Remote);
    let tags = matching(RefKind::Tag);
    let empty = branches.is_empty() && remotes.is_empty() && tags.is_empty();

    if !query.is_empty() && empty {
        ui.add_space(6.0);
        ui.weak("No matching refs");
        return action;
    }

    egui::ScrollArea::vertical()
        .id_salt("refs-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            section(ui, "Branches", branches.len(), |ui| {
                if let Some(head) = &refs.head
                    && head.detached
                {
                    let commit = head
                        .commit
                        .as_deref()
                        .map(|id| id.chars().take(8).collect::<String>())
                        .unwrap_or_else(|| "unborn".into());
                    ui.add_space(1.0);
                    ui.weak(format!("Detached at {commit}"));
                }
                if branches.is_empty() {
                    ui.weak(if query.is_empty() {
                        "No local branches"
                    } else {
                        "No matching branches"
                    });
                } else if query.is_empty() {
                    let mut tree = Tree::default();
                    for item in branches {
                        tree.insert(item);
                    }
                    render_tree(ui, &tree, "branch", head_name, busy, &mut action);
                } else {
                    for item in branches {
                        render_leaf(ui, item, &item.name, head_name, busy, &mut action);
                    }
                }
            });

            section(ui, "Remotes", remotes.len(), |ui| {
                if remotes.is_empty() {
                    ui.weak(if query.is_empty() {
                        "No remotes"
                    } else {
                        "No matching remotes"
                    });
                } else if query.is_empty() {
                    let mut tree = Tree::default();
                    for item in remotes {
                        tree.insert(item);
                    }
                    render_tree(ui, &tree, "remote", head_name, busy, &mut action);
                } else {
                    for item in remotes {
                        render_leaf(ui, item, &item.name, head_name, busy, &mut action);
                    }
                }
            });

            section(ui, "Tags", tags.len(), |ui| {
                if tags.is_empty() {
                    ui.weak(if query.is_empty() {
                        "No tags"
                    } else {
                        "No matching tags"
                    });
                } else if query.is_empty() {
                    let mut tree = Tree::default();
                    for item in tags {
                        tree.insert(item);
                    }
                    render_tree(ui, &tree, "tag", head_name, busy, &mut action);
                } else {
                    for item in tags {
                        render_leaf(ui, item, &item.name, head_name, busy, &mut action);
                    }
                }
            });

            if query.is_empty() && empty {
                ui.add_space(6.0);
                ui.weak("Branches and tags appear after the first commit.");
            }
        });
    action
}

fn section(ui: &mut egui::Ui, title: &str, count: usize, contents: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(3.0);
    egui::CollapsingHeader::new(
        egui::RichText::new(format!("{title}  {count}"))
            .strong()
            .size(12.0),
    )
    .id_salt(("refs-section", title))
    .default_open(true)
    .show_background(false)
    .show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        contents(ui);
    });
}

fn render_tree(
    ui: &mut egui::Ui,
    tree: &Tree<'_>,
    section: &str,
    head_name: Option<&str>,
    busy: bool,
    action: &mut Option<Action>,
) {
    for (segment, node) in &tree.children {
        let path = format!("{section}/{segment}");
        if node.children.is_empty() {
            if let Some(item) = node.item {
                render_leaf(ui, item, segment, head_name, busy, action);
            }
        } else {
            egui::CollapsingHeader::new(
                egui::RichText::new(segment)
                    .color(ui.visuals().weak_text_color())
                    .size(12.0),
            )
            .id_salt(("refs-folder", section, &path))
            .default_open(true)
            .show_background(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                if let Some(item) = node.item {
                    render_leaf(ui, item, segment, head_name, busy, action);
                }
                render_tree(ui, node, section, head_name, busy, action);
            });
        }
    }
}

fn render_leaf(
    ui: &mut egui::Ui,
    item: &RefItem,
    label: &str,
    head_name: Option<&str>,
    busy: bool,
    action: &mut Option<Action>,
) {
    let current = Some(item.full_name.as_str()) == head_name;
    let enabled = !busy && !current;
    let response = row(ui, label, item.kind, current, enabled).on_hover_text(if current {
        format!("{} · current branch", item.full_name)
    } else if busy {
        format!("{} · branch operation in progress", item.full_name)
    } else if item.kind == RefKind::Branch {
        format!("Switch to {}\n{}", item.full_name, short(&item.commit))
    } else {
        format!("Jump to {}\n{}", item.full_name, short(&item.commit))
    });
    if response.clicked() && enabled {
        *action = Some(match item.kind {
            RefKind::Branch => Action::Switch(item.name.clone()),
            RefKind::Remote | RefKind::Tag => Action::Jump(item.commit.clone()),
        });
    }
}

fn row(
    ui: &mut egui::Ui,
    label: &str,
    kind: RefKind,
    current: bool,
    enabled: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            if current {
                format!("{label}, current branch")
            } else {
                label.to_owned()
            },
        )
    });
    if current {
        ui.painter().rect_filled(
            rect,
            5.0,
            ui.visuals().selection.bg_fill.gamma_multiply(0.32),
        );
    } else if response.hovered() && enabled {
        ui.painter()
            .rect_filled(rect, 5.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    let color = if current {
        ui.visuals().strong_text_color()
    } else if enabled {
        ui.visuals().text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    let icon = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 9.0, rect.center().y),
        egui::vec2(ICON_SIZE, ICON_SIZE),
    );
    paint_icon(ui.painter(), icon, kind, color.gamma_multiply(0.9));
    let text_right = rect.right() - if current { 20.0 } else { 6.0 };
    crate::bounded_text::paint(
        ui,
        egui::Rect::from_min_max(
            egui::pos2(rect.left() + 19.0, rect.top()),
            egui::pos2(text_right, rect.bottom()),
        ),
        label,
        egui::FontId::proportional(12.0),
        color,
    );
    if current {
        let check = egui::pos2(rect.right() - 10.0, rect.center().y);
        let stroke = egui::Stroke::new(1.4, color);
        ui.painter().line_segment(
            [check + egui::vec2(-3.0, 0.0), check + egui::vec2(0.0, 3.0)],
            stroke,
        );
        ui.painter().line_segment(
            [check + egui::vec2(0.0, 3.0), check + egui::vec2(4.0, -4.0)],
            stroke,
        );
    }
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

pub(crate) fn paint_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    kind: RefKind,
    color: egui::Color32,
) {
    let p = |x: f32, y: f32| egui::pos2(rect.left() + x, rect.top() + y);
    let stroke = egui::Stroke::new(1.15, color);
    match kind {
        RefKind::Branch => {
            painter.circle_filled(p(3.5, 3.0), 1.5, color);
            painter.circle_filled(p(3.5, 9.0), 1.5, color);
            painter.circle_filled(p(8.5, 4.0), 1.5, color);
            painter.line_segment([p(3.5, 4.0), p(3.5, 8.0)], stroke);
            painter.line_segment([p(3.5, 6.0), p(8.5, 5.0)], stroke);
        }
        RefKind::Remote => {
            painter.circle_stroke(rect.center(), 4.2, stroke);
            painter.line_segment(
                [
                    rect.center() - egui::vec2(4.2, 0.0),
                    rect.center() + egui::vec2(4.2, 0.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    rect.center() - egui::vec2(0.0, 4.2),
                    rect.center() + egui::vec2(0.0, 4.2),
                ],
                stroke,
            );
        }
        RefKind::Tag => {
            let points = vec![
                p(2.0, 3.0),
                p(8.0, 3.0),
                p(10.5, 5.5),
                p(5.5, 10.5),
                p(2.0, 7.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                points,
                egui::Color32::TRANSPARENT,
                stroke,
            ));
            painter.circle_filled(p(4.5, 5.0), 1.0, color);
        }
    }
}

fn short(id: &str) -> String {
    id.chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        let commit = "a".repeat(40);
        Snapshot {
            head: Some(gg_engine::refs::Head {
                name: Some("refs/heads/main".into()),
                detached: false,
                commit: Some(commit.clone()),
            }),
            items: vec![
                RefItem {
                    name: "main".into(),
                    full_name: "refs/heads/main".into(),
                    kind: RefKind::Branch,
                    commit: commit.clone(),
                },
                RefItem {
                    name: "feature/nested".into(),
                    full_name: "refs/heads/feature/nested".into(),
                    kind: RefKind::Branch,
                    commit: commit.clone(),
                },
                RefItem {
                    name: "origin/main".into(),
                    full_name: "refs/remotes/origin/main".into(),
                    kind: RefKind::Remote,
                    commit: commit.clone(),
                },
                RefItem {
                    name: "v1.0".into(),
                    full_name: "refs/tags/v1.0".into(),
                    kind: RefKind::Tag,
                    commit,
                },
            ],
        }
    }

    #[test]
    fn shows_grouped_sections_and_current_branch() {
        let ctx = egui::Context::default();
        let refs = snapshot();
        let mut filter = String::new();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(190.0, 480.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let action = show(ui, &refs, &mut filter, false);
                    assert!(action.is_none());
                });
            },
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for expected in [
            "Refs", "Branches", "main", "Remotes", "origin", "Tags", "v1.0",
        ] {
            assert!(
                text.iter().any(|line| line.contains(expected)),
                "{expected}: {text:?}"
            );
        }
        assert!(text.iter().any(|line| line.contains("feature")));
    }

    #[test]
    fn filter_flattens_and_clears_unmatched_groups() {
        let ctx = egui::Context::default();
        let refs = snapshot();
        let mut filter = "nomatch".into();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(190.0, 480.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, &refs, &mut filter, false);
                });
            },
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.iter().any(|line| line.contains("No matching refs")));
        assert!(!text.iter().any(|line| line.contains("Branches  1")));
    }
}
