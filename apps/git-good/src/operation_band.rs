//! The operation band for a merge, cherry-pick, or revert that Git holds
//! open, and for a rewrite while it runs, in the rebase stop's layout
//! ([`crate::rebase_stop`]).
//!
//! The first line holds the icon, the title (`Merge conflict · merging
//! origin/main 0f51a473 into main`), the status (`2 conflicts left in 2
//! files`), and the buttons; the second line holds what to do and, when the
//! operation has steps, the step strip. Amber while conflicts remain, purple
//! once Continue can run. A running Continue, Skip, or Abort keeps the band
//! and its buttons in place: the icon turns into a spinner, the title names
//! the job, and the buttons disable.

use crate::{phosphor, rebase_stop};
use eframe::egui;
use gg_engine::mutation::{Operation, RebaseProgress};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    ResolveConflicts,
    Continue,
    Skip,
    Abort,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Purple: nothing blocks Continue, or a job runs.
    Ready,
    /// Amber: conflicts remain.
    Conflict,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Pause,
    Warning,
    Spinner,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The filled blue primary.
    Primary,
    Secondary,
    /// Red text: Abort and restore.
    Danger,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Button {
    pub command: Command,
    pub label: &'static str,
    pub kind: Kind,
    pub key: Option<&'static str>,
    pub enabled: bool,
    pub tip: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepState {
    Done,
    Current,
    Next,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub commit: String,
    pub action: String,
    pub subject: String,
    pub state: StepState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Band {
    pub tone: Tone,
    pub icon: Icon,
    /// The title before the commit ID, the 8-character ID (monospace), and
    /// what follows it.
    pub title: String,
    pub hash: String,
    pub post: String,
    pub status: String,
    pub sub: String,
    pub buttons: Vec<Button>,
    pub steps: Vec<Step>,
}

impl Band {
    pub fn title_text(&self) -> String {
        format!("{}{}{}", self.title, self.hash, self.post)
    }

    pub fn button(&self, command: Command) -> Option<&Button> {
        self.buttons.iter().find(|button| button.command == command)
    }
}

/// What the band names: the operation, the branch it runs on, the merged
/// ref, and the commit Git records for it.
pub struct Subject<'a> {
    pub operation: Operation,
    pub current: &'a str,
    pub incoming: Option<&'a str>,
    pub commit: Option<&'a str>,
}

impl Subject<'_> {
    fn hash(&self) -> String {
        self.commit
            .map(|commit| commit.chars().take(8).collect())
            .unwrap_or_default()
    }

    /// What is merged: the ref, else the commit.
    fn incoming_name(&self) -> String {
        self.incoming
            .map(str::to_string)
            .unwrap_or_else(|| self.hash())
    }
}

/// Conflicts left: blocks (or whole files where blocks are unknown) and files.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Conflicts {
    pub files: usize,
    pub blocks: usize,
}

pub fn plural(count: usize, word: &str) -> String {
    format!("{count} {word}{}", if count == 1 { "" } else { "s" })
}

fn label(operation: Operation) -> &'static str {
    match operation {
        Operation::Merge => "Merge",
        Operation::CherryPick => "Cherry-pick",
        Operation::Revert => "Revert",
        Operation::Rebase => "Rebase",
    }
}

/// The worker label a Continue, Skip, or Abort click shows in the title.
pub fn busy_label(operation: Operation, command: Command) -> String {
    let verb = operation.as_str();
    match command {
        Command::Skip => format!("Skipping {verb}"),
        Command::Abort => format!("Aborting {verb}"),
        Command::Continue | Command::ResolveConflicts => format!("Continuing {verb}"),
    }
}

/// The title's description after the lead, split around the commit ID.
fn description(subject: &Subject) -> (String, String, String) {
    let hash = subject.hash();
    let current = subject.current;
    match subject.operation {
        Operation::Merge => match subject.incoming {
            Some(incoming) if hash.is_empty() => (
                format!("merging {incoming}"),
                hash,
                format!(" into {current}"),
            ),
            Some(incoming) => (
                format!("merging {incoming} "),
                hash,
                format!(" into {current}"),
            ),
            None => ("merging ".into(), hash, format!(" into {current}")),
        },
        Operation::CherryPick => ("applying ".into(), hash, format!(" onto {current}")),
        Operation::Revert => ("reverting ".into(), hash, format!(" on {current}")),
        Operation::Rebase => (format!("rebasing {current}"), String::new(), String::new()),
    }
}

