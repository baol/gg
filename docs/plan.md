# Plan

Purpose: sequence M5 delivery and acceptance.

## M5 · Merge, conflicts, cherry-pick, revert, stash, and recovery

Goal: integrate, undo, and temporarily park commits with conflict-aware views and recoverable in-progress operations, without weakening read-only browsing or existing mutations.

Contracts: [history mutations and recovery](architecture.md#history-mutations-and-recovery--m5). M4 is complete; [evidence](verification.md#m4-clone-fetch-push-tracking-and-credentials--2026-10-22).

### Increments

1. **Stash · complete.** Add `git good stash <list|push|apply|pop|drop>` with shared entries and receipts, native **Stash changes…** and **Manage stashes…** actions, untracked and keep-index options, conflict reporting, and drop confirmation.
2. **Cherry-pick and revert · complete.** Apply a single commit or invert it with `git good cherry-pick|revert COMMIT`, detect conflicts, continue/skip/abort the in-progress operation, and expose the same actions from the **Commit** menu.
3. **Merge · complete.** Merge a local branch, remote-tracking branch, or tag with fast-forward, up-to-date, and merge-commit results, `--no-ff`, conflict detection, and abort/continue support.
4. **Conflict resolution · complete.** Show unmerged paths with base/ours/theirs panes, select a side or stage manually resolved content, and continue or abort the merge, cherry-pick, or revert.
5. **Recovery · complete.** Detect an in-progress merge, cherry-pick, or revert; expose **Continue**, **Abort**, and **Skip** where Git supports it; and restore the pre-operation selection and status after recovery.
6. **Native operation state · complete.** Show the in-progress operation in the toolbar and status line, serialize conflicting actions, refresh refs/history/status at safe boundaries, and report hook, conflict, and abort failures without deleting user files.

M5 is complete. Stash, cherry-pick, revert, merge, conflict resolution, recovery, and operation-state handling are implemented and verified.

## Context menus

Goal: expose commit and ref actions through right-click and keyboard context menus.

### Increments

1. **Commit-row menu · in progress.** Add **Cherry-pick**, **Revert**, **Reset here** (Soft, Mixed, Hard with confirmation), **Copy hash**, and **Copy subject** to history rows, with right-click access.
2. **Compare to local · complete.** Compare the selected commit against the current branch tip in a review diff, exposed from the commit-row menu and `git good show --against REF`.
3. **Ref menus · complete.** Add checkout/switch, merge, delete branch (safe, then confirmed force), delete tag, track remote, and copy actions to branch, tag, and remote items.

### Acceptance

| Scenario | Expected result | Method |
| --- | --- | --- |
| Commit menu | Right-click or keyboard opens the commit actions; copy writes the exact hash/subject | egui and native review |
| Reset here | Soft, Mixed, and Hard reset match Git; Hard warns before discarding changes | Git oracle, CLI, and native review |
| Compare to local | Selected commit is compared with the current branch tip | Git oracle and native review |
| Ref menus | Branch/tag/remote actions match Git; destructive actions confirm | Git oracle, CLI, and native review |

### Acceptance

| Scenario | Expected result | Method |
| --- | --- | --- |
| Stash list | Entries match `git stash list` order, reference, commit, and message; empty stashes are explicit | Git fixtures and CLI |
| Stash push | Modified, staged, and untracked changes are recorded per selected options; `--keep-index` leaves staged content in place; clean trees are rejected | Git oracle and CLI |
| Stash apply and pop | Changes are restored; `pop` removes the entry only when the apply succeeds; `--index` restores staged state | Git oracle and native review |
| Stash conflicts | Conflicting paths are listed, apply keeps the entry, pop keeps the entry, and the worktree retains the conflict markers | Git oracle, CLI, and native review |
| Stash drop | The selected entry is removed after confirmation; working files are untouched | Git fixtures and native review |
| Cherry-pick and revert | A selected commit is applied or inverted with the expected parent and tree, or reports conflicts and leaves the operation recoverable | Git oracle and CLI |
| Merge | Fast-forward and merge-commit results match Git; conflicts and non-fast-forward cases are explicit | Git oracle and CLI |
| Conflict resolution | Unmerged paths show base/ours/theirs content, staging marks a path resolved, and the operation continues or aborts | Git fixtures and native review |
| Recovery | An in-progress operation is detected; continue, skip, and abort match Git and restore the pre-operation state on abort | Git fixtures and native review |
| Read-only safety | Browsing, refs, and tracking reads leave objects, refs, index, and working files unchanged | Byte/reference comparison |

Record native wide/narrow captures and verification in [verification](verification.md).
