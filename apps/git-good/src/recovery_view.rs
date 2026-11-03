//! Repository › Recovery…: gg's private rewrite backups and the reflog in
//! one window, as the inspect proposal draws it. A Backups | Reflog switch
//! in the header, a 22 px table with the time first, the selected entry as
//! a label grid, **Restore <branch>…** as the one primary (a confirmed hard
//! reset of a clean tree that first saves the current tip as a backup),
//! Reset ▸ Soft/Mixed/Hard and Copy under ⋯, **Delete backup…** for
//! backups, and results in the window's 22 px status strip. There is no
//! Refresh: the window re-reads on open, when gg gets focus back, and after
//! every operation, always on a background thread.
use crate::inspect_ui::{self, Tone};
use crate::menus;
use crate::return_location::RecoverySource;
use eframe::egui::{self, RichText};
use gg_engine::{recovery::Backup, reflog::Entry};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

/// What the window asks the application to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Open the commit in Review, returning to this source and entry.
    Open {
        source: RecoverySource,
        reference: Option<String>,
        commit: String,
        subject: String,
        entry: String,
    },
    CreateBranch {
        source: RecoverySource,
        commit: String,
        selector: String,
    },
    /// Reset ▸ Soft/Mixed/Hard under ⋯, through the shared reset question.
    Reset {
        commit: String,
        mode: gg_engine::reset::Mode,
    },
    /// A restore or delete finished; re-read refs, history, and status.
    Changed,
}

/// One background read of everything the window shows.
struct Snapshot {
    backups: Result<Vec<Backup>, String>,
    reference: String,
    reflog: Result<Vec<Entry>, String>,
    /// The checked-out branch and its tip; `None` on a detached HEAD.
    current: Option<String>,
    head: Option<String>,
    /// Tracked files with changes, which block a restore.
    uncommitted: Option<usize>,
    /// Branch tip and differing commits for each backup, by reference.
    branch_now: Vec<(String, (String, usize))>,
    /// Local UTC offset in minutes, for absolute times.
    offset: i32,
}

