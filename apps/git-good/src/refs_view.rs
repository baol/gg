//! Branch, remote, and tag navigation.
use eframe::egui;
use gg_engine::refs::{RefItem, RefKind, Snapshot};
use std::collections::BTreeMap;

const ICON_SIZE: f32 = 12.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Switch(String),
    /// Switch to a remote branch (`origin/feature`): the local branch of
    /// that name, or a new one tracking it (as the branch switcher does).
    SwitchRemote(String),
    Jump(String),
    Merge(String),
    Rebase(String),
    DeleteBranch(String),
    DeleteTag(String),
    VerifyTag(String),
    /// Open the Stashes picker on the entry at this index.
    OpenStash(usize),
    /// Open another worktree of this repository.
    OpenWorktree(String),
    /// A worktree row's right-click choice, on the worktree at this path.
    Worktree(String, WorktreeChoice),
    /// Rename a local branch (dialog).
    Rename(String),
    /// Set a local branch's upstream (dialog).
    SetUpstream(String),
    /// New branch at a ref's commit (dialog): commit and ref name.
    NewBranchAt(String, String),
    /// Compare a ref's commit with HEAD.
    Compare(String),
    /// Push the current branch.
    Push,
    /// Delete a remote branch on its remote (confirmed): its short name
    /// (`origin/feature`) and last fetched commit.
    DeleteRemote(String, String),
}

/// A choice in a worktree row's right-click menu (WTR-17).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorktreeChoice {
    Reveal,
    /// Remove… asks first, in the Worktrees window.
    Remove,
}

/// Ref names split at `/`: each node may be a ref (its index in the
/// snapshot) and a folder of longer names.
#[derive(Default)]
struct Tree {
    item: Option<usize>,
    children: BTreeMap<String, Tree>,
}

impl Tree {
    fn insert(&mut self, item: usize, name: &str) {
        let mut node = self;
        for segment in name.split('/').filter(|segment| !segment.is_empty()) {
            node = node.children.entry(segment.to_string()).or_default();
        }
        node.item = Some(item);
    }
}

thread_local! {
    static CURSOR: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Outline this ref's row (full name) in the next [`show`]: the keyboard
/// cursor of the refs drawer.
pub fn set_cursor(full_name: Option<String>) {
    CURSOR.with(|cursor| *cursor.borrow_mut() = full_name);
}

/// The rows Up/Down move over in the refs drawer, in list order: local
/// branches, then with a filter also matching remotes and tags (their
/// sections open while filtering).
pub fn cursor_rows(refs: &Snapshot, filter: &str) -> Vec<RefItem> {
    let query = filter.trim().to_lowercase();
    let kinds: &[RefKind] = if query.is_empty() {
        &[RefKind::Branch]
    } else {
        &[RefKind::Branch, RefKind::Remote, RefKind::Tag]
    };
    kinds
        .iter()
        .flat_map(|kind| {
            let mut rows = refs
                .items
                .iter()
                .filter(|item| item.kind == *kind)
                .filter(|item| query.is_empty() || item.name.to_lowercase().contains(&query))
                .cloned()
                .collect::<Vec<_>>();
            // The unfiltered branch tree lists names segment by segment.
            if query.is_empty() {
                rows.sort_by(|a, b| a.name.split('/').cmp(b.name.split('/')));
            }
            rows
        })
        .collect()
}

/// The Filter refs field, focused when the refs drawer opens.
pub fn filter_id() -> egui::Id {
    egui::Id::unique("refs-filter")
}

/// One line of the sidebar list. The list is built once per change of
/// refs, filter, stashes, worktrees or open groups and folders, then drawn
/// a screenful at a time.
#[derive(Clone, Debug, PartialEq)]
enum Line {
    /// Space before a group heading.
    Space(f32),
    /// A group heading: `BRANCHES 2`, or `▸ 25` when collapsed.
    Header {
        title: &'static str,
        count: usize,
        open: bool,
        filtering: bool,
    },
    /// Weak text: a detached HEAD, an unborn branch, no matches.
    Note(String),
    /// A ref folder (`feature/`), open or closed.
    Folder {
        key: String,
        label: String,
        depth: usize,
        open: bool,
    },
    /// A ref: its index in the snapshot and the label it shows.
    Ref {
        item: usize,
        label: String,
        depth: usize,
    },
    /// A stash entry by index.
    Stash(usize),
    /// A worktree by index.
    Worktree(usize),
}

impl Line {
    /// The height a line takes, the 1 px gap after it included.
    fn height(&self) -> f32 {
        match self {
            Line::Space(space) => *space,
            Line::Header { .. } => 18.0 + 1.0,
            _ => crate::density::row() + 1.0,
        }
    }
}

/// What the list was built from: the same inputs give the same lines.
#[derive(Clone)]
struct Key {
    refs: std::sync::Arc<Snapshot>,
    query: String,
    stashes: Vec<(String, String)>,
    worktrees: Vec<(String, String, bool)>,
    sections: [bool; 5],
    closed: std::collections::BTreeSet<String>,
    row: f32,
}

impl PartialEq for Key {
    /// The refs by identity, not content: comparing thousands of refs each
    /// frame is the cost the list saves. The history stream shares one
    /// snapshot across its pages; a new read is a new snapshot.
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.refs, &other.refs)
            && self.query == other.query
            && self.stashes == other.stashes
            && self.worktrees == other.worktrees
            && self.sections == other.sections
            && self.closed == other.closed
            && self.row == other.row
    }
}

/// The built list: its lines, where each starts, and the tracking counts.
struct Built {
    key: Key,
    lines: Vec<Line>,
    tops: Vec<f32>,
    counts: BTreeMap<String, String>,
}

const SECTIONS: [&str; 5] = ["Branches", "Remotes", "Tags", "Stashes", "Worktrees"];

fn section_state(ctx: &egui::Context, title: &str) -> egui::collapsing_header::CollapsingState {
    egui::collapsing_header::CollapsingState::load_with_default_open(
        ctx,
        egui::Id::unique(("refs-section", title)),
        !matches!(title, "Remotes" | "Tags" | "Worktrees"),
    )
}

fn closed_id() -> egui::Id {
    egui::Id::unique("refs-closed-folders")
}

fn built_id() -> egui::Id {
    egui::Id::unique("refs-built-lines")
}

