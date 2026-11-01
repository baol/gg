//! The operation area while an interactive rebase is stopped.
//!
//! The first line names the stop and holds Continue (`Ctrl+Enter`), Skip
//! commit, and Abort and restore; the second explains what to do and shows the
//! step chips (done, current, next). A reopened commit also lists the commits
//! made at the stop.

use crate::operation_band;
use eframe::egui;
use gg_engine::mutation::{RebaseProgress, RebaseStop};

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
            "Its changes are staged. Edit, unstage parts to split, then continue."
        }
        RebaseStop::Reopened { base: None, .. } => {
            "A root commit stays at HEAD. Amend it in Local Changes, then continue."
        }
        RebaseStop::Other => "Git stopped here. Fix the cause, then continue, skip, or abort.",
    }
}

/// The stop band's colors, as in the stop mockups: purple for a reopened
/// or paused step, amber for a conflict.
#[derive(Clone, Copy)]
pub struct Palette {
    pub band: egui::Color32,
    pub edge: egui::Color32,
    pub ink: egui::Color32,
    pub chip: egui::Color32,
    pub chip_edge: egui::Color32,
}

pub fn palette(progress: &RebaseProgress) -> Palette {
    tone_palette(progress.stop == RebaseStop::Conflict)
}

/// The amber conflict palette, or the purple one.
pub fn tone_palette(conflict: bool) -> Palette {
    use egui::Color32 as C;
    if conflict {
        Palette {
            band: C::from_rgb(51, 39, 15),
            edge: C::from_rgb(106, 74, 20),
            ink: C::from_rgb(240, 195, 90),
            chip: C::from_rgb(77, 58, 20),
            chip_edge: C::from_rgb(192, 144, 46),
        }
    } else {
        Palette {
            band: C::from_rgb(36, 33, 58),
            edge: C::from_rgb(58, 51, 96),
            ink: C::from_rgb(200, 192, 245),
            chip: C::from_rgb(58, 51, 96),
            chip_edge: C::from_rgb(125, 112, 208),
        }
    }
}

/// One bordered step chip.
pub(crate) fn chip(
    ui: &mut egui::Ui,
    colors: Palette,
    text: String,
    current: bool,
    done: bool,
) -> egui::Response {
    let (fill, edge, color) = if current {
        (colors.chip, colors.chip_edge, egui::Color32::WHITE)
    } else if done {
        (
            egui::Color32::TRANSPARENT,
            egui::Color32::from_gray(58),
            egui::Color32::from_gray(140),
        )
    } else {
        (
            egui::Color32::TRANSPARENT,
            egui::Color32::from_gray(58),
            egui::Color32::from_gray(200),
        )
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, edge))
        .corner_radius(3.0)
        .inner_margin(egui::Margin::symmetric(6, 0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(11.5).color(color))
        })
        .inner
}

/// The stop as an operation band: the title, Continue's reason as the
/// status, Resolve conflicts while the workspace is closed, Continue, Skip commit, Abort and
/// restore, the explanation (or the commits made at a reopened commit), and
/// the step chips.
pub fn band(
    progress: &RebaseProgress,
    conflicts: usize,
    workspace_open: bool,
    pending: bool,
    busy: Option<&str>,
    backup: Option<&str>,
) -> operation_band::Band {
    use operation_band::{Button, Command as C, Icon, Kind, StepState, Tone};
    let reason = continue_reason(progress, conflicts);
    let idle = !pending && busy.is_none();
    let (step, total) = progress.position();
    let mut buttons = Vec::new();
    // Resolve conflicts is the primary until the workspace is open.
    if conflicts > 0 && !workspace_open {
        buttons.push(Button {
            command: C::ResolveConflicts,
            label: "Resolve conflicts",
            kind: Kind::Primary,
            key: None,
            enabled: idle,
            tip: "Open the conflict workspace in Changes".into(),
        });
    }
    buttons.push(Button {
        command: C::Continue,
        label: "Continue",
        kind: if idle && reason.is_none() {
            Kind::Primary
        } else {
            Kind::Secondary
        },
        key: Some("Ctrl+Enter"),
        enabled: idle && reason.is_none(),
        tip: match &reason {
            _ if !idle => "Wait for the current operation to finish".into(),
            Some(reason) => reason.clone(),
            None => "Ctrl+Enter".into(),
        },
    });
    buttons.push(Button {
        command: C::Skip,
        label: "Skip commit",
        kind: Kind::Secondary,
        key: None,
        enabled: idle,
        tip: "Leave this commit out and continue with the next step.".into(),
    });
    buttons.push(Button {
        command: C::Abort,
        label: "Abort and restore",
        kind: Kind::Danger,
        key: None,
        enabled: idle,
        tip: match backup {
            Some(backup) => {
                format!("Stop the rebase and restore the branch as it was.\nBackup: {backup}")
            }
            None => "Stop the rebase and restore the branch as it was.".to_string(),
        },
    });
    let sub = match &progress.stop {
        RebaseStop::Reopened { made, .. } if !made.is_empty() => format!(
            "Commits made at this stop: {}",
            made.iter()
                .map(|step| format!("{} {}", short(&step.commit), step.subject))
                .collect::<Vec<_>>()
                .join(" · ")
        ),
        _ => explanation(progress).to_string(),
    };
    let mut steps = operation_band::steps_from_progress(progress);
    // The reopened step reads `reopen`, as in the stop mockup.
    if matches!(progress.stop, RebaseStop::Reopened { .. })
        && let Some(current) = steps
            .iter_mut()
            .find(|step| step.state == StepState::Current)
    {
        current.action = "reopen".into();
    }
    let conflict = progress.stop == RebaseStop::Conflict;
    operation_band::Band {
        tone: if conflict && busy.is_none() {
            Tone::Conflict
        } else {
            Tone::Ready
        },
        icon: match busy {
            Some(_) => Icon::Spinner,
            None if conflict => Icon::Warning,
            None => Icon::Pause,
        },
        title: match busy {
            Some(label) => format!("{label} · step {step} of {total}"),
            None => title(progress),
        },
        hash: String::new(),
        post: String::new(),
        status: if busy.is_some() {
            String::new()
        } else {
            reason.unwrap_or_default()
        },
        sub,
        buttons,
        steps,
    }
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
        let progress = reopened(2, 0);
        assert_eq!(continue_reason(&progress, 0), None);
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

    #[test]
    fn a_conflict_stop_offers_resolve_conflicts_until_the_workspace_opens() {
        use operation_band::{Command, Kind};
        let mut conflict = reopened(0, 0);
        conflict.stop = RebaseStop::Conflict;
        let band = band(&conflict, 1, false, false, None, None);
        assert_eq!(
            band.button(Command::ResolveConflicts)
                .map(|button| (button.label, button.kind)),
            Some(("Resolve conflicts", Kind::Primary))
        );
        assert_eq!(band.status, "Resolve and stage 1 conflicted file first.");
        assert!(
            band.button(Command::Continue)
                .is_some_and(|button| !button.enabled)
        );
        let open = super::band(&conflict, 1, true, false, None, None);
        assert!(open.button(Command::ResolveConflicts).is_none());
        // Busy: the job names itself, the buttons stay and disable.
        let busy = super::band(&conflict, 0, true, true, Some("Continuing rebase"), None);
        assert_eq!(busy.title, "Continuing rebase · step 2 of 3");
        assert!(busy.buttons.iter().all(|button| !button.enabled));
    }
}
