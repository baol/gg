//! The welcome view, shown while no repository is open: open, clone, or
//! initialize; reopen a recent repository; explain a launch folder that is
//! not a repository.
use crate::settings::Recent;
use eframe::egui;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::mpsc::Receiver,
};

/// More recent repositories than this make typing filter the list.
pub const FILTER_FROM: usize = 6;
/// The centered content column.
pub const CONTENT_WIDTH: f32 = 640.0;
/// Space above the column.
pub const TOP: f32 = 44.0;
pub const ROW_HEIGHT: f32 = 22.0;
pub const NAME_WIDTH: f32 = 132.0;
pub const BRANCH_WIDTH: f32 = 128.0;
pub const OPENED_WIDTH: f32 = 76.0;
pub const REMOVE_WIDTH: f32 = 22.0;
/// Below this window width the logo and version line hide.
pub const COMPACT_WINDOW: f32 = 760.0;
/// Below this column width the branch column drops.
const BRANCH_COLUMN_FROM: f32 = 600.0;
pub const PRIMARY: egui::Color32 = egui::Color32::from_rgb(11, 85, 119);
pub const DROP_BORDER: egui::Color32 = egui::Color32::from_rgb(47, 143, 196);

/// Why a recent repository did not open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Missing,
    NotRepository,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Open,
    Clone,
    /// New repository…, on this folder when given.
    NewRepository(Option<PathBuf>),
    OpenRecent(PathBuf),
    /// Remove from recent; the folder is untouched.
    Remove(PathBuf),
    /// Pick the moved folder; it replaces the entry.
    Locate(PathBuf),
    /// Preferences › General.
    Preferences,
}

/// What the background read learned about one recent folder.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Probe {
    pub exists: bool,
    pub branch: Option<String>,
}

#[derive(Default)]
pub struct Welcome {
    /// Index into the visible (filtered) rows.
    pub selected: usize,
    pub filter: String,
    /// The recent row being opened; the view stays until it opens.
    pub opening: Option<PathBuf>,
    /// The recent row that failed to open, and why.
    pub failed: Option<(PathBuf, Failure)>,
    /// The launch (or Open…) folder that is not a repository, with Git's
    /// message.
    pub launch: Option<(PathBuf, String)>,
    /// The requested folder whose failed read was already classified.
    pub settled: Option<PathBuf>,
    probes: HashMap<PathBuf, Probe>,
    probing: Vec<PathBuf>,
    probe_rx: Option<Receiver<(PathBuf, Probe)>>,
    git_version: Option<String>,
    git_rx: Option<Receiver<Option<String>>>,
    /// A dropped folder being checked: is it a repository?
    pub drop_rx: Option<Receiver<(PathBuf, bool)>>,
}

/// Everything the view shows besides its own state.
pub struct Info<'a> {
    pub recent: &'a [Recent],
    pub clone_folder: &'a Path,
    pub home: Option<&'a str>,
    pub now: u64,
    pub window_width: f32,
}

impl Welcome {
    /// Start the background reads the view needs once: Git's version and,
    /// whenever the list changes, each recent folder's branch.
    pub fn read(&mut self, ctx: &egui::Context, recent: &[Recent]) {
        if self.git_version.is_none() && self.git_rx.is_none() {
            let (tx, rx) = std::sync::mpsc::channel();
            let repaint = ctx.clone();
            std::thread::spawn(move || {
                let _ = tx.send(crate::git_version());
                repaint.request_repaint();
            });
            self.git_rx = Some(rx);
        }
        if let Some(rx) = &self.git_rx
            && let Ok(version) = rx.try_recv()
        {
            self.git_version = Some(
                version
                    .map(|version| version.trim_start_matches("git version ").to_string())
                    .unwrap_or_default(),
            );
            self.git_rx = None;
        }
        let paths = recent
            .iter()
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>();
        if paths != self.probing {
            self.probing = paths.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            let repaint = ctx.clone();
            std::thread::spawn(move || {
                for path in paths {
                    let probe = Probe {
                        exists: path.is_dir(),
                        branch: current_branch(&path),
                    };
                    if tx.send((path, probe)).is_err() {
                        return;
                    }
                }
                repaint.request_repaint();
            });
            self.probe_rx = Some(rx);
        }
        if let Some(rx) = &self.probe_rx {
            while let Ok((path, probe)) = rx.try_recv() {
                self.probes.insert(path, probe);
            }
        }
    }

