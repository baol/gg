//! The conflict workspace: the Changes view while a merge, rebase,
//! cherry-pick, or revert has conflicted files, in the Final · Rebase
//! conflict layout. Left: CONFLICTED (with what is left in each file),
//! RESOLVED, STAGED (− reopens a file), ALSO CHANGED BY THIS <OPERATION>,
//! and the Ours/Theirs legend. Right: the file header (path, state, Block by
//! block | Whole file, Open in editor, Refresh external edits), the blocks
//! with Base / Ours / Theirs columns, and the editable, line-numbered Result
//! with **Stage resolution**.

use crate::conflict_blocks::{self, Labels, palette};
use crate::conflict_result::{Origin, ResultDoc, line_origins, rebase_spans};
use crate::{ConflictStage, ConflictView};
use eframe::egui;
use gg_engine::conflict::{Resolution, SideNames};
use gg_engine::mutation::Operation;
use gg_engine::review::display_path;

/// Width of the file list, as in the mockup (narrower below 940 px).
pub const PATHS_WIDTH: f32 = 300.0;
/// Height of each list row and section heading.
pub const ROW: f32 = 22.0;
/// Height of the file header.
pub const HEADER: f32 = 28.0;
/// Width of the Result's line-number gutter.
pub const GUTTER: f32 = 40.0;
/// Height of each Result line.
pub const RESULT_LINE: f32 = 18.0;

/// What the workspace asks the app to do.
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Select(usize),
    Stage(ConflictStage),
    /// Ask before **Replace with blocks…** overwrites edits made elsewhere.
    ConfirmReplace,
    Refresh,
    OpenInEditor,
    /// Unstage a resolved file, by its exact path bytes: back to conflicted.
    Unstage(Vec<u8>),
}

/// What the workspace shows about the operation.
pub struct Info<'a> {
    pub operation: Option<Operation>,
    /// Another job runs: staging waits.
    pub pending: bool,
    pub editor_tip: &'a str,
}

fn short_name(name: &str) -> &str {
    name.split_whitespace().next().unwrap_or(name)
}

/// The side labels for the blocks: what each side is in this operation,
/// with the stage number in the tooltip.
pub fn labels(operation: Option<Operation>, names: Option<&SideNames>) -> Labels {
    let ours_name = names.map(|names| names.ours.as_str()).unwrap_or_default();
    let theirs_name = names.map(|names| names.theirs.as_str()).unwrap_or_default();
    let theirs_short = short_name(theirs_name);
    let (ours, theirs) = match operation {
        Some(Operation::Merge) => (
            side("Ours", ours_name, "current branch"),
            side("Theirs", theirs_short, "being merged"),
        ),
        Some(Operation::CherryPick) => (
            side("Ours", ours_name, "current branch"),
            side("Theirs", theirs_short, "picked commit"),
        ),
        Some(Operation::Revert) => (
            side("Ours", ours_name, "current branch"),
            if theirs_short.is_empty() {
                "Theirs · undo of the reverted commit".into()
            } else {
                format!("Theirs · undo of {theirs_short}")
            },
        ),
        Some(Operation::Rebase) => (
            match ours_name {
                "" => "Ours · rewritten branch".into(),
                onto => format!("Ours · rewritten branch ({onto})"),
            },
            side("Theirs", theirs_short, "replayed commit"),
        ),
        None => {
            let named = |title: &str, name: &str| match name {
                "" => title.to_string(),
                name => format!("{title} · {name}"),
            };
            (named("Ours", ours_name), named("Theirs", theirs_short))
        }
    };
    let full = |stage: u8, name: &str| match name {
        "" => format!("Stage {stage}"),
        name => format!("Stage {stage} · {name}"),
    };
    Labels {
        base: "Base".into(),
        ours,
        theirs,
        base_full: "Stage 1 · merge base".into(),
        ours_full: full(2, ours_name),
        theirs_full: full(3, theirs_name),
    }
}

fn side(title: &str, name: &str, role: &str) -> String {
    match name {
        "" => format!("{title} · {role}"),
        name => format!("{title} · {name} ({role})"),
    }
}

/// The labels written on unresolved markers in the Result.
pub fn markers(
    operation: Option<Operation>,
    names: Option<&SideNames>,
) -> crate::conflict_result::Markers {
    let ours = names.map(|names| names.ours.as_str()).unwrap_or_default();
    let theirs = short_name(names.map(|names| names.theirs.as_str()).unwrap_or_default());
    let mut markers = crate::conflict_result::Markers::default();
    if !ours.is_empty() && operation != Some(Operation::Rebase) {
        markers.ours = ours.to_string();
    }
    if !theirs.is_empty() {
        markers.theirs = match operation {
            Some(Operation::CherryPick) => format!("{theirs} (picked commit)"),
            Some(Operation::Revert) => format!("undo of {theirs}"),
            Some(Operation::Rebase) => format!("{theirs} (replayed commit)"),
            _ => theirs.to_string(),
        };
    }
    markers
}

