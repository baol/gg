//! Shared column geometry for headers, rows, and overlap assertions.
use crate::avatars::Avatars;
use eframe::egui;
use gg_engine::Commit;

/// Compact row height; runtime code uses `crate::density::row()`.
#[cfg(test)]
pub const ROW_HEIGHT: f32 = crate::density::COMPACT_ROW;
pub const MIN_WIDTH: f32 = 610.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefAction {
    Select,
}

pub fn columns(rect: egui::Rect) -> [egui::Rect; 5] {
    let widths = [
        92.0,
        (rect.width() - 92.0 - 150.0 - 66.0 - 92.0).max(0.0),
        150.0,
        66.0,
        92.0,
    ];
    let mut x = rect.left();
    widths.map(|width| {
        let cell =
            egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(width, rect.height()));
        x += width;
        cell
    })
}

fn text(ui: &egui::Ui, rect: egui::Rect, value: &str, color: egui::Color32, mono: bool) {
    let rect = rect.shrink2(egui::vec2(6.0, 0.0));
    if rect.width() <= 0.0 {
        return;
    }
    crate::bounded_text::paint(
        ui,
        rect,
        value,
        if mono {
            egui::FontId::monospace(11.0)
        } else {
            egui::FontId::proportional(12.0)
        },
        color,
    );
}

pub fn sort_labels(refs: &mut [gg_engine::refs::RefItem], head_name: Option<&str>) {
    // Tags first, then branches (the checked-out one first), then remotes,
    // as in the History mockup.
    refs.sort_by_key(|item| {
        (
            match item.kind {
                gg_engine::refs::RefKind::Tag => 0,
                gg_engine::refs::RefKind::Branch => 1,
                gg_engine::refs::RefKind::Remote => 2,
            },
            Some(item.full_name.as_str()) != head_name,
            item.name.clone(),
        )
    });
}

/// Column titles, as in the History mockup: `Commit · 164`, and the active
/// filter as a chip with a clear button. Returns true when the filter is
/// cleared.
pub fn header(ui: &mut egui::Ui, count: &str, filter: &str) -> bool {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), crate::density::row()),
        egui::Sense::hover(),
    );
    let cells = columns(rect);
    let commit = format!("Commit · {count}");
    for (cell, label) in cells
        .into_iter()
        .zip(["", commit.as_str(), "Author", "Hash", "Date"])
    {
        text(ui, cell, label, ui.visuals().weak_text_color(), false);
    }
    let mut cleared = false;
    if !filter.is_empty() {
        let width = ui
            .painter()
            .layout_no_wrap(
                commit,
                egui::FontId::proportional(12.0),
                egui::Color32::PLACEHOLDER,
            )
            .size()
            .x;
        let slot = egui::Rect::from_min_max(
            egui::pos2(cells[1].left() + width + 12.0, rect.top() + 1.0),
            egui::pos2(cells[1].right() - 4.0, rect.bottom() - 1.0),
        );
        if slot.width() > 60.0 {
            let mut chip = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(slot)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            chip.spacing_mut().item_spacing.x = 4.0;
            chip.add(
                egui::Label::new(
                    egui::RichText::new(format!("filter “{filter}”"))
                        .small()
                        .background_color(ui.visuals().faint_bg_color),
                )
                .truncate(),
            )
            .on_hover_text("Ctrl+F or Ctrl+K changes the filter");
            cleared = chip
                .small_button("×")
                .on_hover_text("Clear the filter")
                .clicked();
        }
    }
    ui.painter().hline(
        rect.x_range(),
        rect.bottom(),
        ui.visuals().widgets.noninteractive.bg_stroke,
    );
    cleared
}