fn sub(subject: &Subject, conflicts: bool, busy: Option<Command>) -> String {
    let hash = subject.hash();
    let current = subject.current;
    let verb = subject.operation.as_str();
    if let Some(command) = busy {
        return match (command, subject.operation) {
            (Command::Abort, _) => format!("Restoring {current} to its tip before the {verb}…"),
            (Command::Skip, _) => format!("Leaving {hash} out…"),
            (_, Operation::Merge) => "Writing the merge commit…".into(),
            (_, Operation::CherryPick) => "Committing the picked change…".into(),
            (_, Operation::Revert) => "Committing the revert…".into(),
            (_, Operation::Rebase) => "Applying the remaining steps…".into(),
        };
    }
    match (subject.operation, conflicts) {
        (Operation::Merge, true) => format!(
            "Ours is {current}, Theirs is {}. Resolve and stage each file, then continue.",
            subject.incoming_name()
        ),
        (Operation::Merge, false) => format!(
            "Continue writes the merge commit of {} into {current}.",
            subject.incoming_name()
        ),
        (Operation::CherryPick, true) => {
            "Theirs is the picked commit. Skip commit leaves it out and ends the cherry-pick."
                .into()
        }
        (Operation::CherryPick, false) => {
            format!("Continue commits {hash}’s change on {current} with its original message.")
        }
        (Operation::Revert, true) => {
            format!("Theirs undoes {hash}. Resolve and stage each file, then continue.")
        }
        (Operation::Revert, false) => "Continue commits “Revert …” with Git’s message.".into(),
        (Operation::Rebase, true) => {
            "Resolve each conflicted file and stage the result, then continue.".into()
        }
        (Operation::Rebase, false) => "Continue applies the remaining steps.".into(),
    }
}

/// Abort's tooltip: what it restores.
pub fn abort_tip(operation: Operation, current: &str) -> String {
    match operation {
        Operation::Merge => format!("Abort the merge and restore {current} to its previous tip"),
        Operation::Rebase => "Stop the rebase and restore the branch as it was".into(),
        _ => format!("Abort the {} and restore {current}", operation.as_str()),
    }
}

/// The note in the conflict workspace's footer.
pub fn abort_note(operation: Operation, current: &str) -> String {
    format!(
        "Abort restores {current} to its tip before the {}",
        operation.as_str()
    )
}

/// The band for a merge, cherry-pick, or revert Git holds open. `pending`
/// disables every button while another job runs; `busy` is the Continue,
/// Skip, or Abort that job runs, shown in the title.
pub fn mutation_band(
    subject: &Subject,
    conflicts: Conflicts,
    workspace_open: bool,
    pending: bool,
    busy: Option<Command>,
    steps: Vec<Step>,
) -> Band {
    let conflicted = conflicts.files > 0;
    let idle = !pending && busy.is_none();
    let (pre, hash, post) = description(subject);
    let lead = match busy {
        Some(command) => busy_label(subject.operation, command),
        None if conflicted => format!("{} conflict", label(subject.operation)),
        None if subject.operation == Operation::Rebase => "Rebase stopped".into(),
        None => format!("{} ready", label(subject.operation)),
    };
    let status = if busy.is_some() {
        String::new()
    } else if conflicted {
        format!(
            "{} left in {}",
            plural(conflicts.blocks.max(conflicts.files), "conflict"),
            plural(conflicts.files, "file")
        )
    } else {
        "All files staged".into()
    };
    let mut buttons = Vec::new();
    if conflicted && !workspace_open {
        buttons.push(Button {
            command: Command::ResolveConflicts,
            label: "Resolve conflicts",
            kind: Kind::Primary,
            key: None,
            enabled: idle,
            tip: "Open the conflict workspace in Changes".into(),
        });
    }
    buttons.push(Button {
        command: Command::Continue,
        label: "Continue",
        kind: if conflicted || !idle {
            Kind::Secondary
        } else {
            Kind::Primary
        },
        key: Some("Ctrl+Enter"),
        enabled: idle && !conflicted,
        tip: if !idle {
            "Wait for the current operation to finish".into()
        } else if conflicted {
            "Resolve and stage every conflicted file first".into()
        } else {
            "Ctrl+Enter".into()
        },
    });
    if subject.operation != Operation::Merge {
        let remaining = steps.iter().any(|step| step.state == StepState::Next);
        buttons.push(Button {
            command: Command::Skip,
            label: "Skip commit",
            kind: Kind::Secondary,
            key: None,
            enabled: idle,
            tip: if remaining {
                "Leave this commit out and continue with the next one".into()
            } else {
                format!(
                    "Leave this commit out and end the {}",
                    subject.operation.as_str()
                )
            },
        });
    }
    buttons.push(Button {
        command: Command::Abort,
        label: "Abort and restore",
        kind: Kind::Danger,
        key: None,
        enabled: idle,
        tip: abort_tip(subject.operation, subject.current),
    });
    Band {
        tone: if conflicted && busy.is_none() {
            Tone::Conflict
        } else {
            Tone::Ready
        },
        icon: match busy {
            Some(_) => Icon::Spinner,
            None if conflicted => Icon::Warning,
            None => Icon::Pause,
        },
        title: format!("{lead} · {pre}"),
        hash,
        post,
        status,
        sub: sub(subject, conflicted, busy),
        buttons,
        steps,
    }
}

