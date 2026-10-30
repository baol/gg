//! Branch, remote, and tag navigation.
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
    NewTag,
    Track(String),
    Merge(String),
    Rebase(String),
    PullRebase,
    DeleteBranch(String),
    DeleteTag(String),
    VerifyTag(String),
    /// Open the toolbar stash picker on the entry at this index.
    OpenStash(usize),
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

pub fn show(
    ui: &mut egui::Ui,
    refs: &Snapshot,
    stashes: &[gg_engine::stash::StashEntry],
    filter: &mut String,
    busy: bool,
    lanes: &BTreeMap<String, u8>,
) -> Option<Action> {
    let mut action = None;
    ui.add_space(4.0);
    ui.strong("Refs");
    ui.add(
        egui::TextEdit::singleline(filter)
            .hint_text("Filter refs")
            .desired_width(f32::INFINITY),
    );
    let has_head = refs.head.as_ref().is_some_and(|head| head.commit.is_some());
    ui.horizontal(|ui| {
        let width = ((ui.available_width() - 6.0) / 2.0).max(60.0);
        if ui
            .add_enabled(
                !busy && has_head,
                egui::Button::new("+  Branch").min_size(egui::vec2(width, 24.0)),
            )
            .on_hover_text("Create a branch from HEAD")
            .clicked()
        {
            action = Some(Action::NewBranch);
        }
        if ui
            .add_enabled(
                !busy && has_head,
                egui::Button::new("+  Tag").min_size(egui::vec2(width, 24.0)),
            )
            .on_hover_text("Create an annotated or signed tag at an explicit target")
            .clicked()
        {
            action = Some(Action::NewTag);
        }
    });
    if let Some(tracking) = refs.current_tracking() {
        ui.add_space(2.0);
        tracking_row(ui, tracking, busy, &mut action);
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

    let stash_rows = stashes
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            query.is_empty()
                || entry.reference.to_lowercase().contains(&query)
                || entry.message.to_lowercase().contains(&query)
        })
        .collect::<Vec<_>>();

    if !query.is_empty() && empty && stash_rows.is_empty() {
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
                    render_tree(ui, &tree, "branch", head_name, busy, lanes, &mut action);
                } else {
                    for item in branches {
                        render_leaf(ui, item, &item.name, head_name, busy, lanes, &mut action);
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
                    render_tree(ui, &tree, "remote", head_name, busy, lanes, &mut action);
                } else {
                    for item in remotes {
                        render_leaf(ui, item, &item.name, head_name, busy, lanes, &mut action);
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
                    render_tree(ui, &tree, "tag", head_name, busy, lanes, &mut action);
                } else {
                    for item in tags {
                        render_leaf(ui, item, &item.name, head_name, busy, lanes, &mut action);
                    }
                }
            });

            section(ui, "Stashes", stash_rows.len(), |ui| {
                if stash_rows.is_empty() {
                    ui.weak(if query.is_empty() {
                        "No stashes"
                    } else {
                        "No matching stashes"
                    });
                }
                for (index, entry) in stash_rows {
                    let short = entry
                        .reference
                        .strip_prefix("stash")
                        .unwrap_or(&entry.reference);
                    let response = ui
                        .add(
                            egui::Button::new((
                                egui::RichText::new(short).monospace().weak(),
                                egui::RichText::new(&entry.message),
                            ))
                            .frame_when_inactive(false)
                            .truncate()
                            .min_size(egui::vec2(ui.available_width(), ROW_HEIGHT)),
                        )
                        .on_hover_text(format!(
                            "{}\n{}\nOpen in the stash picker to apply, pop, show, or drop",
                            entry.reference, entry.message
                        ));
                    if response.clicked() {
                        action = Some(Action::OpenStash(index));
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

#[allow(clippy::too_many_arguments)]
fn render_tree(
    ui: &mut egui::Ui,
    tree: &Tree<'_>,
    section: &str,
    head_name: Option<&str>,
    busy: bool,
    lanes: &BTreeMap<String, u8>,
    action: &mut Option<Action>,
) {
    for (segment, node) in &tree.children {
        let path = format!("{section}/{segment}");
        if node.children.is_empty() {
            if let Some(item) = node.item {
                render_leaf(ui, item, segment, head_name, busy, lanes, action);
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
                    render_leaf(ui, item, segment, head_name, busy, lanes, action);
                }
                render_tree(ui, node, section, head_name, busy, lanes, action);
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
    lanes: &BTreeMap<String, u8>,
    action: &mut Option<Action>,
) {
    let current = Some(item.full_name.as_str()) == head_name;
    let lane = (item.kind == RefKind::Branch)
        .then(|| {
            lanes
                .get(&item.full_name)
                .map(|lane| crate::graph::lane_color(*lane))
        })
        .flatten();
    let inspect = format!("Inspect {}\n{}", item.full_name, short(&item.commit));
    let hover = if current {
        format!(
            "{} · current branch\nClick to inspect {}",
            item.full_name,
            short(&item.commit)
        )
    } else if item.kind == RefKind::Branch {
        format!("{inspect}\nDouble-click to switch\nRight-click for ref actions")
    } else {
        format!("{inspect}\nRight-click for ref actions")
    };
    let response = row(ui, label, item.kind, current, lane).on_hover_text(hover);
    if response.double_clicked() {
        if item.kind == RefKind::Branch && !current && !busy {
            *action = Some(Action::Switch(item.name.clone()));
        }
    } else if response.clicked() {
        *action = Some(Action::Jump(item.commit.clone()));
    }
    response.context_menu(|ui| {
        match item.kind {
            RefKind::Branch => {
                if ui
                    .add_enabled(
                        !current && !busy,
                        egui::Button::new(format!("Switch to {}", item.name)),
                    )
                    .on_hover_text(if current {
                        "Already on this branch"
                    } else if busy {
                        "Wait for the current operation to finish."
                    } else {
                        "Switch the worktree after a clean-worktree check"
                    })
                    .clicked()
                {
                    *action = Some(Action::Switch(item.name.clone()));
                    ui.close();
                }
                if ui
                    .add_enabled(!current && !busy, egui::Button::new("Merge into current"))
                    .clicked()
                {
                    *action = Some(Action::Merge(item.name.clone()));
                    ui.close();
                }
                if ui
                    .add_enabled(
                        !current && !busy,
                        egui::Button::new("Rebase current onto this"),
                    )
                    .clicked()
                {
                    *action = Some(Action::Rebase(item.name.clone()));
                    ui.close();
                }
                if ui
                    .add_enabled(!current && !busy, egui::Button::new("Delete branch…"))
                    .clicked()
                {
                    *action = Some(Action::DeleteBranch(item.name.clone()));
                    ui.close();
                }
            }
            RefKind::Remote => {
                if ui
                    .add_enabled(!busy, egui::Button::new("Create tracking branch…"))
                    .clicked()
                {
                    *action = Some(Action::Track(item.name.clone()));
                    ui.close();
                }
                if ui
                    .add_enabled(!busy, egui::Button::new("Merge into current"))
                    .clicked()
                {
                    *action = Some(Action::Merge(item.name.clone()));
                    ui.close();
                }
                if ui
                    .add_enabled(!busy, egui::Button::new("Rebase current onto this"))
                    .clicked()
                {
                    *action = Some(Action::Rebase(item.name.clone()));
                    ui.close();
                }
            }
            RefKind::Tag => {
                if ui
                    .add_enabled(!busy, egui::Button::new("Merge into current"))
                    .clicked()
                {
                    *action = Some(Action::Merge(item.name.clone()));
                    ui.close();
                }
                if ui
                    .add_enabled(!busy, egui::Button::new("Rebase current onto this"))
                    .clicked()
                {
                    *action = Some(Action::Rebase(item.name.clone()));
                    ui.close();
                }
                if ui
                    .add_enabled(!busy, egui::Button::new("Signature details…"))
                    .clicked()
                {
                    *action = Some(Action::VerifyTag(item.full_name.clone()));
                    ui.close();
                }
                if ui
                    .add_enabled(!busy, egui::Button::new("Delete tag…"))
                    .clicked()
                {
                    *action = Some(Action::DeleteTag(item.name.clone()));
                    ui.close();
                }
            }
        }
        ui.separator();
        if ui.button("Copy name").clicked() {
            ui.ctx().copy_text(item.name.clone());
            ui.close();
        }
        if ui.button("Copy hash").clicked() {
            ui.ctx().copy_text(item.commit.clone());
            ui.close();
        }
    });
}

fn row(
    ui: &mut egui::Ui,
    label: &str,
    kind: RefKind,
    current: bool,
    lane: Option<egui::Color32>,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Button,
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
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, 5.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    let color = if current {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().text_color()
    };
    let icon = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 9.0, rect.center().y),
        egui::vec2(ICON_SIZE, ICON_SIZE),
    );
    let icon_color = if let Some(dot) = lane {
        ui.painter()
            .circle_filled(icon.center(), ICON_SIZE / 2.0, dot);
        crate::graph::on_lane_color(dot)
    } else {
        color.gamma_multiply(0.9)
    };
    paint_icon(ui.painter(), icon, kind, icon_color);
    let text_right = rect.right() - if current { 20.0 } else { 6.0 };
    let text_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 19.0, rect.top()),
        egui::pos2(text_right, rect.bottom()),
    );
    let font = egui::FontId::proportional(12.0);
    if current {
        crate::bounded_text::paint_bold(ui, text_rect, label, font, color);
    } else {
        crate::bounded_text::paint(ui, text_rect, label, font, color);
    }
    if current {
        let check = egui::pos2(rect.right() - 10.0, rect.center().y);
        let stroke = egui::Stroke::new(1.4_f32, color);
        ui.painter().line_segment(
            [check + egui::vec2(-3.0, 0.0), check + egui::vec2(0.0, 3.0)],
            stroke,
        );
        ui.painter().line_segment(
            [check + egui::vec2(0.0, 3.0), check + egui::vec2(4.0, -4.0)],
            stroke,
        );
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(crate) fn paint_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    kind: RefKind,
    color: egui::Color32,
) {
    let p = |x: f32, y: f32| egui::pos2(rect.left() + x, rect.top() + y);
    let stroke = egui::Stroke::new(1.15_f32, color);
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

fn tracking_row(
    ui: &mut egui::Ui,
    tracking: &gg_engine::refs::Tracking,
    busy: bool,
    action: &mut Option<Action>,
) {
    use crate::phosphor;
    let weak = ui.visuals().weak_text_color();
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        let info = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.label(
                    egui::RichText::new(&tracking.upstream)
                        .size(11.0)
                        .color(weak),
                );
                if let (Some(ahead), Some(behind)) = (tracking.ahead, tracking.behind) {
                    ui.label(phosphor::icon(phosphor::ARROW_UP).size(10.0).color(weak));
                    ui.label(
                        egui::RichText::new(ahead.to_string())
                            .size(11.0)
                            .color(weak),
                    );
                    ui.label(phosphor::icon(phosphor::ARROW_DOWN).size(10.0).color(weak));
                    ui.label(
                        egui::RichText::new(behind.to_string())
                            .size(11.0)
                            .color(weak),
                    );
                } else {
                    ui.label(egui::RichText::new("· unavailable").size(11.0).color(weak));
                }
            })
            .response;
        info.on_hover_text(format!(
            "{} · {}\nAhead and behind the configured upstream.",
            tracking.upstream_full_name, tracking.remote
        ));
        if ui
            .add_enabled(!busy, egui::Button::new("Pull --rebase").small())
            .on_hover_text(format!(
                "Fetch {} and rebase this branch onto {}",
                tracking.remote, tracking.upstream
            ))
            .on_disabled_hover_text("Wait for the current operation to finish.")
            .clicked()
        {
            *action = Some(Action::PullRebase);
        }
    });
}

