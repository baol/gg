//! Per-block conflict resolution: each block shows Base, Ours, and Theirs and
//! takes one of four choices (keys 1–4 on the current block; `[` and `]` move
//! between blocks). The choices write into the editable Result
//! ([`crate::conflict_result`]), which keeps markers for blocks without a
//! choice and keeps hand edits.

use crate::conflict_result::{Markers, ResultDoc};
use eframe::egui;
use gg_engine::conflict::{BlockChoice, Segment, block_count};

pub struct Blocks {
    segments: Vec<Segment>,
    pub choices: Vec<Option<BlockChoice>>,
    /// Block that keys 1–4 act on.
    pub current: usize,
    /// The Result as it will be staged.
    pub doc: ResultDoc,
    markers: Markers,
}

/// Column titles for one block, such as `Base`, `Ours · main (current
/// branch)` and `Theirs · origin/main (being merged)`; the `_full` forms are
/// their tooltips, with the stage number.
pub struct Labels {
    pub base: String,
    pub ours: String,
    pub theirs: String,
    pub base_full: String,
    pub ours_full: String,
    pub theirs_full: String,
}

impl Labels {
    /// What a block choice takes, naming both sides.
    pub fn describe(&self, choice: BlockChoice) -> String {
        let (ours, theirs) = (&self.ours, &self.theirs);
        match choice {
            BlockChoice::Ours => ours.clone(),
            BlockChoice::Theirs => theirs.clone(),
            BlockChoice::OursThenTheirs => format!("{ours}, then {theirs}"),
            BlockChoice::TheirsThenOurs => format!("{theirs}, then {ours}"),
        }
    }
}

/// Colors from the conflict mockup.
pub mod palette {
    use eframe::egui::Color32 as C;
    pub const OURS: C = C::from_rgb(156, 202, 245);
    pub const THEIRS: C = C::from_rgb(240, 195, 90);
    pub const BOTH: C = C::from_rgb(150, 220, 170);
    pub const OURS_FILL: C = C::from_rgb(29, 58, 87);
    pub const THEIRS_FILL: C = C::from_rgb(77, 58, 20);
    pub const BOTH_FILL: C = C::from_rgb(47, 58, 47);
    pub const BUTTON: C = C::from_rgb(58, 58, 58);
    pub const TEXT: C = C::from_rgb(230, 230, 230);
    pub const WEAK: C = C::from_rgb(140, 140, 140);
    pub const BASE_TEXT: C = C::from_rgb(176, 176, 176);
    pub const HEADER: C = C::from_rgb(35, 35, 35);
    pub const BASE_BG: C = C::from_rgb(30, 30, 30);
    pub const SIDE_BG: C = C::from_rgb(27, 27, 27);
    pub const OURS_BG: C = C::from_rgb(29, 42, 54);
    pub const THEIRS_BG: C = C::from_rgb(46, 38, 20);
    pub const UNRESOLVED_BG: C = C::from_rgb(51, 39, 15);
    pub const MARKER: C = C::from_rgb(240, 166, 78);
    pub const DIVIDER: C = C::from_rgb(42, 42, 42);
    pub const OPEN_EDGE: C = C::from_rgb(106, 74, 20);
    pub const OPEN_EDGE_CURRENT: C = C::from_rgb(192, 144, 46);
    pub const CHOSEN_EDGE: C = C::from_rgb(47, 90, 58);
    pub const CHOSEN_EDGE_CURRENT: C = C::from_rgb(90, 160, 110);
}

/// The block header row (title, line, options).
pub const HEADER_HEIGHT: f32 = 24.0;
/// Padding inside each side column.
pub const COLUMN_PAD: egui::Vec2 = egui::vec2(8.0, 4.0);
/// Monospace size of the side text.
pub const CODE_SIZE: f32 = 11.5;

/// What happened in the blocks this frame.
#[derive(Default)]
pub struct Events {
    /// An option was clicked for a block.
    pub chosen: Option<(usize, BlockChoice)>,
}

fn option_colors(choice: BlockChoice) -> (egui::Color32, egui::Color32) {
    match choice {
        BlockChoice::Ours => (palette::OURS_FILL, palette::OURS),
        BlockChoice::Theirs => (palette::THEIRS_FILL, palette::THEIRS),
        _ => (palette::BOTH_FILL, palette::BOTH),
    }
}