fn read(path: &Path, reference: &str) -> Snapshot {
    let backups = gg_git::backup::list(path);
    let branch_now = backups
        .as_ref()
        .map(|backups| {
            backups
                .iter()
                .filter_map(|backup| {
                    let branch = backup.branch.as_deref()?;
                    let now = gg_git::backup::branch_now(path, branch, &backup.commit)?;
                    Some((backup.reference.clone(), now))
                })
                .collect()
        })
        .unwrap_or_default();
    let reflog = gg_git::reflog::read(path, reference).map(|snapshot| snapshot.entries);
    let head = gg_git::process::git()
        .current_dir(path)
        .args(["rev-parse", "--verify", "--quiet", "HEAD"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string());
    let offset = gg_git::process::git()
        .current_dir(path)
        .args(["log", "-1", "--format=%cd", "--date=format-local:%z"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .ok()
        .and_then(|output| inspect_ui::parse_offset(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or_default();
    Snapshot {
        backups,
        reference: reference.to_string(),
        reflog,
        current: gg_git::backup::current_branch(path),
        head,
        uncommitted: gg_git::backup::uncommitted(path).ok(),
        branch_now,
        offset,
    }
}

/// A question the window asks before it changes anything.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Confirm {
    Restore {
        branch: Option<String>,
        commit: String,
        subject: String,
    },
    Delete {
        reference: String,
        branch: Option<String>,
        commit: String,
        subject: String,
    },
}

/// The ⋯ menu's commands.
#[derive(Clone, Debug)]
enum More {
    CopyCommit,
    CopyRef,
    Reset(gg_engine::reset::Mode),
    Delete,
}

/// The selected row, whichever source it comes from.
struct Selected {
    commit: String,
    subject: String,
    /// The backup reference or the reflog selector.
    key: String,
    /// The branch a restore moves; `None` means the current branch.
    branch: Option<String>,
}

pub struct RecoveryView {
    pub source: RecoverySource,
    pub open: bool,
    path: PathBuf,
    reference: String,
    snapshot: Option<Snapshot>,
    reading: Option<mpsc::Receiver<Snapshot>>,
    /// Another read was asked for while one ran.
    again: bool,
    backup: Option<String>,
    entry: Option<String>,
    result: Option<(Tone, String)>,
    confirm: Option<Confirm>,
    job: Option<mpsc::Receiver<Result<String, String>>>,
    ctx: egui::Context,
}

pub const WINDOW_ID: &str = "recovery";

impl RecoveryView {
    pub fn new(path: PathBuf, ctx: &egui::Context) -> Self {
        let mut view = Self {
            source: RecoverySource::Backups,
            open: true,
            path,
            reference: "HEAD".into(),
            snapshot: None,
            reading: None,
            again: false,
            backup: None,
            entry: None,
            result: None,
            confirm: None,
            job: None,
            ctx: ctx.clone(),
        };
        view.reread();
        view
    }

    /// Re-read backups, the reflog, and the branch state in the background.
    pub fn reread(&mut self) {
        if self.reading.is_some() {
            self.again = true;
            return;
        }
        let (send, receive) = mpsc::channel();
        let path = self.path.clone();
        let reference = self.reference.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let _ = send.send(read(&path, &reference));
            ctx.request_repaint();
        });
        self.reading = Some(receive);
    }

    #[cfg(test)]
    pub fn wait(&mut self) {
        while self.reading.is_some() || self.job.is_some() {
            if let Some(job) = &self.job
                && let Ok(result) = job.recv()
            {
                self.job = None;
                self.finish(result);
            }
            if let Some(reading) = self.reading.take()
                && let Ok(snapshot) = reading.recv()
            {
                self.accept(snapshot);
                if std::mem::take(&mut self.again) {
                    self.reread();
                }
            }
        }
    }

    fn accept(&mut self, snapshot: Snapshot) {
        if let Ok(backups) = &snapshot.backups
            && !self
                .backup
                .as_ref()
                .is_some_and(|key| backups.iter().any(|backup| &backup.reference == key))
        {
            self.backup = backups.first().map(|backup| backup.reference.clone());
        }
        if let Ok(entries) = &snapshot.reflog
            && snapshot.reference == self.reference
            && !self
                .entry
                .as_ref()
                .is_some_and(|key| entries.iter().any(|entry| &entry.selector == key))
        {
            self.entry = entries.first().map(|entry| entry.selector.clone());
        }
        self.snapshot = Some(snapshot);
    }

    fn poll(&mut self) {
        if let Some(result) = self.job.as_ref().and_then(|job| job.try_recv().ok()) {
            self.job = None;
            self.finish(result);
        }
        if let Some(snapshot) = self.reading.as_ref().and_then(|read| read.try_recv().ok()) {
            self.reading = None;
            self.accept(snapshot);
            if std::mem::take(&mut self.again) {
                self.reread();
            }
        }
    }

    fn finish(&mut self, result: Result<String, String>) {
        self.result = Some(match result {
            Ok(text) => (Tone::Green, text),
            Err(error) => (Tone::Red, error),
        });
        self.reread();
    }

    #[cfg(test)]
    pub fn reference(&self) -> &str {
        &self.reference
    }

    pub fn set_reference(&mut self, reference: &str) {
        if reference != self.reference {
            self.reference = reference.to_owned();
            self.entry = None;
            self.reread();
        }
    }

    /// Select a backup reference or a reflog selector; kept until the list
    /// that holds it arrives.
    pub fn select(&mut self, source: RecoverySource, key: &str) {
        self.source = source;
        match source {
            RecoverySource::Backups => self.backup = Some(key.to_owned()),
            RecoverySource::Reflog => self.entry = Some(key.to_owned()),
        }
    }

    #[cfg(test)]
    pub fn selected_selector(&self) -> Option<&str> {
        self.entry.as_deref()
    }

    /// A result from outside, such as a branch created from an entry.
    pub fn notice(&mut self, notice: String) {
        self.result = Some((Tone::Green, notice));
        self.reread();
    }

    /// Up/Down and page keys move the selection in the shown source.
    fn move_selection(&mut self, step: isize) {
        let keys: Vec<String> = match self.source {
            RecoverySource::Backups => self.backups().iter().map(|b| b.reference.clone()).collect(),
            RecoverySource::Reflog => self.entries().iter().map(|e| e.selector.clone()).collect(),
        };
        if keys.is_empty() {
            return;
        }
        let current = match self.source {
            RecoverySource::Backups => self.backup.as_ref(),
            RecoverySource::Reflog => self.entry.as_ref(),
        };
        let index = current
            .and_then(|key| keys.iter().position(|candidate| candidate == key))
            .unwrap_or(0) as isize;
        let next = keys[(index + step).clamp(0, keys.len() as isize - 1) as usize].clone();
        match self.source {
            RecoverySource::Backups => self.backup = Some(next),
            RecoverySource::Reflog => self.entry = Some(next),
        }
    }

    fn backups(&self) -> &[Backup] {
        match self.snapshot.as_ref().map(|snapshot| &snapshot.backups) {
            Some(Ok(backups)) => backups,
            _ => &[],
        }
    }

    fn entries(&self) -> &[Entry] {
        match self.snapshot.as_ref() {
            Some(snapshot) if snapshot.reference == self.reference => match &snapshot.reflog {
                Ok(entries) => entries,
                Err(_) => &[],
            },
            _ => &[],
        }
    }

    fn selected(&self) -> Option<Selected> {
        match self.source {
            RecoverySource::Backups => {
                let key = self.backup.as_ref()?;
                let backup = self.backups().iter().find(|b| &b.reference == key)?;
                Some(Selected {
                    commit: backup.commit.clone(),
                    subject: backup.subject.clone(),
                    key: backup.reference.clone(),
                    branch: backup.branch.clone(),
                })
            }
            RecoverySource::Reflog => {
                let key = self.entry.as_ref()?;
                let entry = self.entries().iter().find(|e| &e.selector == key)?;
                Some(Selected {
                    commit: entry.id.clone(),
                    subject: entry.subject.clone(),
                    key: entry.selector.clone(),
                    branch: None,
                })
            }
        }
    }

    /// The branch a restore of `selected` moves, if one can be named.
    fn restore_branch(&self, selected: &Selected) -> Option<String> {
        selected.branch.clone().or_else(|| {
            self.snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.current.clone())
        })
    }

    fn restore_label(&self, selected: &Selected) -> String {
        let here = if self.source == RecoverySource::Reflog {
            " here"
        } else {
            ""
        };
        match self.restore_branch(selected) {
            Some(branch) => format!("Restore {branch}{here}…"),
            None => format!("Restore{here}…"),
        }
    }

    /// Why the confirmed restore cannot run now.
    fn restore_reason(&self, branch: Option<&str>) -> Option<String> {
        if self.job.is_some() {
            return Some("Wait for the current operation to finish.".into());
        }
        let snapshot = self.snapshot.as_ref()?;
        let Some(current) = snapshot.current.as_deref() else {
            return Some("Check out a branch first.".into());
        };
        let branch = branch.unwrap_or(current);
        if branch != current {
            return Some(format!("{branch} is not checked out. Switch to it first."));
        }
        match snapshot.uncommitted {
            Some(0) => None,
            Some(count) => Some(gg_git::backup::uncommitted_reason(count)),
            None => Some("The working tree status could not be read.".into()),
        }
    }

    fn start_restore(&mut self, ctx: &egui::Context, commit: String, branch: String) {
        let (send, receive) = mpsc::channel();
        let path = self.path.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = gg_git::backup::restore(&path, &commit, &branch).map(|receipt| {
                format!(
                    "Restored {} to {} · the previous tip {} is saved as a backup",
                    receipt.branch,
                    inspect_ui::short(&receipt.commit),
                    inspect_ui::short(&receipt.previous)
                )
            });
            let _ = send.send(result);
            ctx.request_repaint();
        });
        self.job = Some(receive);
    }

    fn start_delete(&mut self, ctx: &egui::Context, reference: String, commit: String) {
        let (send, receive) = mpsc::channel();
        let path = self.path.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = gg_git::backup::delete(&path, &reference)
                .map(|()| format!("Deleted the backup at {}", inspect_ui::short(&commit)));
            let _ = send.send(result);
            ctx.request_repaint();
        });
        self.job = Some(receive);
    }

    pub fn draw(&mut self, ctx: &egui::Context, references: &[String]) -> Option<Action> {
        let idle = self.job.is_none();
        self.poll();
        let mut action = None;
        let max_width = (ctx.content_rect().width() - 24.0).max(320.0);
        let max_height = (ctx.content_rect().height() - 24.0).max(320.0);
        // The list keeps one height while entries arrive.
        let list_height = (ctx.content_rect().height() - 360.0)
            .clamp(3.0 * inspect_ui::ROW, 8.0 * inspect_ui::ROW);
        let id = egui::Id::unique(WINDOW_ID);
        let keys = inspect_ui::owns_keys(ctx, id) && self.confirm.is_none();
        let mut close = false;
        if keys {
            let step = crate::navigation::step(ctx, 8);
            if step != 0 {
                self.move_selection(step);
            }
        }
        egui::Window::new("Recovery")
            .id(id)
            .title_bar(false)
            .collapsible(false)
            .resizable(true)
            .default_width(max_width.min(800.0))
            .max_width(max_width)
            .max_height(max_height)
            .default_pos(ctx.content_rect().center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                close = self.header(ui, references);
                match self.source {
                    RecoverySource::Backups => {
                        ui.weak("Saved by Git Good before each rebase, amend, rewrite or reset. Hidden from the refs sidebar.");
                        self.backup_list(ui, list_height);
                    }
                    RecoverySource::Reflog => self.reflog_list(ui, list_height),
                }
                ui.separator();
                let selected = self.selected();
                self.details(ui, selected.as_ref());
                if let Some(next) = self.actions(ui, selected.as_ref(), keys) {
                    action = Some(next);
                }
                let (tone, text) = self.strip();
                inspect_ui::status_strip(ui, &text, tone.text());
            });
        if keys && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            close = true;
        }
        if close {
            self.open = false;
        }
        if let Some(next) = self.draw_confirm(ctx) {
            action = Some(next);
        }
        if !idle && self.job.is_none() {
            action = Some(Action::Changed);
        }
        action
    }

    fn header(&mut self, ui: &mut egui::Ui, references: &[String]) -> bool {
        let backups = self.backups().len();
        let current = self
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.current.clone());
        let mut source = None;
        let mut reference = None;
        let close = inspect_ui::header(ui, "Recovery", None, |ui| {
            source = inspect_ui::segmented(
                ui,
                self.source,
                &[
                    (RecoverySource::Backups, format!("Backups {backups}")),
                    (RecoverySource::Reflog, "Reflog".into()),
                ],
            );
            if self.source == RecoverySource::Reflog {
                ui.weak("Reference");
                egui::ComboBox::from_id_salt("recovery-reference")
                    .selected_text(inspect_ui::mono(&self.reference))
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        for candidate in references {
                            let text = inspect_ui::mono(candidate);
                            let mut row = menus::row(candidate == &self.reference, text);
                            if current.as_deref() == Some(candidate.as_str()) {
                                row = row.shortcut_text("current");
                            }
                            if ui.add(row).clicked() {
                                reference = Some(candidate.clone());
                            }
                            if candidate == "HEAD" && references.len() > 1 {
                                ui.separator();
                            }
                        }
                    });
            }
        });
        if let Some(source) = source {
            self.source = source;
        }
        if let Some(reference) = reference {
            self.set_reference(&reference);
        }
        close
    }

    fn backup_list(&mut self, ui: &mut egui::Ui, height: f32) {
        let columns = [76.0, 150.0, 64.0, 70.0, 0.0];
        inspect_ui::table_header(
            ui,
            &columns,
            &["SAVED", "BRANCH", "BEFORE", "TIP", "SUBJECT"],
        );
        let now = inspect_ui::now();
        let mut chosen = None;
        let backups = self.backups().to_vec();
        let loaded = self.snapshot.is_some();
        reserve(ui, height, |ui| {
            if backups.is_empty() {
                if loaded {
                    empty(
                        ui,
                        height,
                        "No rewrite backups yet.",
                        Some("Git Good saves one before every rebase, amend, rewrite or reset."),
                    );
                }
                return;
            }
            egui::ScrollArea::vertical()
                .id_salt("recovery-backups")
                .max_height(height)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for backup in &backups {
                        let on = self.backup.as_deref() == Some(backup.reference.as_str());
                        let response = inspect_ui::table_row(ui, on, &columns, |cells| {
                            cells[0].label(RichText::new(saved_label(backup, now)).weak());
                            match &backup.branch {
                                Some(branch) => {
                                    inspect_ui::pill(&mut cells[1], branch, Tone::Blue);
                                }
                                None => {
                                    cells[1].weak("—");
                                }
                            }
                            cells[2].label(&backup.label);
                            cells[3].label(inspect_ui::mono(inspect_ui::short(&backup.commit)));
                            cells[4].add(egui::Label::new(&backup.subject).truncate());
                        });
                        if response.clicked() {
                            chosen = Some(backup.reference.clone());
                        }
                    }
                });
        });
        if let Some(chosen) = chosen {
            self.backup = Some(chosen);
        }
    }

    fn reflog_list(&mut self, ui: &mut egui::Ui, height: f32) {
        let width = ui.available_width();
        let what = (width - 16.0 - 64.0 - 68.0 - 70.0 - 30.0).max(80.0);
        let columns = [64.0, 68.0, what, 0.0];
        inspect_ui::table_header(ui, &columns, &["WHEN", "ENTRY", "WHAT HAPPENED", "COMMIT"]);
        let now = inspect_ui::now();
        let entries = self.entries().to_vec();
        let loaded = self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.reference == self.reference);
        let mut chosen = None;
        reserve(ui, height, |ui| {
            if entries.is_empty() {
                if loaded {
                    empty(
                        ui,
                        height,
                        &format!("No reflog entries for {}.", self.reference),
                        None,
                    );
                }
                return;
            }
            egui::ScrollArea::vertical()
                .id_salt("recovery-reflog")
                .max_height(height)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for entry in &entries {
                        let on = self.entry.as_deref() == Some(entry.selector.as_str());
                        let response = inspect_ui::table_row(ui, on, &columns, |cells| {
                            cells[0]
                                .label(RichText::new(inspect_ui::relative(entry.time, now)).weak());
                            let index = entry
                                .selector
                                .rfind('@')
                                .map_or(entry.selector.as_str(), |at| &entry.selector[at..]);
                            cells[1].label(inspect_ui::mono(index).weak());
                            cells[2].add(egui::Label::new(plain(entry)).truncate());
                            cells[3].label(inspect_ui::mono(inspect_ui::short(&entry.id)));
                        });
                        if response.clicked() {
                            chosen = Some(entry.selector.clone());
                        }
                    }
                });
        });
        if let Some(chosen) = chosen {
            self.entry = Some(chosen);
        }
    }

    fn details(&self, ui: &mut egui::Ui, selected: Option<&Selected>) {
        let offset = self.snapshot.as_ref().map_or(0, |snapshot| snapshot.offset);
        // Four rows always, so the footer never moves.
        reserve(ui, 4.0 * inspect_ui::ROW, |ui| {
            let Some(selected) = selected else {
                return;
            };
            match self.source {
                RecoverySource::Backups => {
                    let Some(backup) = self.backups().iter().find(|b| b.reference == selected.key)
                    else {
                        return;
                    };
                    let now = self.snapshot.as_ref().and_then(|snapshot| {
                        snapshot
                            .branch_now
                            .iter()
                            .find(|(reference, _)| reference == &backup.reference)
                            .map(|(_, now)| now.clone())
                    });
                    inspect_ui::grid(ui, "recovery-backup-details", |ui| {
                        commit_row(ui, "TIP", &backup.commit, &backup.subject);
                        inspect_ui::row(ui, "SAVED", |ui| {
                            let when = if backup.saved > 0 {
                                inspect_ui::calendar(backup.saved, offset)
                            } else {
                                backup.time.clone()
                            };
                            let what = match &backup.branch {
                                Some(branch) => format!("before {} of {branch}", backup.label),
                                None => format!("before {}", backup.label),
                            };
                            ui.label(format!("{when} · {what}"));
                        });
                        inspect_ui::row(ui, "BRANCH NOW", |ui| match (&backup.branch, now) {
                            (Some(_), Some((tip, differ))) => {
                                ui.label(inspect_ui::mono(inspect_ui::short(&tip)));
                                ui.weak(match differ {
                                    0 => "same as the backup".to_string(),
                                    1 => "1 commit differs from the backup".to_string(),
                                    n => format!("{n} commits differ from the backup"),
                                });
                            }
                            (Some(branch), None) => {
                                ui.weak(format!("{branch} no longer exists"));
                            }
                            (None, _) => {
                                ui.weak("Not recorded for this backup");
                            }
                        });
                        inspect_ui::row(ui, "REF", |ui| {
                            ui.add(
                                egui::Label::new(inspect_ui::mono(&backup.reference).weak())
                                    .truncate(),
                            );
                            inspect_ui::copy_button(ui, &backup.reference, "backup ref");
                        });
                    });
                }
                RecoverySource::Reflog => {
                    let Some(entry) = self.entries().iter().find(|e| e.selector == selected.key)
                    else {
                        return;
                    };
                    inspect_ui::grid(ui, "recovery-reflog-details", |ui| {
                        commit_row(ui, "COMMIT", &entry.id, &entry.subject);
                        inspect_ui::row(ui, "WHEN", |ui| {
                            ui.label(format!(
                                "{} · {}",
                                inspect_ui::calendar(entry.time, offset),
                                inspect_ui::relative(entry.time, inspect_ui::now())
                            ));
                        });
                        inspect_ui::row(ui, "ENTRY", |ui| {
                            ui.label(inspect_ui::mono(&entry.selector));
                            inspect_ui::copy_button(ui, &entry.selector, "reflog ref");
                        });
                        inspect_ui::row(ui, "GIT MESSAGE", |ui| {
                            ui.add(
                                egui::Label::new(inspect_ui::mono(raw(entry)).weak()).truncate(),
                            );
                        });
                    });
                }
            }
        });
    }

    fn actions(
        &mut self,
        ui: &mut egui::Ui,
        selected: Option<&Selected>,
        keys: bool,
    ) -> Option<Action> {
        let mut action = None;
        let mut more = None;
        let backups = self.source == RecoverySource::Backups;
        let restore_label = selected.map(|selected| self.restore_label(selected));
        let mut restore = false;
        let mut delete = false;
        ui.separator();
        ui.horizontal(|ui| {
            ui.set_min_height(26.0);
            if backups
                && ui
                    .add_enabled(
                        selected.is_some(),
                        egui::Button::new(RichText::new("Delete backup…").color(Tone::Red.text())),
                    )
                    .clicked()
            {
                delete = true;
            }
            ui.add_enabled_ui(selected.is_some(), |ui| {
                ui.menu_button("⋯", |ui| {
                    use menus::item;
                    let mut list = vec![
                        item("Copy commit ID", "Ctrl+C", More::CopyCommit).entry(),
                        item("Copy ref", "Ctrl+Shift+C", More::CopyRef).entry(),
                        menus::Entry::Separator,
                        menus::Entry::Submenu(
                            "Reset branch here".into(),
                            vec![
                                item(
                                    "Soft",
                                    "keep index and files",
                                    More::Reset(gg_engine::reset::Mode::Soft),
                                )
                                .entry(),
                                item(
                                    "Mixed",
                                    "keep files",
                                    More::Reset(gg_engine::reset::Mode::Mixed),
                                )
                                .entry(),
                                item(
                                    "Hard…",
                                    "discard changes",
                                    More::Reset(gg_engine::reset::Mode::Hard),
                                )
                                .danger()
                                .entry(),
                            ],
                        ),
                    ];
                    if backups {
                        list.push(menus::Entry::Separator);
                        list.push(item("Delete backup…", "Del", More::Delete).danger().entry());
                    }
                    menus::entries(ui, &list, &mut more);
                })
                .response
                .on_hover_text("More actions");
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let Some(selected) = selected else {
                    ui.add_enabled(false, inspect_ui::primary("Restore…", Some("Enter")));
                    return;
                };
                if ui
                    .add(inspect_ui::primary(
                        restore_label.clone().unwrap_or_default(),
                        Some("Enter"),
                    ))
                    .clicked()
                {
                    restore = true;
                }
                if ui.button("Create branch here…").clicked() {
                    action = Some(Action::CreateBranch {
                        source: self.source,
                        commit: selected.commit.clone(),
                        selector: selected.key.clone(),
                    });
                }
                if ui.button("Open commit").clicked() {
                    action = Some(self.open_action(selected));
                }
            });
        });
        if keys && selected.is_some() {
            // Ctrl+Shift+C is consumed before Ctrl+C.
            // The platform turns Ctrl+C into a Copy event.
            let (enter, del, copy_ref, copy_commit) = ui.input_mut(|i| {
                let copied = i
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Copy));
                let shift = i.modifiers.shift;
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Delete),
                    i.consume_key(
                        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                        egui::Key::C,
                    ) || (copied && shift),
                    i.consume_key(egui::Modifiers::COMMAND, egui::Key::C) || (copied && !shift),
                )
            });
            restore |= enter;
            delete |= del && backups;
            if copy_ref {
                more = Some(More::CopyRef);
            } else if copy_commit {
                more = Some(More::CopyCommit);
            }
        }
        let selected = selected?;
        match more {
            Some(More::CopyCommit) => {
                ui.ctx().copy_text(selected.commit.clone());
                self.result = Some((
                    Tone::Neutral,
                    format!("Copied {}", inspect_ui::short(&selected.commit)),
                ));
            }
            Some(More::CopyRef) => {
                ui.ctx().copy_text(selected.key.clone());
                self.result = Some((Tone::Neutral, format!("Copied {}", selected.key)));
            }
            Some(More::Reset(mode)) => {
                action = Some(Action::Reset {
                    commit: selected.commit.clone(),
                    mode,
                });
            }
            Some(More::Delete) => delete = true,
            None => {}
        }
        if restore {
            self.confirm = Some(Confirm::Restore {
                branch: self.restore_branch(selected),
                commit: selected.commit.clone(),
                subject: selected.subject.clone(),
            });
        } else if delete && backups {
            self.confirm = Some(Confirm::Delete {
                reference: selected.key.clone(),
                branch: selected.branch.clone(),
                commit: selected.commit.clone(),
                subject: selected.subject.clone(),
            });
        }
        action
    }

    fn open_action(&self, selected: &Selected) -> Action {
        Action::Open {
            source: self.source,
            reference: (self.source == RecoverySource::Reflog).then(|| self.reference.clone()),
            commit: selected.commit.clone(),
            subject: selected.subject.clone(),
            entry: selected.key.clone(),
        }
    }

    /// The status strip: the last result, a read error, or a count.
    fn strip(&self) -> (Tone, String) {
        if let Some((tone, text)) = &self.result {
            return (*tone, text.clone());
        }
        let Some(snapshot) = &self.snapshot else {
            return (Tone::Neutral, String::new());
        };
        match self.source {
            RecoverySource::Backups => match &snapshot.backups {
                Ok(backups) => (
                    Tone::Neutral,
                    crate::consequence::counted(backups.len(), "backup", "backups"),
                ),
                Err(error) => (Tone::Red, error.clone()),
            },
            RecoverySource::Reflog => match &snapshot.reflog {
                Ok(entries) => (
                    Tone::Neutral,
                    format!(
                        "{} · {}",
                        crate::consequence::counted(entries.len(), "entry", "entries"),
                        snapshot.reference
                    ),
                ),
                Err(error) => (
                    Tone::Red,
                    error.lines().next().unwrap_or_default().to_string(),
                ),
            },
        }
    }

    fn draw_confirm(&mut self, ctx: &egui::Context) -> Option<Action> {
        let confirm = self.confirm.clone()?;
        let decision = match &confirm {
            Confirm::Restore {
                branch,
                commit,
                subject,
            } => {
                let reason = self.restore_reason(branch.as_deref());
                let tip = self
                    .snapshot
                    .as_ref()
                    .and_then(|snapshot| {
                        let branch = branch.as_deref()?;
                        if snapshot.current.as_deref() == Some(branch) {
                            snapshot.head.clone()
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                let title = match branch {
                    Some(branch) => format!("Restore {branch}"),
                    None => "Restore branch".to_string(),
                };
                inspect_ui::confirm(
                    ctx,
                    "restore-backup",
                    &title,
                    "Restore branch",
                    reason.as_deref(),
                    false,
                    |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Move");
                            if let Some(branch) = branch {
                                inspect_ui::pill(ui, branch, Tone::Blue);
                            }
                            if !tip.is_empty() {
                                ui.label("from");
                                ui.label(inspect_ui::mono(inspect_ui::short(&tip)));
                            }
                            ui.label("back to");
                            ui.label(inspect_ui::mono(inspect_ui::short(commit)));
                            ui.label(format!("{subject}?"));
                        });
                        let good = Tone::Green.text();
                        ui.horizontal(|ui| {
                            ui.colored_label(good, "✓");
                            ui.weak("Files are checked out to match the backup.");
                        });
                        ui.horizontal(|ui| {
                            ui.colored_label(good, "✓");
                            ui.weak(if tip.is_empty() {
                                "The current tip is saved as a new backup, so this can be undone."
                                    .to_string()
                            } else {
                                format!(
                                    "The current tip {} is saved as a new backup, so this can be undone.",
                                    inspect_ui::short(&tip)
                                )
                            });
                        });
                    },
                )
            }
            Confirm::Delete {
                branch,
                commit,
                subject,
                ..
            } => inspect_ui::confirm(
                ctx,
                "delete-backup",
                "Delete backup",
                "Delete backup",
                self.job
                    .is_some()
                    .then_some("Wait for the current operation to finish."),
                true,
                |ui| {
                    ui.label(match branch {
                        Some(branch) => format!(
                            "Delete the backup of {branch} at {} {subject}?",
                            inspect_ui::short(commit)
                        ),
                        None => format!(
                            "Delete the backup at {} {subject}?",
                            inspect_ui::short(commit)
                        ),
                    });
                    ui.weak("Only Git Good's private ref is removed. The commit stays while a branch, tag, or the reflog still reaches it.");
                },
            ),
        };
        match decision? {
            inspect_ui::Decision::Cancel => self.confirm = None,
            inspect_ui::Decision::Confirm => {
                self.confirm = None;
                match confirm {
                    Confirm::Restore { branch, commit, .. } => {
                        if let Some(branch) = branch.or_else(|| {
                            self.snapshot
                                .as_ref()
                                .and_then(|snapshot| snapshot.current.clone())
                        }) {
                            self.start_restore(ctx, commit, branch);
                        }
                    }
                    Confirm::Delete {
                        reference, commit, ..
                    } => self.start_delete(ctx, reference, commit),
                }
            }
        }
        None
    }
}

