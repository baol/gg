//! Compact commit details with a fixed portrait and bounded text fields.
use crate::CommitCommand;
use crate::avatars::Avatars;
use eframe::egui;
use gg_engine::Commit;

/// Stable popup identity shared by the visible button and the Alt+A route.
pub fn commit_actions_popup_id() -> egui::Id {
    egui::Id::unique("selected-commit-actions")
}

/// What the narrow commit details asked for.
pub enum PageAction {
    /// `‹ History` (Esc): back to the list.
    Back,
    /// Newer (`-1`, k) or older (`1`, j) commit.
    Step(i32),
    /// Review the commit, opened on the chosen file when one is given.
    Review(Option<String>),
    Command(CommitCommand),
}

/// Everything the narrow commit details show besides the commit itself.
pub struct PageInfo<'a> {
    /// The full message has not arrived yet; its slot stays empty.
    pub loading: bool,
    pub refs: &'a [gg_engine::refs::RefItem],
    /// File and line counts with the first files; `None` while counting.
    pub stats: Option<&'a crate::commit_stats::Stats>,
}

/// Height of the narrow details' top bar and bottom action row.
pub const PAGE_BAR_HEIGHT: f32 = 34.0;

/// The narrow commit details (narrow proposal): Space replaces the history
/// list with this page. `‹ History Esc`, the subject, and newer/older on top;
/// a label grid, the message, and the FILES list (Enter or a double click
/// reviews the chosen file); **Actions** and the primary **Review** at the
/// bottom.
pub fn page(
    ui: &mut egui::Ui,
    commit: Option<&Commit>,
    info: &PageInfo,
    selected_file: &mut usize,
    actions: impl FnOnce(&mut egui::Ui) -> Option<CommitCommand>,
) -> Option<PageAction> {
    let mut action = None;
    let files = info.stats.map_or(&[][..], |stats| stats.rows.as_slice());
    *selected_file = (*selected_file).min(files.len().saturating_sub(1));
    egui::Panel::top("narrow-details-bar")
        .exact_size(PAGE_BAR_HEIGHT)
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                if ui
                    .button(("‹ History", egui::RichText::new("Esc").small().weak()))
                    .clicked()
                {
                    action = Some(PageAction::Back);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let older = ui
                        .add(egui::Button::new(crate::phosphor::icon(
                            crate::phosphor::CARET_DOWN,
                        )))
                        .on_hover_text("Older commit · j");
                    if older.clicked() {
                        action = Some(PageAction::Step(1));
                    }
                    let newer = ui
                        .add(egui::Button::new(crate::phosphor::icon(
                            crate::phosphor::CARET_UP,
                        )))
                        .on_hover_text("Newer commit · k");
                    if newer.clicked() {
                        action = Some(PageAction::Step(-1));
                    }
                    if let Some(commit) = commit {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(commit.subject()).strong().size(13.5),
                                )
                                .truncate(),
                            )
                            .on_hover_text(commit.subject());
                        });
                    }
                });
            });
        });
    egui::Panel::bottom("narrow-details-actions")
        .exact_size(PAGE_BAR_HEIGHT)
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Review is the primary action, filled and rightmost.
                    let review = ui.add_enabled(
                        commit.is_some(),
                        egui::Button::new((
                            egui::RichText::new("Review").color(egui::Color32::WHITE),
                            egui::RichText::new("Enter")
                                .small()
                                .color(egui::Color32::from_rgb(0xb8, 0xd4, 0xe6)),
                        ))
                        .fill(egui::Color32::from_rgb(0x0b, 0x55, 0x77)),
                    );
                    if review.clicked() {
                        action = Some(PageAction::Review(None));
                    }
                    let response = ui.add_enabled(
                        commit.is_some(),
                        egui::Button::new(("Actions", egui::RichText::new("Alt+A").weak().small())),
                    );
                    egui::Popup::menu(&response)
                        .id(commit_actions_popup_id())
                        .show(|ui| {
                            ui.set_min_width(250.0);
                            if let Some(command) = actions(ui) {
                                action = Some(PageAction::Command(command));
                                ui.close();
                            }
                        });
                });
            });
        });
    let Some(commit) = commit else {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.weak("Select a commit to inspect it.");
        });
        return action;
    };
    egui::CentralPanel::default().show(ui, |ui| {
        let page_height = ui.available_height();
        egui::ScrollArea::vertical()
            .id_salt("commit-details")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                crate::navigation::scroll(ui, page_height);
                ui.add_space(4.0);
                egui::Grid::new("commit-metadata")
                    .num_columns(2)
                    .min_col_width(74.0)
                    .spacing([12.0, 5.0])
                    .show(ui, |ui| {
                        ui.weak("Commit");
                        ui.horizontal(|ui| {
                            let short: String = commit.id.chars().take(8).collect();
                            ui.label(egui::RichText::new(short).monospace().size(11.5))
                                .on_hover_text(&commit.id);
                            if ui
                                .small_button(crate::phosphor::icon(crate::phosphor::COPY))
                                .on_hover_text("Copy ID · Ctrl+C")
                                .clicked()
                            {
                                ui.ctx().copy_text(commit.id.clone());
                            }
                        });
                        ui.end_row();
                        ui.weak("Author");
                        ui.add(
                            egui::Label::new(format!("{} <{}>", commit.author, commit.email))
                                .truncate(),
                        );
                        ui.end_row();
                        ui.weak("Date");
                        ui.label(&commit.time);
                        ui.end_row();
                        ui.weak(if commit.parents.len() == 1 {
                            "Parent"
                        } else {
                            "Parents"
                        });
                        let parents = if commit.parents.is_empty() {
                            "Root commit".to_owned()
                        } else {
                            commit
                                .parents
                                .iter()
                                .map(|parent| parent.chars().take(8).collect::<String>())
                                .collect::<Vec<_>>()
                                .join("  ")
                        };
                        ui.label(egui::RichText::new(parents).monospace().size(11.5));
                        ui.end_row();
                        if !info.refs.is_empty() {
                            ui.weak("Refs");
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                for item in info.refs {
                                    ui.label(
                                        egui::RichText::new(format!(" {} ", item.name))
                                            .size(11.0)
                                            .background_color(ui.visuals().faint_bg_color),
                                    )
                                    .on_hover_text(&item.full_name);
                                }
                            });
                            ui.end_row();
                        }
                        ui.weak("Signature");
                        if ui
                            .link("Verify…")
                            .on_hover_text("Verify this commit's signature")
                            .clicked()
                        {
                            action = Some(PageAction::Command(CommitCommand::OpenSignature));
                        }
                        ui.end_row();
                    });
                ui.add_space(10.0);
                // While the full message loads its slot stays empty.
                let body = if info.loading {
                    ""
                } else {
                    commit
                        .message
                        .split_once('\n')
                        .map_or("", |(_, body)| body.trim())
                };
                if !body.is_empty() {
                    ui.add(egui::Label::new(body).wrap());
                    ui.add_space(10.0);
                }
                ui.separator();
                // FILES with the line totals; rows keep their slots.
                ui.horizontal(|ui| {
                    let label = match info.stats {
                        Some(stats) if stats.complete => format!("FILES {}", stats.files),
                        Some(stats) => format!("FILES {}+", stats.files),
                        None => "FILES".into(),
                    };
                    ui.label(egui::RichText::new(label).small().weak());
                    if let Some(stats) = info.stats {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let red = crate::colors::diff(
                                ui.visuals(),
                                gg_engine::diff::RowKind::Deleted,
                            )
                            .1;
                            let green =
                                crate::colors::diff(ui.visuals(), gg_engine::diff::RowKind::Added)
                                    .1;
                            ui.label(
                                egui::RichText::new(format!("−{}", stats.deleted))
                                    .small()
                                    .color(red),
                            );
                            ui.label(
                                egui::RichText::new(format!("+{}", stats.added))
                                    .small()
                                    .color(green),
                            );
                        });
                    }
                });
                ui.spacing_mut().item_spacing.y = 0.0;
                for (index, file) in files.iter().enumerate() {
                    let selected = index == *selected_file;
                    let response = ui.add_sized(
                        [ui.available_width(), crate::density::row()],
                        egui::Button::new("")
                            .fill(if selected {
                                crate::colors::selection(ui.visuals())
                            } else {
                                egui::Color32::TRANSPARENT
                            })
                            .stroke(egui::Stroke::NONE)
                            .selected(selected),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(egui::Role::ListItem, true, selected, &file.path)
                    });
                    let rect = response.rect.shrink2(egui::vec2(6.0, 0.0));
                    let (label, color) = crate::colors::change(ui.visuals(), &file.kind);
                    crate::bounded_text::paint(
                        ui,
                        egui::Rect::from_min_size(rect.min, egui::vec2(14.0, rect.height())),
                        crate::colors::change_symbol(&file.kind),
                        egui::FontId::proportional(13.0),
                        color,
                    );
                    let counts = file
                        .lines
                        .map(|(added, deleted)| format!("+{added} −{deleted}"))
                        .unwrap_or_default();
                    let count_rect = egui::Rect::from_min_max(
                        egui::pos2(rect.right() - 90.0, rect.top()),
                        rect.max,
                    );
                    let galley = ui.painter().layout_no_wrap(
                        counts,
                        egui::FontId::monospace(10.5),
                        ui.visuals().weak_text_color(),
                    );
                    ui.painter().galley(
                        egui::pos2(
                            count_rect.right() - galley.size().x,
                            count_rect.center().y - galley.size().y / 2.0,
                        ),
                        galley,
                        ui.visuals().weak_text_color(),
                    );
                    crate::bounded_text::paint_path(
                        ui,
                        egui::Rect::from_min_max(
                            egui::pos2(rect.left() + 22.0, rect.top()),
                            egui::pos2(count_rect.left() - 6.0, rect.bottom()),
                        ),
                        &file.path,
                        egui::FontId::proportional(13.0),
                        ui.visuals().strong_text_color(),
                    );
                    let response = response.on_hover_text(format!("{} · {label}", file.path));
                    if response.double_clicked() {
                        *selected_file = index;
                        action = Some(PageAction::Review(Some(file.path.clone())));
                    } else if response.clicked() {
                        response.surrender_focus();
                        *selected_file = index;
                    }
                }
                if let Some(stats) = info.stats
                    && stats.files > files.len()
                {
                    ui.add_space(4.0);
                    ui.weak(format!(
                        "{} more files in Review",
                        stats.files - files.len()
                    ));
                }
            });
    });
    action
}

