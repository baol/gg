//! Branch, remote, and tag navigation.
use eframe::egui;
use gg_engine::refs::{RefItem, RefKind, Snapshot};
use std::collections::BTreeMap;

const ICON_SIZE: f32 = 12.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Switch(String),
    Jump(String),
    Track(String),
    Merge(String),
    Rebase(String),
    DeleteBranch(String),
    DeleteTag(String),
    VerifyTag(String),
    /// Open the toolbar stash picker on the entry at this index.
    OpenStash(usize),
    /// Open another worktree of this repository.
    OpenWorktree(String),
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
    worktrees: &[gg_engine::worktree::Worktree],
    filter: &mut String,
    busy: bool,
    lanes: &BTreeMap<String, u8>,
) -> Option<Action> {
    let mut action = None;
    ui.add_space(4.0);
    ui.add(
        egui::TextEdit::singleline(filter)
            .hint_text("Filter refs")
            .desired_width(f32::INFINITY),
    );
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

    // Linked worktrees, as in the History mockup; collapsed until
    // opened, and only listed when there is more than the main one.
    let worktree_rows = worktrees
        .iter()
        .filter(|worktree| {
            query.is_empty()
                || worktree.path.to_lowercase().contains(&query)
                || worktree.branch_label().to_lowercase().contains(&query)
        })
        .collect::<Vec<_>>();
    if !query.is_empty() && empty && stash_rows.is_empty() && worktree_rows.is_empty() {
        ui.add_space(6.0);
        ui.weak("No matching refs");
        return action;
    }

    // `↑1 ↓3` beside each branch with an upstream, as in the mockup.
    let counts = refs
        .tracking
        .iter()
        .filter_map(|tracking| {
            let parts = [
                tracking
                    .ahead
                    .filter(|n| *n > 0)
                    .map(|n| format!("{}{n}", crate::phosphor::ARROW_UP)),
                tracking
                    .behind
                    .filter(|n| *n > 0)
                    .map(|n| format!("{}{n}", crate::phosphor::ARROW_DOWN)),
            ];
            let text = parts.into_iter().flatten().collect::<Vec<_>>().join(" ");
            (!text.is_empty()).then(|| (tracking.full_name.clone(), text))
        })
        .collect::<BTreeMap<_, _>>();
    egui::ScrollArea::vertical()
        .id_salt("refs-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            section(ui, "Branches", branches.len(), !query.is_empty(), |ui| {
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
                    render_tree(
                        ui,
                        &tree,
                        "branch",
                        head_name,
                        busy,
                        lanes,
                        &counts,
                        &mut action,
                    );
                } else {
                    for item in branches {
                        render_leaf(
                            ui,
                            item,
                            &item.name,
                            head_name,
                            busy,
                            lanes,
                            &counts,
                            &mut action,
                        );
                    }
                }
            });

            section(ui, "Remotes", remotes.len(), !query.is_empty(), |ui| {
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
                    render_tree(
                        ui,
                        &tree,
                        "remote",
                        head_name,
                        busy,
                        lanes,
                        &counts,
                        &mut action,
                    );
                } else {
                    for item in remotes {
                        render_leaf(
                            ui,
                            item,
                            &item.name,
                            head_name,
                            busy,
                            lanes,
                            &counts,
                            &mut action,
                        );
                    }
                }
            });

            section(ui, "Tags", tags.len(), !query.is_empty(), |ui| {
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
                    render_tree(
                        ui,
                        &tree,
                        "tag",
                        head_name,
                        busy,
                        lanes,
                        &counts,
                        &mut action,
                    );
                } else {
                    for item in tags {
                        render_leaf(
                            ui,
                            item,
                            &item.name,
                            head_name,
                            busy,
                            lanes,
                            &counts,
                            &mut action,
                        );
                    }
                }
            });

            section(ui, "Stashes", stash_rows.len(), !query.is_empty(), |ui| {
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
                            .min_size(egui::vec2(ui.available_width(), crate::density::row())),
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

            if worktrees.len() > 1 {
                section(
                    ui,
                    "Worktrees",
                    worktree_rows.len(),
                    !query.is_empty(),
                    |ui| {
                        for worktree in worktree_rows {
                            let folder = std::path::Path::new(&worktree.path)
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_else(|| worktree.path.clone());
                            let branch = worktree
                                .branch
                                .as_deref()
                                .map(|name| name.strip_prefix("refs/heads/").unwrap_or(name))
                                .unwrap_or(worktree.branch_label());
                            let mut text = egui::text::LayoutJob::default();
                            text.append(&folder, 0.0, egui::TextFormat::default());
                            text.append(
                                &format!("  {branch}{}", if worktree.current { "  ✓" } else { "" }),
                                0.0,
                                egui::TextFormat {
                                    color: ui.visuals().weak_text_color(),
                                    ..Default::default()
                                },
                            );
                            let response = ui
                                .add(
                                    egui::Button::new(text)
                                        .frame_when_inactive(false)
                                        .truncate()
                                        .min_size(egui::vec2(
                                            ui.available_width(),
                                            crate::density::row(),
                                        )),
                                )
                                .on_hover_text(if worktree.current {
                                    format!("{}\nThe open worktree", worktree.path)
                                } else {
                                    format!("{}\nOpen this worktree", worktree.path)
                                });
                            if response.clicked() && !worktree.current {
                                action = Some(Action::OpenWorktree(worktree.path.clone()));
                            }
                        }
                    },
                );
            }

            if query.is_empty() && empty {
                ui.add_space(6.0);
                ui.weak("Branches and tags appear after the first commit.");
            }
        });
    action
}