pub fn show(
    ui: &mut egui::Ui,
    refs: &std::sync::Arc<Snapshot>,
    stashes: &[gg_engine::stash::StashEntry],
    worktrees: &[gg_engine::worktree::Worktree],
    filter: &mut String,
    busy: bool,
    lanes: &BTreeMap<String, u8>,
) -> Option<Action> {
    let mut action = None;
    ui.add_space(4.0);
    ui.add(
        egui::TextEdit::singleline(filter)
            .id(filter_id())
            .hint_text("Filter refs")
            .desired_width(f32::INFINITY),
    );
    ui.add_space(2.0);

    let ctx = ui.ctx().clone();
    let key = Key {
        refs: refs.clone(),
        query: filter.trim().to_lowercase(),
        stashes: stashes
            .iter()
            .map(|entry| (entry.reference.clone(), entry.message.clone()))
            .collect(),
        worktrees: worktrees
            .iter()
            .map(|worktree| {
                (
                    worktree.path.clone(),
                    worktree.branch_label().to_owned(),
                    worktree.current,
                )
            })
            .collect(),
        sections: SECTIONS.map(|title| section_state(&ctx, title).is_open()),
        closed: ctx.data(|data| data.get_temp(closed_id()).unwrap_or_default()),
        row: crate::density::row(),
    };
    let built: std::sync::Arc<Built> = match ctx
        .data(|data| data.get_temp::<std::sync::Arc<Built>>(built_id()))
        .filter(|built| built.key == key)
    {
        Some(built) => built,
        None => {
            let built = std::sync::Arc::new(build(key, stashes, worktrees));
            ctx.data_mut(|data| data.insert_temp(built_id(), built.clone()));
            built
        }
    };
    let head_name = refs.head.as_ref().and_then(|head| head.name.as_deref());
    let cursor = CURSOR.with(|cursor| cursor.borrow().clone());
    let total = built.tops.last().copied().unwrap_or(0.0);
    egui::ScrollArea::vertical()
        .id_salt("refs-scroll")
        .auto_shrink([false, false])
        .show_viewport(ui, |ui, viewport| {
            ui.set_height(total);
            let origin = ui.max_rect().min;
            let width = ui.available_width();
            // The keyboard cursor's row is brought into view even when it
            // is not drawn yet.
            if let Some(cursor) = &cursor
                && let Some(index) = built.lines.iter().position(|line| {
                    matches!(line, Line::Ref { item, .. }
                        if refs.items.get(*item).is_some_and(|item| item.full_name == *cursor))
                })
            {
                let top = built.tops[index];
                let rect = egui::Rect::from_min_size(
                    origin + egui::vec2(0.0, top),
                    egui::vec2(width, built.lines[index].height()),
                );
                let bottom = top + built.lines[index].height();
                if top < viewport.min.y || bottom > viewport.max.y {
                    // A screen or more away: jump, as an animation through
                    // thousands of rows would take many frames.
                    let far = top + viewport.height() < viewport.min.y
                        || bottom > viewport.max.y + viewport.height();
                    if far {
                        ui.scroll_to_rect_animation(
                            rect,
                            Some(egui::Align::Center),
                            egui::style::ScrollAnimation::none(),
                        );
                    } else {
                        ui.scroll_to_rect(rect, None);
                    }
                }
            }
            let first = built
                .tops
                .partition_point(|top| *top <= viewport.min.y)
                .saturating_sub(1);
            for (index, line) in built.lines.iter().enumerate().skip(first) {
                let top = built.tops[index];
                if top > viewport.max.y {
                    break;
                }
                let indent = match line {
                    Line::Folder { depth, .. } | Line::Ref { depth, .. } => {
                        *depth as f32 * ui.spacing().indent
                    }
                    _ => 0.0,
                };
                let rect = egui::Rect::from_min_size(
                    origin + egui::vec2(indent, top),
                    egui::vec2(width - indent, line.height() - 1.0),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    draw_line(
                        ui,
                        line,
                        refs,
                        stashes,
                        worktrees,
                        head_name,
                        busy,
                        lanes,
                        &built.counts,
                        &mut action,
                    );
                });
            }
        });
    action
}