/// Height of the bottom commit inspector in History.
pub const INSPECTOR_HEIGHT: f32 = 58.0;

/// The bottom commit inspector from the History mockup: avatar, subject,
/// ID pill, **Review** (`Enter`) and **Actions** (`Alt+A`) on the first line;
/// author, date, and parent on the second. The rows keep their slots while a
/// commit loads, so nothing moves.
pub fn inspector(
    ui: &mut egui::Ui,
    commit: Option<&Commit>,
    stats: Option<&str>,
    avatars: &mut Avatars,
    details: Option<&mut bool>,
    actions: impl FnOnce(&mut egui::Ui) -> Option<CommitCommand>,
) -> Option<CommitCommand> {
    let mut command = None;
    let Some(commit) = commit else {
        ui.weak("Select a commit to inspect it.");
        return None;
    };
    ui.horizontal(|ui| {
        avatars.show(ui, &commit.email, &commit.author, 22.0);
        let buttons = if details.is_some() { 290.0 } else { 190.0 };
        let pill = 84.0;
        let subject_width = (ui.available_width() - buttons - pill).max(80.0);
        ui.allocate_ui_with_layout(
            egui::vec2(subject_width, 22.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_max_width(subject_width);
                ui.add(
                    egui::Label::new(egui::RichText::new(commit.subject()).strong().size(13.5))
                        .truncate(),
                )
                .on_hover_text(commit.message.trim());
            },
        );
        let short: String = commit.id.chars().take(8).collect();
        if ui
            .add(
                egui::Button::new(egui::RichText::new(short).monospace().size(11.0))
                    .fill(ui.visuals().faint_bg_color),
            )
            .on_hover_text(format!("{}\nClick to copy the full ID", commit.id))
            .clicked()
        {
            ui.ctx().copy_text(commit.id.clone());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let response = ui.add(egui::Button::new((
                "Actions",
                egui::RichText::new("Alt+A").weak().small(),
            )));
            egui::Popup::menu(&response)
                .id(commit_actions_popup_id())
                .show(|ui| {
                    ui.set_min_width(250.0);
                    command = actions(ui);
                    if command.is_some() {
                        ui.close();
                    }
                });
            if ui
                .add(egui::Button::new((
                    "Review",
                    egui::RichText::new("Enter").weak().small(),
                )))
                .clicked()
            {
                command = Some(CommitCommand::Menu(crate::FrameCommand::ReviewSelected));
            }
            // Narrow windows: Details (Space) replaces the list with the
            // full commit details (narrow proposal).
            if let Some(details) = details
                && ui
                    .add(egui::Button::new((
                        "Details",
                        egui::RichText::new("Space").weak().small(),
                    )))
                    .clicked()
            {
                *details = true;
            }
        });
    });
    let parents = match commit.parents.as_slice() {
        [] => "root commit".to_string(),
        [parent] => format!("parent {}", &parent[..parent.len().min(8)]),
        parents => format!(
            "parents {}",
            parents
                .iter()
                .map(|parent| &parent[..parent.len().min(8)])
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    // Second line: author, date, parents, then `· 5 files +86 −41` once
    // counted, with the line counts in the diff colors as in the mockup.
    let small = egui::TextStyle::Small.resolve(ui.style());
    let weak = ui.visuals().weak_text_color();
    let mut line = egui::text::LayoutJob::default();
    let mut push = |text: &str, color: egui::Color32| {
        line.append(
            text,
            0.0,
            egui::TextFormat {
                font_id: small.clone(),
                color,
                ..Default::default()
            },
        );
    };
    push(
        &format!("{} · {} · {parents}", commit.author, commit.time),
        weak,
    );
    if let Some(stats) = stats {
        let (files, counts) = stats.split_once(" +").unwrap_or((stats, ""));
        push(&format!(" · {files}"), weak);
        if let Some((added, deleted)) = counts.split_once(' ') {
            let green = crate::colors::diff(ui.visuals(), gg_engine::diff::RowKind::Added).1;
            let red = crate::colors::diff(ui.visuals(), gg_engine::diff::RowKind::Deleted).1;
            push(&format!(" +{added}"), green);
            push(&format!(" {deleted}"), red);
        }
    }
    ui.add(egui::Label::new(line).truncate());
    command
}
