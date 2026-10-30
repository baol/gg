//! The operation area while an interactive rebase is stopped.
//!
//! The first line names the stop and holds Continue (`Ctrl+Enter`), Skip
//! commit, and Abort and restore; the second explains what to do and shows the
//! step chips (done, current, next). A reopened commit also lists the commits
//! made at the stop.

use crate::phosphor;
use eframe::egui;
use gg_engine::mutation::{RebaseProgress, RebaseStop};

/// Chips shown on each side of the current step before collapsing to a count.
const NEIGHBOURS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Continue,
    Skip,
    Abort,
    ResolveConflicts,
}

fn short(id: &str) -> &str {
    &id[..id.len().min(7)]
}

/// Why Continue cannot run yet, if it cannot.
pub fn continue_reason(progress: &RebaseProgress, conflicts: usize) -> Option<String> {
    if conflicts > 0 {
        return Some(format!(
            "Resolve and stage {conflicts} conflicted file{} first.",
            if conflicts == 1 { "" } else { "s" }
        ));
    }
    if matches!(progress.stop, RebaseStop::Reopened { base: Some(_), .. })
        && progress.tracked_changes > 0
    {
        let count = progress.tracked_changes;
        return Some(format!(
            "Commit or stash the {count} remaining changed file{} first.",
            if count == 1 { "" } else { "s" }
        ));
    }
    None
}

/// Whether Continue is the obvious next step: a reopened commit is fully
/// committed again.
pub fn continue_highlighted(progress: &RebaseProgress) -> bool {
    matches!(&progress.stop, RebaseStop::Reopened { made, .. } if !made.is_empty())
        && progress.tracked_changes == 0
}

/// The stop's title, as in the mockups: what stopped, the step, and the
/// commit with an 8-character ID.
fn title(progress: &RebaseProgress) -> String {
    let (step, total) = progress.position();
    let id = |commit: &str| commit[..commit.len().min(8)].to_string();
    let current = progress.done.last();
    match &progress.stop {
        RebaseStop::Conflict => format!(
            "Conflict · step {step} of {total} · applying {}",
            current
                .map(|step| format!("{} {}", id(&step.commit), step.subject))
                .unwrap_or_default()
        ),
        RebaseStop::Reopened { commit, .. } => {
            format!(
                "Rebase stopped · step {step} of {total} · reopened {}",
                id(commit)
            )
        }
        RebaseStop::Other => format!(
            "Rebase stopped · step {step} of {total}{}",
            current
                .map(|step| format!(" · after {}", id(&step.commit)))
                .unwrap_or_default()
        ),
    }
}

fn explanation(progress: &RebaseProgress) -> &'static str {
    match &progress.stop {
        RebaseStop::Conflict => "Resolve each conflicted file and stage the result, then continue.",
        RebaseStop::Reopened { base: Some(_), .. } => {
            "Its changes are staged on its parent in Local Changes. Commit them in one or more parts; unstage what belongs in a later part."
        }
        RebaseStop::Reopened { base: None, .. } => {
            "A root commit stays at HEAD. Amend it in Local Changes, then continue."
        }
        RebaseStop::Other => "Git stopped here. Fix the cause, then continue, skip, or abort.",
    }
}

/// Step chips: done (with a check), the current step, and the next steps,
/// collapsing far steps into counts.
fn chips(ui: &mut egui::Ui, progress: &RebaseProgress) {
    let current = progress.done.len().saturating_sub(1);
    let steps = progress.done.iter().chain(progress.todo.iter());
    let total = progress.done.len() + progress.todo.len();
    let first = current.saturating_sub(NEIGHBOURS);
    let last = (current + NEIGHBOURS).min(total.saturating_sub(1));
    if first > 0 {
        ui.weak(format!("{} {first} done", phosphor::CHECK));
    }
    for (index, step) in steps.enumerate().skip(first).take(last + 1 - first) {
        let label = format!("{} {}", step.action, step.subject);
        let text = if index < current {
            egui::RichText::new(format!("{} {label}", phosphor::CHECK)).weak()
        } else if index == current {
            egui::RichText::new(label)
                .strong()
                .background_color(ui.visuals().selection.bg_fill)
        } else {
            egui::RichText::new(label).weak()
        };
        ui.add(egui::Label::new(text.small()).truncate())
            .on_hover_text(format!("{} · {}", step.commit, step.subject));
    }
    if last + 1 < total {
        ui.weak(format!("+{} next", total - last - 1));
    }
}

