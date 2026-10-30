//! Interactive rebase plan, listed newest first as in History.
//!
//! The plan is pure so its rules can be tested without a window. Squash and
//! fixup combine into the nearest kept commit **below** (older); the engine
//! receives the steps oldest first.

use gg_engine::rewrite::{Action, RangeCommit, Step};

/// Actions in menu order.
pub const ACTIONS: [Action; 6] = [
    Action::Pick,
    Action::Reword,
    Action::Edit,
    Action::Squash,
    Action::Fixup,
    Action::Drop,
];

pub fn label(action: Action) -> &'static str {
    match action {
        Action::Pick => "Pick",
        Action::Reword => "Reword",
        Action::Edit => "Reopen",
        Action::Squash => "Squash",
        Action::Fixup => "Fixup",
        Action::Drop => "Drop",
    }
}

/// Single-letter key on the selected row.
pub fn key(action: Action) -> char {
    match action {
        Action::Pick => 'P',
        Action::Reword => 'R',
        Action::Edit => 'E',
        Action::Squash => 'S',
        Action::Fixup => 'F',
        Action::Drop => 'D',
    }
}

pub fn hint(action: Action) -> &'static str {
    match action {
        Action::Pick => "Keep the commit as it is",
        Action::Reword => "Keep the changes and edit the message",
        Action::Edit => "Stop after this commit to amend or split it",
        Action::Squash => "Combine into the commit below and keep both messages",
        Action::Fixup => "Combine into the commit below and keep its message",
        Action::Drop => "Leave the commit out",
    }
}

const NO_TARGET: &str = "No kept commit below to combine into.";
/// Badge for a dropped row: the commit stays reachable from the backup.
const DROPPED: &str = "dropped · recoverable from the backup";
const LAST_KEPT: &str = "At least one commit must stay. Use Reset to remove them all.";

pub fn short(id: &str) -> &str {
    &id[..id.len().min(7)]
}

#[derive(Clone, Debug)]
pub struct Row {
    pub commit: RangeCommit,
    pub action: Action,
    /// The new message while the row is a reword.
    pub message: String,
    /// Branch and tag names pointing at the commit.
    pub refs: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Plan {
    /// Newest first.
    pub rows: Vec<Row>,
    original: Vec<String>,
}

impl Plan {
    /// Build a plan of picks from commits listed oldest first.
    pub fn new(commits: Vec<RangeCommit>, refs: impl Fn(&str) -> Vec<String>) -> Self {
        let rows = commits
            .into_iter()
            .rev()
            .map(|commit| Row {
                refs: refs(&commit.id),
                message: commit.message.clone(),
                commit,
                action: Action::Pick,
            })
            .collect::<Vec<Row>>();
        let original = rows.iter().map(|row| row.commit.id.clone()).collect();
        Self { rows, original }
    }

    /// The row a squash or fixup at `index` combines into: the nearest row
    /// below that is neither dropped nor itself combined.
    pub fn target(&self, index: usize) -> Option<usize> {
        (index + 1..self.rows.len()).find(|&below| {
            !matches!(
                self.rows[below].action,
                Action::Drop | Action::Squash | Action::Fixup
            )
        })
    }

    /// Rows above `index` that combine into it.
    pub fn absorbed(&self, index: usize) -> Vec<usize> {
        (0..index)
            .filter(|&above| {
                matches!(self.rows[above].action, Action::Squash | Action::Fixup)
                    && self.target(above) == Some(index)
            })
            .collect()
    }

    /// Why `action` cannot be chosen for the row at `index`.
    pub fn unavailable(&self, index: usize, action: Action) -> Option<&'static str> {
        match action {
            Action::Squash | Action::Fixup if self.target(index).is_none() => Some(NO_TARGET),
            Action::Drop
                if self
                    .rows
                    .iter()
                    .enumerate()
                    .all(|(other, row)| other == index || row.action == Action::Drop) =>
            {
                Some(LAST_KEPT)
            }
            _ => None,
        }
    }

