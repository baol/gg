//! Incremental commit-lane layout for the history graph.
use eframe::egui;
use gg_engine::Commit;

const MAX_LANES: usize = 64;
const COLORS: [egui::Color32; 8] = [
    egui::Color32::from_rgb(96, 170, 255),
    egui::Color32::from_rgb(255, 176, 96),
    egui::Color32::from_rgb(150, 220, 120),
    egui::Color32::from_rgb(230, 130, 210),
    egui::Color32::from_rgb(110, 220, 220),
    egui::Color32::from_rgb(240, 140, 140),
    egui::Color32::from_rgb(190, 170, 250),
    egui::Color32::from_rgb(210, 210, 120),
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Row {
    pub lane: u8,
    pub before: u64,
    pub after: u64,
    pub parents: Vec<u8>,
}

#[derive(Default)]
pub struct Graph {
    lanes: Vec<Option<String>>,
}

impl Graph {
    pub fn push(&mut self, commit: &Commit) -> Row {
        let lane = self
            .lanes
            .iter()
            .position(|lane| lane.as_deref() == Some(commit.id.as_str()))
            .or_else(|| self.lanes.iter().position(Option::is_none))
            .unwrap_or_else(|| {
                if self.lanes.len() < MAX_LANES {
                    self.lanes.push(None);
                }
                self.lanes.len().saturating_sub(1).min(MAX_LANES - 1)
            });
        let before = mask(&self.lanes);
        if let Some(slot) = self.lanes.get_mut(lane) {
            *slot = None;
        }
        let mut parents = Vec::with_capacity(commit.parents.len());
        for (position, parent) in commit.parents.iter().enumerate() {
            let target = if let Some(existing) = self
                .lanes
                .iter()
                .position(|lane| lane.as_deref() == Some(parent.as_str()))
            {
                existing
            } else {
                let target = if position == 0 {
                    lane
                } else {
                    self.lanes
                        .iter()
                        .position(Option::is_none)
                        .unwrap_or_else(|| {
                            if self.lanes.len() < MAX_LANES {
                                self.lanes.push(None);
                            }
                            self.lanes.len().saturating_sub(1).min(MAX_LANES - 1)
                        })
                };
                if let Some(slot) = self.lanes.get_mut(target) {
                    *slot = Some(parent.clone());
                }
                target
            };
            parents.push(target.min(MAX_LANES - 1) as u8);
        }
        while self.lanes.last().is_some_and(Option::is_none) {
            self.lanes.pop();
        }
        Row {
            lane: lane.min(MAX_LANES - 1) as u8,
            before,
            after: mask(&self.lanes),
            parents,
        }
    }
}

fn mask(lanes: &[Option<String>]) -> u64 {
    let mut mask = 0;
    for (index, lane) in lanes.iter().enumerate().take(MAX_LANES) {
        if lane.is_some() {
            mask |= 1 << index;
        }
    }
    mask
}

fn x(rect: egui::Rect, lane: u8) -> f32 {
    rect.left() + 9.0 + lane as f32 * 11.0
}

fn color(lane: u8) -> egui::Color32 {
    COLORS[lane as usize % COLORS.len()]
}

fn parent_color(row: &Row, parent: u8) -> egui::Color32 {
    let lane_was_active = row.before & (1 << row.lane) != 0;
    let parent_was_active = row.before & (1 << parent) != 0;
    let new_single_parent_tip = row.parents.len() == 1 && !lane_was_active;
    if new_single_parent_tip && parent_was_active {
        color(row.lane)
    } else {
        color(parent)
    }
}

fn starts_below(row: &Row, lane: u8) -> bool {
    row.after & (1 << lane) != 0 && lane == row.lane
}

pub fn paint(ui: &egui::Ui, rect: egui::Rect, row: &Row, selected: bool) {
    let painter = ui.painter().with_clip_rect(rect);
    let center = egui::pos2(x(rect, row.lane), rect.center().y);
    for lane in 0..MAX_LANES as u8 {
        let before = row.before & (1 << lane) != 0;
        let after = row.after & (1 << lane) != 0;
        if before && after {
            painter.line_segment(
                [
                    egui::pos2(x(rect, lane), rect.top()),
                    egui::pos2(x(rect, lane), rect.bottom()),
                ],
                egui::Stroke::new(1.75, color(lane)),
            );
        } else if before {
            painter.line_segment(
                [
                    egui::pos2(x(rect, lane), rect.top()),
                    egui::pos2(x(rect, lane), center.y),
                ],
                egui::Stroke::new(1.75, color(lane)),
            );
        } else if starts_below(row, lane) {
            painter.line_segment(
                [
                    egui::pos2(x(rect, lane), center.y),
                    egui::pos2(x(rect, lane), rect.bottom()),
                ],
                egui::Stroke::new(1.75, color(lane)),
            );
        }
    }
    let join = center.y + (rect.bottom() - center.y) * 0.62;
    for parent in &row.parents {
        if *parent == row.lane {
            continue;
        }
        let end = egui::pos2(x(rect, *parent), rect.bottom());
        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
            [
                center,
                egui::pos2(center.x, join),
                egui::pos2(end.x, join),
                end,
            ],
            false,
            egui::Color32::TRANSPARENT,
            egui::Stroke::new(1.75, parent_color(row, *parent)),
        ));
    }
    if selected {
        painter.circle_filled(center, 6.0, color(row.lane).gamma_multiply(0.28));
    }
    painter.circle_filled(center, if selected { 4.0 } else { 3.25 }, color(row.lane));
    if row.parents.len() > 1 {
        painter.circle_stroke(center, 6.0, egui::Stroke::new(1.0, color(row.lane)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(id: &str, parents: &[&str]) -> Commit {
        Commit {
            id: id.into(),
            parents: parents.iter().map(|parent| (*parent).into()).collect(),
            author: String::new(),
            email: String::new(),
            time: String::new(),
            message: String::new(),
        }
    }

    #[test]
    fn linear_history_stays_in_lane_zero() {
        let mut graph = Graph::default();
        let rows = [
            graph.push(&commit("c", &["b"])),
            graph.push(&commit("b", &["a"])),
            graph.push(&commit("a", &[])),
        ];
        assert_eq!(rows[0].lane, 0);
        assert_eq!(rows[0].parents, vec![0]);
        assert_eq!(rows[1].before, 1);
        assert_eq!(rows[2].after, 0);
    }

    #[test]
    fn merge_parents_get_distinct_lanes() {
        let mut graph = Graph::default();
        let merge = graph.push(&commit("m", &["a", "b"]));
        let first = graph.push(&commit("a", &["c"]));
        let second = graph.push(&commit("b", &["c"]));
        let root = graph.push(&commit("c", &[]));
        assert_eq!(merge.parents.len(), 2);
        assert_ne!(merge.parents[0], merge.parents[1]);
        assert!(second.before & (1 << second.lane) != 0);
        assert_eq!(root.after, 0);
        assert!(first.lane <= 1);
    }

    #[test]
    fn a_new_branch_tip_uses_its_own_color_until_it_joins_an_existing_lane() {
        let mut graph = Graph::default();
        let first = graph.push(&commit("first", &["root"]));
        let second = graph.push(&commit("second", &["root"]));
        assert_eq!(first.lane, 0);
        assert_eq!(second.lane, 1);
        assert!(second.before & (1 << first.lane) != 0);
        assert_eq!(second.parents, vec![first.lane]);
        assert_eq!(parent_color(&second, first.lane), color(second.lane));
    }

    #[test]
    fn merge_parents_keep_the_color_of_the_lane_they_start() {
        let mut graph = Graph::default();
        let merge = graph.push(&commit("merge", &["first", "second"]));
        let second = merge.parents[1];
        assert_ne!(second, merge.lane);
        assert_eq!(parent_color(&merge, second), color(second));

        let active_parent = Row {
            lane: 0,
            before: 1 << second,
            after: 1 | (1 << second),
            parents: vec![0, second],
        };
        assert_eq!(parent_color(&active_parent, second), color(second));
    }

    #[test]
    fn only_the_rows_own_lane_starts_below_the_commit() {
        let mut graph = Graph::default();
        let merge = graph.push(&commit("merge", &["main", "side"]));
        let side = merge.parents[1];
        assert_ne!(side, merge.lane);
        assert!(merge.after & (1 << side) != 0);
        assert!(!starts_below(&merge, side));
        assert!(starts_below(&merge, merge.lane));
    }
}
