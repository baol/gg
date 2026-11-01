//! Per-block conflict resolution: each block shows Base, Ours, and Theirs and
//! takes one of four choices (keys 1–4 on the current block; `[` and `]` move
//! between blocks). The result
//! preview keeps markers for blocks without a choice.

use eframe::egui;
use gg_engine::conflict::{BlockChoice, Segment, block_count, compose};

pub struct Blocks {
    segments: Vec<Segment>,
    pub choices: Vec<Option<BlockChoice>>,
    /// Block that keys 1–4 act on.
    pub current: usize,
}

/// Column titles for one block, such as `Base`, `Ours (main)` and
/// `Theirs (feature/login)`, shortened to keep the layout; the `_full`
/// forms are shown on hover and name the sides in the choice tooltips.
pub struct Labels {
    pub base: String,
    pub ours: String,
    pub theirs: String,
    pub ours_full: String,
    pub theirs_full: String,
}

impl Labels {
    /// What a block choice takes, naming both sides.
    pub fn describe(&self, choice: BlockChoice) -> String {
        let (ours, theirs) = (&self.ours_full, &self.theirs_full);
        match choice {
            BlockChoice::Ours => ours.clone(),
            BlockChoice::Theirs => theirs.clone(),
            BlockChoice::OursThenTheirs => format!("{ours}, then {theirs}"),
            BlockChoice::TheirsThenOurs => format!("{theirs}, then {ours}"),
        }
    }
}

impl Blocks {
    pub fn new(segments: Vec<Segment>) -> Self {
        let count = block_count(&segments);
        Self {
            segments,
            choices: vec![None; count],
            current: 0,
        }
    }

    pub fn count(&self) -> usize {
        self.choices.len()
    }

    /// Blocks still without a choice.
    pub fn left(&self) -> usize {
        self.count() - self.chosen()
    }

    /// The line where each block starts in our version of the file (1-based):
    /// common lines plus our side of the blocks before it.
    pub fn start_lines(&self) -> Vec<usize> {
        let mut line = 1;
        let mut starts = Vec::new();
        for segment in &self.segments {
            match segment {
                Segment::Common(text) => line += text.lines().count(),
                Segment::Block { ours, .. } => {
                    starts.push(line);
                    line += ours.lines().count();
                }
            }
        }
        starts
    }

    pub fn chosen(&self) -> usize {
        self.choices.iter().flatten().count()
    }

    /// Every block has a choice, so the result has no markers left.
    pub fn complete(&self) -> bool {
        self.chosen() == self.count()
    }

    pub fn text(&self) -> String {
        compose(&self.segments, &self.choices)
    }

    /// Choose for block `index`, then move to the next block without a choice.
    pub fn choose(&mut self, index: usize, choice: BlockChoice) {
        let Some(slot) = self.choices.get_mut(index) else {
            return;
        };
        *slot = Some(choice);
        let count = self.count();
        self.current = (1..=count)
            .map(|step| (index + step) % count)
            .find(|&next| self.choices[next].is_none())
            .unwrap_or(index);
    }

    /// Move the current block by `step`, wrapping at either end.
    pub fn step(&mut self, step: isize) {
        let count = self.count() as isize;
        if count > 0 {
            self.current = (self.current as isize + step).rem_euclid(count) as usize;
        }
    }