/// `picking`, `rewording`, … for the step Git is applying.
fn applying(action: &str) -> String {
    match action {
        "pick" | "p" => "picking".into(),
        "reword" | "r" => "rewording".into(),
        "edit" | "e" => "reopening".into(),
        "squash" | "s" => "squashing".into(),
        "fixup" | "f" => "fixing up".into(),
        "drop" | "d" => "dropping".into(),
        "exec" | "x" => "running".into(),
        other => other.into(),
    }
}

/// The steps of a rebase Git is running: done, the current one, the rest.
pub fn steps_from_progress(progress: &RebaseProgress) -> Vec<Step> {
    let current = progress.done.len().saturating_sub(1);
    progress
        .done
        .iter()
        .chain(progress.todo.iter())
        .enumerate()
        .map(|(index, step)| Step {
            commit: step.commit.clone(),
            action: step.action.clone(),
            subject: step.subject.clone(),
            state: if index < current {
                StepState::Done
            } else if index == current && !progress.done.is_empty() {
                StepState::Current
            } else {
                StepState::Next
            },
        })
        .collect()
}

/// The band while a rewrite runs (no buttons: Git is writing, and the
/// backup named in the footer is the way back), then while the new history
/// is read.
pub fn rewrite_band(branch: &str, mut steps: Vec<Step>, refreshing: bool) -> Band {
    let sub =
        "Browsing stays available; the old History tree is labelled until the new one is accepted."
            .to_string();
    if refreshing {
        for step in &mut steps {
            step.state = StepState::Done;
        }
        return Band {
            tone: Tone::Ready,
            icon: Icon::Spinner,
            title: "Refreshing the History tree…".into(),
            hash: String::new(),
            post: String::new(),
            status: String::new(),
            sub,
            buttons: Vec::new(),
            steps,
        };
    }
    let total = steps.len();
    let (title, hash) = match steps
        .iter()
        .position(|step| step.state == StepState::Current)
    {
        Some(index) => (
            format!(
                "Rewriting {branch} · step {} of {total} · {} ",
                index + 1,
                applying(&steps[index].action)
            ),
            steps[index].commit.chars().take(8).collect(),
        ),
        None => (
            format!("Rewriting {branch} · {}", plural(total, "step")),
            String::new(),
        ),
    };
    Band {
        tone: Tone::Ready,
        icon: Icon::Spinner,
        title,
        hash,
        post: String::new(),
        status: String::new(),
        sub,
        buttons: Vec::new(),
        steps,
    }
}

fn palette(tone: Tone) -> rebase_stop::Palette {
    rebase_stop::tone_palette(tone == Tone::Conflict)
}

/// Status and second-line colors from the band mockups.
fn secondary_colors(tone: Tone) -> (egui::Color32, egui::Color32) {
    use egui::Color32 as C;
    match tone {
        Tone::Ready => (C::from_rgb(180, 172, 220), C::from_rgb(216, 212, 238)),
        Tone::Conflict => (C::from_rgb(200, 184, 143), C::from_rgb(217, 203, 166)),
    }
}

/// Height of each band line; the band reserves [`HEIGHT`] for both.
const LINE: f32 = 20.0;
/// The band's fixed height: two lines, the gap, and the padding.
pub const HEIGHT: f32 = 6.0 + LINE + 6.0 + LINE + 6.0;
/// Space between chips.
const CHIP_GAP: f32 = 4.0;

