//! The 22 px footer: one message at a time, chosen by priority (a failure,
//! then something blocked, then a running job, then a receipt, then the key
//! hints). Failures and blocks are tinted and carry one recovery action,
//! **Details…**, and ×; a running network job shows its progress bar and
//! **Cancel**; receipts carry a check and fade back to the hints after 8 s.

use crate::RecoveryAction;
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Key hints, or a quiet note such as the rewrite backup.
    Idle,
    /// A job runs: spinner.
    Progress,
    /// A receipt: check.
    Success,
    /// Amber: blocked, the user's call.
    Blocked,
    /// Red: failed.
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Cancel,
    Details,
    Recovery(RecoveryAction),
    Dismiss,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Button {
    pub label: String,
    pub act: Act,
    pub primary: bool,
    pub enabled: bool,
    pub tip: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub tone: Tone,
    pub text: String,
    /// Grey text after the message: ` · upstream origin/main`.
    pub tail: String,
    /// The progress bar's fill, 0–1.
    pub bar: Option<f32>,
    pub buttons: Vec<Button>,
    pub dismiss: bool,
    /// Grey note at the right edge.
    pub right: String,
    /// Changed files, shown before the idle hints.
    pub changes: Option<usize>,
}

impl Message {
    pub fn new(tone: Tone, text: impl Into<String>) -> Self {
        Self {
            tone,
            text: text.into(),
            tail: String::new(),
            bar: None,
            buttons: Vec::new(),
            dismiss: false,
            right: String::new(),
            changes: None,
        }
    }

    /// A receipt or failure, split at its first ` · ` into text and tail.
    pub fn split(tone: Tone, summary: &str) -> Self {
        let mut message = Self::new(tone, summary);
        if let Some((text, tail)) = summary.split_once(" · ") {
            message.text = text.into();
            message.tail = format!(" · {tail}");
        }
        message
    }

    #[cfg(test)]
    pub fn full_text(&self) -> String {
        format!("{}{}", self.text, self.tail)
    }

    #[cfg(test)]
    pub fn button(&self, act: Act) -> Option<&Button> {
        self.buttons.iter().find(|button| button.act == act)
    }
}

pub fn button(label: impl Into<String>, act: Act, primary: bool) -> Button {
    Button {
        label: label.into(),
        act,
        primary,
        enabled: true,
        tip: String::new(),
    }
}

/// `Receiving objects 42% · 812/1930` and the bar's fill from Git's progress.
pub fn progress_tail(progress: &gg_engine::sync::Progress) -> (String, Option<f32>) {
    let mut tail = format!(
        " · {}{}",
        progress.phase,
        progress
            .percent
            .map(|percent| format!(" {percent}%"))
            .unwrap_or_default()
    );
    let message = progress.message.trim();
    if !message.is_empty() {
        tail.push_str(&format!(" · {message}"));
    }
    (
        tail,
        progress
            .percent
            .map(|percent| f32::from(percent.min(100)) / 100.0),
    )
}

struct Colors {
    text: egui::Color32,
    fill: Option<egui::Color32>,
}

fn colors(tone: Tone, visuals: &egui::Visuals) -> Colors {
    use egui::Color32 as C;
    match tone {
        Tone::Idle => Colors {
            text: visuals.weak_text_color(),
            fill: None,
        },
        Tone::Progress | Tone::Success => Colors {
            text: C::from_gray(207),
            fill: None,
        },
        Tone::Blocked => Colors {
            text: C::from_rgb(240, 195, 90),
            fill: Some(C::from_rgb(42, 36, 20)),
        },
        Tone::Failed => Colors {
            text: C::from_rgb(245, 160, 160),
            fill: Some(C::from_rgb(42, 30, 30)),
        },
    }
}