/// The legend under the file list, as pieces with the Ours and Theirs
/// words colored.
pub fn legend(operation: Option<Operation>, names: Option<&SideNames>) -> Vec<(String, bool)> {
    let ours = names
        .map(|names| names.ours.as_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("the current branch");
    let theirs = short_name(names.map(|names| names.theirs.as_str()).unwrap_or_default());
    let (lead, middle, tail) = match operation {
        Some(Operation::Merge) => (
            "In a merge, ".to_string(),
            format!(" is {ours}, the branch you are on, and "),
            format!(
                " is {}, the branch coming in.",
                if theirs.is_empty() {
                    "the merged branch"
                } else {
                    theirs
                }
            ),
        ),
        Some(Operation::CherryPick) => (
            "In a cherry-pick, ".to_string(),
            format!(" is {ours} and "),
            format!(
                " is the change from {} being applied.",
                if theirs.is_empty() {
                    "the picked commit"
                } else {
                    theirs
                }
            ),
        ),
        Some(Operation::Revert) => (
            "In a revert, ".to_string(),
            format!(" is {ours} as it is now and "),
            format!(
                " is {ours} with {} undone.",
                if theirs.is_empty() {
                    "the commit"
                } else {
                    theirs
                }
            ),
        ),
        Some(Operation::Rebase) => (
            "In a rebase, ".to_string(),
            " is the rewritten branch so far and ".to_string(),
            " is the commit being replayed.".to_string(),
        ),
        None => (
            String::new(),
            " is stage 2 and ".into(),
            " is stage 3.".into(),
        ),
    };
    vec![
        (lead, false),
        ("Ours".into(), true),
        (middle, false),
        ("Theirs".into(), true),
        (tail, false),
    ]
}

/// The workspace footer's key hints.
pub const FOOTER_HINTS: [&str; 4] = [
    "1 / 2 / 3 / 4: ours · theirs · ours+theirs · theirs+ours",
    "[ ]: previous / next conflict",
    "Ctrl+S: stage file",
    "Esc: changes list",
];

fn upper(operation: Option<Operation>) -> &'static str {
    match operation {
        Some(Operation::Merge) => "MERGE",
        Some(Operation::Rebase) => "REBASE",
        Some(Operation::CherryPick) => "CHERRY-PICK",
        Some(Operation::Revert) => "REVERT",
        None => "OPERATION",
    }
}

fn heading(ui: &mut egui::Ui, left: &str, right: Option<String>) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW), egui::Sense::hover());
    let font = egui::FontId::proportional(11.0);
    let painter = ui.painter_at(rect);
    let mut job =
        egui::text::LayoutJob::simple_singleline(left.into(), font.clone(), palette::WEAK);
    job.sections[0].format.extra_letter_spacing = 0.5;
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    painter.galley(
        egui::pos2(rect.left() + 4.0, rect.center().y - galley.size().y / 2.0),
        galley,
        palette::WEAK,
    );
    if let Some(right) = right {
        painter.text(
            egui::pos2(rect.right() - 4.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            right,
            font,
            palette::WEAK,
        );
    }
}