/// Draw the stop; returns the command chosen with the pointer.
pub fn show(
    ui: &mut egui::Ui,
    progress: &RebaseProgress,
    conflicts: usize,
    pending: bool,
) -> Option<Command> {
    let mut command = None;
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(egui::RichText::new(title(progress)).strong()).truncate());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(!pending, egui::Button::new("Abort and restore"))
                .on_hover_text("Stop the rebase and restore the branch as it was.")
                .clicked()
            {
                command = Some(Command::Abort);
            }
            if ui
                .add_enabled(!pending, egui::Button::new("Skip commit"))
                .on_hover_text("Leave this commit out and continue with the next step.")
                .clicked()
            {
                command = Some(Command::Skip);
            }
            let reason = continue_reason(progress, conflicts);
            let button = egui::Button::new(egui::RichText::new("Continue").strong())
                .selected(continue_highlighted(progress));
            let response = ui
                .add_enabled(!pending && reason.is_none(), button)
                .on_hover_text("Ctrl+Enter");
            let response = match &reason {
                Some(reason) => response.on_disabled_hover_text(reason),
                None => response,
            };
            if response.clicked() {
                command = Some(Command::Continue);
            }
            if conflicts > 0
                && ui
                    .add_enabled(!pending, egui::Button::new("Resolve conflicts…"))
                    .clicked()
            {
                command = Some(Command::ResolveConflicts);
            }
            if let Some(reason) = &reason {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(reason)
                            .small()
                            .color(ui.visuals().warn_fg_color),
                    )
                    .truncate(),
                );
            }
        });
    });
    // Second line, fixed: what to do (or the commits made at a reopened
    // commit) in a fixed-width slot on the left, the step chips on the right.
    ui.horizontal(|ui| {
        let note = match &progress.stop {
            RebaseStop::Reopened { made, .. } if !made.is_empty() => format!(
                "Commits made at this stop: {}",
                made.iter()
                    .map(|step| format!("{} {}", short(&step.commit), step.subject))
                    .collect::<Vec<_>>()
                    .join(" · ")
            ),
            _ => explanation(progress).to_string(),
        };
        let width = (ui.available_width() * 0.5).floor();
        ui.allocate_ui_with_layout(
            egui::vec2(width, ui.spacing().interact_size.y),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(width);
                ui.add(egui::Label::new(egui::RichText::new(&note).weak()).truncate())
                    .on_hover_text(&note);
            },
        );
        ui.spacing_mut().item_spacing.x = 10.0;
        chips(ui, progress);
    });
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use gg_engine::mutation::RebaseStep;

    fn step(action: &str, id: char, subject: &str) -> RebaseStep {
        RebaseStep {
            action: action.into(),
            commit: id.to_string().repeat(40),
            subject: subject.into(),
        }
    }

    fn reopened(made: usize, tracked_changes: usize) -> RebaseProgress {
        RebaseProgress {
            branch: Some("feature".into()),
            done: vec![step("pick", 'a', "one"), step("pick", 'b', "two")],
            todo: vec![step("pick", 'c', "three")],
            stop: RebaseStop::Reopened {
                base: Some("a".repeat(40)),
                commit: "b".repeat(40),
                subject: "two".into(),
                message: "two".into(),
                made: (0..made)
                    .map(|index| step("commit", 'd', &format!("part {index}")))
                    .collect(),
            },
            tracked_changes,
        }
    }

    #[test]
    fn continue_waits_for_conflicts_and_remaining_changes() {
        let progress = reopened(0, 2);
        assert_eq!(
            continue_reason(&progress, 0).as_deref(),
            Some("Commit or stash the 2 remaining changed files first.")
        );
        assert!(!continue_highlighted(&progress));
        let progress = reopened(2, 0);
        assert_eq!(continue_reason(&progress, 0), None);
        assert!(continue_highlighted(&progress));
        assert_eq!(
            continue_reason(&progress, 1).as_deref(),
            Some("Resolve and stage 1 conflicted file first.")
        );
        let mut root = reopened(0, 1);
        root.stop = RebaseStop::Reopened {
            base: None,
            commit: "b".repeat(40),
            subject: "two".into(),
            message: "two".into(),
            made: vec![],
        };
        assert_eq!(continue_reason(&root, 0), None, "a root commit is amended");
    }

    #[test]
    fn titles_name_the_stop() {
        assert_eq!(
            title(&reopened(0, 1)),
            "Rebase stopped · step 2 of 3 · reopened bbbbbbbb"
        );
        let mut conflict = reopened(0, 0);
        conflict.stop = RebaseStop::Conflict;
        assert_eq!(
            title(&conflict),
            "Conflict · step 2 of 3 · applying bbbbbbbb two"
        );
    }
}