    /// Re-read every folder, for example after a failed open.
    pub fn reread(&mut self) {
        self.probing.clear();
    }

    /// The `Git <version>` part of the version line, once read.
    pub fn git_version(&self) -> Option<&str> {
        self.git_version
            .as_deref()
            .filter(|version| !version.is_empty())
    }

    /// The list's keys, taken before any other view: ↑ ↓ choose, Enter
    /// opens, Del removes, and with more than [`FILTER_FROM`] repositories
    /// typing filters (Esc clears). `available` is false while a text field,
    /// popup, or dialog has the keyboard.
    pub fn keys(
        &mut self,
        ctx: &egui::Context,
        recent: &[Recent],
        available: bool,
    ) -> Option<Command> {
        if !available || recent.is_empty() {
            return None;
        }
        let filterable = recent.len() > FILTER_FROM;
        if filterable {
            let typed = ctx.input_mut(|input| {
                let mut typed = String::new();
                input.events.retain(|event| match event {
                    egui::Event::Text(text) if !text.contains('?') => {
                        typed.push_str(text);
                        false
                    }
                    _ => true,
                });
                typed
            });
            if !typed.is_empty() {
                self.filter.push_str(&typed);
                self.selected = 0;
            }
            if !self.filter.is_empty()
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace))
            {
                self.filter.pop();
                self.selected = 0;
            }
            if !self.filter.is_empty()
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            {
                self.filter.clear();
                self.selected = 0;
            }
        }
        let rows = visible(recent, &self.filter);
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
            self.selected = (self.selected + 1).min(rows.len().saturating_sub(1));
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
            self.selected = self.selected.saturating_sub(1);
        }
        self.selected = self.selected.min(rows.len().saturating_sub(1));
        let chosen = rows
            .get(self.selected)
            .map(|&index| recent[index].path.clone());
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
            return chosen.map(Command::OpenRecent);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)) {
            return chosen.map(Command::Remove);
        }
        None
    }

    /// The welcome column. Returns the command chosen this frame.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        info: &Info,
        logo: &mut Option<egui::TextureHandle>,
    ) -> Option<Command> {
        let mut command = None;
        let compact = info.window_width < COMPACT_WINDOW;
        let first_run = info.recent.is_empty();
        let available = ui.available_rect_before_wrap();
        let width = (available.width() - 32.0).clamp(0.0, CONTENT_WIDTH);
        let column = egui::Rect::from_min_size(
            egui::pos2(
                available.center().x - width / 2.0,
                available.top() + if compact { 16.0 } else { TOP },
            ),
            egui::vec2(width, (available.height() - TOP).max(0.0)),
        );
        let mut ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(column)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let ui = &mut ui;
        ui.spacing_mut().item_spacing.y = 0.0;
        // Title, with the logo and the version line on wide windows.
        ui.horizontal(|ui| {
            if !compact {
                crate::branding::logo(ui, logo, 36.0);
                ui.add_space(10.0);
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let title = if first_run {
                    "Welcome to Git Good"
                } else {
                    "Open a repository"
                };
                ui.label(egui::RichText::new(title).size(20.0).strong());
                if first_run {
                    ui.weak("Open a repository to see its history and changes.");
                } else if !compact {
                    // The Git part fills in when read; the line is always there.
                    let mut line = format!("Git Good {}", env!("CARGO_PKG_VERSION"));
                    if let Some(git) = self.git_version() {
                        line.push_str(&format!(" · Git {git}"));
                    }
                    ui.weak(line);
                }
            });
        });
        ui.add_space(14.0);
        if let Some((path, error)) = &self.launch {
            let shown = tilde(path, info.home);
            let text = format!("{shown} is not a Git repository.");
            let hover = format!("{}\n{error}", path.display());
            if warning_line(ui, &text, &hover, &[("Initialize here…", 0)]) == Some(0) {
                command = Some(Command::NewRepository(Some(path.clone())));
            }
            ui.add_space(10.0);
        }
        // Three equal action cards.
        let clone_into = tilde(info.clone_folder, info.home);
        let cards: [(&str, String, Command); 3] = if compact || width < 560.0 {
            [
                ("Open…", "Ctrl+O".into(), Command::Open),
                ("Clone…", format!("into {clone_into}"), Command::Clone),
                (
                    "New repository…",
                    "git init".into(),
                    Command::NewRepository(None),
                ),
            ]
        } else {
            [
                (
                    "Open…",
                    "A folder on this computer · Ctrl+O".into(),
                    Command::Open,
                ),
                (
                    "Clone…",
                    format!("From a URL into {clone_into}"),
                    Command::Clone,
                ),
                (
                    "New repository…",
                    "git init in a folder".into(),
                    Command::NewRepository(None),
                ),
            ]
        };
        let gap = 8.0;
        let card_width = ((width - 2.0 * gap) / 3.0).max(0.0);
        let (row, _) = ui.allocate_exact_size(egui::vec2(width, 52.0), egui::Sense::hover());
        for (index, (title, subtitle, chosen)) in cards.into_iter().enumerate() {
            let rect = egui::Rect::from_min_size(
                row.min + egui::vec2(index as f32 * (card_width + gap), 0.0),
                egui::vec2(card_width, row.height()),
            );
            if card(ui, rect, title, &subtitle, index == 0) {
                command = Some(chosen);
            }
        }
        ui.add_space(18.0);
        if first_run {
            ui.weak(
                "Repositories you open are listed here. Drop a folder on this window to open it.",
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.weak("Clone folder:");
                ui.label(egui::RichText::new(&clone_into).monospace().size(11.5));
                ui.weak("·");
                if ui.link("Change in Preferences…").clicked() {
                    command = Some(Command::Preferences);
                }
            });
            return command;
        }
        // RECENT and its count.
        let rows = visible(info.recent, &self.filter);
        self.selected = self.selected.min(rows.len().saturating_sub(1));
        let (header, _) = ui.allocate_exact_size(egui::vec2(width, 20.0), egui::Sense::hover());
        let weak = ui.visuals().weak_text_color();
        let small = egui::FontId::proportional(11.0);
        let mut label = "RECENT".to_string();
        if !self.filter.is_empty() {
            label.push_str(&format!("   “{}”  ·  Esc: clear", self.filter));
        }
        ui.painter().text(
            header.left_center() + egui::vec2(6.0, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            small.clone(),
            weak,
        );
        ui.painter().text(
            header.right_center() - egui::vec2(6.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            count_label(rows.len(), info.recent.len(), &self.filter),
            small,
            weak,
        );
        let branch_column = !compact && width >= BRANCH_COLUMN_FROM;
        // The list keeps its height while filtering.
        let (list, _) = ui.allocate_exact_size(
            egui::vec2(width, info.recent.len() as f32 * ROW_HEIGHT),
            egui::Sense::hover(),
        );
        for (slot, &index) in rows.iter().enumerate() {
            let entry = &info.recent[index];
            let rect = egui::Rect::from_min_size(
                list.min + egui::vec2(0.0, slot as f32 * ROW_HEIGHT),
                egui::vec2(width, ROW_HEIGHT),
            );
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .id_salt(("recent", index)),
            );
            let failed = self
                .failed
                .as_ref()
                .filter(|(path, _)| *path == entry.path)
                .map(|(_, failure)| *failure);
            if let Some(failure) = failed {
                let shown = tilde(&entry.path, info.home);
                let (text, buttons): (String, &[(&str, usize)]) = match failure {
                    Failure::Missing => (
                        format!("{shown} no longer exists."),
                        &[("Remove from recent", 0), ("Locate…", 1)],
                    ),
                    Failure::NotRepository => (
                        format!("{shown} is no longer a Git repository."),
                        &[("Remove from recent", 0)],
                    ),
                };
                match warning_line(
                    &mut child,
                    &text,
                    &entry.path.display().to_string(),
                    buttons,
                ) {
                    Some(0) => command = Some(Command::Remove(entry.path.clone())),
                    Some(_) => command = Some(Command::Locate(entry.path.clone())),
                    None => {}
                }
                continue;
            }
            let probe = self.probes.get(&entry.path);
            let row = RowView {
                entry,
                selected: slot == self.selected,
                missing: probe.is_some_and(|probe| !probe.exists),
                branch: probe.and_then(|probe| probe.branch.as_deref()),
                branch_column,
                opened: entry
                    .opened
                    .map(|opened| opened_label(opened, info.now))
                    .unwrap_or_default(),
                home: info.home,
            };
            match row.show(&mut child, rect) {
                Some(RowAction::Open) => {
                    self.selected = slot;
                    command = Some(Command::OpenRecent(entry.path.clone()));
                }
                Some(RowAction::Remove) => command = Some(Command::Remove(entry.path.clone())),
                None => {}
            }
        }
        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            ui.weak("Drop a folder anywhere on this window to open it.");
        });
        command
    }
}