/// One side's title and text laid out to `width`, wrapping inside it.
fn side_galleys(
    ui: &egui::Ui,
    width: f32,
    title: &str,
    title_color: egui::Color32,
    text: &str,
    text_color: egui::Color32,
) -> (std::sync::Arc<egui::Galley>, std::sync::Arc<egui::Galley>) {
    let inner = (width - 2.0 * COLUMN_PAD.x).max(8.0);
    let title = ui.fonts_mut(|fonts| {
        let mut job = egui::text::LayoutJob::simple_singleline(
            title.to_string(),
            egui::FontId::proportional(11.0),
            title_color,
        );
        job.wrap = egui::text::TextWrapping::truncate_at_width(inner);
        fonts.layout_job(job)
    });
    let text = text.strip_suffix('\n').unwrap_or(text);
    let body = ui.fonts_mut(|fonts| {
        let mut job = egui::text::LayoutJob::simple(
            if text.is_empty() {
                "(no lines)".to_string()
            } else {
                text.to_string()
            },
            egui::FontId::monospace(CODE_SIZE),
            if text.is_empty() {
                palette::WEAK
            } else {
                text_color
            },
            inner,
        );
        job.wrap.break_anywhere = false;
        fonts.layout_job(job)
    });
    (title, body)
}

impl Blocks {
    #[cfg(test)]
    pub fn new(segments: Vec<Segment>) -> Self {
        Self::with_markers(segments, Markers::default())
    }

    /// Blocks whose unresolved markers name the sides.
    pub fn with_markers(segments: Vec<Segment>, markers: Markers) -> Self {
        let count = block_count(&segments);
        let choices = vec![None; count];
        let doc = ResultDoc::new(&segments, &choices, &markers);
        Self {
            segments,
            choices,
            current: 0,
            doc,
            markers,
        }
    }

    pub fn count(&self) -> usize {
        self.choices.len()
    }

