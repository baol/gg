//! Blame in Full file: runs of lines from one commit collapse to one label
//! (author, short ID, relative age) with an age bar (brighter = newer), and
//! the selected line's commit sits in one 26 px metadata bar at the reader
//! bottom.
use eframe::egui;
use gg_engine::blame::{Blame, Line};

/// Width of the blame gutter in front of the line numbers.
pub const GUTTER: f32 = 184.0;
/// Height of the metadata bar at the reader bottom.
pub const BAR: f32 = 26.0;

/// True when `line` starts a run: the first attributed line, or a line
/// whose commit differs from the line above.
pub fn run_start(blame: &Blame, line: usize) -> bool {
    let Some(entry) = blame.line(line) else {
        return false;
    };
    line <= blame.start_line
        || blame
            .line(line - 1)
            .is_none_or(|above| above.commit != entry.commit)
}

/// The oldest and newest author times in the attribution.
pub fn span(blame: &Blame) -> Option<(i64, i64)> {
    let oldest = blame.lines.iter().map(|line| line.time).min()?;
    let newest = blame.lines.iter().map(|line| line.time).max()?;
    Some((oldest, newest))
}

/// 0 for the oldest commit, 1 for the newest.
pub fn freshness(time: i64, (oldest, newest): (i64, i64)) -> f32 {
    if newest <= oldest {
        1.0
    } else {
        ((time - oldest) as f32 / (newest - oldest) as f32).clamp(0.0, 1.0)
    }
}

/// The age bar's colour: dim for old commits, bright for new ones.
pub fn age_color(freshness: f32) -> egui::Color32 {
    let old = [46.0, 74.0, 99.0];
    let new = [122.0, 184.0, 245.0];
    let mix = |index: usize| (old[index] + (new[index] - old[index]) * freshness).round() as u8;
    egui::Color32::from_rgb(mix(0), mix(1), mix(2))
}

/// `5d`, `3w`: the gutter's compact age.
pub fn short_age(seconds: i64, now: i64) -> String {
    let delta = (now - seconds).max(0);
    match delta {
        d if d < 3_600 => format!("{}m", d / 60),
        d if d < 86_400 => format!("{}h", d / 3_600),
        d if d < 14 * 86_400 => format!("{}d", d / 86_400),
        d if d < 60 * 86_400 => format!("{}w", d / (7 * 86_400)),
        d if d < 365 * 86_400 => format!("{}mo", d / (30 * 86_400)),
        d => format!("{}y", d / (365 * 86_400)),
    }
}