/// One 22 px file row: the mark, the path (monospace, truncated), and a
/// note or a button on the right.
fn file_row(
    ui: &mut egui::Ui,
    mark: &str,
    mark_color: egui::Color32,
    path: &str,
    note: &str,
    selected: bool,
    clickable: bool,
) -> egui::Response {
    let sense = if clickable {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW), sense);
    let painter = ui.painter_at(rect);
    if selected {
        painter.rect_filled(rect, 3.0, egui::Color32::from_rgb(14, 58, 79));
    } else if clickable && response.hovered() {
        painter.rect_filled(rect, 3.0, egui::Color32::from_rgb(38, 38, 38));
    }
    painter.text(
        egui::pos2(rect.left() + 6.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        mark,
        egui::FontId::proportional(12.5),
        mark_color,
    );
    let note_width = if note.is_empty() {
        0.0
    } else {
        let galley = ui.fonts_mut(|fonts| {
            fonts.layout_no_wrap(note.into(), egui::FontId::proportional(11.0), palette::WEAK)
        });
        let width = galley.size().x;
        painter.galley(
            egui::pos2(
                rect.right() - 6.0 - width,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            palette::WEAK,
        );
        width + 12.0
    };
    let left = rect.left() + 26.0;
    let room = (rect.right() - 6.0 - note_width - left).max(10.0);
    let mut job = egui::text::LayoutJob::simple_singleline(
        path.into(),
        egui::FontId::monospace(11.5),
        palette::TEXT,
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(room);
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    painter.galley(
        egui::pos2(left, rect.center().y - galley.size().y / 2.0),
        galley,
        palette::TEXT,
    );
    response.on_hover_text(path)
}

/// A flat 20 px button in the mockup's grey.
fn flat_button(text: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(text)
        .fill(palette::BUTTON)
        .corner_radius(4.0)
        .min_size(egui::vec2(0.0, 20.0))
}

fn left_pane(ui: &mut egui::Ui, view: &ConflictView, info: &Info, actions: &mut Vec<Action>) {
    let legend = legend(info.operation, view.names.as_ref());
    let width = ui.available_width();
    let mut job = egui::text::LayoutJob::default();
    for (text, colored) in &legend {
        let color = match (colored, text.as_str()) {
            (true, "Ours") => palette::OURS,
            (true, _) => palette::THEIRS,
            _ => palette::WEAK,
        };
        job.append(
            text,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(11.0),
                color,
                line_height: Some(17.0),
                ..Default::default()
            },
        );
    }
    job.wrap.max_width = width - 8.0;
    let legend = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let legend_height = legend.size().y + 8.0;
    let list = ui.available_height() - legend_height;
    let left_in = |index: usize, path: &Vec<u8>| {
        if index == view.selected
            && let Some(blocks) = view.blocks.as_ref().filter(|_| !view.whole_file)
        {
            return Some(blocks.left());
        }
        view.counts.get(path).copied()
    };
    egui::ScrollArea::vertical()
        .id_salt("conflict-paths")
        .max_height(list.max(ROW))
        .min_scrolled_height(list.max(ROW))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            heading(ui, "CONFLICTED", Some(view.paths.len().to_string()));
            for (index, path) in view.paths.iter().enumerate() {
                let note = match left_in(index, path) {
                    Some(0) => "ready".to_string(),
                    Some(left) => format!("{left} left"),
                    None => String::new(),
                };
                if file_row(
                    ui,
                    "!",
                    palette::MARKER,
                    &display_path(path),
                    &note,
                    index == view.selected,
                    true,
                )
                .clicked()
                {
                    actions.push(Action::Select(index));
                }
            }
            if view.paths.is_empty() && !view.loading {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(crate::keymap::shown(
                            "All conflicts resolved. Continue with Ctrl+Enter.",
                        ))
                        .color(palette::BOTH),
                    )
                    .wrap(),
                );
            }
            ui.add_space(10.0);
            heading(
                ui,
                "RESOLVED, STAGED",
                Some(view.resolved.len().to_string()),
            );
            for path in &view.resolved {
                let response = file_row(
                    ui,
                    "✓",
                    egui::Color32::from_rgb(120, 200, 150),
                    &display_path(path),
                    "",
                    false,
                    false,
                );
                let button = egui::Rect::from_center_size(
                    egui::pos2(response.rect.right() - 12.0, response.rect.center().y),
                    egui::vec2(20.0, 18.0),
                );
                if ui
                    .put(
                        button,
                        egui::Button::new(
                            egui::RichText::new("−").color(egui::Color32::from_rgb(245, 160, 160)),
                        )
                        .fill(egui::Color32::from_rgb(47, 47, 47))
                        .corner_radius(3.0),
                    )
                    .on_hover_text("Unstage resolution · back to conflicted")
                    .on_disabled_hover_text("Wait for the current operation to finish")
                    .clicked()
                    && !info.pending
                {
                    actions.push(Action::Unstage(path.clone()));
                }
            }
            let also: Vec<_> = view
                .also
                .iter()
                .filter(|path| !view.resolved.contains(path))
                .collect();
            if !also.is_empty() {
                ui.add_space(10.0);
                heading(
                    ui,
                    &format!("ALSO CHANGED BY THIS {}", upper(info.operation)),
                    None,
                );
                for path in also {
                    file_row(
                        ui,
                        "~",
                        egui::Color32::from_rgb(220, 195, 125),
                        &display_path(path),
                        "",
                        false,
                        false,
                    );
                }
            }
            if let Some(error) = &view.error {
                ui.add_space(10.0);
                ui.add(
                    egui::Label::new(egui::RichText::new(error).color(ui.visuals().error_fg_color))
                        .wrap(),
                );
            }
        });
    let rect = ui.available_rect_before_wrap();
    ui.painter().galley(
        egui::pos2(rect.left() + 4.0, rect.bottom() - legend_height + 4.0),
        legend,
        palette::WEAK,
    );
}