    /// Keys 1–4 choose for the current block; `[` and `]` move between blocks.
    pub fn keys(&mut self, ctx: &egui::Context) {
        for (key, step) in [(egui::Key::OpenBracket, -1), (egui::Key::CloseBracket, 1)] {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                self.step(step);
            }
        }
        for (key, choice) in [
            egui::Key::Num1,
            egui::Key::Num2,
            egui::Key::Num3,
            egui::Key::Num4,
        ]
        .into_iter()
        .zip(BlockChoice::ALL)
        {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                self.choose(self.current, choice);
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, labels: &Labels) {
        let count = self.count();
        let mut chosen = None;
        let mut focus = None;
        let wide = ui.available_width() >= 560.0;
        let starts = self.start_lines();
        egui::ScrollArea::vertical()
            .id_salt("conflict-blocks")
            .max_height((ui.available_height() - 190.0).max(160.0))
            .min_scrolled_height((ui.available_height() - 190.0).max(160.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut index = 0;
                for segment in &self.segments {
                    let Segment::Block { ours, base, theirs } = segment else {
                        continue;
                    };
                    let current = index == self.current;
                    let stroke = if current {
                        ui.visuals().selection.stroke
                    } else {
                        ui.visuals().widgets.noninteractive.bg_stroke
                    };
                    let frame = egui::Frame::group(ui.style())
                        .stroke(stroke)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.strong(format!("Conflict {} of {count}", index + 1));
                                if let Some(line) = starts.get(index) {
                                    ui.weak(format!("line {line}"));
                                }
                                if self.choices[index].is_none() {
                                    ui.colored_label(ui.visuals().warn_fg_color, "unresolved");
                                }
                                // The choices at the right, as in the mockup;
                                // the chosen one is filled.
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        for (key, choice) in
                                            BlockChoice::ALL.into_iter().enumerate().rev()
                                        {
                                            let selected = self.choices[index] == Some(choice);
                                            let mut button = egui::Button::new(choice.label())
                                                .min_size(egui::vec2(0.0, 20.0));
                                            if selected {
                                                button = button
                                                    .fill(egui::Color32::from_rgb(11, 85, 119));
                                            }
                                            let response = ui.add(button).on_hover_text(format!(
                                                "{} · key {} on the current block",
                                                labels.describe(choice),
                                                key + 1
                                            ));
                                            if response.clicked() {
                                                chosen = Some((index, choice));
                                            }
                                        }
                                    },
                                );
                            });
                            let columns = [
                                ((labels.base.as_str(), labels.base.as_str()), base),
                                ((labels.ours.as_str(), labels.ours_full.as_str()), ours),
                                (
                                    (labels.theirs.as_str(), labels.theirs_full.as_str()),
                                    theirs,
                                ),
                            ];
                            // Ours in blue and Theirs in amber, as in the mockup.
                            let side =
                                |ui: &mut egui::Ui, (title, full): (&str, &str), text: &str| {
                                    let color = if title.starts_with("Ours") {
                                        egui::Color32::from_rgb(156, 202, 245)
                                    } else if title.starts_with("Theirs") {
                                        egui::Color32::from_rgb(240, 166, 78)
                                    } else {
                                        ui.visuals().weak_text_color()
                                    };
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(title).small().color(color),
                                        )
                                        .truncate(),
                                    )
                                    .on_hover_text(full);
                                    let text = if text.is_empty() { "(no lines)" } else { text };
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(text.trim_end()).monospace(),
                                        )
                                        .extend(),
                                    );
                                };
                            if wide {
                                ui.columns(3, |cols| {
                                    for (col, (title, text)) in cols.iter_mut().zip(columns) {
                                        side(col, title, text);
                                    }
                                });
                            } else {
                                for (title, text) in columns {
                                    side(ui, title, text);
                                }
                            }
                        });
                    if frame.response.interact(egui::Sense::click()).clicked() {
                        focus = Some(index);
                    }
                    index += 1;
                }
            });
        if let Some(index) = focus {
            self.current = index;
        }
        if let Some((index, choice)) = chosen {
            self.choose(index, choice);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocks() -> Blocks {
        let block = |n: &str| Segment::Block {
            ours: format!("ours {n}\n"),
            base: format!("base {n}\n"),
            theirs: format!("theirs {n}\n"),
        };
        Blocks::new(vec![
            Segment::Common("top\n".into()),
            block("1"),
            block("2"),
            block("3"),
        ])
    }

    #[test]
    fn choosing_moves_to_the_next_open_block_and_completes() {
        let mut blocks = blocks();
        assert_eq!(blocks.count(), 3);
        blocks.choose(1, BlockChoice::Theirs);
        assert_eq!(blocks.current, 2);
        blocks.choose(2, BlockChoice::Ours);
        assert_eq!(blocks.current, 0, "wraps to the first open block");
        assert!(!blocks.complete());
        assert!(blocks.text().contains("<<<<<<< ours\nours 1\n"));
        blocks.choose(0, BlockChoice::OursThenTheirs);
        assert!(blocks.complete());
        assert_eq!(blocks.text(), "top\nours 1\ntheirs 1\ntheirs 2\nours 3\n");
    }

    #[test]
    fn blocks_know_their_start_lines_and_how_many_are_left() {
        let mut blocks = blocks();
        // "top" is line 1; each block has one line on our side.
        assert_eq!(blocks.start_lines(), [2, 3, 4]);
        assert_eq!(blocks.left(), 3);
        blocks.choose(0, BlockChoice::Ours);
        assert_eq!(blocks.left(), 2);
    }

    #[test]
    fn choice_tooltips_name_both_sides() {
        let labels = Labels {
            base: "Base".into(),
            ours: "Ours (main)".into(),
            theirs: "Theirs (feature/login)".into(),
            ours_full: "Ours (main)".into(),
            theirs_full: "Theirs (feature/login)".into(),
        };
        assert_eq!(labels.describe(BlockChoice::Ours), "Ours (main)");
        assert_eq!(
            labels.describe(BlockChoice::TheirsThenOurs),
            "Theirs (feature/login), then Ours (main)"
        );
    }

    #[test]
    fn brackets_move_between_blocks_and_wrap() {
        let mut blocks = blocks();
        blocks.step(-1);
        assert_eq!(blocks.current, 2);
        blocks.step(1);
        assert_eq!(blocks.current, 0);
        blocks.step(1);
        assert_eq!(blocks.current, 1);
    }

    #[test]
    fn number_keys_choose_for_the_current_block() {
        let ctx = egui::Context::default();
        let mut blocks = blocks();
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Num4,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let _ = crate::tests::run_ui(&ctx, input, |ui| blocks.keys(ui.ctx()));
        assert_eq!(blocks.choices[0], Some(BlockChoice::TheirsThenOurs));
        assert_eq!(blocks.current, 1);
    }
}