fn paint_refs_and_subject(
    ui: &egui::Ui,
    rect: egui::Rect,
    refs: &[gg_engine::refs::RefItem],
    head_name: Option<&str>,
    commit: &Commit,
    selected: bool,
) -> Option<RefAction> {
    let max_labels = if rect.width() < 280.0 { 2 } else { 4 };
    let subject_color = if selected {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().strong_text_color().gamma_multiply(0.8)
    };
    let inner = rect.shrink2(egui::vec2(6.0, 0.0));
    if inner.width() <= 0.0 {
        return None;
    }
    let mut action = None;
    let painter = ui.painter().with_clip_rect(rect);
    let font = egui::FontId::proportional(10.5);
    let mut x = inner.left();
    let mut shown = 0;
    for item in refs.iter().take(max_labels) {
        let (background, foreground) = ref_label_colors(ui.visuals(), item.kind, selected);
        let galley = painter.layout_no_wrap(item.name.clone(), font.clone(), foreground);
        let width = galley.size().x + 18.0;
        if x + width > inner.right() - 24.0 {
            break;
        }
        let pill = egui::Rect::from_min_size(
            egui::pos2(x, inner.center().y - 7.5),
            egui::vec2(width, 15.0),
        );
        painter.rect_filled(pill, 4.0, background);
        crate::refs_view::paint_icon(
            &painter,
            egui::Rect::from_center_size(
                egui::pos2(pill.left() + 7.0, pill.center().y),
                egui::vec2(9.0, 9.0),
            ),
            item.kind,
            foreground,
        );
        painter.galley(
            egui::pos2(pill.left() + 13.0, pill.center().y - galley.size().y / 2.0),
            galley,
            foreground,
        );
        let current = Some(item.full_name.as_str()) == head_name;
        let hint = if current {
            "Current branch · click to inspect this commit"
        } else {
            "Click to inspect this commit"
        };
        let response = ui
            .interact(
                pill,
                ui.scope_id().with(("ref-pill", &item.full_name)),
                egui::Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(format!("{}\n{hint}", item.full_name));
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::Role::Button,
                true,
                format!("{:?} {}", item.kind, item.name),
            )
        });
        if response.clicked() {
            action = Some(RefAction::Select);
        }
        x = pill.right() + 4.0;
        shown += 1;
    }
    if refs.len() > shown {
        let more = format!("+{}", refs.len() - shown);
        let galley = painter.layout_no_wrap(
            more,
            font,
            ui.visuals().weak_text_color().gamma_multiply(0.9),
        );
        let width = galley.size().x + 8.0;
        if x + width <= inner.right() {
            let rect = egui::Rect::from_min_size(
                egui::pos2(x, inner.center().y - 7.5),
                egui::vec2(width, 15.0),
            );
            painter.rect_filled(
                rect,
                4.0,
                ui.visuals()
                    .widgets
                    .noninteractive
                    .bg_fill
                    .gamma_multiply(0.7),
            );
            painter.galley(
                egui::pos2(rect.left() + 4.0, rect.center().y - galley.size().y / 2.0),
                galley,
                ui.visuals().weak_text_color(),
            );
            x = rect.right() + 4.0;
        }
    }
    let subject_rect = egui::Rect::from_min_max(
        egui::pos2(x + if shown > 0 { 2.0 } else { 0.0 }, rect.top()),
        egui::pos2(rect.right() - 6.0, rect.bottom()),
    );
    crate::bounded_text::paint(
        ui,
        subject_rect,
        commit.subject(),
        egui::FontId::proportional(12.0),
        subject_color,
    );
    action
}

fn ref_label_colors(
    visuals: &egui::Visuals,
    kind: gg_engine::refs::RefKind,
    selected: bool,
) -> (egui::Color32, egui::Color32) {
    let base = match kind {
        gg_engine::refs::RefKind::Branch => egui::Color32::from_rgb(112, 166, 242),
        gg_engine::refs::RefKind::Remote => egui::Color32::from_rgb(160, 164, 176),
        gg_engine::refs::RefKind::Tag => egui::Color32::from_rgb(224, 178, 96),
    };
    // Dark mode uses the mockup's pill colors: gold tags, blue branches.
    if visuals.dark_mode {
        match kind {
            gg_engine::refs::RefKind::Tag => {
                return (
                    egui::Color32::from_rgb(0x4d, 0x3a, 0x14),
                    egui::Color32::from_rgb(0xf0, 0xc3, 0x5a),
                );
            }
            gg_engine::refs::RefKind::Branch => {
                return (
                    egui::Color32::from_rgb(0x1d, 0x3a, 0x57),
                    egui::Color32::from_rgb(0x9c, 0xca, 0xf5),
                );
            }
            gg_engine::refs::RefKind::Remote => {}
        }
    }
    let foreground = if selected {
        visuals.strong_text_color()
    } else {
        base.gamma_multiply(if visuals.dark_mode { 1.15 } else { 0.75 })
    };
    let background = base.gamma_multiply(if visuals.dark_mode {
        if selected { 0.28 } else { 0.20 }
    } else {
        0.16
    });
    (background, foreground)
}