enum RowAction {
    Open,
    Remove,
}

struct RowView<'a> {
    entry: &'a Recent,
    selected: bool,
    missing: bool,
    branch: Option<&'a str>,
    branch_column: bool,
    opened: String,
    home: Option<&'a str>,
}

impl RowView<'_> {
    fn show(&self, ui: &mut egui::Ui, rect: egui::Rect) -> Option<RowAction> {
        let response = ui.interact(rect, ui.scope_id().with("row"), egui::Sense::click());
        let hovered = response.hovered();
        let visuals = ui.visuals().clone();
        if self.selected {
            ui.painter()
                .rect_filled(rect, 3.0, crate::colors::selection(&visuals));
        } else if hovered {
            ui.painter()
                .rect_filled(rect, 3.0, visuals.widgets.hovered.weak_bg_fill);
        }
        let text = visuals.text_color();
        let weak = visuals.weak_text_color();
        let name = self
            .entry
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.entry.path.display().to_string());
        let left = rect.left() + 6.0;
        let name_rect = egui::Rect::from_min_size(
            egui::pos2(left, rect.top()),
            egui::vec2(NAME_WIDTH - 8.0, rect.height()),
        );
        paint_elided(
            ui,
            name_rect,
            &name,
            egui::FontId::proportional(13.0),
            if self.missing {
                weak.gamma_multiply(0.7)
            } else {
                text
            },
        );
        let right = rect.right() - REMOVE_WIDTH;
        let opened_rect = egui::Rect::from_min_max(
            egui::pos2(right - OPENED_WIDTH, rect.top()),
            egui::pos2(right - 6.0, rect.bottom()),
        );
        ui.painter().text(
            opened_rect.right_center(),
            egui::Align2::RIGHT_CENTER,
            &self.opened,
            egui::FontId::proportional(11.5),
            weak,
        );
        let branch_left = if self.branch_column {
            opened_rect.left() - BRANCH_WIDTH
        } else {
            opened_rect.left() - if self.missing { 64.0 } else { 0.0 }
        };
        let branch_rect = egui::Rect::from_min_max(
            egui::pos2(branch_left, rect.top()),
            egui::pos2(opened_rect.left() - 6.0, rect.bottom()),
        );
        if self.missing {
            missing_chip(ui, branch_rect);
        } else if self.branch_column
            && let Some(branch) = self.branch
        {
            paint_elided(ui, branch_rect, branch, egui::FontId::monospace(11.0), weak);
        }
        let path_rect = egui::Rect::from_min_max(
            egui::pos2(left + NAME_WIDTH, rect.top()),
            egui::pos2(branch_rect.left() - 8.0, rect.bottom()),
        );
        let shown = tilde(&self.entry.path, self.home);
        paint_elided(ui, path_rect, &shown, egui::FontId::monospace(11.0), weak);
        let mut action = None;
        if hovered || self.selected {
            let remove = egui::Rect::from_min_size(
                egui::pos2(right, rect.top()),
                egui::vec2(REMOVE_WIDTH, rect.height()),
            );
            let response = ui
                .interact(remove, ui.scope_id().with("remove"), egui::Sense::click())
                .on_hover_text("Remove from recent; the folder is untouched");
            let color = if response.hovered() { text } else { weak };
            ui.painter().text(
                remove.center(),
                egui::Align2::CENTER_CENTER,
                "×",
                egui::FontId::proportional(14.0),
                color,
            );
            if response.clicked() {
                action = Some(RowAction::Remove);
            }
        }
        if action.is_none()
            && response
                .on_hover_text(self.entry.path.display().to_string())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
        {
            action = Some(RowAction::Open);
        }
        action
    }
}