fn chip_text(step: &Step) -> String {
    let mark = match step.state {
        StepState::Done => phosphor::CHECK.to_string(),
        StepState::Current => phosphor::CARET_RIGHT.into(),
        StepState::Next => "·".into(),
    };
    format!(
        "{mark} {} {}",
        &step.commit[..step.commit.len().min(8)],
        step.action
    )
}

/// The chips that fit in `width`, given each chip's width and the width of
/// a count label: the current step always, then as many next steps as fit,
/// then done steps; the rest collapse into `✓ n done` and `+n next`.
pub fn visible_steps(
    steps: &[Step],
    width: f32,
    chip: impl Fn(usize) -> f32,
    count: f32,
) -> (usize, usize) {
    if steps.is_empty() {
        return (0, 0);
    }
    let current = steps
        .iter()
        .position(|step| step.state == StepState::Current)
        .unwrap_or_else(|| {
            steps
                .iter()
                .position(|step| step.state == StepState::Next)
                .unwrap_or(steps.len() - 1)
        });
    let (mut first, mut last) = (current, current);
    let used = |first: usize, last: usize| {
        let chips: f32 = (first..=last).map(|index| chip(index) + CHIP_GAP).sum();
        let counts = (first > 0) as u8 as f32 + (last + 1 < steps.len()) as u8 as f32;
        chips + counts * (count + CHIP_GAP)
    };
    while last + 1 < steps.len() && used(first, last + 1) <= width {
        last += 1;
    }
    while first > 0 && used(first - 1, last) <= width {
        first -= 1;
    }
    (first, last)
}