/// The file header: path and state, then the mode switch and the editor
/// buttons on the right.
fn header(ui: &mut egui::Ui, view: &mut ConflictView, info: &Info, actions: &mut Vec<Action>) {
    let path = view
        .paths
        .get(view.selected)
        .map(|path| display_path(path))
        .unwrap_or_default();
    let blocks = view.blocks.is_some();
    let state = if view.paths.is_empty() {
        String::new()
    } else if view.whole_file || !blocks {
        "whole file".to_string()
    } else {
        match view.blocks.as_ref().map(conflict_blocks::Blocks::left) {
            Some(0) => "ready to stage".into(),
            Some(left) => format!("{left} conflict{} left", if left == 1 { "" } else { "s" }),
            None => String::new(),
        }
    };
    // The buttons keep their room at the right; path and state take the rest.
    ui.spacing_mut().item_spacing.x = 8.0;
    let narrow = ui.available_width() < 640.0;
    let file = !path.is_empty();
    if ui
        .add_enabled(
            file,
            flat_button(if narrow {
                "Refresh"
            } else {
                "Refresh external edits"
            }),
        )
        .on_hover_text("Refresh external edits: reload the working file after editing it elsewhere")
        .clicked()
    {
        actions.push(Action::Refresh);
    }
    if ui
        .add_enabled(file, flat_button("Open in editor"))
        .on_hover_text(info.editor_tip)
        .clicked()
    {
        actions.push(Action::OpenInEditor);
    }
    // Block by block | Whole file, a segmented switch.
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(38, 38, 38))
        .corner_radius(4.0)
        .inner_margin(2.0)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let segment = |selected: bool, text: &str| {
                egui::Button::new(egui::RichText::new(text).color(if selected {
                    egui::Color32::WHITE
                } else {
                    palette::TEXT
                }))
                .fill(if selected {
                    egui::Color32::from_rgb(11, 85, 119)
                } else {
                    egui::Color32::TRANSPARENT
                })
                .corner_radius(3.0)
                .min_size(egui::vec2(0.0, 18.0))
            };
            let whole = view.whole_file || !blocks;
            // Right to left: Whole file first.
            if ui.add(segment(whole, "Whole file")).clicked() {
                view.whole_file = true;
            }
            if ui
                .add_enabled(blocks, segment(!whole, "Block by block"))
                .on_disabled_hover_text(view.blocks_note.as_deref().unwrap_or("No text blocks."))
                .clicked()
            {
                view.whole_file = false;
            }
        });
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add(
            egui::Label::new(
                egui::RichText::new(&path)
                    .monospace()
                    .size(11.5)
                    .color(palette::TEXT),
            )
            .truncate(),
        )
        .on_hover_text(&path);
        ui.add(
            egui::Label::new(egui::RichText::new(state).size(11.0).color(palette::WEAK)).truncate(),
        );
    });
}

fn origin_fill(origin: Origin) -> Option<egui::Color32> {
    match origin {
        Origin::Ours => Some(palette::OURS_BG),
        Origin::Theirs => Some(palette::THEIRS_BG),
        Origin::Unresolved => Some(palette::UNRESOLVED_BG),
        Origin::Common | Origin::Edited => None,
    }
}

fn is_marker(line: &str) -> bool {
    ["<<<<<<<", "=======", ">>>>>>>", "|||||||"]
        .iter()
        .any(|marker| line.starts_with(marker))
}

/// The Result's text colors: markers of unresolved blocks in the warning
/// color, everything else in the text color, 18 px lines.
pub fn highlight(text: &str, origins: &[Origin]) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let unresolved = origins.get(index) == Some(&Origin::Unresolved);
        let color = if unresolved && is_marker(line) {
            palette::MARKER
        } else {
            palette::TEXT
        };
        job.append(
            line,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::monospace(12.0),
                color,
                line_height: Some(RESULT_LINE),
                ..Default::default()
            },
        );
    }
    if job.sections.is_empty() {
        job.append(
            "",
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::monospace(12.0),
                line_height: Some(RESULT_LINE),
                ..Default::default()
            },
        );
    }
    job.wrap.max_width = f32::INFINITY;
    job
}