/// Amber warning colors: background, border, text.
pub fn amber(visuals: &egui::Visuals) -> (egui::Color32, egui::Color32, egui::Color32) {
    if visuals.dark_mode {
        (
            egui::Color32::from_rgb(0x3a, 0x2f, 0x14),
            egui::Color32::from_rgb(0x5a, 0x48, 0x18),
            egui::Color32::from_rgb(0xf0, 0xc3, 0x5a),
        )
    } else {
        (
            egui::Color32::from_rgb(0xfc, 0xf3, 0xd9),
            egui::Color32::from_rgb(0xe3, 0xcc, 0x86),
            egui::Color32::from_rgb(0x7a, 0x58, 0x00),
        )
    }
}

fn missing_chip(ui: &egui::Ui, slot: egui::Rect) {
    let (fill, border, ink) = amber(ui.visuals());
    let font = egui::FontId::proportional(10.5);
    let galley = ui.painter().layout_no_wrap("missing".into(), font, ink);
    let size = galley.size() + egui::vec2(10.0, 2.0);
    let chip = egui::Rect::from_min_size(
        egui::pos2(slot.left(), slot.center().y - size.y / 2.0),
        size,
    );
    ui.painter().rect_filled(chip, 3.0, fill);
    ui.painter()
        .rect_stroke(chip, 3.0, (1.0, border), egui::StrokeKind::Inside);
    ui.painter()
        .galley(chip.center() - galley.size() / 2.0, galley, ink);
}

