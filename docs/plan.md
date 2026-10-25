# Plan

Purpose: sequence milestone delivery and acceptance.

## M7 · File reading, history, attribution, and large repositories

Goal: make the selected file a first-class reading surface, then follow a file or line through history before adding the remaining large-repository features. Reading stays read-only, bounded, cancellable, and independent of the history selection.

Contracts: [file reading and history](architecture.md#file-reading-and-history--m7).

### Increments

1. **File reading surface · complete.** Add a **Diff / Full file** toggle that keeps the selected file, Before/After side, and reading position. Show exact line numbers and changed-line markers, select a line or range with the pointer or keyboard, move to previous/next change with `[`/`]` and visible buttons, and move between files with `Alt+Up/Down` and **Prev file**/**Next file**. Read one side on the existing cancellable selected-file worker, bounded to 2 MiB and 100,000 lines, with explicit absent, binary, gitlink, symlink, and budget states. Keep the status line and help text in step with the bindings.
2. **File history · complete.** Follow the selected file through a worker-backed cursor over content versions: skip unchanged commits, follow renames, and name the change kind, commit, subject, author, date, and old path. `Before`/`After` move one version and disable at the boundaries with a visible reason instead of wrapping. A unique predecessor is followed silently; ambiguous predecessors show a chooser and never default to the first parent. **Full file** shows the current version, **Diff** shows the previous-to-current edge, and the Old/New inspection sides stay separate from the version control. Preload a bounded number of versions and page older ones; `H` opens history and `Esc` backs out one level.
3. **Blame and line/range history · planned.** Attribute each line to a commit and open the same version timeline anchored to `path bytes + revision + start_line..end_line`. Map a single line through blame and range boundaries through diff hunks, and ask which predecessor to follow when a range spans divergent hunks. Deleted lines anchor the before side explicitly instead of dropping the place. `B` opens blame.
4. **Image diffs · planned.** Compare images with explicit dimensions, fit modes, and a side-by-side or swipe presentation while keeping the bounded preview and export rules.
5. **Worktrees · planned.** List, create, open, and remove worktrees without disturbing user files; keep each worktree's drafts and selection separate.
6. **Submodules · planned.** Show submodule status, referenced commits, and explicit update actions without silently changing the parent repository.
7. **Git LFS · planned.** Identify LFS pointers, show their object identity, and keep fetch/smudge actions explicit.
8. **Commit and tag signing · planned.** Verify signatures where the repository provides them and keep signing configuration explicit.

### Acceptance

| Scenario | Expected result | Method |
| --- | --- | --- |
| Full-file reading | Diff and Full file keep the same file, side, and reading position; full mode shows exact line numbers and changed-line markers; added/deleted files open the side that exists | egui interaction, Git fixture, and native review |
| Bounded reads | One side is read on the selected-file worker, cancelled by selection or mode changes, and bounded to 2 MiB and 100,000 lines; binary, gitlink, symlink, absent, and over-budget sides are explicit | Worker tests, Git fixtures, and native review |
| Line place | Clicking or moving the cursor sets a line; Shift extends a range; the place carries exact path bytes, revision, and start/end lines | egui interaction and engine tests |
| Change navigation | `[`/`]` and the visible buttons move between changed blocks in full mode and hunks in diff mode without changing files; the position and total are named, and both directions disable at their boundaries and after the only change | egui interaction and native review |
| File navigation | `Alt+Up/Down` and **Prev file**/**Next file** change the selected file while staying in reading mode and preserving an approximate line position | egui interaction and native review |
| File history | The timeline is a cursor over content versions, skips unchanged commits, follows renames, and names the old path; Before/After disable with a visible reason at the boundaries and never wrap; an ambiguous predecessor opens a chooser and never silently takes the first parent; Full shows the current version and Diff the previous-to-current edge | Git oracle, egui interaction, and native review |
| Line history | A line/range timeline keeps the place through renames and anchors deleted ranges to the before side | Git oracle, egui interaction, and native review |
| Read-only safety | Reading, full-file loads, and history navigation never write objects, refs, the index, or working files | Repository snapshot comparison |

Record native wide/narrow captures and verification in [verification](verification.md).

## UI-flow consolidation · pre-M7

Goal: improve continuity, explicit decisions, and recovery feedback in the existing native workflows before adding M7 features.

Contracts: [working status, committing, and amend](architecture.md#committing).

### Increments

1. **Commit drafts · complete.** Retain normal and amend commit text and the selected file when leaving Local Changes for history, review, or a refresh; restore them for the same canonical worktree, clear the submitted draft only after a successful commit, retain it on hook failure and conflict, and never transfer drafts between worktrees.
2. **Merge, rebase, and recovery workers · complete.** Run merge, rebase, and continue/skip/abort on one background operation worker; close the dialog immediately, name the pending operation in the status line, disable duplicate or incompatible mutation actions, and refresh mutation state, history, refs, status, reflog, and backups once the result is accepted.
3. **Conflict workflow · complete.** Name the operation, unresolved count, and operation commit above the conflict content; explain Ours/Theirs per operation, especially rebase; state that whole-file choices replace and stage while **Stage resolved** stages external edits; disable Continue while unresolved paths remain; run conflict listing, content reads, and side resolution on workers; select the next unresolved path after resolution; and use side tabs with bounded scrolling at narrow widths.
4. **Comparison direction and scope · complete.** Rename the commit-row action to **Compare with current HEAD** and show a persistent Before/After header with the requested ref name and resolved commit IDs. Label whole-file staging actions **Stage file**, **Unstage file**, and **Discard file**, keep hunk and selected-line actions distinct, use **Discard** consistently, and state the exact file/scope plus unchanged index in the confirmation.
5. **Commit readiness and amend mode · complete.** Derive readiness from the completed, validated status scan: a normal commit requires a nonempty message and at least one staged entry, message-only amend stays valid, and disabled states name the reason. The amend banner names the affected commit and subject above the commit action. The message editor expands and collapses without permanently displacing the file list.
6. **Responsive staging · complete.** At narrow widths, show a **Staged**/**Unstaged** switch with change counts, focus the nonempty pane when the other is empty, and offer **Split** to stack both comparisons. Keep both panes discoverable and preserve each pane's selection and scroll across focus and layout changes.
7. **Action discoverability · complete.** Add a visible **Actions** menu and **Commit actions** window for the selected commit, sharing the commit-row context-menu definitions and grouped as Inspect, Integrate, Rewrite and destructive, and Recovery. Use target-aware labels, explain disabled mutation commands, add the Alt+A shortcut, and add a **Help** keyboard-shortcut window.
8. **Readable and actionable outcomes · complete.** Give operation results a severity, concise summary, optional full details, and at most one recovery action. Keep the fixed-height status line, add **Details…** and context recovery buttons such as **Resolve conflicts…**, **Backups…**, **Retry push**, **Retry fetch**, and **Manage stashes…**, and keep errors ahead of success notices.

### Acceptance

| Scenario | Expected result | Method |
| --- | --- | --- |
| Commit drafts | A multiline draft and selected file survive history/review/refresh; hook failure and conflict retain the text; a successful commit clears only that draft; another worktree starts empty | egui interaction, Git fixture, and native review |
| Operation responsiveness | Merge, rebase, and recovery start without blocking frames or navigation; duplicate submission starts one operation; success, failure, conflict, and worker disconnection leave actionable state and refresh affected views once | egui interaction, Git fixture, and native review |
| Conflict workflow | Operation-aware side identities and staging consequences are explicit; Continue waits for zero unresolved paths; conflict listing, content, and resolution are worker-backed; the next unresolved path is selected after resolution; narrow widths use side tabs and bounded scrolling | egui interaction, Git fixture, and native review |
| Comparison direction and scope | Compare with current HEAD names the Before ref and resolved commit; parent, root, and merge comparisons name their exact base and target; Stage file, Unstage file, and Discard file never imply repository scope; Discard confirms the exact file and scope and states the index is unchanged | egui interaction, Git oracle, and native review |
| Commit readiness and amend mode | Commit enables only after the scan with a nonempty message and staged files; message-only amend stays valid; disabled states explain why; the amend banner names the affected commit and subject; the message editor expands and collapses | egui interaction, Git fixture, and native review |
| Responsive staging | Narrow widths switch between Staged and Unstaged with counts, focus the nonempty pane by default, and can stack both panes with Split; both panes stay discoverable and preserve selection and scroll across layout changes | egui geometry, Git fixture, and native review |
| Action discoverability | A visible Actions menu and Commit actions window share the context-menu definitions, group inspect/integrate/rewrite/recovery commands, use target-aware labels, explain disabled commands, open with Alt+A, and provide a Help shortcut window; keyboard-only users can inspect, compare, switch a branch from Refs, and reach recovery | egui interaction, Git fixture, and native review |
| Readable outcomes | Merge, rebase, recovery, rewrite, push, fetch, and stash outcomes keep a concise status summary with optional selectable Details and one recovery action; errors outrank success notices; a remaining conflict or worker failure stays visible | egui interaction, Git fixture, and native review |

Implementation and verification: [commit drafts](verification.md#pre-m7-commit-draft-retention--2026-10-23), [operation workers](verification.md#pre-m7-operation-workers--2026-10-23).

## M6 · Rebase, reorder, squash, amend, and reflog recovery

Goal: rewrite history deliberately and recover previous tips without weakening read-only browsing, clean-worktree checks, or existing mutations. Every rewrite records a private backup ref before publication.

Contracts: [history rewrites and recovery](architecture.md#history-rewrites-and-recovery--m6), [M5 recovery](architecture.md#history-mutations-and-recovery--m5). M5 is complete; [evidence](verification.md#m5-recovery-and-operation-state--2026-10-22).

### Increments

1. **Amend and Local Changes · complete.** Remove the stand-alone **Commit** menu and **Status** button. Add **Local Changes** and **All Commits** to the left navigation sidebar, keep the toolbar and sidebar visible while Local Changes is open, and keep cherry-pick/revert/reset in the commit-row context menu. Add `git good commit --amend -m MESSAGE` and an **Amend** toggle in the Local Changes commit bar. Amend the latest commit with the staged index and an editable message, allow a message-only amend, preserve the original author, run commit hooks plus `post-rewrite`, and save the previous tip under a private backup ref. Amend mode compares the index with HEAD's parent so the staged rows are the commit's actual contents: staging adds working changes, while unstaging a file, hunk, or line removes that change from the amended commit without touching working files.
2. **Reflog · complete.** Add `git good reflog [REF]`, a **Repository → Reflog…** window showing `HEAD` by default with a local-branch selector, confirmed Soft/Mixed/Hard reset to an entry, **Create branch here…** without switching, and **Copy hash**. Read-only listing never changes refs, the index, or working files.
3. **Rebase onto · complete.** Rebase the current branch onto a local branch, remote-tracking ref, or tag through Git with a clean-worktree check, an explicit autostash decision, conflict recovery, continue/skip/abort, and a backup ref. Ref menus open a confirmed rebase dialog, and the existing recovery banner reports in-progress rebases.
4. **Reorder and squash · complete.** Add `git good rewrite --from COMMIT --pick COMMIT... --squash COMMIT...` and a commit-row **Reorder and squash to HEAD…** modal. Generate a rebase todo from the ordered steps and run it through Git with a non-interactive sequence editor. Run the rewrite on a background worker, keep the modal open with rewriting/refreshing progress, and close it only after the replacement history is loaded. Require a linear range without merge commits, include every range commit exactly once, keep the first step a pick, preserve combined squash messages, support root ranges, use the same clean/autostash and backup rules as rebase, report conflicts through the recovery banner, and refresh history and the pre-rewrite selection after completion, skip, or abort.
5. **Rewrite recovery · complete.** List private `refs/gg/backup/...` refs without adding them to the refs sidebar, expose **Repository → Backups…** and a banner **Backups…** action, and offer Open commit, Reset branch here, Create branch here…, Copy hash, and Copy ref. Surface in-progress rebase state through the existing banner, restore the pre-rewrite selection after completion or abort, and document amend, rebase, reorder, squash, and backup recovery paths.

### Acceptance

| Scenario | Expected result | Method |
| --- | --- | --- |
| Local Changes navigation | Local Changes and All Commits are peers in the sidebar, the toolbar and refs stay visible, and a narrow-window fallback switches between them | egui layout and native review |
| Amend contents | Amend mode shows the parent-to-index changes as staged; Stage adds working changes, Unstage removes files/hunks/lines from the amended commit without touching working files, and the rewritten commit carries the edited message | Git oracle, CLI, and native review |
| Amend identity and hooks | The original author is preserved, the committer is current, hooks run in Git order, and `post-rewrite` receives the old and new IDs | Git oracle and hook fixtures |
| Amend safety | The previous tip is reachable through a private `refs/gg/backup/...` ref, backup refs stay out of the refs sidebar, and backup failure aborts | Git oracle and refs snapshot |
| Reflog listing | Entries match `git reflog show`, newest first, with selector, commit, time, action, and detail; the default is `HEAD` | Git oracle and CLI |
| Reflog recovery | Reset to an entry matches Git after confirmation, Hard warns, and branch creation at an entry does not switch the current branch | Git oracle, CLI, and native review |
| Rebase onto | Rebase matching Git rewrites the branch onto a branch, remote, or tag; a clean worktree or explicit autostash is required; conflicts and continue/skip/abort stay recoverable; the previous tip is reachable through a private backup ref | Git oracle, CLI, and native review |
| Reorder and squash | A linear range reorders or squashes exactly like the generated Git todo; every commit appears once, the first step is a pick, root ranges work, merge ranges are rejected, combined messages are preserved, and conflicts continue/skip/abort | Git oracle, CLI, and native review |
| Rewrite recovery | Private backup refs stay out of the refs sidebar but are listed in **Backups…**; Open commit, Reset branch here (confirmed, Hard warns), Create branch here…, and Copy actions recover each rewrite without deleting user files or backup refs | Git fixtures, egui, and native review |
| Operation state | An in-progress rewrite is reported as a rebase with conflict count, Resolve conflicts…, Continue, Skip, Abort, and Backups…; completion, skip, and abort restore the pre-rewrite selection | Git fixtures and native review |

M6 is complete: amend, reflog, rebase, reorder, squash, backup listing, and rewrite recovery are implemented and verified.

Record native wide/narrow captures and verification in [verification](verification.md).

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

1. **Commit-row menu · complete.** Add **Cherry-pick**, **Revert**, **Reset here** (Soft, Mixed, Hard with confirmation), **Copy hash**, and **Copy subject** to history rows, with right-click access.
2. **Compare to local · complete.** Compare the selected commit against the current branch tip in a review diff, exposed from the commit-row menu and `git good show --against REF`.
3. **Ref menus · complete.** Add checkout/switch, merge, delete branch (safe, then confirmed force), delete tag, track remote, and copy actions to branch, tag, and remote items.

### Acceptance

| Scenario | Expected result | Method |
| --- | --- | --- |
| Commit-row actions | Right-click or keyboard opens the commit actions; copy writes the exact hash/subject | egui and native review |
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