/// The Result in `rect`: a line-numbered monospace editor whose lines are
/// tinted by origin. `doc` is editable; without it `fixed` is shown read-only
/// with every line in `fixed_origin`.
fn result_editor(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    doc: Option<&mut ResultDoc>,
    fixed: &str,
    fixed_origin: Origin,
) {
    ui.painter().rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, palette::DIVIDER),
        egui::StrokeKind::Inside,
    );
    let inner = rect.shrink2(egui::vec2(1.0, 4.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    child.set_clip_rect(inner.intersect(ui.clip_rect()));
    let now = child.input(|i| i.time);
    egui::ScrollArea::both()
        .id_salt("conflict-result")
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            let backgrounds = ui.painter().add(egui::Shape::Noop);
            let (mut buffer, base) = match &doc {
                Some(doc) => (
                    doc.text.clone(),
                    Some((doc.text.clone(), doc.spans.clone())),
                ),
                None => (fixed.to_string(), None),
            };
            let editable = base.is_some();
            let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
                let text = text.as_str();
                let origins = match &base {
                    Some((old, spans)) => line_origins(text, &rebase_spans(old, spans, text)),
                    None => vec![fixed_origin; text.split_inclusive('\n').count() + 1],
                };
                ui.fonts_mut(|fonts| fonts.layout_job(highlight(text, &origins)))
            };
            let min = egui::vec2(ui.available_width(), ui.available_height());
            let output = egui::TextEdit::multiline(&mut buffer)
                .id_salt("conflict-result-text")
                .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                    left: GUTTER as i8,
                    right: 4,
                    top: 0,
                    bottom: 0,
                }))
                .code_editor()
                .lock_focus(true)
                .interactive(editable)
                .desired_width(f32::INFINITY)
                .min_size(min)
                .layouter(&mut layouter)
                .show(ui);
            let origins = match doc {
                Some(doc) => {
                    if buffer != doc.text {
                        doc.typed(buffer, now);
                    }
                    line_origins(&doc.text, &doc.spans)
                }
                None => vec![fixed_origin; fixed.split_inclusive('\n').count() + 1],
            };
            let clip = ui.clip_rect();
            let mut shapes = Vec::new();
            for (index, row) in output.galley.rows.iter().enumerate() {
                let row_rect = row.rect().translate(output.galley_pos.to_vec2());
                let line = egui::Rect::from_x_y_ranges(clip.x_range(), row_rect.y_range());
                if let Some(fill) = origins.get(index).copied().and_then(origin_fill) {
                    shapes.push(egui::Shape::rect_filled(line, 0.0, fill));
                }
                // The last empty line after a final newline has no number.
                if index + 1 == output.galley.rows.len() && row.row.glyphs.is_empty() && index > 0 {
                    continue;
                }
                let number = ui.fonts_mut(|fonts| {
                    fonts.layout_no_wrap(
                        (index + 1).to_string(),
                        egui::FontId::monospace(12.0),
                        egui::Color32::from_gray(122),
                    )
                });
                let at = egui::pos2(
                    output.galley_pos.x - 10.0 - number.size().x,
                    row_rect.center().y - number.size().y / 2.0,
                );
                shapes.push(egui::Shape::galley(
                    at,
                    number,
                    egui::Color32::from_gray(122),
                ));
            }
            ui.painter().set(backgrounds, egui::Shape::Vec(shapes));
        });
}