    /// Choose `action` for the row at `index`, or say why it cannot be chosen.
    pub fn set_action(&mut self, index: usize, action: Action) -> Result<(), &'static str> {
        if let Some(reason) = self.unavailable(index, action) {
            return Err(reason);
        }
        let row = &mut self.rows[index];
        if action == Action::Reword && row.action != Action::Reword {
            row.message = row.commit.message.clone();
        }
        row.action = action;
        Ok(())
    }

    /// Move the row at `from` to `to`; the selection follows the row.
    pub fn move_row(&mut self, from: usize, to: usize) {
        if from < self.rows.len() && to < self.rows.len() && from != to {
            let row = self.rows.remove(from);
            self.rows.insert(to, row);
        }
    }

    /// The consequence shown beside the row, if any.
    pub fn badge(&self, index: usize) -> Option<String> {
        let row = &self.rows[index];
        let mut parts = Vec::new();
        match row.action {
            Action::Squash | Action::Fixup => {
                return Some(match self.target(index) {
                    Some(target) => format!(
                        "{} {} into {}",
                        crate::phosphor::ARROW_DOWN,
                        label(row.action).to_lowercase(),
                        short(&self.rows[target].commit.id)
                    ),
                    None => NO_TARGET.into(),
                });
            }
            Action::Drop => return Some(DROPPED.into()),
            Action::Edit => parts.push(format!(
                "{} stops here · reopens staged in Changes",
                crate::phosphor::PAUSE
            )),
            Action::Pick | Action::Reword => {}
        }
        let absorbed = self.absorbed(index);
        if !absorbed.is_empty() {
            parts.push(format!(
                "absorbs {}",
                absorbed
                    .iter()
                    .map(|&above| short(&self.rows[above].commit.id))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        (!parts.is_empty()).then(|| parts.join(" · "))
    }

    /// `6 commits → 4 · 1 dropped · 1 stop`, with a Phosphor arrow.
    pub fn summary(&self) -> String {
        let count = |action: Action| self.rows.iter().filter(|r| r.action == action).count();
        let total = self.rows.len();
        let result = self
            .rows
            .iter()
            .filter(|row| !matches!(row.action, Action::Drop | Action::Squash | Action::Fixup))
            .count();
        let mut text = format!(
            "{total} commit{} {} {result}",
            if total == 1 { "" } else { "s" },
            crate::phosphor::ARROW_RIGHT
        );
        let dropped = count(Action::Drop);
        if dropped > 0 {
            text.push_str(&format!(" · {dropped} dropped"));
        }
        let stops = count(Action::Edit);
        if stops > 0 {
            text.push_str(&format!(
                " · {stops} stop{}",
                if stops == 1 { "" } else { "s" }
            ));
        }
        text
    }

    /// Whether the plan differs from the current history.
    pub fn changed(&self) -> bool {
        self.rows.iter().any(|row| row.action != Action::Pick)
            || self
                .rows
                .iter()
                .map(|row| &row.commit.id)
                .ne(self.original.iter())
    }

    /// Engine steps, oldest first, or the first reason the plan cannot start.
    pub fn steps(&self) -> Result<Vec<Step>, String> {
        if !self.changed() {
            return Err("The plan matches the current history.".into());
        }
        if self.rows.iter().all(|row| row.action == Action::Drop) {
            return Err(LAST_KEPT.into());
        }
        let mut steps = Vec::with_capacity(self.rows.len());
        for (index, row) in self.rows.iter().enumerate().rev() {
            let id = short(&row.commit.id);
            match row.action {
                Action::Squash | Action::Fixup if self.target(index).is_none() => {
                    return Err(format!("{} {id}: {NO_TARGET}", label(row.action)));
                }
                Action::Reword if row.message.trim().is_empty() => {
                    return Err(format!("Reword {id} needs a message."));
                }
                Action::Reword => steps.push(Step::reword(&row.commit.id, row.message.clone())),
                action => steps.push(Step::new(action, &row.commit.id)),
            }
        }
        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(id: &str) -> RangeCommit {
        RangeCommit {
            id: id.repeat(40),
            subject: format!("subject {id}"),
            author: "Test".into(),
            date: "now".into(),
            message: format!("subject {id}"),
        }
    }

    /// Oldest first: a, b, c, d; shown newest first: d, c, b, a.
    fn plan() -> Plan {
        Plan::new(["a", "b", "c", "d"].map(commit).to_vec(), |id| {
            if id.starts_with('d') {
                vec!["feature".into()]
            } else {
                vec![]
            }
        })
    }

    fn ids(steps: &[Step]) -> Vec<(Action, char)> {
        steps
            .iter()
            .map(|step| (step.action, step.commit.chars().next().unwrap_or('?')))
            .collect()
    }

    #[test]
    fn lists_newest_first_and_starts_unchanged() {
        let plan = plan();
        let shown = plan
            .rows
            .iter()
            .map(|row| short(&row.commit.id).to_string())
            .collect::<Vec<_>>();
        assert_eq!(shown, ["ddddddd", "ccccccc", "bbbbbbb", "aaaaaaa"]);
        assert_eq!(plan.rows[0].refs, ["feature"]);
        assert!(!plan.changed());
        assert_eq!(
            plan.steps(),
            Err("The plan matches the current history.".into())
        );
        assert_eq!(
            plan.summary(),
            format!("4 commits {} 4", crate::phosphor::ARROW_RIGHT)
        );
    }

    #[test]
    fn squash_and_fixup_combine_into_the_nearest_kept_commit_below() -> Result<(), String> {
        let mut plan = plan();
        // d fixup, c squash: both combine into b; a is dropped.
        plan.set_action(3, Action::Drop)?;
        plan.set_action(1, Action::Squash)?;
        plan.set_action(0, Action::Fixup)?;
        assert_eq!(plan.target(0), Some(2));
        assert_eq!(plan.target(1), Some(2));
        assert_eq!(
            plan.badge(0).as_deref(),
            Some(format!("{} fixup into bbbbbbb", crate::phosphor::ARROW_DOWN).as_str())
        );
        assert_eq!(plan.badge(2).as_deref(), Some("absorbs ddddddd, ccccccc"));
        assert_eq!(plan.badge(3).as_deref(), Some(DROPPED));
        assert_eq!(
            plan.summary(),
            format!("4 commits {} 1 · 1 dropped", crate::phosphor::ARROW_RIGHT)
        );
        assert_eq!(
            ids(&plan.steps()?),
            [
                (Action::Drop, 'a'),
                (Action::Pick, 'b'),
                (Action::Squash, 'c'),
                (Action::Fixup, 'd'),
            ]
        );
        Ok(())
    }

    #[test]
    fn the_oldest_kept_commit_cannot_be_combined() -> Result<(), String> {
        let mut plan = plan();
        assert_eq!(plan.unavailable(3, Action::Squash), Some(NO_TARGET));
        assert_eq!(plan.set_action(3, Action::Fixup), Err(NO_TARGET));
        plan.set_action(2, Action::Squash)?;
        // Dropping the commit it combines into leaves the squash without a target.
        plan.set_action(3, Action::Drop)?;
        assert_eq!(plan.badge(2).as_deref(), Some(NO_TARGET));
        let error = plan.steps().err().unwrap_or_default();
        assert!(error.contains("No kept commit below"), "{error}");
        Ok(())
    }

    #[test]
    fn reword_starts_from_the_original_message_and_needs_text() -> Result<(), String> {
        let mut plan = plan();
        plan.set_action(1, Action::Reword)?;
        assert_eq!(plan.rows[1].message, "subject c");
        plan.rows[1].message = "  ".into();
        assert_eq!(plan.steps(), Err("Reword ccccccc needs a message.".into()));
        plan.rows[1].message = "new c".into();
        let steps = plan.steps()?;
        assert_eq!(steps[2].action, Action::Reword);
        assert_eq!(steps[2].message.as_deref(), Some("new c"));
        Ok(())
    }

    #[test]
    fn reopen_stops_and_the_last_kept_commit_cannot_be_dropped() -> Result<(), String> {
        let mut plan = plan();
        plan.set_action(1, Action::Edit)?;
        assert_eq!(
            plan.badge(1).as_deref(),
            Some(
                format!(
                    "{} stops here · reopens staged in Changes",
                    crate::phosphor::PAUSE
                )
                .as_str()
            )
        );
        assert_eq!(
            plan.summary(),
            format!("4 commits {} 4 · 1 stop", crate::phosphor::ARROW_RIGHT)
        );
        plan.set_action(0, Action::Drop)?;
        plan.set_action(1, Action::Drop)?;
        plan.set_action(2, Action::Drop)?;
        assert_eq!(plan.unavailable(3, Action::Drop), Some(LAST_KEPT));
        assert_eq!(plan.set_action(3, Action::Drop), Err(LAST_KEPT));
        Ok(())
    }

    #[test]
    fn moving_rows_reorders_the_steps() -> Result<(), String> {
        let mut plan = plan();
        plan.move_row(0, 3);
        assert!(plan.changed());
        assert_eq!(
            ids(&plan.steps()?),
            [
                (Action::Pick, 'd'),
                (Action::Pick, 'a'),
                (Action::Pick, 'b'),
                (Action::Pick, 'c'),
            ]
        );
        plan.move_row(3, 0);
        assert!(!plan.changed());
        Ok(())
    }

    #[test]
    fn every_action_has_a_distinct_key() {
        let keys = ACTIONS.map(key);
        assert_eq!(keys, ['P', 'R', 'E', 'S', 'F', 'D']);
    }
}
