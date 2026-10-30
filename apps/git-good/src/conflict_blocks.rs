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

/// Column titles for one block: base, ours, theirs.
pub struct Labels {
    pub base: &'static str,
    pub ours: &'static str,
    pub theirs: &'static str,
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
                                for (key, choice) in BlockChoice::ALL.into_iter().enumerate() {
                                    let selected = self.choices[index] == Some(choice);
                                    let response =
                                        ui.selectable_label(
                                            selected,
                                            format!("{} {}", key + 1, choice.label()),
                                        )
                                        .on_hover_text(
                                            format!("Key {} on the current block", key + 1),
                                        );
                                    if response.clicked() {
                                        chosen = Some((index, choice));
                                    }
                                }
                                if self.choices[index].is_none() {
                                    ui.colored_label(ui.visuals().warn_fg_color, "unresolved");
                                }
                            });
                            let columns = [
                                (labels.base, base),
                                (labels.ours, ours),
                                (labels.theirs, theirs),
                            ];
                            let side = |ui: &mut egui::Ui, title: &str, text: &str| {
                                ui.weak(title);
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