    /// Conflict blocks still marked in the Result.
    pub fn left(&self) -> usize {
        self.doc.left()
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

    /// The Result has no conflict markers left.
    pub fn complete(&self) -> bool {
        self.left() == 0
    }

    /// What **Stage resolution** stages: the Result, hand edits included.
    pub fn text(&self) -> String {
        self.doc.text.clone()
    }

    fn block(&self, index: usize) -> Option<&Segment> {
        self.segments
            .iter()
            .filter(|segment| matches!(segment, Segment::Block { .. }))
            .nth(index)
    }

    /// Choosing for block `index` would replace text typed into it.
    pub fn needs_confirm(&self, index: usize) -> bool {
        self.doc.block_edited(index)
    }

    /// Choose for block `index` (rewriting only its part of the Result),
    /// then move to the next block without a choice.
    pub fn choose(&mut self, index: usize, choice: BlockChoice) {
        let Some(segment) = self.block(index).cloned() else {
            return;
        };
        self.choices[index] = Some(choice);
        self.doc
            .set_block(index, &segment, Some(choice), &self.markers);
        let count = self.count();
        self.current = (1..=count)
            .map(|step| (index + step) % count)
            .find(|&next| self.choices[next].is_none())
            .unwrap_or(index);
    }

    /// Undo the last change to the Result; the choices follow the text.
    pub fn undo(&mut self) -> bool {
        let undone = self.doc.undo();
        self.sync_choices();
        undone
    }

    pub fn redo(&mut self) -> bool {
        let redone = self.doc.redo();
        self.sync_choices();
        redone
    }

    /// A choice holds only while its block still shows that choice.
    fn sync_choices(&mut self) {
        use crate::conflict_result::Origin;
        for index in 0..self.count() {
            let unresolved = self
                .doc
                .spans
                .iter()
                .any(|span| span.block == Some(index) && span.origin == Origin::Unresolved);
            if unresolved {
                self.choices[index] = None;
            }
        }
    }

    /// Move the current block by `step`, wrapping at either end.
    pub fn step(&mut self, step: isize) {
        let count = self.count() as isize;
        if count > 0 {
            self.current = (self.current as isize + step).rem_euclid(count) as usize;
        }
    }

    /// Keys 1–4 choose for the current block; `[` and `]` move between
    /// blocks. Returns a choice that needs confirmation first.
    pub fn keys(&mut self, ctx: &egui::Context) -> Option<(usize, BlockChoice)> {
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
                if self.needs_confirm(self.current) {
                    return Some((self.current, choice));
                }
                self.choose(self.current, choice);
            }
        }
        None
    }

    /// Draw every block at the full width of `ui`: the header with the
    /// options, then Base, Ours, and Theirs in equal columns. Each column
    /// wraps its text inside itself.
    pub fn show(&mut self, ui: &mut egui::Ui, labels: &Labels) -> Events {
        let mut events = Events::default();
        let count = self.count();
        let starts = self.start_lines();
        let width = ui.available_width();
        let column = (width / 3.0).floor();
        let mut index = 0;
        let mut focus = None;
        for segment in &self.segments {
            let Segment::Block { ours, base, theirs } = segment else {
                continue;
            };
            let choice = self.choices[index];
            let current = index == self.current;
            let takes_ours = matches!(choice, Some(c) if c != BlockChoice::Theirs);
            let takes_theirs = matches!(choice, Some(c) if c != BlockChoice::Ours);
            let sides = [
                (
                    labels.base.as_str(),
                    labels.base_full.as_str(),
                    palette::WEAK,
                    base.as_str(),
                    palette::BASE_TEXT,
                    palette::BASE_BG,
                ),
                (
                    labels.ours.as_str(),
                    labels.ours_full.as_str(),
                    palette::OURS,
                    ours.as_str(),
                    palette::TEXT,
                    if takes_ours {
                        palette::OURS_BG
                    } else {
                        palette::SIDE_BG
                    },
                ),
                (
                    labels.theirs.as_str(),
                    labels.theirs_full.as_str(),
                    palette::THEIRS,
                    theirs.as_str(),
                    palette::TEXT,
                    if takes_theirs {
                        palette::THEIRS_BG
                    } else {
                        palette::SIDE_BG
                    },
                ),
            ];
            let galleys: Vec<_> = sides
                .iter()
                .enumerate()
                .map(|(at, (title, _, title_color, text, color, _))| {
                    let w = if at == 2 {
                        width - 2.0 * column
                    } else {
                        column
                    };
                    side_galleys(ui, w, title, *title_color, text, *color)
                })
                .collect();
            let body = galleys
                .iter()
                .map(|(title, body)| title.size().y + 2.0 + body.size().y)
                .fold(0.0, f32::max)
                + 2.0 * COLUMN_PAD.y;
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(width, HEADER_HEIGHT + body),
                egui::Sense::click(),
            );
            if response.clicked() {
                focus = Some(index);
            }
            let painter = ui.painter_at(rect);
            let header = egui::Rect::from_min_size(rect.min, egui::vec2(width, HEADER_HEIGHT));
            painter.rect_filled(header, 0.0, palette::HEADER);
            // Header: title and line on the left, the options on the right.
            let mut row = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(header.shrink2(egui::vec2(6.0, 3.0)))
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            row.set_clip_rect(header.intersect(ui.clip_rect()));
            // The options keep their room at the right; the title takes the rest.
            row.spacing_mut().item_spacing.x = 6.0;
            for (key, option) in BlockChoice::ALL.into_iter().enumerate().rev() {
                let (fill, ink) = if choice == Some(option) {
                    option_colors(option)
                } else {
                    (palette::BUTTON, palette::TEXT)
                };
                let response = row
                    .add(
                        egui::Button::new(
                            egui::RichText::new(option.label()).size(12.0).color(ink),
                        )
                        .fill(fill)
                        .corner_radius(3.0)
                        .min_size(egui::vec2(0.0, 18.0)),
                    )
                    .on_hover_text(format!(
                        "{} · key {} on the current block",
                        labels.describe(option),
                        key + 1
                    ));
                if response.clicked() {
                    events.chosen = Some((index, option));
                }
            }
            row.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(format!("Conflict {}/{count}", index + 1))
                            .size(12.5)
                            .color(palette::TEXT),
                    )
                    .truncate(),
                );
                if let Some(line) = starts.get(index) {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!("line {line}"))
                                .monospace()
                                .size(11.0)
                                .color(palette::WEAK),
                        )
                        .truncate(),
                    );
                }
            });
            // The columns, each clipped to its own rectangle.
            let mut x = rect.left();
            for (at, ((title, body), side)) in galleys.into_iter().zip(&sides).enumerate() {
                let w = if at == 2 {
                    width - 2.0 * column
                } else {
                    column
                };
                let cell = egui::Rect::from_min_size(
                    egui::pos2(x, header.bottom()),
                    egui::vec2(w, rect.bottom() - header.bottom()),
                );
                painter.rect_filled(cell, 0.0, side.5);
                let clip = ui.painter_at(cell.intersect(ui.clip_rect()));
                let top = cell.min + COLUMN_PAD;
                let title_rect = egui::Rect::from_min_size(top, title.size());
                clip.galley(top, title.clone(), side.2);
                clip.galley(
                    egui::pos2(top.x, top.y + title.size().y + 2.0),
                    body,
                    side.4,
                );
                ui.interact(
                    title_rect,
                    ui.scope_id().with(("conflict-side-title", index, at)),
                    egui::Sense::hover(),
                )
                .on_hover_text(side.1);
                if at < 2 {
                    painter.vline(
                        cell.right() - 0.5,
                        cell.y_range(),
                        egui::Stroke::new(1.0, palette::DIVIDER),
                    );
                }
                x += w;
            }
            let edge = match (choice.is_some(), current) {
                (true, true) => palette::CHOSEN_EDGE_CURRENT,
                (true, false) => palette::CHOSEN_EDGE,
                (false, true) => palette::OPEN_EDGE_CURRENT,
                (false, false) => palette::OPEN_EDGE,
            };
            ui.painter().rect_stroke(
                rect.shrink(0.5),
                5.0,
                egui::Stroke::new(1.0, edge),
                egui::StrokeKind::Inside,
            );
            ui.add_space(8.0 - ui.spacing().item_spacing.y);
            index += 1;
        }
        if let Some(index) = focus {
            self.current = index;
        }
        events
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

    fn labels() -> Labels {
        Labels {
            base: "Base".into(),
            ours: "Ours · main (current branch)".into(),
            theirs: "Theirs · origin/main (being merged)".into(),
            base_full: "Stage 1 · merge base".into(),
            ours_full: "Stage 2 · main".into(),
            theirs_full: "Stage 3 · origin/main".into(),
        }
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
        let labels = labels();
        assert_eq!(
            labels.describe(BlockChoice::Ours),
            "Ours · main (current branch)"
        );
        assert_eq!(
            labels.describe(BlockChoice::TheirsThenOurs),
            "Theirs · origin/main (being merged), then Ours · main (current branch)"
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
        let _ = crate::tests::run_ui(&ctx, input, |ui| {
            let _ = blocks.keys(ui.ctx());
        });
        assert_eq!(blocks.choices[0], Some(BlockChoice::TheirsThenOurs));
        assert_eq!(blocks.current, 1);
    }

    #[test]
    fn a_hand_edited_block_asks_before_an_option_replaces_it() {
        let mut blocks = blocks();
        blocks.choose(0, BlockChoice::Ours);
        let edited = blocks.doc.text.replace("ours 1", "ours 1 by hand");
        blocks.doc.typed(edited, 0.0);
        assert!(blocks.needs_confirm(0));
        assert!(!blocks.needs_confirm(1));
        assert!(blocks.undo());
        assert!(!blocks.needs_confirm(0));
        assert!(blocks.undo());
        assert_eq!(
            blocks.choices[0], None,
            "undoing the choice reopens the block"
        );
    }

    /// Every text a frame painted, with its visible rectangle.
    fn texts(output: egui::FullOutput) -> Vec<(String, egui::Rect)> {
        output
            .shapes
            .into_iter()
            .filter_map(|shape| match shape.shape {
                egui::epaint::Shape::Text(text) => Some((
                    text.galley.job.text.clone(),
                    text.visual_bounding_rect().intersect(shape.clip_rect),
                )),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn long_lines_wrap_inside_their_column() -> Result<(), String> {
        let long = r#"eframe = { version = "0.37", default-features = false, features = ["default_fonts", "glow", "persistence", "wayland", "x11"] }"#;
        let mut blocks = Blocks::new(vec![Segment::Block {
            ours: format!("{long}\n"),
            base: "eframe = \"0.36\"\n".into(),
            theirs: format!("{}\n", long.replace("0.37", "0.38")),
        }]);
        let ctx = egui::Context::default();
        for width in [1120.0, 720.0] {
            let input = || egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 600.0),
                )),
                ..Default::default()
            };
            let mut area = egui::Rect::NOTHING;
            let mut output = None;
            for _ in 0..2 {
                output = Some(crate::tests::run_ui(&ctx, input(), |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        area = ui.max_rect();
                        let _ = blocks.show(ui, &labels());
                    });
                }));
            }
            let texts = texts(output.ok_or("no frame")?);
            let column = (area.width() / 3.0).floor();
            let columns = [0.0, 1.0, 2.0].map(|at| {
                egui::Rect::from_x_y_ranges(
                    area.left() + at * column..=area.left() + (at + 1.0) * column,
                    area.y_range(),
                )
            });
            for side in ["0.37", "0.38"] {
                let (_, rect) = texts
                    .iter()
                    .find(|(text, _)| text.contains(side) && text.starts_with("eframe = {"))
                    .ok_or(format!("{side} painted at {width}: {texts:?}"))?;
                let column = columns
                    .iter()
                    .find(|column| column.contains(rect.center()))
                    .ok_or(format!("{side} in a column"))?;
                assert!(
                    column.expand(0.5).contains_rect(*rect),
                    "{side} {rect:?} stays inside {column:?} at {width}"
                );
                assert!(rect.height() > 20.0, "{side} wraps at {width}");
            }
        }
        Ok(())
    }
}