/// The lines for `key`: the groups in order, each open one with its rows.
fn build(
    key: Key,
    stashes: &[gg_engine::stash::StashEntry],
    worktrees: &[gg_engine::worktree::Worktree],
) -> Built {
    let refs = &key.refs;
    let query = key.query.as_str();
    let filtering = !query.is_empty();
    let matching = |kind: RefKind| {
        refs.items
            .iter()
            .enumerate()
            .filter(move |(_, item)| item.kind == kind)
            .filter(|(_, item)| query.is_empty() || item.name.to_lowercase().contains(query))
            .collect::<Vec<_>>()
    };
    let branches = matching(RefKind::Branch);
    let remotes = matching(RefKind::Remote);
    let tags = matching(RefKind::Tag);
    let empty = branches.is_empty() && remotes.is_empty() && tags.is_empty();
    let stash_rows: Vec<usize> = stashes
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            query.is_empty()
                || entry.reference.to_lowercase().contains(query)
                || entry.message.to_lowercase().contains(query)
        })
        .map(|(index, _)| index)
        .collect();
    // Linked worktrees, as in the History mockup; collapsed until opened,
    // and only listed when there is more than the main one.
    let worktree_rows: Vec<usize> = worktrees
        .iter()
        .enumerate()
        .filter(|(_, worktree)| {
            query.is_empty()
                || worktree.path.to_lowercase().contains(query)
                || worktree.branch_label().to_lowercase().contains(query)
        })
        .map(|(index, _)| index)
        .collect();
    // `↑1 ↓3` beside each branch with an upstream, as in the mockup.
    let counts = refs
        .tracking
        .iter()
        .filter_map(|tracking| {
            let parts = [
                tracking
                    .ahead
                    .filter(|n| *n > 0)
                    .map(|n| format!("{}{n}", crate::phosphor::ARROW_UP)),
                tracking
                    .behind
                    .filter(|n| *n > 0)
                    .map(|n| format!("{}{n}", crate::phosphor::ARROW_DOWN)),
            ];
            let text = parts.into_iter().flatten().collect::<Vec<_>>().join(" ");
            (!text.is_empty()).then(|| (tracking.full_name.clone(), text))
        })
        .collect::<BTreeMap<_, _>>();

    let mut lines = Vec::new();
    if filtering && empty && stash_rows.is_empty() && worktree_rows.is_empty() {
        lines.push(Line::Space(6.0));
        lines.push(Line::Note("No matching refs".into()));
    } else {
        let header = |lines: &mut Vec<Line>, index: usize, count: usize| -> bool {
            let title = SECTIONS[index];
            let open = filtering || key.sections[index];
            lines.push(Line::Space(if index == 0 { 2.0 } else { 6.0 }));
            lines.push(Line::Header {
                title,
                count,
                open,
                filtering,
            });
            open
        };
        let head = refs.head.as_ref();
        if header(&mut lines, 0, branches.len()) {
            if let Some(head) = head
                && head.detached
            {
                let commit = head
                    .commit
                    .as_deref()
                    .map(|id| id.chars().take(8).collect::<String>())
                    .unwrap_or_else(|| "unborn".into());
                lines.push(Line::Note(format!("Detached at {commit}")));
            }
            if branches.is_empty() {
                // No filler lines (proposal): an unborn branch is listed as
                // having no commits, so it agrees with the branch button; a
                // filter says it matched nothing.
                if filtering {
                    lines.push(Line::Note("No matching branches".into()));
                } else if let Some(name) = head.and_then(|head| head.name.as_deref())
                    && head.is_some_and(|head| head.commit.is_none())
                {
                    lines.push(Line::Note(format!(
                        "{}  · no commits",
                        name.strip_prefix("refs/heads/").unwrap_or(name)
                    )));
                }
            } else {
                ref_lines(&mut lines, &branches, "branch", filtering, &key.closed);
            }
        }
        for (index, rows, kind, none) in [
            (1, &remotes, "remote", "No matching remotes"),
            (2, &tags, "tag", "No matching tags"),
        ] {
            if header(&mut lines, index, rows.len()) {
                if rows.is_empty() {
                    if filtering {
                        lines.push(Line::Note(none.into()));
                    }
                } else {
                    ref_lines(&mut lines, rows, kind, filtering, &key.closed);
                }
            }
        }
        if header(&mut lines, 3, stash_rows.len()) {
            if stash_rows.is_empty() && filtering {
                lines.push(Line::Note("No matching stashes".into()));
            }
            lines.extend(stash_rows.iter().map(|index| Line::Stash(*index)));
        }
        if worktrees.len() > 1 && header(&mut lines, 4, worktree_rows.len()) {
            lines.extend(worktree_rows.iter().map(|index| Line::Worktree(*index)));
        }
        if !filtering && empty {
            lines.push(Line::Space(6.0));
            lines.push(Line::Note(
                "Branches and tags appear after the first commit.".into(),
            ));
        }
    }
    let mut tops = Vec::with_capacity(lines.len() + 1);
    let mut top = 0.0;
    for line in &lines {
        tops.push(top);
        top += line.height();
    }
    tops.push(top);
    Built {
        key,
        lines,
        tops,
        counts,
    }
}

/// The rows of one ref group: a folder tree of name segments, or flat full
/// names while filtering.
fn ref_lines(
    lines: &mut Vec<Line>,
    rows: &[(usize, &RefItem)],
    section: &str,
    filtering: bool,
    closed: &std::collections::BTreeSet<String>,
) {
    if filtering {
        lines.extend(rows.iter().map(|(index, item)| Line::Ref {
            item: *index,
            label: item.name.clone(),
            depth: 0,
        }));
        return;
    }
    let mut tree = Tree::default();
    for (index, item) in rows {
        tree.insert(*index, &item.name);
    }
    tree_lines(lines, &tree, section, "", 0, closed);
}

fn tree_lines(
    lines: &mut Vec<Line>,
    tree: &Tree,
    section: &str,
    parent: &str,
    depth: usize,
    closed: &std::collections::BTreeSet<String>,
) {
    for (segment, node) in &tree.children {
        let path = format!("{parent}/{segment}");
        if node.children.is_empty() {
            if let Some(item) = node.item {
                lines.push(Line::Ref {
                    item,
                    label: segment.clone(),
                    depth,
                });
            }
            continue;
        }
        let key = format!("{section}{path}");
        let open = !closed.contains(&key);
        lines.push(Line::Folder {
            key,
            label: segment.clone(),
            depth,
            open,
        });
        if open {
            if let Some(item) = node.item {
                lines.push(Line::Ref {
                    item,
                    label: segment.clone(),
                    depth: depth + 1,
                });
            }
            tree_lines(lines, node, section, &path, depth + 1, closed);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_line(
    ui: &mut egui::Ui,
    line: &Line,
    refs: &Snapshot,
    stashes: &[gg_engine::stash::StashEntry],
    worktrees: &[gg_engine::worktree::Worktree],
    head_name: Option<&str>,
    busy: bool,
    lanes: &BTreeMap<String, u8>,
    counts: &BTreeMap<String, String>,
    action: &mut Option<Action>,
) {
    match line {
        Line::Space(_) => {}
        Line::Header {
            title,
            count,
            open,
            filtering,
        } => section_header(ui, title, *count, *open, *filtering),
        Line::Note(text) => {
            ui.weak(text);
        }
        Line::Folder {
            key, label, open, ..
        } => folder(ui, key, label, *open),
        Line::Ref { item, label, .. } => {
            if let Some(item) = refs.items.get(*item) {
                render_leaf(ui, item, label, head_name, busy, lanes, counts, action);
            }
        }
        Line::Stash(index) => {
            if let Some(entry) = stashes.get(*index) {
                stash_row(ui, *index, entry, action);
            }
        }
        Line::Worktree(index) => {
            if let Some(worktree) = worktrees.get(*index) {
                worktree_row(ui, worktree, action);
            }
        }
    }
}

/// A group heading like the mockup: `BRANCHES 2`. Branches and Stashes
/// start open; Remotes, Tags, and Worktrees start collapsed. Every group
/// opens while the filter has text, so matches are never hidden.
fn section_header(ui: &mut egui::Ui, title: &str, count: usize, open: bool, filtering: bool) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::click());
    let weak = ui.visuals().weak_text_color();
    let font = egui::FontId::proportional(11.0);
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title.to_uppercase(),
        font.clone(),
        weak,
    );
    // Open groups show their count; collapsed ones `▸ 25`, as in the mockup.
    ui.painter().text(
        egui::pos2(rect.right() - 4.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        if open {
            count.to_string()
        } else {
            format!("{} {count}", crate::phosphor::CARET_RIGHT)
        },
        font,
        weak,
    );
    let response = response
        .on_hover_text(if open { "Collapse" } else { "Expand" })
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Button,
            ui.is_enabled(),
            format!("{title} {count}"),
        )
    });
    if response.clicked() && !filtering {
        let mut state = section_state(ui.ctx(), title);
        state.toggle(ui);
        state.store(ui.ctx());
    }
}