/// A collapsible group headed like the mockup: `BRANCHES 2`. Branches and
/// Stashes start open; Remotes, Tags, and Worktrees start collapsed. Every group opens
/// while the filter has text, so matches are never hidden.
fn section(
    ui: &mut egui::Ui,
    title: &str,
    count: usize,
    filtering: bool,
    contents: impl FnOnce(&mut egui::Ui),
) {
    let id = ui.make_persistent_id(("refs-section", title));
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        id,
        !matches!(title, "Remotes" | "Tags" | "Worktrees"),
    );
    if filtering {
        state.set_open(true);
    }
    ui.add_space(if title == "Branches" { 2.0 } else { 6.0 });
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::click());
    let weak = ui.visuals().weak_text_color();
    let font = egui::FontId::proportional(11.0);
    let open = state.is_open();
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title.to_uppercase(),
        font.clone(),
        weak,
    );
    // Open groups show their count; collapsed ones `▸ 25`, as in the mockup.
    ui.painter().text(
        egui::pos2(rect.right() - 4.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        if open {
            count.to_string()
        } else {
            format!("{} {count}", crate::phosphor::CARET_RIGHT)
        },
        font,
        weak,
    );
    let response = response
        .on_hover_text(if open { "Collapse" } else { "Expand" })
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Button,
            ui.is_enabled(),
            format!("{title} {count}"),
        )
    });
    if response.clicked() && !filtering {
        state.toggle(ui);
    }
    state.show_body_unindented(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        contents(ui);
    });
    state.store(ui.ctx());
}