/// Lay `content` out in a fixed `height`, so nothing below moves.
fn reserve(ui: &mut egui::Ui, height: f32, content: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width();
    ui.allocate_ui_with_layout(
        egui::vec2(width, height),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_min_size(egui::vec2(width, height));
            content(ui);
        },
    );
}

fn empty(ui: &mut egui::Ui, height: f32, title: &str, detail: Option<&str>) {
    ui.add_space((height / 2.0 - 22.0).max(0.0));
    ui.vertical_centered(|ui| {
        ui.label(title);
        if let Some(detail) = detail {
            ui.weak(detail);
        }
    });
}

fn commit_row(ui: &mut egui::Ui, label: &str, commit: &str, subject: &str) {
    inspect_ui::row(ui, label, |ui| {
        ui.label(inspect_ui::mono(inspect_ui::short(commit)));
        ui.add(egui::Label::new(subject).truncate());
        inspect_ui::copy_button(ui, commit, "commit ID");
    });
}

fn saved_label(backup: &Backup, now: i64) -> String {
    if backup.saved > 0 {
        inspect_ui::relative(backup.saved, now)
    } else {
        backup.time.get(..10).unwrap_or(&backup.time).to_string()
    }
}

/// Git's reflog subject as it was written: `rebase (pick): Add footer notice`.
pub fn raw(entry: &Entry) -> String {
    if entry.message.is_empty() {
        entry.action.clone()
    } else {
        format!("{}: {}", entry.action, entry.message)
    }
}