/// Draw the workspace in the space given; returns what to do.
pub fn show(ui: &mut egui::Ui, view: &mut ConflictView, info: &Info) -> Vec<Action> {
    let mut actions = Vec::new();
    let full = ui.max_rect();
    // 300 px as in the mockup; narrower windows give the file list less.
    let paths = (full.width() * 0.32).clamp(220.0, PATHS_WIDTH).floor();
    let left = egui::Rect::from_min_size(full.min, egui::vec2(paths, full.height()));
    let main = egui::Rect::from_min_max(egui::pos2(left.right() + 1.0, full.top()), full.max);
    let line = egui::Stroke::new(1.0, egui::Color32::from_rgb(46, 46, 46));
    ui.painter().vline(left.right() + 0.5, full.y_range(), line);
    let mut pane = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(left.shrink(6.0))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    pane.set_clip_rect(left.intersect(ui.clip_rect()));
    left_pane(&mut pane, view, info, &mut actions);

    let head = egui::Rect::from_min_size(main.min, egui::vec2(main.width(), HEADER));
    ui.painter().hline(
        head.x_range(),
        head.bottom() - 0.5,
        egui::Stroke::new(1.0, palette::DIVIDER),
    );
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(head.shrink2(egui::vec2(10.0, 4.0)))
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    row.set_clip_rect(head.intersect(ui.clip_rect()));
    header(&mut row, view, info, &mut actions);

    let body = egui::Rect::from_min_max(
        egui::pos2(main.left() + 10.0, head.bottom() + 8.0),
        egui::pos2(main.right() - 10.0, main.bottom() - 8.0),
    );
    let mut ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(body)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    ui.set_clip_rect(body.intersect(ui.clip_rect()));
    let ui = &mut ui;
    let Some(content) = view.content.clone() else {
        return actions;
    };
    ui.spacing_mut().item_spacing.y = 8.0;
    let labels = labels(info.operation, view.names.as_ref());
    let blocks_mode = !view.whole_file && view.blocks.is_some();
    if blocks_mode && view.edited {
        ui.horizontal(|ui| {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "This file was edited outside the conflict blocks.",
            );
            if ui
                .add_enabled(!info.pending, flat_button("Use working file"))
                .on_hover_text("Stage the working file as it is")
                .on_disabled_hover_text("Wait for the current operation to finish.")
                .clicked()
            {
                actions.push(Action::Stage(ConflictStage::WorkingFile));
            }
        });
    }
    // The blocks take up to 60% of the height and scroll; the Result gets
    // the rest.
    let share = ((ui.available_height() - ROW - 12.0) * 0.6).max(80.0);
    if blocks_mode && let Some(blocks) = &mut view.blocks {
        let events = egui::ScrollArea::vertical()
            .id_salt("conflict-blocks")
            .max_height(share)
            .auto_shrink([false, true])
            .show(ui, |ui| blocks.show(ui, &labels))
            .inner;
        if let Some((index, choice)) = events.chosen {
            if blocks.needs_confirm(index) {
                view.confirm_block = Some((index, choice));
            } else {
                blocks.choose(index, choice);
            }
        }
    } else {
        whole_file(ui, view, &content, &labels);
    }
    // Result header: the title, what it is, and Stage resolution.
    let ready = match &view.blocks {
        Some(blocks) if blocks_mode => blocks.complete(),
        _ => crate::conflict_preview_available(view),
    };
    let note = if blocks_mode {
        "working file preview, not staged"
    } else if view.preview == Resolution::Working {
        "working file as edited, not staged"
    } else {
        "preview, not staged"
    };
    ui.add_space(4.0 - ui.spacing().item_spacing.y + 4.0);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), ROW),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(egui::RichText::new("Result").color(palette::TEXT));
            ui.label(egui::RichText::new(note).color(palette::WEAK));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let enabled = ready && !info.pending;
                let replace = blocks_mode && view.edited;
                let label = if replace {
                    "Replace with blocks…"
                } else {
                    "Stage resolution"
                };
                let mut button = egui::Button::new((
                    egui::RichText::new(label).color(if enabled {
                        egui::Color32::WHITE
                    } else {
                        palette::WEAK
                    }),
                    egui::RichText::new(crate::keymap::shown("Ctrl+S"))
                        .small()
                        .color(if enabled {
                            egui::Color32::from_rgb(184, 212, 230)
                        } else {
                            palette::WEAK
                        }),
                ))
                .corner_radius(4.0)
                .min_size(egui::vec2(0.0, 22.0));
                button = button.fill(if enabled {
                    egui::Color32::from_rgb(11, 85, 119)
                } else {
                    egui::Color32::from_rgb(43, 43, 43)
                });
                let response = ui.add_enabled(enabled, button);
                let response = if info.pending {
                    response.on_disabled_hover_text("Wait for the current operation to finish.")
                } else if blocks_mode {
                    response.on_disabled_hover_text("Choose a resolution for every block first.")
                } else {
                    response.on_disabled_hover_text(crate::conflict_preview_reason(view))
                };
                if response
                    .on_hover_text("Write the result and stage this file")
                    .clicked()
                {
                    actions.push(if replace {
                        Action::ConfirmReplace
                    } else {
                        Action::Stage(ConflictStage::Preview)
                    });
                }
            });
        },
    );
    let rest = ui.available_rect_before_wrap();
    let rest = egui::Rect::from_min_max(rest.min, egui::pos2(rest.right(), body.bottom()));
    if rest.height() > 0.0 {
        if blocks_mode && let Some(blocks) = &mut view.blocks {
            result_editor(ui, rest, Some(&mut blocks.doc), "", Origin::Common);
        } else {
            let text = view
                .snapshot
                .as_ref()
                .map(|snapshot| crate::conflict_preview_text(&content, snapshot, view.preview))
                .unwrap_or_default();
            let origin = match view.preview {
                Resolution::Ours => Origin::Ours,
                Resolution::Theirs => Origin::Theirs,
                _ => Origin::Common,
            };
            result_editor(ui, rest, None, &text, origin);
        }
        ui.allocate_rect(rest, egui::Sense::hover());
    }
    actions
}