#[allow(clippy::too_many_arguments)]
fn render_tree(
    ui: &mut egui::Ui,
    tree: &Tree<'_>,
    section: &str,
    head_name: Option<&str>,
    busy: bool,
    lanes: &BTreeMap<String, u8>,
    counts: &BTreeMap<String, String>,
    action: &mut Option<Action>,
) {
    for (segment, node) in &tree.children {
        let path = format!("{section}/{segment}");
        if node.children.is_empty() {
            if let Some(item) = node.item {
                render_leaf(ui, item, segment, head_name, busy, lanes, counts, action);
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
                    render_leaf(ui, item, segment, head_name, busy, lanes, counts, action);
                }
                render_tree(ui, node, section, head_name, busy, lanes, counts, action);
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_leaf(
    ui: &mut egui::Ui,
    item: &RefItem,
    label: &str,
    head_name: Option<&str>,
    busy: bool,
    lanes: &BTreeMap<String, u8>,
    counts: &BTreeMap<String, String>,
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
    let count = counts.get(&item.full_name).map(String::as_str);
    let response = row(ui, label, item.kind, current, lane, count).on_hover_text(hover);
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
    count: Option<&str>,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), crate::density::row()),
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
    // Branches get their graph lane's dot, as in the mockup; remotes and
    // tags keep their glyph.
    if kind == RefKind::Branch {
        let dot = lane.unwrap_or_else(|| ui.visuals().weak_text_color());
        ui.painter().circle_filled(icon.center(), 3.5, dot);
    } else {
        paint_icon(ui.painter(), icon, kind, color.gamma_multiply(0.9));
    }
    let mut text_right = rect.right() - if current { 20.0 } else { 6.0 };
    if let Some(count) = count {
        let galley = ui.painter().layout_no_wrap(
            count.to_owned(),
            egui::FontId::proportional(11.0),
            ui.visuals().weak_text_color(),
        );
        let at = egui::pos2(
            text_right - galley.size().x,
            rect.center().y - galley.size().y / 2.0,
        );
        text_right = at.x - 6.0;
        ui.painter()
            .galley(at, galley, ui.visuals().weak_text_color());
    }
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
    fn worktrees_are_listed_when_linked_ones_exist_and_open_on_click() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let refs = snapshot();
        let worktree = |path: &str, branch: &str, current: bool| gg_engine::worktree::Worktree {
            path: path.into(),
            path_bytes: path.as_bytes().to_vec(),
            head: Some("a".repeat(40)),
            branch: Some(format!("refs/heads/{branch}")),
            detached: false,
            bare: false,
            locked: None,
            prunable: None,
            current,
            main: current,
            dirty: Some(false),
        };
        let worktrees = [
            worktree("/src/gg", "main", true),
            worktree("/src/gg-feature", "feature", false),
        ];
        // The filter opens every group, so the rows are drawn.
        let mut filter = "gg".to_string();
        let mut action = None;
        let mut draw = |events: Vec<egui::Event>| {
            crate::tests::run_ui(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(260.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let chosen = show(
                            ui,
                            &refs,
                            &[],
                            &worktrees,
                            &mut filter,
                            false,
                            &BTreeMap::new(),
                        );
                        action = action.clone().or(chosen);
                    });
                },
            )
        };
        let output = draw(vec![]);
        let row = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.job.text.starts_with("gg-feature") => {
                Some(text.pos + egui::vec2(4.0, 4.0))
            }
            _ => None,
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            text.iter().any(|line| line.as_str() == "WORKTREES"),
            "{text:?}"
        );
        let Some(row) = row else {
            unreachable!("the linked worktree row is drawn: {text:?}");
        };
        let button = |pressed| egui::Event::PointerButton {
            pos: row,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        draw(vec![egui::Event::PointerMoved(row), button(true)]);
        draw(vec![button(false)]);
        assert_eq!(action, Some(Action::OpenWorktree("/src/gg-feature".into())));
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
                    let action = show(ui, &refs, &[], &[], &mut filter, false, &BTreeMap::new());
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
            "BRANCHES",
            "main",
            "REMOTES",
            "TAGS",
            "\u{E13A} 1",
            "2",
            "\u{E08E}2 \u{E03E}1",
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
                ..Default::default()
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
                        action = show(ui, &refs, &stashes, &[], filter, false, &BTreeMap::new());
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
                    show(ui, &refs, &[], &[], &mut filter, false, &BTreeMap::new());
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
                        action = show(ui, &refs, &[], &[], &mut filter, false, &BTreeMap::new());
                    });
                },
            )
        };
        let output = run(vec![], 1.0);
        let position = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text)
                if text.galley.job.text == label && text.pos.y > 40.0 =>
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