fn short_ref(name: &str) -> &str {
    name.strip_prefix("refs/heads/")
        .or_else(|| name.strip_prefix("refs/remotes/"))
        .or_else(|| name.strip_prefix("refs/tags/"))
        .unwrap_or(name)
}

/// A reflog event in plain language; Git's own wording stays under GIT
/// MESSAGE.
pub fn plain(entry: &Entry) -> String {
    let action = entry.action.as_str();
    let message = entry.message.as_str();
    let quoted = |text: &str| format!("\u{201c}{text}\u{201d}");
    match action {
        "commit" => format!("Committed {}", quoted(message)),
        "commit (initial)" => format!("First commit {}", quoted(message)),
        "commit (amend)" => format!("Amended {}", quoted(message)),
        "commit (merge)" => format!("Merge commit {}", quoted(message)),
        "commit (cherry-pick)" | "cherry-pick" => format!("Cherry-picked {}", quoted(message)),
        "revert" => format!("Reverted with {}", quoted(message)),
        "checkout" => match message
            .strip_prefix("moving from ")
            .and_then(|rest| rest.split_once(" to "))
        {
            Some((from, to)) => format!("Switched from {from} to {to}"),
            None => format!("Checked out {message}"),
        },
        "reset" => match message.strip_prefix("moving to ") {
            Some(target) => format!("Reset to {}", short_ref(target)),
            None => "Reset".into(),
        },
        "rebase (start)" | "rebase -i (start)" => match message.strip_prefix("checkout ") {
            Some(onto) => format!("Started rebase onto {}", short_ref(onto)),
            None => "Started rebase".into(),
        },
        "rebase (finish)" | "rebase -i (finish)" => {
            let branch = message
                .strip_prefix("returning to ")
                .or_else(|| message.split(' ').next())
                .unwrap_or(message);
            format!("Finished rebase of {}", short_ref(branch))
        }
        "rebase (pick)" | "rebase -i (pick)" => format!("Rebase picked {}", quoted(message)),
        "rebase (reword)" | "rebase -i (reword)" => format!("Rebase reworded {}", quoted(message)),
        "rebase (squash)" | "rebase -i (squash)" => format!("Rebase squashed {}", quoted(message)),
        "rebase (fixup)" | "rebase -i (fixup)" => format!("Rebase fixed up {}", quoted(message)),
        "rebase (edit)" | "rebase -i (edit)" => {
            format!("Rebase stopped to edit {}", quoted(message))
        }
        "rebase (continue)" | "rebase -i (continue)" => {
            format!("Rebase continued with {}", quoted(message))
        }
        "rebase (abort)" | "rebase -i (abort)" => "Aborted a rebase".into(),
        "branch" => match message.strip_prefix("Created from ") {
            Some(from) => format!("Created the branch from {}", short_ref(from)),
            None => "Created the branch".into(),
        },
        "clone" => "Cloned the repository".into(),
        "pull" | "pull --rebase" if message.contains("Fast-forward") => {
            "Pulled (fast-forward)".into()
        }
        "update by push" => "Pushed".into(),
        _ => {
            if let Some(target) = action.strip_prefix("merge ") {
                if message.contains("Fast-forward") {
                    format!("Fast-forwarded to {target}")
                } else {
                    format!("Merged {target}")
                }
            } else if action.starts_with("pull") {
                format!("Pulled · {message}")
            } else {
                raw(entry)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    type Checked<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn run(path: &Path, args: &[&str]) -> Checked<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.org")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.org")
            .output()?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// `main` at "next" with a rebase backup of "base".
    fn repository() -> Checked<(tempfile::TempDir, String, String)> {
        let dir = tempfile::tempdir()?;
        let path = dir.path();
        run(path, &["init", "-q", "-b", "main"])?;
        run(path, &["config", "user.name", "Test"])?;
        run(path, &["config", "user.email", "test@example.org"])?;
        std::fs::write(path.join("a"), b"base\n")?;
        run(path, &["add", "a"])?;
        run(path, &["commit", "-qm", "base"])?;
        let base = run(path, &["rev-parse", "HEAD"])?;
        std::fs::write(path.join("a"), b"next\n")?;
        run(path, &["commit", "-qam", "next"])?;
        let next = run(path, &["rev-parse", "HEAD"])?;
        gg_git::backup::create(path, "rebase", &base)?;
        Ok((dir, base, next))
    }

    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn frame(
        ctx: &egui::Context,
        view: &mut RecoveryView,
        events: Vec<egui::Event>,
    ) -> (Option<Action>, Vec<String>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1120.0, 740.0),
            )),
            events,
            ..Default::default()
        };
        let mut action = None;
        let output = crate::tests::run_ui(ctx, input, |ui| {
            action = view.draw(ui.ctx(), &["HEAD".into(), "main".into()]);
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect();
        (action, text)
    }

    fn has(text: &[String], expected: &str) -> bool {
        text.iter().any(|line| line.contains(expected))
    }

    #[test]
    fn backups_show_time_branch_and_a_label_grid_without_refresh() -> Checked {
        let (dir, base, _next) = repository()?;
        let ctx = egui::Context::default();
        let mut view = RecoveryView::new(dir.path().into(), &ctx);
        view.wait();
        let _ = frame(&ctx, &mut view, vec![]);
        let (_, text) = frame(&ctx, &mut view, vec![]);
        for expected in [
            "Recovery",
            "Backups 1",
            "Reflog",
            "SAVED",
            "BRANCH",
            "just now",
            "main",
            "rebase",
            inspect_ui::short(&base),
            "TIP",
            "BRANCH NOW",
            "1 commit differs from the backup",
            "REF",
            "before rebase of main",
            "Delete backup…",
            "Open commit",
            "Create branch here…",
            "Restore main…",
            "1 backup",
        ] {
            assert!(has(&text, expected), "missing {expected}: {text:?}");
        }
        assert!(!has(&text, "Refresh"), "refresh is automatic: {text:?}");
        assert!(!has(&text, "Source"), "{text:?}");
        Ok(())
    }

    #[test]
    fn restore_is_one_confirmed_hard_reset_that_saves_the_tip_first() -> Checked {
        let (dir, base, next) = repository()?;
        let ctx = egui::Context::default();
        let mut view = RecoveryView::new(dir.path().into(), &ctx);
        view.wait();
        let _ = frame(&ctx, &mut view, vec![]);
        let (_, text) = frame(&ctx, &mut view, vec![key(egui::Key::Enter)]);
        assert!(view.confirm.is_some(), "Enter asks first: {text:?}");
        let (_, text) = frame(&ctx, &mut view, vec![]);
        for expected in [
            "Restore main",
            "Files are checked out to match the backup.",
            &format!(
                "The current tip {} is saved as a new backup, so this can be undone.",
                inspect_ui::short(&next)
            ),
            "Cancel",
            "Restore branch",
        ] {
            assert!(has(&text, expected), "missing {expected}: {text:?}");
        }
        assert_eq!(
            run(dir.path(), &["rev-parse", "HEAD"])?,
            next,
            "nothing moved yet"
        );
        let _ = frame(&ctx, &mut view, vec![key(egui::Key::Enter)]);
        let mut changed = false;
        let until = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while !changed && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(20));
            changed = frame(&ctx, &mut view, vec![]).0 == Some(Action::Changed);
        }
        assert!(changed, "the application re-reads after a restore");
        assert_eq!(run(dir.path(), &["rev-parse", "HEAD"])?, base);
        let backups = gg_git::backup::list(dir.path())?;
        assert_eq!(backups.len(), 2);
        assert_eq!(backups[0].label, "restore");
        assert_eq!(backups[0].commit, next, "the left tip is a backup");
        view.wait();
        let (_, text) = frame(&ctx, &mut view, vec![]);
        assert!(
            has(
                &text,
                &format!("Restored main to {}", inspect_ui::short(&base))
            ),
            "the result is in the status strip: {text:?}"
        );
        Ok(())
    }

    #[test]
    fn a_dirty_tree_keeps_restore_disabled_and_says_why() -> Checked {
        let (dir, _base, next) = repository()?;
        std::fs::write(dir.path().join("a"), b"edit\n")?;
        let ctx = egui::Context::default();
        let mut view = RecoveryView::new(dir.path().into(), &ctx);
        view.wait();
        let _ = frame(&ctx, &mut view, vec![]);
        let _ = frame(&ctx, &mut view, vec![key(egui::Key::Enter)]);
        let (_, text) = frame(&ctx, &mut view, vec![]);
        assert!(
            has(
                &text,
                "1 file has uncommitted changes. Commit or stash it first."
            ),
            "{text:?}"
        );
        let _ = frame(&ctx, &mut view, vec![key(egui::Key::Enter)]);
        assert!(view.job.is_none(), "Enter does nothing while disabled");
        assert!(view.confirm.is_some());
        assert_eq!(run(dir.path(), &["rev-parse", "HEAD"])?, next);
        let _ = frame(&ctx, &mut view, vec![key(egui::Key::Escape)]);
        assert!(view.confirm.is_none(), "Esc cancels");
        Ok(())
    }

    #[test]
    fn delete_backup_confirms_then_removes_only_that_ref() -> Checked {
        let (dir, _base, next) = repository()?;
        let ctx = egui::Context::default();
        let mut view = RecoveryView::new(dir.path().into(), &ctx);
        view.wait();
        let _ = frame(&ctx, &mut view, vec![]);
        let _ = frame(&ctx, &mut view, vec![key(egui::Key::Delete)]);
        let (_, text) = frame(&ctx, &mut view, vec![]);
        assert!(has(&text, "Delete the backup of main at"), "{text:?}");
        // Cancel has the keyboard: Enter keeps the backup.
        let _ = frame(&ctx, &mut view, vec![key(egui::Key::Enter)]);
        let (_, text) = frame(&ctx, &mut view, vec![]);
        assert!(!has(&text, "Delete the backup of main at"), "{text:?}");
        assert!(!gg_git::backup::list(dir.path())?.is_empty());
        // Opened again, Cancel has the keyboard again; Shift+Tab reaches
        // Delete backup, and Enter there deletes.
        let _ = frame(&ctx, &mut view, vec![key(egui::Key::Delete)]);
        let _ = frame(&ctx, &mut view, vec![]);
        let shift_tab = egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::SHIFT,
        };
        let _ = frame(&ctx, &mut view, vec![shift_tab]);
        let _ = frame(&ctx, &mut view, vec![key(egui::Key::Enter)]);
        view.wait();
        let (_, text) = frame(&ctx, &mut view, vec![]);
        assert!(gg_git::backup::list(dir.path())?.is_empty());
        assert_eq!(run(dir.path(), &["rev-parse", "HEAD"])?, next);
        for expected in [
            "No rewrite backups yet.",
            "Git Good saves one before every rebase, amend, rewrite or reset.",
        ] {
            assert!(has(&text, expected), "missing {expected}: {text:?}");
        }
        Ok(())
    }

    #[test]
    fn reflog_reads_in_plain_language_with_the_raw_message_below() -> Checked {
        let (dir, _base, next) = repository()?;
        let ctx = egui::Context::default();
        let mut view = RecoveryView::new(dir.path().into(), &ctx);
        view.select(RecoverySource::Reflog, "HEAD@{0}");
        view.wait();
        let _ = frame(&ctx, &mut view, vec![]);
        let (_, text) = frame(&ctx, &mut view, vec![]);
        for expected in [
            "WHEN",
            "ENTRY",
            "WHAT HAPPENED",
            "@{0}",
            "Committed \u{201c}next\u{201d}",
            "COMMIT",
            inspect_ui::short(&next),
            "GIT MESSAGE",
            "commit: next",
            "HEAD@{0}",
            "Restore main here…",
            "2 entries · HEAD",
        ] {
            assert!(has(&text, expected), "missing {expected}: {text:?}");
        }
        assert!(!has(&text, "Delete backup…"), "backups only: {text:?}");
        assert!(!has(&text, "Refresh"));
        let _ = frame(&ctx, &mut view, vec![key(egui::Key::ArrowDown)]);
        assert_eq!(
            view.selected_selector(),
            Some("HEAD@{1}"),
            "Down moves the selection"
        );
        view.set_reference("missing");
        view.wait();
        let (_, text) = frame(&ctx, &mut view, vec![]);
        assert!(view.entries().is_empty());
        assert!(!has(&text, "Committed"), "{text:?}");
        Ok(())
    }

    fn entry(action: &str, message: &str) -> Entry {
        Entry {
            selector: "HEAD@{0}".into(),
            id: "a".repeat(40),
            time: 0,
            action: action.into(),
            message: message.into(),
            subject: String::new(),
        }
    }

    #[test]
    fn reflog_events_read_in_plain_language() {
        for (action, message, expected) in [
            (
                "rebase (finish)",
                "returning to refs/heads/feature/rebase-ui",
                "Finished rebase of feature/rebase-ui",
            ),
            (
                "rebase (pick)",
                "Add footer notice",
                "Rebase picked \u{201c}Add footer notice\u{201d}",
            ),
            (
                "rebase (start)",
                "checkout main",
                "Started rebase onto main",
            ),
            (
                "commit",
                "Add review pane",
                "Committed \u{201c}Add review pane\u{201d}",
            ),
            (
                "checkout",
                "moving from main to feature/rebase-ui",
                "Switched from main to feature/rebase-ui",
            ),
            (
                "commit (amend)",
                "fix typo",
                "Amended \u{201c}fix typo\u{201d}",
            ),
            ("reset", "moving to HEAD~1", "Reset to HEAD~1"),
            ("merge feature", "Fast-forward", "Fast-forwarded to feature"),
            ("something new", "detail", "something new: detail"),
        ] {
            assert_eq!(plain(&entry(action, message)), expected);
        }
        assert_eq!(
            raw(&entry("rebase (pick)", "Add footer notice")),
            "rebase (pick): Add footer notice"
        );
    }
}