/// `5 days ago`: the metadata bar's age.
pub fn long_age(seconds: i64, now: i64) -> String {
    let delta = (now - seconds).max(0);
    let (count, unit) = match delta {
        d if d < 60 => return "just now".into(),
        d if d < 3_600 => (d / 60, "minute"),
        d if d < 86_400 => (d / 3_600, "hour"),
        d if d < 14 * 86_400 => (d / 86_400, "day"),
        d if d < 60 * 86_400 => (d / (7 * 86_400), "week"),
        d if d < 365 * 86_400 => (d / (30 * 86_400), "month"),
        d => (d / (365 * 86_400), "year"),
    };
    format!("{count} {unit}{} ago", if count == 1 { "" } else { "s" })
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

/// Paint one row's gutter: the age bar always, the label only on a run's
/// first line, and a rule above every run but the first.
pub fn paint_gutter(
    ui: &egui::Ui,
    row: egui::Rect,
    entry: &Line,
    start: bool,
    first: bool,
    span: (i64, i64),
    now: i64,
) {
    let painter = ui.painter();
    let bar = egui::Rect::from_min_size(
        row.min + egui::vec2(2.0, 0.0),
        egui::vec2(3.0, row.height()),
    );
    painter.rect_filled(bar, 0.0, age_color(freshness(entry.time, span)));
    if !start {
        return;
    }
    if !first {
        painter.line_segment(
            [row.left_top(), egui::pos2(row.left() + GUTTER, row.top())],
            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        );
    }
    let weak = ui.visuals().weak_text_color();
    let slot = |from: f32, to: f32| {
        egui::Rect::from_min_max(
            egui::pos2(row.left() + from, row.top()),
            egui::pos2(row.left() + to, row.bottom()),
        )
    };
    crate::bounded_text::paint(
        ui,
        slot(10.0, 90.0),
        &entry.author,
        egui::FontId::proportional(11.0),
        ui.visuals().text_color(),
    );
    painter.with_clip_rect(slot(94.0, 150.0)).text(
        slot(94.0, 150.0).left_center(),
        egui::Align2::LEFT_CENTER,
        entry.short_commit(),
        egui::FontId::monospace(10.0),
        weak,
    );
    painter.with_clip_rect(slot(150.0, GUTTER - 6.0)).text(
        slot(150.0, GUTTER - 6.0).right_center(),
        egui::Align2::RIGHT_CENTER,
        short_age(entry.time, now),
        egui::FontId::proportional(10.0),
        weak,
    );
}

#[derive(Debug, PartialEq, Eq)]
pub enum Act {
    OpenCommit,
    LineHistory,
}

/// The 26 px bar: `LINE 42 · author · id · 5 days ago · subject`, then
/// **Open commit** and **Line history**. Without a selected line it names
/// how to choose one; its height never changes.
pub fn bar(ui: &mut egui::Ui, line: Option<(&Line, (usize, usize))>, now: i64) -> Option<Act> {
    let mut act = None;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), BAR), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.left_top(), rect.right_top()],
        egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(egui::vec2(6.0, 1.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let ui = &mut child;
    let Some((entry, (start, end))) = line else {
        ui.weak("Select a line to see the commit that last changed it");
        return None;
    };
    ui.label(
        egui::RichText::new(if start == end {
            format!("LINE {start}")
        } else {
            format!("LINES {start}–{end}")
        })
        .small()
        .weak(),
    );
    let buttons = 196.0;
    let width = (ui.available_width() - buttons).max(60.0);
    let (text, _) = ui.allocate_exact_size(egui::vec2(width, BAR - 2.0), egui::Sense::hover());
    let summary = format!(
        "{} · {} · {} · {}",
        entry.author,
        entry.short_commit(),
        long_age(entry.time, now),
        entry.summary
    );
    crate::bounded_text::paint(
        ui,
        text,
        &summary,
        egui::FontId::proportional(12.0),
        ui.visuals().text_color(),
    );
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if ui
            .button("Line history")
            .on_hover_text("Follow these lines through earlier versions")
            .clicked()
        {
            act = Some(Act::LineHistory);
        }
        if ui
            .button("Open commit")
            .on_hover_text(&entry.commit)
            .clicked()
        {
            act = Some(Act::OpenCommit);
        }
    });
    act
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(line: usize, commit: &str, time: i64) -> Line {
        Line {
            line,
            origin_line: line,
            commit: commit.repeat(40 / commit.len().max(1)),
            author: "Ada".into(),
            email: "ada@example.org".into(),
            time,
            summary: "Wire action chips".into(),
        }
    }

    fn blame(lines: Vec<Line>) -> Blame {
        Blame {
            revision: "HEAD".into(),
            path: "a".into(),
            path_bytes: b"a".to_vec(),
            start_line: 1,
            end_line: lines.len(),
            lines,
            complete: true,
            note: None,
        }
    }

    #[test]
    fn runs_collapse_to_one_label_per_commit() {
        let blame = blame(vec![
            line(1, "a", 10),
            line(2, "a", 10),
            line(3, "b", 20),
            line(4, "a", 10),
        ]);
        let starts = (1..=4).map(|n| run_start(&blame, n)).collect::<Vec<_>>();
        assert_eq!(starts, [true, false, true, true]);
        assert!(!run_start(&blame, 9), "an unattributed line starts nothing");
    }

    #[test]
    fn newer_commits_draw_brighter_age_bars() {
        let span = (100, 200);
        assert_eq!(freshness(100, span), 0.0);
        assert_eq!(freshness(200, span), 1.0);
        assert_eq!(freshness(5, (7, 7)), 1.0);
        let brightness = |color: egui::Color32| {
            u32::from(color.r()) + u32::from(color.g()) + u32::from(color.b())
        };
        assert!(brightness(age_color(1.0)) > brightness(age_color(0.5)));
        assert!(brightness(age_color(0.5)) > brightness(age_color(0.0)));
    }

    #[test]
    fn ages_read_compactly_in_the_gutter_and_fully_in_the_bar() {
        let now = 100 * 86_400;
        assert_eq!(short_age(now - 5 * 86_400, now), "5d");
        assert_eq!(short_age(now - 21 * 86_400, now), "3w");
        assert_eq!(short_age(now - 2 * 3_600, now), "2h");
        assert_eq!(long_age(now - 5 * 86_400, now), "5 days ago");
        assert_eq!(long_age(now - 86_400, now), "1 day ago");
        assert_eq!(long_age(now, now), "just now");
    }

    #[test]
    fn the_bar_keeps_its_height_with_and_without_a_line() {
        let ctx = egui::Context::default();
        let entry = line(42, "9a4f9a95", 0);
        for selected in [false, true] {
            let mut height = 0.0;
            let output = crate::tests::run_ui(
                &ctx,
                crate::tests::frame_input(900.0, 200.0, vec![]),
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        let before = ui.cursor().top();
                        let shown = selected.then_some((&entry, (42, 42)));
                        bar(ui, shown, 6 * 86_400);
                        height = ui.cursor().top() - before;
                    });
                },
            );
            assert!((height - BAR).abs() < 8.0 + f32::EPSILON, "{height}");
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if selected {
                assert!(text.contains(&"LINE 42".to_string()), "{text:?}");
                assert!(text.contains(&"Open commit".to_string()));
                assert!(text.contains(&"Line history".to_string()));
            }
        }
    }
}