/// A ref folder: a caret and the weak segment name; a click opens or
/// closes it (folders start open).
fn folder(ui: &mut egui::Ui, key: &str, label: &str, open: bool) {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), crate::density::row()),
        egui::Sense::click(),
    );
    let weak = ui.visuals().weak_text_color();
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 5.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        if open {
            crate::phosphor::CARET_DOWN
        } else {
            crate::phosphor::CARET_RIGHT
        },
        egui::FontId::new(11.0, egui::FontFamily::Name("phosphor-regular".into())),
        weak,
    );
    crate::bounded_text::paint(
        ui,
        egui::Rect::from_min_max(
            egui::pos2(rect.left() + 18.0, rect.top()),
            egui::pos2(rect.right() - 4.0, rect.bottom()),
        ),
        label,
        egui::FontId::proportional(12.0),
        weak,
    );
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), format!("{label}/"))
    });
    if response.clicked() {
        ui.ctx().data_mut(|data| {
            let closed =
                data.get_temp_mut_or_default::<std::collections::BTreeSet<String>>(closed_id());
            if !closed.remove(key) {
                closed.insert(key.to_owned());
            }
        });
    }
}

fn stash_row(
    ui: &mut egui::Ui,
    index: usize,
    entry: &gg_engine::stash::StashEntry,
    action: &mut Option<Action>,
) {
    let short = entry
        .reference
        .strip_prefix("stash")
        .unwrap_or(&entry.reference);
    let response = ui
        .add(
            egui::Button::new((
                egui::RichText::new(short).monospace().weak(),
                egui::RichText::new(&entry.message),
            ))
            .frame_when_inactive(false)
            .truncate()
            .min_size(egui::vec2(ui.available_width(), crate::density::row())),
        )
        .on_hover_text(format!(
            "{}\n{}\nOpen in the stash picker to apply, pop, show, or drop",
            entry.reference, entry.message
        ));
    if response.clicked() {
        *action = Some(Action::OpenStash(index));
    }
}