/// One amber line with a warning glyph, truncating text (full text on
/// hover), and buttons on the right. Returns the clicked button's number.
pub fn warning_line(
    ui: &mut egui::Ui,
    text: &str,
    hover: &str,
    buttons: &[(&str, usize)],
) -> Option<usize> {
    let width = ui.available_width();
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, ROW_HEIGHT), egui::Sense::hover());
    let (fill, border, ink) = amber(ui.visuals());
    ui.painter().rect_filled(rect, 3.0, fill);
    ui.painter()
        .rect_stroke(rect, 3.0, (1.0, border), egui::StrokeKind::Inside);
    ui.painter().text(
        rect.left_center() + egui::vec2(8.0, 0.0),
        egui::Align2::LEFT_CENTER,
        crate::phosphor::WARNING,
        egui::FontId::new(13.0, egui::FontFamily::Name("phosphor-regular".into())),
        ink,
    );
    let mut clicked = None;
    let mut right = rect.right() - 4.0;
    for (label, number) in buttons.iter().rev() {
        let galley =
            ui.painter()
                .layout_no_wrap((*label).into(), egui::FontId::proportional(12.0), ink);
        let button = egui::Rect::from_min_max(
            egui::pos2(right - galley.size().x - 12.0, rect.top() + 2.0),
            egui::pos2(right, rect.bottom() - 2.0),
        );
        let response = ui.interact(
            button,
            ui.scope_id().with(("warning", *number)),
            egui::Sense::click(),
        );
        if response.hovered() {
            ui.painter().rect_filled(button, 3.0, border);
        }
        ui.painter()
            .rect_stroke(button, 3.0, (1.0, border), egui::StrokeKind::Inside);
        ui.painter()
            .galley(button.center() - galley.size() / 2.0, galley, ink);
        if response.clicked() {
            clicked = Some(*number);
        }
        right = button.left() - 6.0;
    }
    let text_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 28.0, rect.top()),
        egui::pos2(right - 6.0, rect.bottom()),
    );
    paint_elided(ui, text_rect, text, egui::FontId::proportional(12.5), ink);
    response.on_hover_text(hover);
    clicked
}