/// Whole file: one source wins outright. The options, then what each side
/// holds.
fn whole_file(
    ui: &mut egui::Ui,
    view: &mut ConflictView,
    content: &gg_engine::conflict::Content,
    labels: &Labels,
) {
    let deleted = content.deleted_side();
    let chosen = view.preview;
    let edge = if matches!(chosen, Resolution::Working) {
        palette::OPEN_EDGE
    } else {
        palette::CHOSEN_EDGE
    };
    let frame = egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, edge))
        .corner_radius(5.0)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            egui::Frame::new()
                .fill(palette::HEADER)
                .inner_margin(egui::Margin::symmetric(6, 3))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.label(egui::RichText::new("Whole file").color(palette::TEXT));
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(
                                    view.blocks_note.as_deref().unwrap_or(
                                        "for binary files, files without text blocks, or when one side wins outright",
                                    ),
                                )
                                .size(11.0)
                                .color(palette::WEAK),
                            )
                            .truncate(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mut options = vec![
                                (Resolution::Ours, "Take ours", palette::OURS_FILL, palette::OURS),
                                (
                                    Resolution::Theirs,
                                    "Take theirs",
                                    palette::THEIRS_FILL,
                                    palette::THEIRS,
                                ),
                                (
                                    Resolution::Working,
                                    "Keep working file",
                                    palette::BOTH_FILL,
                                    palette::BOTH,
                                ),
                            ];
                            if deleted.is_some() {
                                options.push((
                                    Resolution::Deletion,
                                    "Keep deletion",
                                    palette::BOTH_FILL,
                                    palette::BOTH,
                                ));
                            }
                            for (resolution, label, fill, ink) in options.into_iter().rev() {
                                let on = chosen == resolution;
                                let tip = match resolution {
                                    Resolution::Ours => labels.ours.clone(),
                                    Resolution::Theirs => labels.theirs.clone(),
                                    Resolution::Deletion => {
                                        "Delete the file and stage its removal".into()
                                    }
                                    _ => "The working file as it is now".into(),
                                };
                                if ui
                                    .add(
                                        egui::Button::new(egui::RichText::new(label).color(
                                            if on { ink } else { palette::TEXT },
                                        ))
                                        .fill(if on { fill } else { palette::BUTTON })
                                        .corner_radius(3.0)
                                        .min_size(egui::vec2(0.0, 18.0)),
                                    )
                                    .on_hover_text(tip)
                                    .clicked()
                                {
                                    view.preview = resolution;
                                }
                            }
                        });
                    });
                });
            let describe = |text: &Option<String>, side| match text {
                Some(text) if content.binary => format!("{} bytes · binary", text.len()),
                Some(text) => format!("{} lines · text", text.lines().count()),
                None if deleted == Some(side) => "(deleted on this side)".into(),
                None => "(absent)".into(),
            };
            let columns = [
                (
                    labels.base.clone(),
                    labels.base_full.clone(),
                    palette::WEAK,
                    describe(&content.base, gg_engine::conflict::Side::Base),
                    palette::BASE_BG,
                ),
                (
                    labels.ours.clone(),
                    labels.ours_full.clone(),
                    palette::OURS,
                    describe(&content.ours, gg_engine::conflict::Side::Ours),
                    if chosen == Resolution::Ours {
                        palette::OURS_BG
                    } else {
                        palette::SIDE_BG
                    },
                ),
                (
                    labels.theirs.clone(),
                    labels.theirs_full.clone(),
                    palette::THEIRS,
                    describe(&content.theirs, gg_engine::conflict::Side::Theirs),
                    if chosen == Resolution::Theirs {
                        palette::THEIRS_BG
                    } else {
                        palette::SIDE_BG
                    },
                ),
            ];
            let width = ui.available_width();
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(width, 44.0), egui::Sense::hover());
            let column = (width / 3.0).floor();
            for (at, (title, tip, color, info, fill)) in columns.into_iter().enumerate() {
                let w = if at == 2 { width - 2.0 * column } else { column };
                let cell = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + at as f32 * column, rect.top()),
                    egui::vec2(w, rect.height()),
                );
                let painter = ui.painter_at(cell);
                painter.rect_filled(cell, 0.0, fill);
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(cell.shrink2(conflict_blocks::COLUMN_PAD))
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                child.set_clip_rect(cell.intersect(ui.clip_rect()));
                child.spacing_mut().item_spacing.y = 2.0;
                child
                    .add(egui::Label::new(egui::RichText::new(title).size(11.0).color(color)).truncate())
                    .on_hover_text(tip);
                child.add(
                    egui::Label::new(
                        egui::RichText::new(info).size(11.0).color(egui::Color32::from_gray(207)),
                    )
                    .truncate(),
                );
                if at < 2 {
                    painter.vline(
                        cell.right() - 0.5,
                        cell.y_range(),
                        egui::Stroke::new(1.0, palette::DIVIDER),
                    );
                }
            }
        });
    let _ = frame;
    if content.truncated {
        ui.colored_label(ui.visuals().warn_fg_color, "Content truncated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(ours: &str, theirs: &str) -> SideNames {
        SideNames {
            ours: ours.into(),
            theirs: theirs.into(),
        }
    }

    #[test]
    fn side_labels_name_the_real_refs_with_the_stage_in_the_tooltip() {
        let merge = labels(Some(Operation::Merge), Some(&names("main", "origin/main")));
        assert_eq!(merge.ours, "Ours · main (current branch)");
        assert_eq!(merge.theirs, "Theirs · origin/main (being merged)");
        assert_eq!(merge.ours_full, "Stage 2 · main");
        assert_eq!(merge.base_full, "Stage 1 · merge base");
        let pick = labels(
            Some(Operation::CherryPick),
            Some(&names("main", "9a4f9a95 Add the login form")),
        );
        assert_eq!(pick.theirs, "Theirs · 9a4f9a95 (picked commit)");
        assert_eq!(pick.theirs_full, "Stage 3 · 9a4f9a95 Add the login form");
        let revert = labels(
            Some(Operation::Revert),
            Some(&names("main", "7151cc83 Drop the cache")),
        );
        assert_eq!(revert.theirs, "Theirs · undo of 7151cc83");
        let rebase = labels(
            Some(Operation::Rebase),
            Some(&names("onto main", "0f99a42b Reword")),
        );
        assert_eq!(rebase.ours, "Ours · rewritten branch (onto main)");
        assert_eq!(rebase.theirs, "Theirs · 0f99a42b (replayed commit)");
        let bare = labels(Some(Operation::Rebase), None);
        assert_eq!(
            (bare.ours.as_str(), bare.theirs.as_str()),
            ("Ours · rewritten branch", "Theirs · replayed commit")
        );
        assert!(
            ![merge.ours, merge.theirs]
                .iter()
                .any(|label| label.contains("stage")),
            "stage numbers live in the tooltips"
        );
    }

    #[test]
    fn markers_and_legend_name_the_sides() {
        let names = names("main", "origin/main");
        let markers = markers(Some(Operation::Merge), Some(&names));
        assert_eq!(
            (markers.ours.as_str(), markers.theirs.as_str()),
            ("main", "origin/main")
        );
        let text: String = legend(Some(Operation::Merge), Some(&names))
            .into_iter()
            .map(|(text, _)| text)
            .collect();
        assert_eq!(
            text,
            "In a merge, Ours is main, the branch you are on, and Theirs is origin/main, the branch coming in."
        );
        let text: String = legend(Some(Operation::Rebase), None)
            .into_iter()
            .map(|(text, _)| text)
            .collect();
        assert_eq!(
            text,
            "In a rebase, Ours is the rewritten branch so far and Theirs is the commit being replayed."
        );
    }

    #[test]
    fn markers_of_unresolved_blocks_take_the_warning_color() {
        let text = "a\n<<<<<<< main\nb\n=======\nc\n>>>>>>> x\n";
        let origins = [
            Origin::Common,
            Origin::Unresolved,
            Origin::Unresolved,
            Origin::Unresolved,
            Origin::Unresolved,
            Origin::Unresolved,
            Origin::Common,
        ];
        let job = highlight(text, &origins);
        let colors: Vec<_> = job
            .sections
            .iter()
            .map(|section| section.format.color)
            .collect();
        assert_eq!(
            colors,
            [
                palette::TEXT,
                palette::MARKER,
                palette::TEXT,
                palette::MARKER,
                palette::TEXT,
                palette::MARKER
            ]
        );
        assert!(
            job.sections
                .iter()
                .all(|section| section.format.line_height == Some(RESULT_LINE))
        );
        assert_eq!(origin_fill(Origin::Ours), Some(palette::OURS_BG));
        assert_eq!(origin_fill(Origin::Theirs), Some(palette::THEIRS_BG));
        assert_eq!(
            origin_fill(Origin::Unresolved),
            Some(palette::UNRESOLVED_BG)
        );
        assert_eq!(origin_fill(Origin::Edited), None);
    }
}
