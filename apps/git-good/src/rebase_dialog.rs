//! Interactive rebase dialog: the plan on the left, the selected commit on the
//! right in the shared review reader.
//!
//! A large modal over the current view. Keys act on the selected row unless a
//! text field has focus: Up/Down (j/k) select, Alt+Up/Down move, P R E S F D
//! choose the action, Enter opens the commit in Review, Esc cancels, and
//! Ctrl+Enter starts the rebase.

use crate::{
    phosphor,
    rebase_plan::{self, Plan},
    review_view::Review,
};
use eframe::egui;
use gg_engine::rewrite::{Action, RangeCommit, Step};
use std::path::PathBuf;

const ROW: f32 = 28.0;
/// Height reserved for the selected commit's message or reword field.
const MESSAGE_SLOT: f32 = 86.0;

pub enum Outcome {
    Start {
        from: String,
        steps: Vec<Step>,
        autostash: bool,
    },
    Cancel,
    OpenReview {
        commit: String,
        subject: String,
    },
}

pub struct Dialog {
    /// Oldest commit of the range.
    pub from: String,
    pub plan: Plan,
    pub selected: usize,
    /// Checked-out branch, or `detached HEAD`.
    pub branch: String,
    /// Commit the range is replayed onto; `None` for a root range.
    pub base: Option<String>,
    /// Other branches and tags in the range; they keep the original commits.
    pub stay: Vec<String>,
    pub autostash: bool,
    pub error: Option<String>,
    /// Hidden while the selected commit is open in Review.
    pub held: bool,
    repository: PathBuf,
    review: Option<(String, Review)>,
    drag: Option<usize>,
    /// Focus the message field on the next frame after choosing Reword.
    focus_message: bool,
}

impl Dialog {
    pub fn new(
        repository: PathBuf,
        commits: Vec<RangeCommit>,
        base: Option<String>,
        refs: &gg_engine::refs::Snapshot,
    ) -> Result<Self, String> {
        let from = commits
            .first()
            .ok_or("The rewrite range is empty.")?
            .id
            .clone();
        let current = refs
            .head
            .as_ref()
            .filter(|head| !head.detached)
            .and_then(|head| head.name.as_deref())
            .map(|name| name.strip_prefix("refs/heads/").unwrap_or(name).to_string());
        let local = |id: &str| {
            refs.items
                .iter()
                .filter(|item| {
                    item.commit == id
                        && matches!(
                            item.kind,
                            gg_engine::refs::RefKind::Branch | gg_engine::refs::RefKind::Tag
                        )
                })
                .map(|item| item.name.clone())
                .collect::<Vec<_>>()
        };
        let stay = commits
            .iter()
            .flat_map(|commit| local(&commit.id))
            .filter(|name| Some(name) != current.as_ref())
            .collect();
        Ok(Self {
            from,
            plan: Plan::new(commits, local),
            selected: 0,
            branch: current.unwrap_or_else(|| "detached HEAD".into()),
            base,
            stay,
            autostash: false,
            error: None,
            held: false,
            repository,
            review: None,
            drag: None,
            focus_message: false,
        })
    }

    /// Select `commit` and give it `action`, as Commit › Reword… and its
    /// siblings do.
    pub fn preset(&mut self, commit: &str, action: Action) {
        if let Some(index) = self
            .plan
            .rows
            .iter()
            .position(|row| row.commit.id == commit)
        {
            self.selected = index;
            self.set_action(action);
        }
    }

    fn range_label(&self) -> String {
        let onto = self.base.as_deref().map_or("the root".to_string(), |base| {
            rebase_plan::short(base).to_string()
        });
        format!("{}…HEAD onto {onto}", rebase_plan::short(&self.from))
    }

    fn set_action(&mut self, action: Action) {
        let index = self.selected;
        self.error = self
            .plan
            .set_action(index, action)
            .err()
            .map(str::to_string);
        self.focus_message = action == Action::Reword && self.error.is_none();
    }

    fn move_selected(&mut self, up: bool) {
        let index = self.selected;
        let to = if up {
            index.checked_sub(1)
        } else {
            (index + 1 < self.plan.rows.len()).then_some(index + 1)
        };
        if let Some(to) = to {
            self.plan.move_row(index, to);
            self.selected = to;
        }
    }