/// The step strip, left to right in `rect`: `✓ n done`, the chips that fit
/// (the current one always), `+n next`; the full plan is on hover.
fn strip(ui: &mut egui::Ui, rect: egui::Rect, colors: rebase_stop::Palette, steps: &[Step]) {
    let font = egui::FontId::proportional(11.5);
    let measure = |text: String| {
        ui.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(text, font.clone(), egui::Color32::WHITE)
                .size()
                .x
        })
    };
    let widths: Vec<f32> = steps
        .iter()
        .map(|step| measure(chip_text(step)) + 14.0)
        .collect();
    let count = measure(format!("+{} next", steps.len().max(10))).max(measure(format!(
        "{} {} done",
        phosphor::CHECK,
        steps.len().max(10)
    )));
    let (first, last) = visible_steps(steps, rect.width(), |index| widths[index], count);
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    row.set_clip_rect(rect.intersect(ui.clip_rect()));
    row.spacing_mut().item_spacing.x = CHIP_GAP;
    let all = steps
        .iter()
        .map(|step| {
            format!(
                "{} {} {}",
                &step.commit[..step.commit.len().min(8)],
                step.action,
                step.subject
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if first > 0 {
        row.label(
            egui::RichText::new(format!("{} {first} done", phosphor::CHECK))
                .size(11.5)
                .weak(),
        )
        .on_hover_text(&all);
    }
    for step in &steps[first..=last] {
        rebase_stop::chip(
            &mut row,
            colors,
            chip_text(step),
            step.state == StepState::Current,
            step.state == StepState::Done,
        )
        .on_hover_text(format!("{} · {}", step.commit, step.subject));
    }
    if last + 1 < steps.len() {
        row.label(
            egui::RichText::new(format!("+{} next", steps.len() - last - 1))
                .size(11.5)
                .weak(),
        )
        .on_hover_text(&all);
    }
}

/// Draw the band in the space given (at least [`HEIGHT`] tall); returns
/// the command chosen with the pointer and each button's rectangle, in
/// order. Line 1: icon, title (truncates first, full text on hover), the
/// status at normal size, the buttons. Line 2: what to do, then the steps.
pub fn show(ui: &mut egui::Ui, band: &Band) -> (Option<Command>, Vec<egui::Rect>) {
    let colors = palette(band.tone);
    let (status_color, sub_color) = secondary_colors(band.tone);
    let rect = ui.max_rect().expand2(egui::vec2(8.0, 4.0));
    ui.painter().rect_filled(rect, 0.0, colors.band);
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        egui::Stroke::new(1.0, colors.edge),
    );
    let mut command = None;
    let mut rects = vec![egui::Rect::NOTHING; band.buttons.len()];
    let top = ui.max_rect().top() + 2.0;
    let left = ui.max_rect().left();
    let width = ui.max_rect().width();
    let line = |index: f32| {
        egui::Rect::from_min_size(
            egui::pos2(left, top + index * (LINE + 6.0)),
            egui::vec2(width, LINE),
        )
    };
    let mut first = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(line(0.0))
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    first.set_clip_rect(line(0.0).intersect(ui.clip_rect()));
    let ui1 = &mut first;
    ui1.spacing_mut().item_spacing.x = 6.0;
    for (index, button) in band.buttons.iter().enumerate().rev() {
        let text = match button.kind {
            Kind::Primary if button.enabled => {
                egui::RichText::new(button.label).color(egui::Color32::WHITE)
            }
            Kind::Danger => {
                egui::RichText::new(button.label).color(egui::Color32::from_rgb(245, 160, 160))
            }
            _ => egui::RichText::new(button.label),
        };
        let mut widget = match button.key {
            Some(key) => egui::Button::new((
                text,
                egui::RichText::new(crate::keymap::shown(key))
                    .small()
                    .color(if button.kind == Kind::Primary && button.enabled {
                        egui::Color32::from_rgb(184, 212, 230)
                    } else {
                        egui::Color32::from_gray(140)
                    }),
            )),
            None => egui::Button::new(text),
        };
        widget = widget.min_size(egui::vec2(0.0, LINE));
        if button.kind == Kind::Primary && button.enabled {
            widget = widget.fill(egui::Color32::from_rgb(11, 85, 119));
        }
        let response = ui1.add_enabled(button.enabled, widget);
        rects[index] = response.rect;
        let response = if button.enabled {
            response.on_hover_text(crate::keymap::shown(&button.tip))
        } else {
            response.on_disabled_hover_text(crate::keymap::shown(&button.tip))
        };
        if response.clicked() {
            command = Some(button.command);
        }
    }
    ui1.add_space(4.0);
    // Left of the buttons: icon, title, then the status right after it, as
    // in the mockup. The status keeps its full size; the title truncates
    // first, down to a few characters.
    let rest = ui1.available_size();
    ui1.allocate_ui_with_layout(
        rest,
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            match band.icon {
                Icon::Spinner => {
                    ui.add(egui::Spinner::new().size(12.0).color(colors.ink));
                }
                Icon::Warning => {
                    ui.label(phosphor::icon(phosphor::WARNING).color(colors.ink));
                }
                Icon::Pause => {
                    ui.label(phosphor::icon(phosphor::PAUSE).color(colors.ink));
                }
            }
            let format = |family: egui::FontFamily, size: f32| egui::TextFormat {
                font_id: egui::FontId::new(size, family),
                color: colors.ink,
                ..Default::default()
            };
            let status_width = if band.status.is_empty() {
                0.0
            } else {
                ui.fonts_mut(|fonts| {
                    fonts
                        .layout_no_wrap(
                            band.status.clone(),
                            egui::FontId::proportional(12.5),
                            status_color,
                        )
                        .size()
                        .x
                }) + 10.0
            };
            let room = ui.available_width();
            let status_room = status_width.min((room - 80.0).max(60.0));
            let mut title = egui::text::LayoutJob::default();
            title.append(
                &band.title,
                0.0,
                format(egui::FontFamily::Proportional, 12.5),
            );
            title.append(&band.hash, 0.0, format(egui::FontFamily::Monospace, 11.0));
            title.append(
                &band.post,
                0.0,
                format(egui::FontFamily::Proportional, 12.5),
            );
            title.wrap =
                egui::text::TextWrapping::truncate_at_width((room - status_room).max(40.0));
            // Painted as laid out: a Label would re-wrap the job to the row.
            let galley = ui.fonts_mut(|fonts| fonts.layout_job(title));
            let (slot, response) =
                ui.allocate_exact_size(egui::vec2(galley.size().x, LINE), egui::Sense::hover());
            ui.painter().galley(
                egui::pos2(slot.left(), slot.center().y - galley.size().y / 2.0),
                galley,
                colors.ink,
            );
            response.on_hover_text(band.title_text());
            if !band.status.is_empty() {
                ui.add_space(4.0);
                ui.scope(|ui| {
                    ui.set_max_width(status_room);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&band.status)
                                .size(12.5)
                                .color(status_color),
                        )
                        .truncate(),
                    )
                    .on_hover_text(&band.status);
                });
            }
        },
    );
    let mut second = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(line(1.0))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    second.set_clip_rect(line(1.0).intersect(ui.clip_rect()));
    let ui2 = &mut second;
    // The explanation on the left, the steps right-aligned in a reserved
    // width; neither overlaps the other.
    let row = line(1.0);
    // The explanation takes its width, leaving the steps at least 45% of the
    // row; the steps follow it.
    let sub_width = ui.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(
                band.sub.clone(),
                egui::FontId::proportional(12.5),
                sub_color,
            )
            .size()
            .x
    }) + 2.0;
    let note_width = if band.steps.is_empty() {
        row.width()
    } else {
        sub_width.min((row.width() * 0.55).floor())
    };
    let note = egui::Rect::from_min_size(row.min, egui::vec2(note_width, row.height()));
    ui2.scope_builder(
        egui::UiBuilder::new()
            .max_rect(note)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.set_clip_rect(note.intersect(ui.clip_rect()));
            ui.add(
                egui::Label::new(egui::RichText::new(&band.sub).size(12.5).color(sub_color))
                    .truncate(),
            )
            .on_hover_text(&band.sub);
        },
    );
    if !band.steps.is_empty() {
        let rect = egui::Rect::from_min_max(egui::pos2(note.right() + 12.0, row.top()), row.max);
        strip(ui2, rect, colors, &band.steps);
    }
    ui.allocate_rect(
        egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(width, HEIGHT - 8.0)),
        egui::Sense::hover(),
    );
    (command, rects)
}
#[cfg(test)]
mod tests {
    use super::*;