/// One action card; the primary one is filled.
fn card(ui: &mut egui::Ui, rect: egui::Rect, title: &str, subtitle: &str, primary: bool) -> bool {
    let response = ui
        .interact(
            rect,
            ui.scope_id().with(("card", title)),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let visuals = ui.visuals();
    let (fill, ink, weak) = if primary {
        (
            if response.hovered() {
                PRIMARY.gamma_multiply(1.15)
            } else {
                PRIMARY
            },
            egui::Color32::WHITE,
            egui::Color32::from_rgb(184, 212, 230),
        )
    } else {
        (
            if response.hovered() {
                visuals.widgets.hovered.weak_bg_fill
            } else {
                visuals.widgets.inactive.weak_bg_fill
            },
            visuals.text_color(),
            visuals.weak_text_color(),
        )
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    if !primary {
        ui.painter().rect_stroke(
            rect,
            6.0,
            visuals.widgets.noninteractive.bg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    let inner = rect.shrink2(egui::vec2(12.0, 9.0));
    paint_elided(
        ui,
        egui::Rect::from_min_size(inner.min, egui::vec2(inner.width(), 18.0)),
        title,
        egui::FontId::proportional(14.0),
        ink,
    );
    paint_elided(
        ui,
        egui::Rect::from_min_max(egui::pos2(inner.left(), inner.bottom() - 16.0), inner.max),
        subtitle,
        egui::FontId::proportional(11.5),
        weak,
    );
    response.clicked()
}

/// One line of text in `rect`, cut with an ellipsis when it does not fit.
pub fn paint_elided(
    ui: &egui::Ui,
    rect: egui::Rect,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
) {
    if rect.width() <= 0.0 {
        return;
    }
    let mut job = egui::text::LayoutJob::simple(
        text.replace(['\n', '\r', '\t'], " "),
        font,
        color,
        rect.width(),
    );
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let pos = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
    ui.painter().with_clip_rect(rect).galley(pos, galley, color);
}

/// `3 of 10` while filtering, else the count.
pub fn count_label(shown: usize, total: usize, filter: &str) -> String {
    if filter.is_empty() {
        total.to_string()
    } else {
        format!("{shown} of {total}")
    }
}

/// Indices of the recent entries that match every word of `filter` in
/// their folder name or path.
pub fn visible(recent: &[Recent], filter: &str) -> Vec<usize> {
    let words = filter
        .split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    recent
        .iter()
        .enumerate()
        .take(crate::settings::RECENT_LIMIT)
        .filter(|(_, entry)| {
            let path = entry.path.to_string_lossy().to_lowercase();
            words.iter().all(|word| path.contains(word.as_str()))
        })
        .map(|(index, _)| index)
        .collect()
}

/// `path` with the home folder written as `~`.
pub fn tilde(path: &Path, home: Option<&str>) -> String {
    let path = path.display().to_string();
    match home.filter(|home| !home.is_empty()) {
        Some(home) if path == home => "~".into(),
        Some(home) => match path.strip_prefix(home) {
            Some(rest) if rest.starts_with('/') => format!("~{rest}"),
            _ => path,
        },
        None => path,
    }
}

/// When a repository was last opened, relative to `now` (both seconds
/// since the Unix epoch): `just now`, `5 min ago`, `2 h ago`, `yesterday`,
/// `3 days ago`, then the date (`Oct 12`, or `Oct 12, 2025` in another
/// year).
pub fn opened_label(opened: u64, now: u64) -> String {
    let elapsed = now.saturating_sub(opened);
    const DAY: u64 = 86_400;
    if elapsed < 60 {
        "just now".into()
    } else if elapsed < 3_600 {
        format!("{} min ago", elapsed / 60)
    } else if elapsed < DAY {
        format!("{} h ago", elapsed / 3_600)
    } else if elapsed < 2 * DAY {
        "yesterday".into()
    } else if elapsed < 7 * DAY {
        format!("{} days ago", elapsed / DAY)
    } else {
        let (year, month, day) = civil((opened / DAY) as i64);
        let (this_year, _, _) = civil((now / DAY) as i64);
        const MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        let month = MONTHS[(month as usize).clamp(1, 12) - 1];
        if year == this_year {
            format!("{month} {day}")
        } else {
            format!("{month} {day}, {year}")
        }
    }
}

/// Year, month, and day of `days` since 1970-01-01 (proleptic Gregorian).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// The branch a repository folder has checked out, read from its HEAD file
/// (a worktree's `.git` file points at its own HEAD); a short commit ID
/// when HEAD is detached.
pub fn current_branch(path: &Path) -> Option<String> {
    let dot_git = path.join(".git");
    let git_dir = if dot_git.is_file() {
        let text = std::fs::read_to_string(&dot_git).ok()?;
        let target = text.trim().strip_prefix("gitdir:")?.trim();
        path.join(target)
    } else if dot_git.is_dir() {
        dot_git
    } else if path.join("HEAD").is_file() {
        path.to_path_buf()
    } else {
        return None;
    };
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    match head.strip_prefix("ref:") {
        Some(reference) => {
            let reference = reference.trim();
            Some(
                reference
                    .strip_prefix("refs/heads/")
                    .unwrap_or(reference)
                    .to_string(),
            )
        }
        None => Some(head.chars().take(7).collect()),
    }
}

/// The folder a dropped path stands for: itself, or a file's folder.
pub fn dropped_folder(path: &Path) -> PathBuf {
    if path.is_file() {
        path.parent()
            .map_or_else(|| path.to_path_buf(), Path::to_path_buf)
    } else {
        path.to_path_buf()
    }
}

/// The drag-over overlay: a dashed border and the folder to be opened.
pub fn paint_drop_overlay(ctx: &egui::Context, folder: Option<&str>) {
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::unique("welcome-drop"),
    ));
    let visuals = ctx.global_style().visuals.clone();
    painter.rect_filled(screen, 0.0, visuals.panel_fill.gamma_multiply(0.85));
    let frame = screen.shrink(12.0);
    let corners = [
        frame.left_top(),
        frame.right_top(),
        frame.right_bottom(),
        frame.left_bottom(),
        frame.left_top(),
    ];
    painter.extend(egui::Shape::dashed_line(
        &corners,
        egui::Stroke::new(2.0, DROP_BORDER),
        8.0,
        6.0,
    ));
    let text = match folder {
        Some(folder) => format!("Drop to open {folder}"),
        None => "Drop to open".into(),
    };
    painter.text(
        frame.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(18.0),
        visuals.text_color(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recent(paths: &[&str]) -> Vec<Recent> {
        paths.iter().map(|path| Recent::new(path.into())).collect()
    }

    #[test]
    fn opened_times_read_relative_then_as_dates() {
        let now = 1_793_620_800; // 2026-11-02 12:00 UTC
        assert_eq!(opened_label(now - 10, now), "just now");
        assert_eq!(opened_label(now - 5 * 60, now), "5 min ago");
        assert_eq!(opened_label(now - 2 * 3_600, now), "2 h ago");
        assert_eq!(opened_label(now - 30 * 3_600, now), "yesterday");
        assert_eq!(opened_label(now - 3 * 86_400, now), "3 days ago");
        assert_eq!(opened_label(now - 21 * 86_400, now), "Oct 12");
        assert_eq!(opened_label(now - 400 * 86_400, now), "Sep 28, 2025");
    }

    #[test]
    fn every_word_must_match_the_name_or_path_and_the_list_is_capped() {
        let list = recent(&[
            "/home/me/dev/gg",
            "/home/me/dev/gg-wt/login",
            "/home/me/src/egg-timer",
            "/home/me/src/dotfiles",
        ]);
        assert_eq!(visible(&list, ""), [0, 1, 2, 3]);
        assert_eq!(visible(&list, "gg"), [0, 1, 2]);
        assert_eq!(visible(&list, "GG login"), [1]);
        assert_eq!(visible(&list, "nothing"), Vec::<usize>::new());
        assert_eq!(count_label(3, 10, "gg"), "3 of 10");
        assert_eq!(count_label(10, 10, ""), "10");
        let many = (0..14)
            .map(|index| Recent::new(format!("/r{index}").into()))
            .collect::<Vec<_>>();
        assert_eq!(visible(&many, "").len(), crate::settings::RECENT_LIMIT);
    }

    #[test]
    fn paths_shorten_the_home_folder() {
        assert_eq!(
            tilde(Path::new("/home/me/dev/gg"), Some("/home/me")),
            "~/dev/gg"
        );
        assert_eq!(tilde(Path::new("/home/me"), Some("/home/me")), "~");
        assert_eq!(
            tilde(Path::new("/home/meow/x"), Some("/home/me")),
            "/home/meow/x"
        );
        assert_eq!(tilde(Path::new("/srv/x"), None), "/srv/x");
    }

    #[test]
    fn the_branch_is_read_from_head_for_repositories_and_worktrees()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(repo.join(".git"))?;
        std::fs::write(repo.join(".git/HEAD"), "ref: refs/heads/feature/login\n")?;
        assert_eq!(current_branch(&repo).as_deref(), Some("feature/login"));
        let worktree = dir.path().join("wt");
        std::fs::create_dir_all(dir.path().join("repo/.git/worktrees/wt"))?;
        std::fs::write(
            dir.path().join("repo/.git/worktrees/wt/HEAD"),
            "0123456789abcdef\n",
        )?;
        std::fs::create_dir_all(&worktree)?;
        std::fs::write(worktree.join(".git"), "gitdir: ../repo/.git/worktrees/wt\n")?;
        assert_eq!(current_branch(&worktree).as_deref(), Some("0123456"));
        assert_eq!(current_branch(dir.path()), None);
        Ok(())
    }

    #[test]
    fn a_dropped_file_stands_for_its_folder() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("a.txt");
        std::fs::write(&file, b"a")?;
        assert_eq!(dropped_folder(&file), dir.path());
        assert_eq!(dropped_folder(dir.path()), dir.path());
        Ok(())
    }
}