    fn open_review(&self) -> Outcome {
        let row = &self.plan.rows[self.selected];
        Outcome::OpenReview {
            commit: row.commit.id.clone(),
            subject: row.commit.subject.clone(),
        }
    }

    /// Start the plan as it stands, as Start rebase would; `None` when the
    /// plan cannot run.
    pub fn start_now(&self) -> Option<Outcome> {
        self.start()
    }

    fn start(&self) -> Option<Outcome> {
        self.plan.steps().ok().map(|steps| Outcome::Start {
            from: self.from.clone(),
            steps,
            autostash: self.autostash,
        })
    }

    /// Keyboard commands; letters and arrows wait while a text field has focus.
    fn keys(&mut self, ctx: &egui::Context, blocked: bool) -> Option<Outcome> {
        let start = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter));
        if start && !blocked {
            return self.start();
        }
        if ctx.text_edit_focused() || egui::Popup::is_any_open(ctx) {
            return None;
        }
        let pressed = |key, modifiers| ctx.input_mut(|i| i.consume_key(modifiers, key));
        let none = egui::Modifiers::NONE;
        if pressed(egui::Key::Escape, none) {
            return Some(Outcome::Cancel);
        }
        if pressed(egui::Key::Enter, none) {
            return Some(self.open_review());
        }
        if pressed(egui::Key::ArrowUp, egui::Modifiers::ALT) {
            self.move_selected(true);
        } else if pressed(egui::Key::ArrowDown, egui::Modifiers::ALT) {
            self.move_selected(false);
        } else if pressed(egui::Key::ArrowUp, none) || pressed(egui::Key::K, none) {
            self.selected = self.selected.saturating_sub(1);
        } else if pressed(egui::Key::ArrowDown, none) || pressed(egui::Key::J, none) {
            self.selected = (self.selected + 1).min(self.plan.rows.len() - 1);
        }
        for action in rebase_plan::ACTIONS {
            let key = match rebase_plan::key(action) {
                'P' => egui::Key::P,
                'R' => egui::Key::R,
                'E' => egui::Key::E,
                'S' => egui::Key::S,
                'F' => egui::Key::F,
                _ => egui::Key::D,
            };
            if pressed(key, none) {
                self.set_action(action);
            }
        }
        None
    }

    /// Draw the dialog. `blocked` names why the rebase cannot start now.
    pub fn show(&mut self, ctx: &egui::Context, blocked: Option<&str>) -> Option<Outcome> {
        if self.held || self.plan.rows.is_empty() {
            return None;
        }
        self.selected = self.selected.min(self.plan.rows.len() - 1);
        let mut outcome = self.keys(ctx, blocked.is_some());
        let screen = ctx.content_rect();
        let size = egui::vec2(
            (screen.width() - 48.0).clamp(560.0, 1400.0),
            (screen.height() - 64.0).clamp(420.0, 1000.0),
        );
        egui::Modal::new(egui::Id::unique("interactive-rebase")).show(ctx, |ui| {
            ui.set_min_size(size);
            ui.set_max_size(size);
            if let Some(chosen) = self.header(ui, blocked) {
                outcome = Some(chosen);
            }
            ui.separator();
            let footer = 70.0;
            let body = egui::Rect::from_min_size(
                ui.cursor().min,
                egui::vec2(size.x, (ui.available_height() - footer).max(160.0)),
            );
            let plan_width = (size.x * 0.48).clamp(320.0, 620.0);
            let left =
                egui::Rect::from_min_max(body.min, egui::pos2(body.min.x + plan_width, body.max.y));
            let right =
                egui::Rect::from_min_max(egui::pos2(left.max.x + 12.0, body.min.y), body.max);
            ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
                ui.set_clip_rect(left);
                self.draw_plan(ui);
            });
            ui.painter().vline(
                left.max.x + 6.0,
                body.y_range(),
                ui.visuals().widgets.noninteractive.bg_stroke,
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                ui.set_clip_rect(right);
                if let Some(chosen) = self.draw_selected(ui) {
                    outcome = Some(chosen);
                }
            });
            ui.allocate_rect(body, egui::Sense::hover());
            ui.separator();
            self.footer(ui);
        });
        outcome
    }

    fn header(&mut self, ui: &mut egui::Ui, blocked: Option<&str>) -> Option<Outcome> {
        let mut outcome = None;
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let steps = self.plan.steps();
                let reason = blocked
                    .map(str::to_string)
                    .or_else(|| steps.as_ref().err().cloned());
                let start = ui
                    .add_enabled(
                        reason.is_none(),
                        egui::Button::new(egui::RichText::new("Start rebase").strong()),
                    )
                    .on_hover_text("Ctrl+Enter");
                let start = match &reason {
                    Some(reason) => start.on_disabled_hover_text(reason),
                    None => start,
                };
                if start.clicked() {
                    outcome = self.start();
                }
                if ui.button("Cancel").on_hover_text("Esc").clicked() {
                    outcome = Some(Outcome::Cancel);
                }
                // The identity takes the remaining width and truncates. The
                // result count changes as the plan changes, so it comes last
                // and nothing after it can move.
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.heading("Interactive rebase");
                    ui.add(egui::Label::new(egui::RichText::new(&self.branch).strong()).truncate());
                    ui.add(
                        egui::Label::new(egui::RichText::new(self.range_label()).weak()).truncate(),
                    );
                    ui.add(egui::Label::new(self.plan.summary()).truncate());
                });
            });
        });
        outcome
    }

    fn draw_plan(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.weak("PLAN · NEWEST FIRST, LIKE HISTORY");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(format!(
                    "j k select · Alt+{}{} move · P R E S F D",
                    phosphor::ARROW_UP,
                    phosphor::ARROW_DOWN
                ));
            });
        });
        let count = self.plan.rows.len();
        let mut rects = Vec::with_capacity(count);
        let mut chosen = None;
        let mut moved = None;
        let base = self.base.clone();
        egui::ScrollArea::vertical()
            .id_salt("rebase-plan")
            .auto_shrink([false, false])
            .max_height((ui.available_height() - 22.0).max(ROW))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for index in 0..count {
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), ROW),
                        egui::Sense::click(),
                    );
                    rects.push(rect);
                    if index == self.selected {
                        ui.painter()
                            .rect_filled(rect, 0.0, crate::colors::selection(ui.visuals()));
                    }
                    if response.clicked() {
                        self.selected = index;
                    }
                    ui.scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(rect.shrink2(egui::vec2(4.0, 0.0)))
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                        |ui| {
                            if let Some((action, step)) = self.draw_row(ui, index) {
                                chosen = action.map(|action| (index, action));
                                moved = step.map(|up| (index, up));
                            }
                        },
                    );
                }
                // The commit the plan is replayed onto, as in the mockup.
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), ROW),
                    egui::Sense::hover(),
                );
                let base = base.as_deref().map_or("the root".to_string(), |base| {
                    format!("{} base · unchanged", rebase_plan::short(base))
                });
                ui.painter().text(
                    rect.left_center() + egui::vec2(64.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    base,
                    egui::FontId::proportional(12.0),
                    ui.visuals().weak_text_color(),
                );
            });
        ui.add(
            egui::Label::new(
                egui::RichText::new(
                    "Keys on the selected row: P pick · R reword · E reopen · S squash · F fixup · D drop",
                )
                .small()
                .weak(),
            )
            .truncate(),
        );
        if let Some((index, action)) = chosen {
            self.selected = index;
            self.set_action(action);
        }
        if let Some((index, up)) = moved {
            self.selected = index;
            self.move_selected(up);
        }
        // Dragging the handle moves the row to the row under the pointer.
        if let Some(from) = self.drag {
            let pointer = ui.ctx().pointer_latest_pos();
            let over = pointer.and_then(|p| {
                rects
                    .iter()
                    .position(|rect| rect.y_range().contains(p.y))
                    .or_else(|| {
                        let first = rects.first()?;
                        let last = rects.last()?;
                        if p.y < first.top() {
                            Some(0)
                        } else if p.y > last.bottom() {
                            Some(rects.len() - 1)
                        } else {
                            None
                        }
                    })
            });
            if let Some(to) = over
                && let Some(rect) = rects.get(to)
            {
                let y = if to > from { rect.bottom() } else { rect.top() };
                ui.painter().hline(
                    rect.x_range(),
                    y,
                    egui::Stroke::new(2.0, ui.visuals().selection.stroke.color),
                );
            }
            if ui.input(|i| i.pointer.any_released()) {
                if let Some(to) = over {
                    self.plan.move_row(from, to);
                    self.selected = to;
                }
                self.drag = None;
            }
        }
    }

    /// One plan row; returns a chosen action or a move (`true` = up).
    #[allow(clippy::type_complexity)]
    fn draw_row(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
    ) -> Option<(Option<Action>, Option<bool>)> {
        let count = self.plan.rows.len();
        let mut chosen = None;
        let mut moved = None;
        let handle = ui
            .add(
                egui::Label::new(phosphor::icon(phosphor::DOTS_SIX_VERTICAL).weak())
                    .sense(egui::Sense::drag()),
            )
            .on_hover_cursor(egui::CursorIcon::Grab)
            .on_hover_text("Drag to move");
        if handle.drag_started() {
            self.drag = Some(index);
            self.selected = index;
        }
        let small = |glyph| {
            egui::Button::new(phosphor::icon(glyph).size(11.0))
                .frame_when_inactive(false)
                .min_size(egui::vec2(18.0, 18.0))
        };
        if ui
            .add_enabled(index > 0, small(phosphor::ARROW_UP))
            .on_hover_text(format!("Move up · Alt+{}", phosphor::ARROW_UP))
            .clicked()
        {
            moved = Some(true);
        }
        if ui
            .add_enabled(index + 1 < count, small(phosphor::ARROW_DOWN))
            .on_hover_text(format!("Move down · Alt+{}", phosphor::ARROW_DOWN))
            .clicked()
        {
            moved = Some(false);
        }
        let row = &self.plan.rows[index];
        let action = row.action;
        ui.label(
            egui::RichText::new(rebase_plan::short(&row.commit.id))
                .monospace()
                .weak(),
        )
        .on_hover_text(if row.refs.is_empty() {
            row.commit.id.clone()
        } else {
            format!("{} · {}", row.commit.id, row.refs.join(", "))
        });
        let badge = self.plan.badge(index);
        let subject = row.commit.subject.clone();
        let refs = row.refs.clone();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let chip = ui
                .add(
                    egui::Button::new((
                        egui::RichText::new(rebase_plan::label(action))
                            .strong()
                            .color(action_color(ui.visuals(), action)),
                        phosphor::icon(phosphor::CARET_DOWN).size(10.0),
                    ))
                    .min_size(egui::vec2(82.0, 20.0)),
                )
                .on_hover_text(rebase_plan::hint(action));
            egui::Popup::from_toggle_button_response(&chip)
                .id(egui::Id::unique(("rebase-action", index)))
                .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                .show(|ui| {
                    ui.set_min_width(300.0);
                    for option in rebase_plan::ACTIONS {
                        let reason = self.plan.unavailable(index, option);
                        let button = egui::Button::new((
                            egui::RichText::new(rebase_plan::label(option))
                                .strong()
                                .color(action_color(ui.visuals(), option)),
                            egui::RichText::new(rebase_plan::hint(option))
                                .weak()
                                .small(),
                        ))
                        .right_text(rebase_plan::key(option).to_string())
                        .selected(option == action)
                        .min_size(egui::vec2(ui.available_width(), 22.0));
                        let response = ui.add_enabled(reason.is_none(), button);
                        let response = match reason {
                            Some(reason) => response.on_disabled_hover_text(reason),
                            None => response,
                        };
                        if response.clicked() {
                            chosen = Some(option);
                        }
                    }
                });
            if let Some(badge) = &badge {
                let color = match action {
                    Action::Drop => ui.visuals().error_fg_color,
                    Action::Edit => ui.visuals().warn_fg_color,
                    _ => ui.visuals().weak_text_color(),
                };
                ui.add(
                    egui::Label::new(egui::RichText::new(badge).small().color(color)).truncate(),
                );
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                // Pills only where they fit; the hash hover names the refs too.
                let pills = ui.available_width() > 220.0;
                for name in refs.iter().filter(|_| pills) {
                    ui.label(
                        egui::RichText::new(name)
                            .small()
                            .background_color(ui.visuals().faint_bg_color),
                    );
                }
                let mut text = egui::RichText::new(&subject);
                text = match action {
                    Action::Drop => text.strikethrough().weak(),
                    Action::Squash | Action::Fixup => text.weak(),
                    _ => text,
                };
                ui.add(egui::Label::new(text).truncate())
                    .on_hover_text(&subject);
            });
        });
        (chosen.is_some() || moved.is_some()).then_some((chosen, moved))
    }

    fn draw_selected(&mut self, ui: &mut egui::Ui) -> Option<Outcome> {
        let mut outcome = None;
        let index = self.selected;
        let ctx = ui.ctx().clone();
        let row = &mut self.plan.rows[index];
        let action = row.action;
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(rebase_plan::short(&row.commit.id))
                    .monospace()
                    .strong(),
            );
            ui.label(
                egui::RichText::new(rebase_plan::label(action))
                    .strong()
                    .color(action_color(ui.visuals(), action)),
            );
            ui.weak(&row.commit.author);
            ui.weak(&row.commit.date);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button((phosphor::icon(phosphor::ARROW_SQUARE_OUT), "Open in Review"))
                    .on_hover_text("Enter · Esc in Review returns here")
                    .clicked()
                {
                    outcome = Some(Outcome::OpenReview {
                        commit: row.commit.id.clone(),
                        subject: row.commit.subject.clone(),
                    });
                }
            });
        });
        // One fixed-height slot for the message, whatever the action, so the
        // reader below never moves when the action or selection changes.
        let slot = egui::Rect::from_min_size(
            ui.cursor().min,
            egui::vec2(ui.available_width(), MESSAGE_SLOT),
        );
        let focus_message = &mut self.focus_message;
        ui.scope_builder(egui::UiBuilder::new().max_rect(slot), |ui| {
            ui.set_clip_rect(slot.intersect(ui.clip_rect()));
            if action == Action::Reword {
                ui.weak("New message (reword)");
                let field = ui.add(
                    egui::TextEdit::multiline(&mut row.message)
                        .id(egui::Id::unique("rebase-reword"))
                        .desired_rows(2)
                        .desired_width(f32::INFINITY)
                        .hint_text("New commit message"),
                );
                if std::mem::take(focus_message) {
                    field.request_focus();
                }
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(format!("Was: {}", row.commit.subject))
                            .small()
                            .weak(),
                    )
                    .truncate(),
                );
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("rebase-message")
                    .max_height(MESSAGE_SLOT)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(egui::Label::new(&row.commit.message).wrap());
                    });
            }
        });
        ui.allocate_rect(slot, egui::Sense::hover());
        ui.add_space(4.0);
        let id = row.commit.id.clone();
        let subject = row.commit.subject.clone();
        if self.review.as_ref().is_none_or(|(shown, _)| *shown != id) {
            let review = Review::new(
                gg_engine::review::ReviewRequest {
                    path: self.repository.clone(),
                    revision: id.clone(),
                    parent: None,
                    against: None,
                },
                subject,
                &ctx,
            )
            .embedded();
            self.review = Some((id, review));
        }
        let rect = ui.available_rect_before_wrap();
        if let Some((_, review)) = &mut self.review {
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.set_clip_rect(rect.intersect(ui.clip_rect()));
                review.draw_ui(ui);
            });
        }
        if action == Action::Drop {
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 0.0, ui.visuals().panel_fill.gamma_multiply(0.7));
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Dropped · these changes leave the branch",
                egui::FontId::proportional(14.0),
                ui.visuals().error_fg_color,
            );
        }
        ui.allocate_rect(rect, egui::Sense::hover());
        outcome
    }

    fn footer(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.autostash, "Stash working changes first");
            if !self.stay.is_empty() {
                ui.separator();
                ui.weak(format!(
                    "Stay on the original commits: {}",
                    self.stay.join(", ")
                ));
            }
        });
        ui.colored_label(
            ui.visuals().warn_fg_color,
            "Rewrites commits. A private backup ref is created before Git starts; Recovery can restore it. Merge commits are not supported.",
        );
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }
}

fn action_color(visuals: &egui::Visuals, action: Action) -> egui::Color32 {
    match action {
        Action::Pick => visuals.text_color(),
        Action::Reword => visuals.hyperlink_color,
        Action::Edit => visuals.warn_fg_color,
        Action::Squash | Action::Fixup => visuals.selection.stroke.color,
        Action::Drop => visuals.error_fg_color,
    }
}