pub(crate) fn tracking_label(tracking: &gg_engine::refs::Tracking) -> String {
    match (tracking.ahead, tracking.behind) {
        (Some(ahead), Some(behind)) => format!(
            "{}  {}{ahead} {}{behind}",
            tracking.upstream,
            crate::phosphor::ARROW_UP,
            crate::phosphor::ARROW_DOWN
        ),
        _ => format!("{} · upstream unavailable", tracking.upstream),
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
                    commit: commit.clone(),
                },
            ],
            remotes: vec![gg_engine::refs::Remote {
                name: "origin".into(),
                fetch_url: Some("https://example.org/gg".into()),
                push_url: None,
            }],
            tracking: vec![gg_engine::refs::Tracking {
                branch: "main".into(),
                full_name: "refs/heads/main".into(),
                upstream: "origin/main".into(),
                upstream_full_name: "refs/remotes/origin/main".into(),
                remote: "origin".into(),
                remote_branch: "main".into(),
                ahead: Some(2),
                behind: Some(1),
            }],
        }
    }

    #[test]
    fn shows_grouped_sections_and_current_branch() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let refs = snapshot();
        let mut filter = String::new();
        let output = crate::tests::run_ui(
            &ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(190.0, 480.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let action = show(ui, &refs, &[], &mut filter, false, &BTreeMap::new());
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
            "Refs",
            "Branches",
            "main",
            "Remotes",
            "origin",
            "Tags",
            "v1.0",
            "origin/main",
            "2",
        ] {
            assert!(
                text.iter().any(|line| line.contains(expected)),
                "{expected}: {text:?}"
            );
        }
        assert!(text.iter().any(|line| line.contains("feature")));
    }

    #[test]
    fn stash_entries_open_the_picker_and_follow_the_filter() {
        let ctx = egui::Context::default();
        let refs = snapshot();
        let stashes = (0..2)
            .map(|index| gg_engine::stash::StashEntry {
                index,
                reference: format!("stash@{{{index}}}"),
                commit: format!("{index:040x}"),
                message: format!("WIP on main: work {index}"),
            })
            .collect::<Vec<_>>();
        let frame = |filter: &mut String, events: Vec<egui::Event>| {
            let mut action = None;
            let output = crate::tests::run_ui(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(220.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        action = show(ui, &refs, &stashes, filter, false, &BTreeMap::new());
                    });
                },
            );
            (output, action)
        };
        let mut filter = String::new();
        let (output, _) = frame(&mut filter, vec![]);
        let row = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.job.text == "WIP on main: work 1" => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        });
        let Some(row) = row else {
            unreachable!("stash entry is listed")
        };
        let click = vec![
            egui::Event::PointerMoved(row),
            egui::Event::PointerButton {
                pos: row,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: row,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let (_, action) = frame(&mut filter, click);
        assert_eq!(action, Some(Action::OpenStash(1)));

        let mut filter = "work 0".to_string();
        let (output, _) = frame(&mut filter, vec![]);
        let texts = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(texts.iter().any(|text| text == "WIP on main: work 0"));
        assert!(!texts.iter().any(|text| text == "WIP on main: work 1"));
    }

    #[test]
    fn filter_flattens_and_clears_unmatched_groups() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let refs = snapshot();
        let mut filter: String = "nomatch".into();
        let output = crate::tests::run_ui(
            &ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(190.0, 480.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, &refs, &[], &mut filter, false, &BTreeMap::new());
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

    fn click_label(query: &str, label: &str, clicks: usize) -> Option<Action> {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let refs = snapshot();
        let mut filter = query.to_owned();
        let mut action = None;
        let mut run = |events: Vec<egui::Event>, time: f64| {
            crate::tests::run_ui(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(190.0, 480.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        action = show(ui, &refs, &[], &mut filter, false, &BTreeMap::new());
                    });
                },
            )
        };
        let output = run(vec![], 1.0);
        let position = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text)
                if text.galley.job.text == label && text.pos.y > 100.0 =>
            {
                Some(text.pos + egui::vec2(8.0, text.galley.size().y / 2.0))
            }
            _ => None,
        })?;
        for index in 0..clicks {
            let time = 1.1 + index as f64 * 0.2;
            run(
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                time,
            );
            run(
                vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                time + 0.05,
            );
        }
        action
    }

    #[test]
    fn clicking_a_branch_row_inspects_its_commit() {
        let commit = "a".repeat(40);
        assert_eq!(
            click_label("feature", "feature/nested", 1),
            Some(Action::Jump(commit))
        );
    }

    #[test]
    fn double_clicking_a_branch_row_switches() {
        assert_eq!(
            click_label("feature", "feature/nested", 2),
            Some(Action::Switch("feature/nested".into()))
        );
        assert_ne!(
            click_label("mai", "main", 2),
            Some(Action::Switch("main".into()))
        );
    }

    #[test]
    fn clicking_the_current_branch_row_inspects_it_without_switching() {
        assert_eq!(
            click_label("mai", "main", 1),
            Some(Action::Jump("a".repeat(40)))
        );
    }

    #[test]
    fn clicking_a_remote_row_inspects_its_commit() {
        assert_eq!(
            click_label("origin", "origin/main", 1),
            Some(Action::Jump("a".repeat(40)))
        );
    }

    #[test]
    fn double_clicking_a_remote_row_never_creates_a_tracking_branch() {
        let action = click_label("origin", "origin/main", 2);
        assert_ne!(action, Some(Action::Track("origin/main".into())));
        assert_ne!(action, Some(Action::Switch("origin/main".into())));
    }
}