pub fn row(
    ui: &mut egui::Ui,
    commit: &Commit,
    graph: Option<&crate::graph::Row>,
    refs: &[gg_engine::refs::RefItem],
    head_name: Option<&str>,
    selected: bool,
    avatars: &mut Avatars,
) -> (egui::Response, Option<RefAction>) {
    let response = ui.add_sized(
        [ui.available_width(), crate::density::row()],
        egui::Button::new("")
            .fill(if selected {
                crate::colors::selection(ui.visuals())
            } else {
                egui::Color32::TRANSPARENT
            })
            .stroke(egui::Stroke::NONE)
            .corner_radius(0.0)
            .selected(selected),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::Role::ListItem,
            ui.is_enabled(),
            selected,
            commit.subject(),
        )
    });
    let [dot, subject, author, hash, date] = columns(response.rect);
    let color = if selected {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().strong_text_color().gamma_multiply(0.8)
    };
    let muted = if selected {
        color
    } else {
        ui.visuals().text_color()
    };
    if let Some(graph) = graph {
        crate::graph::paint(ui, dot, graph, selected);
    } else {
        ui.painter()
            .circle_filled(dot.center(), 3.0, ui.visuals().selection.stroke.color);
        if commit.parents.len() > 1 {
            ui.painter().circle_stroke(
                dot.center(),
                6.0,
                egui::Stroke::new(1.0_f32, ui.visuals().selection.stroke.color),
            );
        }
    }
    let action = paint_refs_and_subject(ui, subject, refs, head_name, commit, selected);
    let avatar = egui::Rect::from_center_size(
        egui::pos2(author.left() + 14.0, author.center().y),
        egui::vec2(18.0, 18.0),
    );
    avatars.paint(ui, &commit.email, &commit.author, avatar);
    text(
        ui,
        egui::Rect::from_min_max(egui::pos2(author.left() + 25.0, author.top()), author.max),
        &commit.author,
        muted,
        false,
    );
    text(
        ui,
        hash,
        &commit.id.chars().take(8).collect::<String>(),
        muted,
        true,
    );
    let short_date = commit.time.split_whitespace().next().unwrap_or_default();
    text(ui, date, short_date, muted, false);
    let refs = refs
        .iter()
        .map(|item| item.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let response = response.on_hover_text(format!(
        "{}{}\n{}\n{}\n{}",
        if refs.is_empty() {
            String::new()
        } else {
            format!("{refs}\n")
        },
        commit.subject(),
        commit.author,
        commit.id,
        commit.time
    ));
    (response, action)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cells_are_disjoint_and_contained_at_all_supported_widths() {
        for width in [MIN_WIDTH, 720.0, 1120.0, 2560.0] {
            let rect =
                egui::Rect::from_min_size(egui::pos2(17.0, 30.0), egui::vec2(width, ROW_HEIGHT));
            let cells = columns(rect);
            for cell in cells {
                assert!(rect.contains_rect(cell));
                assert_eq!(cell.height(), ROW_HEIGHT);
            }
            for pair in cells.windows(2) {
                assert!(pair[0].right() <= pair[1].left());
            }
            assert!(cells[1].width() >= 200.0);
        }
    }
    #[test]
    fn ref_labels_prefix_the_subject() {
        let ctx = egui::Context::default();
        let commit = Commit {
            id: "a".repeat(40),
            parents: vec![],
            author: "Ada".into(),
            email: String::new(),
            time: "2026-10-17 12:30:00 +0200".into(),
            message: "Subject".into(),
        };
        let refs = [gg_engine::refs::RefItem {
            name: "main".into(),
            full_name: "refs/heads/main".into(),
            kind: gg_engine::refs::RefKind::Branch,
            commit: commit.id.clone(),
        }];
        let mut avatars = Avatars::default();
        let output = crate::tests::run_ui(
            &ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(720.0, 480.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let _ = row(ui, &commit, None, &refs, None, false, &mut avatars);
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
        assert!(text.iter().any(|line| line.contains("main")));
        assert!(text.iter().any(|line| line.contains("Subject")));
        assert!(!text.iter().any(|line| line.contains("[main] Subject")));
    }

    fn click_pill_twice(
        refs: &[gg_engine::refs::RefItem],
        head_name: Option<&str>,
    ) -> (egui::Rect, Option<RefAction>, Option<RefAction>) {
        let ctx = egui::Context::default();
        let commit = Commit {
            id: "a".repeat(40),
            parents: vec![],
            author: "Ada".into(),
            email: String::new(),
            time: "2026-10-17 12:30:00 +0200".into(),
            message: "Subject".into(),
        };
        let mut avatars = Avatars::default();
        let mut run = |events: Vec<egui::Event>, time: f64| {
            let mut row_rect = egui::Rect::NOTHING;
            let mut action = None;
            let _ = crate::tests::run_ui(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(720.0, 480.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let (response, next) =
                            row(ui, &commit, None, refs, head_name, false, &mut avatars);
                        row_rect = response.rect;
                        action = next;
                    });
                },
            );
            (row_rect, action)
        };
        let (row_rect, _) = run(vec![], 1.0);
        let point = egui::pos2(row_rect.left() + 110.0, row_rect.center().y);
        run(vec![egui::Event::PointerMoved(point)], 1.05);
        run(
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            1.10,
        );
        let (_, first) = run(
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            1.15,
        );
        run(
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            1.20,
        );
        let (_, second) = run(
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            1.25,
        );
        (row_rect, first, second)
    }

    #[test]
    fn clicking_a_branch_pill_selects_its_commit_without_switching() {
        let id = "a".repeat(40);
        let refs = [gg_engine::refs::RefItem {
            name: "main".into(),
            full_name: "refs/heads/main".into(),
            kind: gg_engine::refs::RefKind::Branch,
            commit: id.clone(),
        }];
        let (_, first, second) = click_pill_twice(&refs, None);
        assert_eq!(first, Some(RefAction::Select));
        assert_eq!(second, Some(RefAction::Select));
    }

    #[test]
    fn clicking_a_tag_pill_selects_its_commit() {
        let id = "b".repeat(40);
        let refs = [gg_engine::refs::RefItem {
            name: "v1.0".into(),
            full_name: "refs/tags/v1.0".into(),
            kind: gg_engine::refs::RefKind::Tag,
            commit: id.clone(),
        }];
        let (_, first, second) = click_pill_twice(&refs, None);
        assert_eq!(first, Some(RefAction::Select));
        assert_eq!(second, Some(RefAction::Select));
    }

    #[test]
    fn clicking_the_current_branch_pill_only_selects_it() {
        let id = "a".repeat(40);
        let refs = [gg_engine::refs::RefItem {
            name: "main".into(),
            full_name: "refs/heads/main".into(),
            kind: gg_engine::refs::RefKind::Branch,
            commit: id,
        }];
        let (_, _, second) = click_pill_twice(&refs, Some("refs/heads/main"));
        assert_eq!(second, Some(RefAction::Select));
    }

    #[test]
    fn extra_ref_labels_show_an_overflow_count() {
        let ctx = egui::Context::default();
        let commit = Commit {
            id: "a".repeat(40),
            parents: vec![],
            author: "Ada".into(),
            email: String::new(),
            time: "2026-10-17 12:30:00 +0200".into(),
            message: "Subject".into(),
        };
        let refs = (0..5)
            .map(|index| gg_engine::refs::RefItem {
                name: format!("ref{index}"),
                full_name: format!("refs/heads/ref{index}"),
                kind: gg_engine::refs::RefKind::Branch,
                commit: commit.id.clone(),
            })
            .collect::<Vec<_>>();
        let mut avatars = Avatars::default();
        let output = crate::tests::run_ui(
            &ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(720.0, 480.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let _ = row(ui, &commit, None, &refs, None, false, &mut avatars);
                });
            },
        );
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text) if text.galley.job.text.contains("+1")
        )));
    }

    #[test]
    fn tags_sort_first_then_the_current_branch() {
        let id = "a".repeat(40);
        let mut refs = [
            gg_engine::refs::RefItem {
                name: "zzz".into(),
                full_name: "refs/heads/zzz".into(),
                kind: gg_engine::refs::RefKind::Branch,
                commit: id.clone(),
            },
            gg_engine::refs::RefItem {
                name: "v1.0".into(),
                full_name: "refs/tags/v1.0".into(),
                kind: gg_engine::refs::RefKind::Tag,
                commit: id.clone(),
            },
            gg_engine::refs::RefItem {
                name: "main".into(),
                full_name: "refs/heads/main".into(),
                kind: gg_engine::refs::RefKind::Branch,
                commit: id,
            },
        ];
        sort_labels(&mut refs, Some("refs/heads/main"));
        assert_eq!(refs[0].name, "v1.0");
        assert_eq!(refs[1].name, "main");
        assert_eq!(refs[2].name, "zzz");
    }

    #[test]
    fn long_fields_stay_single_line_at_supported_scales() {
        for scale in [1.0, 2.0] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(scale);
            let commit = Commit {
                id: "a".repeat(40),
                parents: vec![],
                author: "Long author\nname ".repeat(20),
                email: "test@example.org".into(),
                time: "2026-10-17 12:30:00 +0200".into(),
                message: "Long subject ".repeat(100),
            };
            let mut avatars = Avatars::default();
            let output = crate::tests::run_ui(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(720.0, 480.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let (response, _) = row(ui, &commit, None, &[], None, true, &mut avatars);
                        assert_eq!(response.rect.height(), ROW_HEIGHT);
                    });
                },
            );
            let mut count = 0;
            for shape in &output.shapes {
                if let egui::epaint::Shape::Text(text) = &shape.shape {
                    if text.galley.job.text.is_empty() {
                        continue;
                    }
                    assert_eq!(text.galley.rows.len(), 1);
                    assert!(text.galley.size().x <= shape.clip_rect.width() + 1.0);
                    count += 1;
                }
            }
            assert!(count >= 5); // Fields, avatar fallback, and overflow controls.
        }
    }
}