fn worktree_row(
    ui: &mut egui::Ui,
    worktree: &gg_engine::worktree::Worktree,
    action: &mut Option<Action>,
) {
    let folder = std::path::Path::new(&worktree.path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| worktree.path.clone());
    let branch = worktree
        .branch
        .as_deref()
        .map(|name| name.strip_prefix("refs/heads/").unwrap_or(name))
        .unwrap_or(worktree.branch_label());
    let mut text = egui::text::LayoutJob::default();
    text.append(&folder, 0.0, egui::TextFormat::default());
    text.append(
        &format!("  {branch}{}", if worktree.current { "  ✓" } else { "" }),
        0.0,
        egui::TextFormat {
            color: ui.visuals().weak_text_color(),
            ..Default::default()
        },
    );
    let response = ui
        .add(
            egui::Button::new(text)
                .frame_when_inactive(false)
                .truncate()
                .min_size(egui::vec2(ui.available_width(), crate::density::row())),
        )
        .on_hover_text(if worktree.current {
            format!("{}\nThe open worktree", worktree.path)
        } else {
            format!("{}\nOpen this worktree", worktree.path)
        });
    if response.clicked() && !worktree.current {
        *action = Some(Action::OpenWorktree(worktree.path.clone()));
    }
    response.context_menu(|ui| {
        use crate::menus::row;
        ui.set_min_width(200.0);
        let path = worktree.path.clone();
        if ui
            .add_enabled(!worktree.current, row(false, "Open"))
            .on_disabled_hover_text("This worktree is open")
            .clicked()
        {
            *action = Some(Action::OpenWorktree(path.clone()));
            ui.close();
        }
        if ui.add(row(false, "Reveal in folder")).clicked() {
            *action = Some(Action::Worktree(path.clone(), WorktreeChoice::Reveal));
            ui.close();
        }
        if ui.add(row(false, "Copy path")).clicked() {
            ui.ctx().copy_text(path.clone());
            ui.close();
        }
        ui.separator();
        let reason = worktree.can_remove().err();
        let label = if reason.is_none() {
            egui::RichText::new("Remove…").color(ui.visuals().error_fg_color)
        } else {
            egui::RichText::new("Remove…")
        };
        if ui
            .add_enabled(reason.is_none(), row(false, label))
            .on_disabled_hover_text(reason.unwrap_or_default())
            .clicked()
        {
            *action = Some(Action::Worktree(path, WorktreeChoice::Remove));
            ui.close();
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn render_leaf(
    ui: &mut egui::Ui,
    item: &RefItem,
    label: &str,
    head_name: Option<&str>,
    busy: bool,
    lanes: &BTreeMap<String, u8>,
    counts: &BTreeMap<String, String>,
    action: &mut Option<Action>,
) {
    let current = Some(item.full_name.as_str()) == head_name;
    let lane = (item.kind == RefKind::Branch)
        .then(|| {
            lanes
                .get(&item.full_name)
                .map(|lane| crate::graph::lane_color(*lane))
        })
        .flatten();
    let inspect = format!("Inspect {}\n{}", item.full_name, short(&item.commit));
    let hover = if current {
        format!(
            "{} · current branch\nClick to inspect {}",
            item.full_name,
            short(&item.commit)
        )
    } else if matches!(item.kind, RefKind::Branch | RefKind::Remote) {
        format!("{inspect}\nDouble-click to switch\nRight-click for ref actions")
    } else {
        format!("{inspect}\nRight-click for ref actions")
    };
    let count = counts.get(&item.full_name).map(String::as_str);
    let response = row(ui, label, item.kind, current, lane, count).on_hover_text(hover);
    if CURSOR.with(|cursor| cursor.borrow().as_deref() == Some(item.full_name.as_str())) {
        ui.painter().rect_stroke(
            response.rect.shrink(0.5),
            5.0,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
        response.scroll_to_me(None);
    }
    if response.double_clicked() {
        if !current && !busy {
            match item.kind {
                RefKind::Branch => *action = Some(Action::Switch(item.name.clone())),
                RefKind::Remote => *action = Some(Action::SwitchRemote(item.name.clone())),
                _ => {}
            }
        }
    } else if response.clicked() {
        *action = Some(Action::Jump(item.commit.clone()));
    }
    // The ref's menu (branch menus proposal): the shared 24 px menu rows; a
    // label ends in "…" when it opens a dialog or confirmation.
    response.context_menu(|ui| {
        ui.set_min_width(230.0);
        let mut entry = |ui: &mut egui::Ui,
                         label: String,
                         key: &'static str,
                         enabled: bool,
                         reason: &str,
                         danger: bool,
                         chosen: Action| {
            let text = if danger {
                egui::RichText::new(label).color(ui.visuals().error_fg_color)
            } else {
                egui::RichText::new(label)
            };
            let response = ui
                .add_enabled(
                    enabled,
                    crate::menus::row(false, text).shortcut_text(crate::keymap::shown(key)),
                )
                .on_disabled_hover_text(reason);
            if response.clicked() {
                *action = Some(chosen);
                ui.close();
            }
        };
        let wait = "Wait for the current operation to finish.";
        let here = "Already on this branch";
        let idle = !busy;
        match item.kind {
            RefKind::Branch => {
                let reason = if current { here } else { wait };
                entry(
                    ui,
                    format!("Switch to {}", item.name),
                    "",
                    !current && idle,
                    reason,
                    false,
                    Action::Switch(item.name.clone()),
                );
                entry(
                    ui,
                    "Merge into current…".into(),
                    "",
                    !current && idle,
                    reason,
                    false,
                    Action::Merge(item.name.clone()),
                );
                entry(
                    ui,
                    "Rebase current onto this…".into(),
                    "",
                    !current && idle,
                    reason,
                    false,
                    Action::Rebase(item.name.clone()),
                );
                entry(
                    ui,
                    "Compare with HEAD".into(),
                    "",
                    !current,
                    here,
                    false,
                    Action::Compare(item.commit.clone()),
                );
                ui.separator();
                entry(
                    ui,
                    "New branch here…".into(),
                    "",
                    idle,
                    wait,
                    false,
                    Action::NewBranchAt(item.commit.clone(), item.name.clone()),
                );
                entry(
                    ui,
                    "Push".into(),
                    "Ctrl+Shift+P",
                    current && idle,
                    if current {
                        wait
                    } else {
                        "Switch to the branch to push it"
                    },
                    false,
                    Action::Push,
                );
                entry(
                    ui,
                    "Set upstream…".into(),
                    "",
                    idle,
                    wait,
                    false,
                    Action::SetUpstream(item.name.clone()),
                );
                entry(
                    ui,
                    "Rename…".into(),
                    "F2",
                    idle,
                    wait,
                    false,
                    Action::Rename(item.name.clone()),
                );
            }
            RefKind::Remote => {
                // One step, as double-click and the switcher: the local
                // branch of that name, or a new one tracking this.
                entry(
                    ui,
                    format!(
                        "Switch to {}",
                        item.name
                            .split_once('/')
                            .map_or(item.name.as_str(), |(_, local)| local)
                    ),
                    "",
                    idle,
                    wait,
                    false,
                    Action::SwitchRemote(item.name.clone()),
                );
                entry(
                    ui,
                    "Merge into current…".into(),
                    "",
                    idle,
                    wait,
                    false,
                    Action::Merge(item.name.clone()),
                );
                entry(
                    ui,
                    "Rebase current onto this…".into(),
                    "",
                    idle,
                    wait,
                    false,
                    Action::Rebase(item.name.clone()),
                );
                entry(
                    ui,
                    "Compare with HEAD".into(),
                    "",
                    true,
                    "",
                    false,
                    Action::Compare(item.commit.clone()),
                );
                entry(
                    ui,
                    "New branch here…".into(),
                    "",
                    idle,
                    wait,
                    false,
                    Action::NewBranchAt(item.commit.clone(), item.name.clone()),
                );
            }
            RefKind::Tag => {
                entry(
                    ui,
                    "Merge into current…".into(),
                    "",
                    idle,
                    wait,
                    false,
                    Action::Merge(item.name.clone()),
                );
                entry(
                    ui,
                    "Rebase current onto this…".into(),
                    "",
                    idle,
                    wait,
                    false,
                    Action::Rebase(item.name.clone()),
                );
                entry(
                    ui,
                    "Compare with HEAD".into(),
                    "",
                    true,
                    "",
                    false,
                    Action::Compare(item.commit.clone()),
                );
                entry(
                    ui,
                    "New branch here…".into(),
                    "",
                    idle,
                    wait,
                    false,
                    Action::NewBranchAt(item.commit.clone(), item.name.clone()),
                );
                entry(
                    ui,
                    "Signature details…".into(),
                    "",
                    idle,
                    wait,
                    false,
                    Action::VerifyTag(item.full_name.clone()),
                );
            }
        }
        ui.separator();
        if ui.add(crate::menus::row(false, "Copy name")).clicked() {
            ui.ctx().copy_text(item.name.clone());
            ui.close();
        }
        if ui.add(crate::menus::row(false, "Copy ID")).clicked() {
            ui.ctx().copy_text(item.commit.clone());
            ui.close();
        }
        match item.kind {
            RefKind::Branch => {
                ui.separator();
                entry(
                    ui,
                    "Delete branch…".into(),
                    "Del",
                    !current && idle,
                    if current { here } else { wait },
                    true,
                    Action::DeleteBranch(item.name.clone()),
                );
            }
            RefKind::Tag => {
                ui.separator();
                entry(
                    ui,
                    "Delete tag…".into(),
                    "",
                    idle,
                    wait,
                    true,
                    Action::DeleteTag(item.name.clone()),
                );
            }
            RefKind::Remote => {
                ui.separator();
                let remote = item
                    .name
                    .split_once('/')
                    .map_or("the remote", |(remote, _)| remote);
                entry(
                    ui,
                    format!("Delete on {remote}…"),
                    "",
                    idle,
                    wait,
                    true,
                    Action::DeleteRemote(item.name.clone(), item.commit.clone()),
                );
            }
        }
    });
}

fn row(
    ui: &mut egui::Ui,
    label: &str,
    kind: RefKind,
    current: bool,
    lane: Option<egui::Color32>,
    count: Option<&str>,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), crate::density::row()),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Button,
            ui.is_enabled(),
            if current {
                format!("{label}, current branch")
            } else {
                label.to_owned()
            },
        )
    });
    if current {
        ui.painter().rect_filled(
            rect,
            5.0,
            ui.visuals().selection.bg_fill.gamma_multiply(0.32),
        );
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, 5.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    let color = if current {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().text_color()
    };
    let icon = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 9.0, rect.center().y),
        egui::vec2(ICON_SIZE, ICON_SIZE),
    );
    // Branches get their graph lane's dot, as in the mockup; remotes and
    // tags keep their glyph.
    if kind == RefKind::Branch {
        let dot = lane.unwrap_or_else(|| ui.visuals().weak_text_color());
        ui.painter().circle_filled(icon.center(), 3.5, dot);
    } else {
        paint_icon(ui.painter(), icon, kind, color.gamma_multiply(0.9));
    }
    let mut text_right = rect.right() - if current { 20.0 } else { 6.0 };
    if let Some(count) = count {
        let galley = ui.painter().layout_no_wrap(
            count.to_owned(),
            egui::FontId::proportional(11.0),
            ui.visuals().weak_text_color(),
        );
        let at = egui::pos2(
            text_right - galley.size().x,
            rect.center().y - galley.size().y / 2.0,
        );
        text_right = at.x - 6.0;
        ui.painter()
            .galley(at, galley, ui.visuals().weak_text_color());
    }
    let text_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 19.0, rect.top()),
        egui::pos2(text_right, rect.bottom()),
    );
    let font = egui::FontId::proportional(12.0);
    if current {
        crate::bounded_text::paint_bold(ui, text_rect, label, font, color);
    } else {
        crate::bounded_text::paint(ui, text_rect, label, font, color);
    }
    if current {
        let check = egui::pos2(rect.right() - 10.0, rect.center().y);
        let stroke = egui::Stroke::new(1.4_f32, color);
        ui.painter().line_segment(
            [check + egui::vec2(-3.0, 0.0), check + egui::vec2(0.0, 3.0)],
            stroke,
        );
        ui.painter().line_segment(
            [check + egui::vec2(0.0, 3.0), check + egui::vec2(4.0, -4.0)],
            stroke,
        );
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(crate) fn paint_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    kind: RefKind,
    color: egui::Color32,
) {
    let p = |x: f32, y: f32| egui::pos2(rect.left() + x, rect.top() + y);
    let stroke = egui::Stroke::new(1.15_f32, color);
    match kind {
        RefKind::Branch => {
            painter.circle_filled(p(3.5, 3.0), 1.5, color);
            painter.circle_filled(p(3.5, 9.0), 1.5, color);
            painter.circle_filled(p(8.5, 4.0), 1.5, color);
            painter.line_segment([p(3.5, 4.0), p(3.5, 8.0)], stroke);
            painter.line_segment([p(3.5, 6.0), p(8.5, 5.0)], stroke);
        }
        RefKind::Remote => {
            painter.circle_stroke(rect.center(), 4.2, stroke);
            painter.line_segment(
                [
                    rect.center() - egui::vec2(4.2, 0.0),
                    rect.center() + egui::vec2(4.2, 0.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    rect.center() - egui::vec2(0.0, 4.2),
                    rect.center() + egui::vec2(0.0, 4.2),
                ],
                stroke,
            );
        }
        RefKind::Tag => {
            let points = vec![
                p(2.0, 3.0),
                p(8.0, 3.0),
                p(10.5, 5.5),
                p(5.5, 10.5),
                p(2.0, 7.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                points,
                egui::Color32::TRANSPARENT,
                stroke,
            ));
            painter.circle_filled(p(4.5, 5.0), 1.0, color);
        }
    }
}

fn short(id: &str) -> String {
    id.chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        let commit = "a".repeat(40);
        Snapshot {
            head: Some(gg_engine::refs::Head {
                name: Some("refs/heads/main".into()),
                detached: false,
                commit: Some(commit.clone()),
            }),
            items: vec![
                RefItem {
                    name: "main".into(),
                    full_name: "refs/heads/main".into(),
                    kind: RefKind::Branch,
                    commit: commit.clone(),
                },
                RefItem {
                    name: "feature/nested".into(),
                    full_name: "refs/heads/feature/nested".into(),
                    kind: RefKind::Branch,
                    commit: commit.clone(),
                },
                RefItem {
                    name: "origin/main".into(),
                    full_name: "refs/remotes/origin/main".into(),
                    kind: RefKind::Remote,
                    commit: commit.clone(),
                },
                RefItem {
                    name: "v1.0".into(),
                    full_name: "refs/tags/v1.0".into(),
                    kind: RefKind::Tag,
                    commit: commit.clone(),
                },
            ],
            remotes: vec![gg_engine::refs::Remote {
                name: "origin".into(),
                fetch_url: Some("https://example.org/gg".into()),
                push_url: None,
            }],
            tracking: vec![gg_engine::refs::Tracking {
                branch: "main".into(),
                full_name: "refs/heads/main".into(),
                upstream: "origin/main".into(),
                upstream_full_name: "refs/remotes/origin/main".into(),
                remote: "origin".into(),
                remote_branch: "main".into(),
                ahead: Some(2),
                behind: Some(1),
            }],
        }
    }

    /// 10,000 branches in 100 folders.
    fn many_branches() -> std::sync::Arc<Snapshot> {
        let commit = "b".repeat(40);
        let items = (0..10_000)
            .map(|n| RefItem {
                name: format!("team{:02}/topic{n:05}", n / 100),
                full_name: format!("refs/heads/team{:02}/topic{n:05}", n / 100),
                kind: RefKind::Branch,
                commit: commit.clone(),
            })
            .collect();
        std::sync::Arc::new(Snapshot {
            head: None,
            items,
            remotes: Vec::new(),
            tracking: Vec::new(),
        })
    }

    fn frame(
        ctx: &egui::Context,
        refs: &std::sync::Arc<Snapshot>,
        events: Vec<egui::Event>,
    ) -> (egui::FullOutput, Option<Action>) {
        let mut filter = String::new();
        let mut action = None;
        let output = crate::tests::run_ui(
            ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(240.0, 600.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    action = show(ui, refs, &[], &[], &mut filter, false, &BTreeMap::new());
                });
            },
        );
        (output, action)
    }

    fn texts(output: &egui::FullOutput) -> Vec<(String, egui::Pos2)> {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some((text.galley.job.text.clone(), text.pos)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn ten_thousand_branches_draw_only_the_rows_in_view_from_one_list() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let refs = many_branches();
        let (output, _) = frame(&ctx, &refs, vec![]);
        let shown = texts(&output);
        assert!(shown.iter().any(|(text, _)| text == "topic00000"));
        assert!(
            shown.len() < 80,
            "only a screenful is laid out: {} texts",
            shown.len()
        );
        let built = |ctx: &egui::Context| {
            ctx.data(|data| data.get_temp::<std::sync::Arc<Built>>(built_id()))
        };
        let first = built(&ctx).ok_or("no list").map_err(|e| e.to_string());
        let _ = frame(&ctx, &refs, vec![]);
        let second = built(&ctx).ok_or("no list").map_err(|e| e.to_string());
        assert!(
            matches!((&first, &second), (Ok(a), Ok(b)) if std::sync::Arc::ptr_eq(a, b)),
            "the list is not built again while nothing changed"
        );
        // 100 folders and 10,000 rows, plus the headings.
        let lines = second.map(|built| built.lines.len()).unwrap_or(0);
        assert!(lines > 10_100, "{lines}");
        // New refs build it again.
        let _ = frame(&ctx, &many_branches(), vec![]);
        let third = built(&ctx).ok_or("no list").map_err(|e| e.to_string());
        assert!(!matches!((&first, &third), (Ok(a), Ok(b)) if std::sync::Arc::ptr_eq(a, b)));
    }

    #[test]
    fn a_folder_closes_and_opens_on_click() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let refs = many_branches();
        let click = |at: egui::Pos2| {
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        let (output, _) = frame(&ctx, &refs, vec![]);
        let folder = texts(&output)
            .into_iter()
            .find(|(text, _)| text == "team00")
            .map(|(_, at)| at + egui::vec2(4.0, 6.0));
        let Some(folder) = folder else {
            unreachable!("the first folder is drawn")
        };
        let _ = frame(&ctx, &refs, click(folder));
        let (output, _) = frame(&ctx, &refs, vec![]);
        let shown = texts(&output);
        assert!(
            !shown.iter().any(|(text, _)| text == "topic00000"),
            "closed"
        );
        assert!(
            shown.iter().any(|(text, _)| text == "team01"),
            "the next folder moves up"
        );
        let _ = frame(&ctx, &refs, click(folder));
        let (output, _) = frame(&ctx, &refs, vec![]);
        assert!(
            texts(&output).iter().any(|(text, _)| text == "topic00000"),
            "open again"
        );
    }

    #[test]
    fn the_keyboard_cursor_scrolls_a_far_branch_into_view() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let refs = many_branches();
        set_cursor(Some("refs/heads/team90/topic09042".into()));
        let mut found = false;
        for _ in 0..4 {
            let (output, _) = frame(&ctx, &refs, vec![]);
            if texts(&output).iter().any(|(text, _)| text == "topic09042") {
                found = true;
                break;
            }
        }
        set_cursor(None);
        assert!(found, "the cursor's row is drawn after scrolling to it");
    }

    #[test]
    fn worktrees_are_listed_when_linked_ones_exist_and_open_on_click() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let refs = std::sync::Arc::new(snapshot());
        let worktree = |path: &str, branch: &str, current: bool| gg_engine::worktree::Worktree {
            path: path.into(),
            path_bytes: path.as_bytes().to_vec(),
            head: Some("a".repeat(40)),
            branch: Some(format!("refs/heads/{branch}")),
            detached: false,
            bare: false,
            locked: None,
            prunable: None,
            current,
            main: current,
            dirty: Some(false),
        };
        let worktrees = [
            worktree("/src/gg", "main", true),
            worktree("/src/gg-feature", "feature", false),
        ];
        // The filter opens every group, so the rows are drawn.
        let mut filter = "gg".to_string();
        let mut action = None;
        let mut draw = |events: Vec<egui::Event>| {
            crate::tests::run_ui(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(260.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let chosen = show(
                            ui,
                            &refs,
                            &[],
                            &worktrees,
                            &mut filter,
                            false,
                            &BTreeMap::new(),
                        );
                        action = action.clone().or(chosen);
                    });
                },
            )
        };
        let output = draw(vec![]);
        let row = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.job.text.starts_with("gg-feature") => {
                Some(text.pos + egui::vec2(4.0, 4.0))
            }
            _ => None,
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            text.iter().any(|line| line.as_str() == "WORKTREES"),
            "{text:?}"
        );
        let Some(row) = row else {
            unreachable!("the linked worktree row is drawn: {text:?}");
        };
        let button = |pressed| egui::Event::PointerButton {
            pos: row,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        draw(vec![egui::Event::PointerMoved(row), button(true)]);
        draw(vec![button(false)]);
        assert_eq!(action, Some(Action::OpenWorktree("/src/gg-feature".into())));
    }

    #[test]
    fn shows_grouped_sections_and_current_branch() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let refs = std::sync::Arc::new(snapshot());
        let mut filter = String::new();
        let output = crate::tests::run_ui(
            &ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(190.0, 480.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let action = show(ui, &refs, &[], &[], &mut filter, false, &BTreeMap::new());
                    assert!(action.is_none());
                });
            },
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for expected in [
            "BRANCHES",
            "main",
            "REMOTES",
            "TAGS",
            "\u{E13A} 1",
            "2",
            "\u{E08E}2 \u{E03E}1",
        ] {
            assert!(
                text.iter().any(|line| line.contains(expected)),
                "{expected}: {text:?}"
            );
        }
        assert!(text.iter().any(|line| line.contains("feature")));
    }

    #[test]
    fn stash_entries_open_the_picker_and_follow_the_filter() {
        let ctx = egui::Context::default();
        let refs = std::sync::Arc::new(snapshot());
        let stashes = (0..2)
            .map(|index| gg_engine::stash::StashEntry {
                index,
                reference: format!("stash@{{{index}}}"),
                commit: format!("{index:040x}"),
                message: format!("WIP on main: work {index}"),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        let frame = |filter: &mut String, events: Vec<egui::Event>| {
            let mut action = None;
            let output = crate::tests::run_ui(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(220.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        action = show(ui, &refs, &stashes, &[], filter, false, &BTreeMap::new());
                    });
                },
            );
            (output, action)
        };
        let mut filter = String::new();
        let (output, _) = frame(&mut filter, vec![]);
        let row = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.job.text == "WIP on main: work 1" => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        });
        let Some(row) = row else {
            unreachable!("stash entry is listed")
        };
        let click = vec![
            egui::Event::PointerMoved(row),
            egui::Event::PointerButton {
                pos: row,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: row,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let (_, action) = frame(&mut filter, click);
        assert_eq!(action, Some(Action::OpenStash(1)));

        let mut filter = "work 0".to_string();
        let (output, _) = frame(&mut filter, vec![]);
        let texts = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(texts.iter().any(|text| text == "WIP on main: work 0"));
        assert!(!texts.iter().any(|text| text == "WIP on main: work 1"));
    }

    #[test]
    fn a_right_click_on_a_worktree_reveals_or_removes_it() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let refs = std::sync::Arc::new(snapshot());
        let worktree = |path: &str, branch: &str, current: bool| gg_engine::worktree::Worktree {
            path: path.into(),
            path_bytes: path.as_bytes().to_vec(),
            head: Some("a".repeat(40)),
            branch: Some(format!("refs/heads/{branch}")),
            detached: false,
            bare: false,
            locked: None,
            prunable: None,
            current,
            main: current,
            dirty: Some(false),
        };
        let worktrees = [
            worktree("/src/gg", "main", true),
            worktree("/src/gg-feature", "feature", false),
        ];
        let draw = |events: Vec<egui::Event>| {
            let mut filter = "gg".to_string();
            let mut action = None;
            let output = crate::tests::run_ui(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(260.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        action = show(
                            ui,
                            &refs,
                            &[],
                            &worktrees,
                            &mut filter,
                            false,
                            &BTreeMap::new(),
                        );
                    });
                },
            );
            (output, action)
        };
        let at = |output: &egui::FullOutput, wanted: &dyn Fn(&str) -> bool| {
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) if wanted(&text.galley.job.text) => {
                        Some(text.visual_bounding_rect().center())
                    }
                    _ => None,
                })
                .next_back()
        };
        let button = |button, pos, pressed| egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        for (item, choice) in [
            ("Reveal in folder", WorktreeChoice::Reveal),
            ("Remove…", WorktreeChoice::Remove),
        ] {
            let (output, _) = draw(vec![]);
            let Some(row) = at(&output, &|text| text.starts_with("gg-feature")) else {
                unreachable!("the linked worktree row is drawn")
            };
            let right = egui::PointerButton::Secondary;
            let _ = draw(vec![
                egui::Event::PointerMoved(row),
                button(right, row, true),
            ]);
            let _ = draw(vec![button(right, row, false)]);
            let (output, _) = draw(vec![]);
            let Some(item_at) = at(&output, &|text| text == item) else {
                unreachable!("{item} is in the menu")
            };
            let left = egui::PointerButton::Primary;
            let _ = draw(vec![
                egui::Event::PointerMoved(item_at),
                button(left, item_at, true),
            ]);
            let (_, action) = draw(vec![button(left, item_at, false)]);
            assert_eq!(
                action,
                Some(Action::Worktree("/src/gg-feature".into(), choice)),
                "{item}"
            );
        }
    }

    #[test]
    fn filter_flattens_and_clears_unmatched_groups() {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let refs = std::sync::Arc::new(snapshot());
        let mut filter: String = "nomatch".into();
        let output = crate::tests::run_ui(
            &ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(190.0, 480.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, &refs, &[], &[], &mut filter, false, &BTreeMap::new());
                });
            },
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(text.iter().any(|line| line.contains("No matching refs")));
        assert!(!text.iter().any(|line| line.contains("Branches  1")));
    }

    fn click_label(query: &str, label: &str, clicks: usize) -> Option<Action> {
        let ctx = egui::Context::default();
        crate::phosphor::install(&ctx);
        let refs = std::sync::Arc::new(snapshot());
        let mut filter = query.to_owned();
        let mut action = None;
        let mut run = |events: Vec<egui::Event>, time: f64| {
            crate::tests::run_ui(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(190.0, 480.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        action = show(ui, &refs, &[], &[], &mut filter, false, &BTreeMap::new());
                    });
                },
            )
        };
        let output = run(vec![], 1.0);
        let position = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text)
                if text.galley.job.text == label && text.pos.y > 40.0 =>
            {
                Some(text.pos + egui::vec2(8.0, text.galley.size().y / 2.0))
            }
            _ => None,
        })?;
        for index in 0..clicks {
            let time = 1.1 + index as f64 * 0.2;
            run(
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                time,
            );
            run(
                vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                time + 0.05,
            );
        }
        action
    }

    #[test]
    fn clicking_a_branch_row_inspects_its_commit() {
        let commit = "a".repeat(40);
        assert_eq!(
            click_label("feature", "feature/nested", 1),
            Some(Action::Jump(commit))
        );
    }

    #[test]
    fn double_clicking_a_branch_row_switches() {
        assert_eq!(
            click_label("feature", "feature/nested", 2),
            Some(Action::Switch("feature/nested".into()))
        );
        assert_ne!(
            click_label("mai", "main", 2),
            Some(Action::Switch("main".into()))
        );
    }

    #[test]
    fn clicking_the_current_branch_row_inspects_it_without_switching() {
        assert_eq!(
            click_label("mai", "main", 1),
            Some(Action::Jump("a".repeat(40)))
        );
    }

    #[test]
    fn clicking_a_remote_row_inspects_its_commit() {
        assert_eq!(
            click_label("origin", "origin/main", 1),
            Some(Action::Jump("a".repeat(40)))
        );
    }

    #[test]
    fn double_clicking_a_remote_row_switches_to_it() {
        // Like the switcher: a local branch of the same name, or track it.
        assert_eq!(
            click_label("origin", "origin/main", 2),
            Some(Action::SwitchRemote("origin/main".into()))
        );
    }
}