    fn subject(operation: Operation) -> Subject<'static> {
        Subject {
            operation,
            current: "main",
            incoming: (operation == Operation::Merge).then_some("origin/main"),
            commit: Some(match operation {
                Operation::Merge => "0f51a473aaaa",
                Operation::CherryPick => "9a4f9a95bbbb",
                _ => "7151cc83cccc",
            }),
        }
    }

    fn two() -> Conflicts {
        Conflicts {
            files: 2,
            blocks: 2,
        }
    }

    fn labels(band: &Band) -> Vec<&'static str> {
        band.buttons.iter().map(|button| button.label).collect()
    }

    #[test]
    fn conflict_bands_name_the_operation_and_what_is_left() {
        let merge = mutation_band(
            &subject(Operation::Merge),
            two(),
            false,
            false,
            None,
            vec![],
        );
        assert_eq!(
            merge.title_text(),
            "Merge conflict · merging origin/main 0f51a473 into main"
        );
        assert_eq!(merge.hash, "0f51a473");
        assert_eq!(merge.status, "2 conflicts left in 2 files");
        assert_eq!(
            merge.sub,
            "Ours is main, Theirs is origin/main. Resolve and stage each file, then continue."
        );
        assert_eq!(merge.tone, Tone::Conflict);
        assert_eq!(merge.icon, Icon::Warning);
        assert_eq!(
            labels(&merge),
            ["Resolve conflicts", "Continue", "Abort and restore"],
            "merge has no Skip commit"
        );
        assert_eq!(
            merge
                .button(Command::Abort)
                .map(|button| button.tip.as_str()),
            Some("Abort the merge and restore main to its previous tip")
        );

        let one = Conflicts {
            files: 1,
            blocks: 1,
        };
        let pick = mutation_band(
            &subject(Operation::CherryPick),
            one,
            false,
            false,
            None,
            vec![],
        );
        assert_eq!(
            pick.title_text(),
            "Cherry-pick conflict · applying 9a4f9a95 onto main"
        );
        assert_eq!(pick.status, "1 conflict left in 1 file");
        assert_eq!(
            labels(&pick),
            [
                "Resolve conflicts",
                "Continue",
                "Skip commit",
                "Abort and restore"
            ]
        );
        let revert = mutation_band(&subject(Operation::Revert), one, false, false, None, vec![]);
        assert_eq!(
            revert.title_text(),
            "Revert conflict · reverting 7151cc83 on main"
        );
        assert_eq!(
            revert
                .button(Command::Abort)
                .map(|button| button.tip.as_str()),
            Some("Abort the revert and restore main")
        );
    }

    #[test]
    fn continue_is_the_primary_only_once_nothing_is_conflicted() {
        let conflicted = mutation_band(
            &subject(Operation::Merge),
            two(),
            false,
            false,
            None,
            vec![],
        );
        let button = conflicted.button(Command::Continue).cloned();
        assert!(button.as_ref().is_some_and(|button| !button.enabled));
        assert_eq!(
            button.map(|button| (button.kind, button.tip, button.key)),
            Some((
                Kind::Secondary,
                "Resolve and stage every conflicted file first".to_string(),
                Some("Ctrl+Enter")
            ))
        );
        assert_eq!(
            conflicted
                .button(Command::ResolveConflicts)
                .map(|button| button.kind),
            Some(Kind::Primary)
        );

        let ready = mutation_band(
            &subject(Operation::Revert),
            Conflicts::default(),
            false,
            false,
            None,
            vec![],
        );
        assert_eq!(
            ready.title_text(),
            "Revert ready · reverting 7151cc83 on main"
        );
        assert_eq!(ready.status, "All files staged");
        assert_eq!(ready.tone, Tone::Ready);
        assert_eq!(ready.icon, Icon::Pause);
        assert_eq!(
            labels(&ready),
            ["Continue", "Skip commit", "Abort and restore"]
        );
        let button = ready.button(Command::Continue);
        assert!(button.is_some_and(|button| button.enabled && button.kind == Kind::Primary));
        let merge = mutation_band(
            &subject(Operation::Merge),
            Conflicts::default(),
            false,
            false,
            None,
            vec![],
        );
        assert_eq!(
            merge.title_text(),
            "Merge ready · merging origin/main 0f51a473 into main"
        );
    }

    #[test]
    fn resolve_conflicts_hides_while_the_workspace_is_open() {
        let band = mutation_band(
            &subject(Operation::CherryPick),
            two(),
            true,
            false,
            None,
            vec![],
        );
        assert!(band.button(Command::ResolveConflicts).is_none());
    }

    #[test]
    fn a_running_job_keeps_the_buttons_and_names_itself() {
        let band = mutation_band(
            &subject(Operation::Merge),
            Conflicts::default(),
            false,
            true,
            Some(Command::Continue),
            vec![],
        );
        assert_eq!(
            band.title_text(),
            "Continuing merge · merging origin/main 0f51a473 into main"
        );
        assert_eq!(band.icon, Icon::Spinner);
        assert_eq!(band.status, "");
        assert_eq!(band.sub, "Writing the merge commit…");
        assert_eq!(labels(&band), ["Continue", "Abort and restore"]);
        assert!(band.buttons.iter().all(|button| !button.enabled));
        assert_eq!(
            busy_label(Operation::CherryPick, Command::Skip),
            "Skipping cherry-pick"
        );
        assert_eq!(
            busy_label(Operation::Revert, Command::Abort),
            "Aborting revert"
        );
    }

    #[test]
    fn the_busy_band_keeps_every_button_in_place() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let draw = |band: &Band| {
            let mut rects = Vec::new();
            for _ in 0..2 {
                let _ = crate::tests::run_ui(&ctx, egui::RawInput::default(), |ui| {
                    rects = show(ui, band).1;
                });
            }
            rects
        };
        let pick = |busy: Option<Command>| {
            mutation_band(
                &subject(Operation::CherryPick),
                Conflicts::default(),
                false,
                busy.is_some(),
                busy,
                vec![],
            )
        };
        let idle = draw(&pick(None));
        assert_eq!(idle.len(), 3);
        for command in [Command::Continue, Command::Skip, Command::Abort] {
            assert_eq!(draw(&pick(Some(command))), idle, "{command:?}");
        }
    }

    #[test]
    fn the_rewrite_band_names_the_step_git_is_applying() {
        let step = |commit: char, action: &str, state| Step {
            commit: commit.to_string().repeat(40),
            action: action.into(),
            subject: String::new(),
            state,
        };
        let steps = vec![
            step('a', "drop", StepState::Done),
            step('b', "reword", StepState::Done),
            step('c', "pick", StepState::Current),
            step('d', "reword", StepState::Next),
            step('e', "fixup", StepState::Next),
        ];
        let band = rewrite_band("repo-and-branch-ux", steps.clone(), false);
        assert_eq!(
            band.title_text(),
            "Rewriting repo-and-branch-ux · step 3 of 5 · picking cccccccc"
        );
        assert_eq!(
            band.sub,
            "Browsing stays available; the old History tree is labelled until the new one is accepted."
        );
        assert!(band.buttons.is_empty(), "no cancel while Git writes");
        assert_eq!(band.steps.len(), 5);
        let band = rewrite_band("repo-and-branch-ux", steps, true);
        assert_eq!(band.title_text(), "Refreshing the History tree…");
        assert!(band.steps.iter().all(|step| step.state == StepState::Done));
    }

    /// Draw `band` in a `width`-wide band slot as the app does; returns the
    /// slot and every painted text with its rectangle.
    fn painted(band: &Band, width: f32) -> (egui::Rect, Vec<(String, egui::Rect)>) {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 400.0),
            )),
            ..Default::default()
        };
        let mut slot = egui::Rect::NOTHING;
        let mut output = None;
        for _ in 0..2 {
            output = Some(crate::tests::run_ui(&ctx, input(), |ui| {
                egui::Panel::top("band").exact_size(HEIGHT).show(ui, |ui| {
                    slot = ui.max_rect().expand2(egui::vec2(8.0, 4.0));
                    ui.set_clip_rect(slot);
                    show(ui, band);
                });
            }));
        }
        let texts = output
            .map(|output| output.shapes)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|shape| match shape.shape {
                egui::epaint::Shape::Text(text) => Some((
                    text.galley.job.text.clone(),
                    text.visual_bounding_rect().intersect(shape.clip_rect),
                )),
                _ => None,
            })
            .collect();
        (slot, texts)
    }

    fn twelve_steps() -> Vec<Step> {
        (0..12)
            .map(|index| Step {
                commit: format!("{index:x}").repeat(40),
                action: "pick".into(),
                subject: format!("step {index}"),
                state: match index {
                    0..=3 => StepState::Done,
                    4 => StepState::Current,
                    _ => StepState::Next,
                },
            })
            .collect()
    }

    #[test]
    fn the_band_holds_its_lines_chips_and_status_without_clipping() {
        let mut band = mutation_band(
            &subject(Operation::CherryPick),
            Conflicts {
                files: 1,
                blocks: 1,
            },
            false,
            false,
            None,
            twelve_steps(),
        );
        band.status = "Resolve and stage 1 conflicted file first.".into();
        for width in [1120.0, 720.0] {
            let (slot, texts) = painted(&band, width);
            for (index, (text, rect)) in texts.iter().enumerate() {
                assert!(
                    slot.contains_rect(*rect),
                    "{text:?} {rect:?} leaves the band {slot:?} at {width}"
                );
                for (other, other_rect) in &texts[index + 1..] {
                    assert!(
                        !rect.shrink(0.5).intersects(other_rect.shrink(0.5)),
                        "{text:?} overlaps {other:?} at {width}"
                    );
                }
            }
            let sub = texts
                .iter()
                .find(|(text, _)| {
                    text == &band.sub
                        || (text.ends_with('…') && band.sub.starts_with(text.trim_end_matches('…')))
                })
                .map(|(_, rect)| *rect);
            assert!(sub.is_some(), "the explanation shows at {width}: {texts:?}");
            let chips = texts
                .iter()
                .filter(|(text, _)| {
                    text.starts_with(['·', '+'])
                        || text.starts_with(phosphor::CARET_RIGHT)
                        || text.starts_with(phosphor::CHECK)
                })
                .collect::<Vec<_>>();
            assert!(
                chips
                    .iter()
                    .any(|(text, _)| text.starts_with(phosphor::CARET_RIGHT)),
                "the current chip shows at {width}: {texts:?}"
            );
            for (index, (text, rect)) in chips.iter().enumerate() {
                assert!(
                    sub.is_none_or(|sub| !rect.intersects(sub)),
                    "{text:?} overlaps the explanation at {width}"
                );
                for (other, other_rect) in &chips[index + 1..] {
                    assert!(
                        !rect.intersects(*other_rect),
                        "{text:?} overlaps {other:?} at {width}"
                    );
                }
            }
            assert!(
                chips.iter().any(|(text, _)| text.starts_with('+')),
                "the rest collapses to +N next at {width}: {texts:?}"
            );
            if width >= 1120.0 {
                assert!(
                    texts.iter().any(|(text, _)| *text == band.status),
                    "the status reads in full at {width}: {texts:?}"
                );
            }
        }
    }

    #[test]
    fn the_strip_keeps_the_current_step_and_collapses_the_rest() {
        let steps = twelve_steps();
        // Room for four chips and both counts: next steps come first.
        assert_eq!(
            visible_steps(&steps, 4.0 * 104.0 + 2.0 * 64.0, |_| 100.0, 60.0),
            (4, 7)
        );
        // Room for one chip only: the current one.
        assert_eq!(visible_steps(&steps, 10.0, |_| 100.0, 60.0), (4, 4));
    }

    #[test]
    fn plurals_read_naturally() {
        assert_eq!(plural(1, "commit"), "1 commit");
        assert_eq!(plural(4, "commit"), "4 commits");
    }
}