/// Draw `message` in the footer; returns the button clicked.
pub fn show(ui: &mut egui::Ui, message: &Message) -> Option<Act> {
    let mut clicked = None;
    let Colors { text, fill } = colors(message.tone, ui.visuals());
    let rect = ui.max_rect().expand2(egui::vec2(8.0, 2.0));
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, 0.0, fill);
    }
    let font = egui::FontId::proportional(11.0);
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        match message.tone {
            Tone::Progress => {
                ui.add(egui::Spinner::new().size(10.0));
            }
            Tone::Success => {
                ui.label(
                    crate::phosphor::icon(crate::phosphor::CHECK)
                        .size(12.0)
                        .color(egui::Color32::from_rgb(120, 200, 150)),
                );
            }
            Tone::Blocked | Tone::Failed => {
                ui.label(
                    crate::phosphor::icon(crate::phosphor::WARNING)
                        .size(12.0)
                        .color(text),
                );
            }
            Tone::Idle => {}
        }
        if let Some(count) = message.changes.filter(|count| *count > 0) {
            let green = crate::colors::diff(ui.visuals(), gg_engine::diff::RowKind::Added).1;
            let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 18.0), egui::Sense::hover());
            ui.painter().circle_filled(dot.center(), 3.5, green);
            ui.label(
                egui::RichText::new(format!(
                    "{count} change{}",
                    if count == 1 { "" } else { "s" }
                ))
                .size(11.0)
                .color(green),
            );
        }
        // Buttons, the bar, and the right note keep their room; the message
        // truncates.
        let measure = |ui: &egui::Ui, text: &str| {
            ui.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap(text.into(), font.clone(), egui::Color32::WHITE)
                    .size()
                    .x
            })
        };
        let reserved = message
            .buttons
            .iter()
            .map(|button| measure(ui, &button.label) + 24.0)
            .sum::<f32>()
            + if message.dismiss { 26.0 } else { 0.0 }
            + if message.bar.is_some() { 98.0 } else { 0.0 }
            + if message.right.is_empty() {
                0.0
            } else {
                measure(ui, &message.right) + 8.0
            };
        let width = (ui.available_width() - reserved).max(60.0);
        let mut job = egui::text::LayoutJob::default();
        job.append(
            &message.text,
            0.0,
            egui::TextFormat {
                font_id: font.clone(),
                color: text,
                ..Default::default()
            },
        );
        job.append(
            &message.tail,
            0.0,
            egui::TextFormat {
                font_id: font.clone(),
                color: egui::Color32::from_gray(140),
                ..Default::default()
            },
        );
        job.wrap = egui::text::TextWrapping::truncate_at_width(width);
        let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
        let full = galley.size().x;
        let (slot, _) =
            ui.allocate_exact_size(egui::vec2(full.min(width), 18.0), egui::Sense::hover());
        ui.painter().galley(
            egui::pos2(slot.left(), slot.center().y - galley.size().y / 2.0),
            galley,
            text,
        );
        if let Some(fill) = message.bar {
            let (bar, _) = ui.allocate_exact_size(egui::vec2(90.0, 4.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(bar, 2.0, egui::Color32::from_gray(51));
            let mut done = bar;
            done.set_width(bar.width() * fill.clamp(0.0, 1.0));
            ui.painter()
                .rect_filled(done, 2.0, egui::Color32::from_rgb(47, 143, 196));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if !message.right.is_empty() {
                ui.label(
                    egui::RichText::new(&message.right)
                        .size(11.0)
                        .color(egui::Color32::from_gray(140)),
                );
            }
            if message.dismiss
                && ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("×")
                                .size(13.0)
                                .color(egui::Color32::from_gray(176)),
                        )
                        .frame(false)
                        .min_size(egui::vec2(18.0, 17.0)),
                    )
                    .on_hover_text("Dismiss")
                    .clicked()
            {
                clicked = Some(Act::Dismiss);
            }
            for button in message.buttons.iter().rev() {
                let widget =
                    egui::Button::new(egui::RichText::new(&button.label).size(11.0).color(
                        if button.primary {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_gray(230)
                        },
                    ))
                    .fill(if button.primary {
                        egui::Color32::from_rgb(11, 85, 119)
                    } else {
                        egui::Color32::from_gray(58)
                    })
                    .corner_radius(3.0)
                    .min_size(egui::vec2(0.0, 17.0));
                let response = ui.add_enabled(button.enabled, widget);
                let response = if button.tip.is_empty() {
                    response
                } else {
                    response.on_disabled_hover_text(&button.tip)
                };
                if response.clicked() {
                    clicked = Some(button.act);
                }
            }
        });
    });
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipts_split_into_text_and_tail() {
        let message = Message::split(
            Tone::Success,
            "Pushed feature/rebase-ui to origin · upstream origin/feature/rebase-ui",
        );
        assert_eq!(message.text, "Pushed feature/rebase-ui to origin");
        assert_eq!(message.tail, " · upstream origin/feature/rebase-ui");
    }

    #[test]
    fn network_progress_reads_phase_percent_and_counts() {
        let (tail, bar) = progress_tail(&gg_engine::sync::Progress {
            operation: gg_engine::sync::Operation::Fetch,
            phase: "Receiving objects".into(),
            percent: Some(42),
            message: "812/1930".into(),
        });
        assert_eq!(tail, " · Receiving objects 42% · 812/1930");
        assert_eq!(bar, Some(0.42));
    }
}
