# Plan

Purpose: sequence milestone delivery and acceptance.

## M7 · File reading, history, attribution, and large repositories

Goal: make the selected file a first-class reading surface, then follow a file or line through history before adding the remaining large-repository features. Reading stays read-only, bounded, cancellable, and independent of the history selection.

Contracts: [file reading and history](architecture.md#file-reading-and-history--m7).

### Increments

1. **File reading surface · complete.** Add a **Diff / Full file** toggle that keeps the selected file, Before/After side, and reading position. Show exact line numbers and changed-line markers, select a line or range with the pointer or keyboard, move to previous/next change with `[`/`]` and visible buttons, and move between files with `Alt+Up/Down` and **Prev file**/**Next file**. Read one side on the existing cancellable selected-file worker, bounded to 2 MiB and 100,000 lines, with explicit absent, binary, gitlink, symlink, and budget states. Keep the status line and help text in step with the bindings.
2. **File history · complete.** Follow the selected file through a worker-backed cursor over content versions: skip unchanged commits, follow renames, and name the change kind, commit, subject, author, date, and old path. **Older version**/**Newer version** change the reader's comparison source and disable at the boundaries with a visible reason instead of wrapping; **Before**/**After** remain inspection sides. A unique predecessor is followed silently; ambiguous predecessors show a chooser and never default to the first parent. **Full file** shows the current version and **Diff** shows the previous-to-current edge. Preload a bounded number of versions and page older ones; `H` opens the version strip in the shared reader and `Esc` closes history before the reading view, file list, and review. Coherence I4 moves history from its former full-screen sub-view into the shared reader.
3. **Blame and line/range history · complete.** Attribute each line to a commit and open the same version timeline anchored to `path bytes + revision + start_line..end_line`. Map a single line through blame and range boundaries through diff hunks, and ask which predecessor to follow when a range spans divergent hunks. Deleted lines anchor the before side explicitly instead of dropping the place. `B` opens blame.
4. **Image diffs · complete.** Compare images with explicit dimensions, fit modes, and a side-by-side or swipe presentation while keeping the bounded preview and export rules.
5. **Worktrees · complete.** List, create, open, and remove worktrees without disturbing user files; keep each worktree's drafts and selection separate.
6. **Submodules · complete.** Show submodule status, referenced commits, and explicit update actions without silently changing the parent repository.
7. **Git LFS · complete.** Identify LFS pointers in Review and Local Changes, show their object identity, keep fetch/smudge actions explicit, and preview locally available objects within a bound. **Follow-up:** Materializing fetched content into working files remains planned.
8. **Commit and tag signing · complete.** Verify commit and tag signatures with distinct validity, signer/key, trust, and format results, and keep signing configuration read-only and explicit. Signed commit and signed-tag creation are deliberate choices; a missing key or agent fails without falling back to unsigned and without losing the commit draft or index.
9. **Image onion skin · complete.** Add an **Onion skin** presentation alongside Side by side and Swipe for two present images. Draw Before full and After over it at an explicit opacity, defaulting to 50%, with a labeled keyboard-adjustable opacity control. Preserve Fit/Actual size, per-side dimensions and byte metadata, checkerboard transparency, and per-side export. Require both sides; one-sided/absent images keep the explicit absence state instead of blending. Normal alpha blending only; difference/heatmap modes are separate.
10. **Force push with lease · complete.** Add an explicit Repository action that rewrites an upstream branch only when the remote still matches the fetched remote-tracking lease. Name the branch, destination, and lease before confirmation; pass `--force-with-lease=<destination>:<lease>` to Git and never accept bare `--force`. Missing/stale leases fail without changing the remote and never silently retry as a normal push.
11. **Local Changes whole-file gestures · complete.** Double-click an Unstaged or Untracked row to stage the whole file, or a Staged row to unstage it; keep conflict and intent-to-add rows inert. Give the selected file header its own **Stage**/**Unstage** button beside **File actions** and remove the duplicate whole-file stage item from the menu. After staging a file, select the next Unstaged row in display order (previous when last, or follow into Staged when none remain); after unstaging, follow the same path into Unstaged.

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
| Image diffs and onion skin | Side-by-side, swipe, and wide-only onion skin keep exact dimensions, Fit/Actual size, checkerboard transparency, and per-side export; onion skin overlays After at a keyboard-adjustable opacity and requires both sides | egui interaction, Git fixtures, and native review |
| Worktrees | Main and linked worktrees list exact paths, branch/detached/bare state, lock/prunable reasons, and clean/dirty status; create, open, and clean-only remove preserve per-worktree drafts and never delete user files | Git oracle, CLI, and native review |
| Submodules | Gitlink details separate Before/After recorded commits, parent index commit, nested HEAD, dirty/init state, and URL/branch; explicit update targets the parent index commit without staging or forcing | Git oracle, CLI, and native review |
| Git LFS | Canonical v1 pointers in Review and Local Changes report OID, declared size, local store availability, `git-lfs` readiness, remote, and a bounded 2 MiB preview; fetch is explicit and never smudges or stages | Git fixtures, CLI, and native review |
| Commit and tag signing | Read-only verification separates state, signer, key, trust, format, and configuration; deliberate commit/tag signing never falls back to unsigned on key or agent failure and retains drafts and index state | Git fixtures, CLI, and native review |
| Force push with lease | Repository confirmation names branch, destination, and exact remote-tracking lease; stale or missing leases fail without changing the remote or offering a non-force retry; bare `--force` is rejected | Git oracle, CLI, and native review |
| Local Changes whole-file gestures | Double-clicking a row or clicking the header **Stage**/**Unstage** button stages or unstages the whole file, advances to the next Unstaged file after staging, and follows the path into Unstaged after unstaging | egui timing/interaction, Git oracle, and native review |
| Read-only safety | Reading, full-file loads, history navigation, and signature/pointer inspection never write objects, refs, trust stores, configuration, the index, or working files | Repository snapshot comparison |

Record native wide/narrow captures and verification in [verification](verification.md).

## Coherent native interface · staged

Goal: consolidate navigation, reading, scoped commands, operations, and recovery into one interaction model without reducing Git capability. The [coherence proposal](ui-coherence.md) owns the design; this plan sequences delivery. Every increment updates its owning architecture section before or with the behavior change.

### Increments

1. **Stable navigation and scoped commands · complete.** Keep Local Changes and History in one stable navigation strip at every width with ref browsing reachable. Make ref selection read-only everywhere, including sidebar rows, menus, and graph pills, and move switching to an explicit **Switch to branch…** command. Give each selected object one visible scoped action surface sharing definitions, targets, readiness reasons, and confirmations with context menus and shortcuts. Retire the global Actions menu and the separate Commit actions window once replacements work. Add a return-location contract so opening a commit from a tool returns to that tool's selected item and viewport.
2. **Shared file workspace and reader · complete.** Extract one reader for comparison identity, exact path, presentation, inspection side, and reading place; migrate commit review first and keep file enumeration and selection workers independent. Full-width reading remains a layout, not another viewer.
3. **Local Changes layout and commit/amend · complete.** Unstaged files upper left and Staged files lower left, one selected comparison on the right, and commit/amend below it. A partially staged path appears in both lists and each entry selects its own comparison. Remove the duplicate Staged/Unstaged reader switch and obsolete stacked layouts while keeping whole-file, hunk, and selected-line safety.
4. **File history inside the reader · complete.** File history becomes a comparison-source mode in the shared reader with Older/Newer version navigation; Before/After remains an inspection-side control. The version strip sits above the same reader in wide and narrow reading layouts. Closing history restores the captured entry context once, version steps do not add Back entries, and selecting another file resets history to that file's newest version without reusing the previous cursor; closing then returns to the saved entry file.
5. **One file workspace, operation area, and responsive rewrite · complete.**
   - **5a. Shared reader in Local Changes · complete.** Migrate Local Changes onto the shared reader so staging, commit review, and file history expose the same Diff/Full file, Before/After, exact endpoints, and reading place. Each view shows one file header, one path, and one scoped action surface: presentation, export, copy, and whole-file staging live in the reader; low-frequency modes, object IDs, and sizes live in an on-demand **File details** disclosure. Remove the staging-specific renderer, the duplicate **View diff** route, and permanently repeated endpoint/path/mode rows while preserving whole-file, hunk, selected-line, and confirmed Discard safety.
   - **5b. Operation area and responsive rewrite · complete.** One operation area owns running, refreshing, needs-attention, and supported continuation controls across views. Validated submission closes configuration; rewrite stops blocking browsing while retained old history is labeled until replacement data is accepted.
6. **Conflict preview followed by Stage resolution · complete.** Conflicts use the shared file workspace and name Base, ours, theirs, and the result. Choosing a source is read-only; **Stage resolution** is the single publication step and validates original bytes, operation, path, and stale content.
7. **Repository tools, recovery, and pending intent · complete.** Consolidate Backups and Reflog under **Repository → Recovery**, add one **Restore stash…** form, share branch creation with an explicit switch choice, and retain blocked-action intent for revalidation without auto-execution. Recovery returns to its source, Reflog reference filter, and selected entry.
8. **Coherence closeout · complete.** Walk the connected native journeys at 1120×740 and 720×480 against the mockups and owning contracts; fix concrete parity defects such as clipped repository tools, narrow-reader reveal stalls, and truncated status hints; record the deferred-element list; and leave no superseded control beside its replacement.

### Acceptance

| Scenario | Expected result | Method |
| --- | --- | --- |
| Stable navigation | Local Changes and History appear in the same order and position at wide and narrow widths; ref browsing remains reachable and destination changes preserve the other view's state | egui geometry and interaction, native review |
| Read-only refs | Clicking a sidebar ref, menu item, or graph pill inspects its target without moving HEAD; switching is an explicit command with the existing dirty-worktree and hook-warning behavior | egui interaction, Git oracle, native review |
| Scoped commands | The visible commit, branch, and file action surfaces, context menus, and shortcuts share target, readiness reason, and confirmation; the global Actions menu and Commit actions window are gone | egui interaction, native review |
| Return locations | Opening a commit from History or a tool returns to that tool's selected item, filter, and viewport; presentation changes do not add Back entries | egui interaction, native review |
| Shared reader | One set of comparison, path, presentation, side, and place controls serves review, staging, and history; late results never appear under a new identity | Worker tests, egui interaction, native review |
| File history source | History renders a version strip above the same reader in wide and narrow reading layouts; Older/Newer never wrap and Before/After remain inspection sides; closing restores the entry context and file once; another file starts its own version cursor while history stays active | Git oracle, egui interaction, native review |
| Local Changes layout | Unstaged and Staged groups stay visible on the left, the selected comparison is on the right, and commit/amend sits below it; both entries of a partially staged path select distinct comparisons | egui geometry and interaction, Git oracle, native review |
| One file workspace | Staging, commit review, and file history use the same reader controls in the same order; the file path, exact endpoints, and scoped actions appear once, and low-frequency details are on demand; there is no separate **View diff** route or second renderer | egui geometry and interaction, native review |
| Operations and conflicts | One operation area owns progress and supported recovery; source preview never stages, and Stage resolution validates original bytes and stale state | Worker tests, Git oracle, native review |
| Recovery and intent | Recovery shares one source selector and returns to its selected entry; stash restore options and branch-creation switch choice are explicit; blocked intent is reoffered and revalidated, never auto-executed | egui interaction, Git oracle, native review |
| Coherence closeout | Connected review, Local Changes, conflict, recovery, stash, and branch journeys keep the same labels, placement, disable reasons, and return rules at 1120×740 and 720×480; no superseded control remains and every housed control stays inside the viewport | Native capture, geometry test, keyboard/pointer review |

Implementation handoff: [phased implementation and verification prompts](implementation-prompts.md). These increments change no deferred M7 scope: LFS smudging/materialization remains planned.

## Final interface · staged

Goal: deliver the agreed [final interface design](final-design.md): one window frame with a menu bar and icon toolbar, compact density, search, a richer interactive rebase, and Preferences. Each increment ships on its own, updates its owning architecture section, and keeps every existing command reachable.

### Increments

1. **Window frame · complete.** Regroup the menus into Repository, Branch, Commit, Changes, View, and Help. Merge the navigation strip into one icon toolbar: refs sidebar toggle, Local Changes/History, Fetch, Pull (rebase) only when behind with its count, Push, New branch, New tag, Merge, Stash changes (disabled on a clean worktree), the Stashes picker with Apply, Pop, Show changes, and confirmed Drop, Reorder and squash (now Interactive rebase) of the current branch's own commits, and Recovery. Icons come from the bundled Phosphor font; every button has a tooltip and the same readiness reason as its menu item.
2. **Compact density · complete.** Shorten the tab to **Changes** with a change-count badge fed by the background summary; 22 px history, Review file, and Local Changes rows; one-character change markers with the full kind on hover; path and endpoints on one reader line. **Follow-ups:** the comfortable density preference moves to Preferences (8); the one-line commit identity in Review moves with the review header rework in 6.
3. **Search · complete.** `Ctrl+K` and a toolbar Search button open grouped Filter, Actions, Branches, Remotes, and Tags results that share the toolbar's command dispatch and readiness; Filter is first and finds commits. **Follow-up:** a Files group, which needs a tracked-path index.
4. **Sidebar stashes, branch switcher, and modal dialogs · complete.** Sidebar stash entries opening the toolbar picker; current-branch button with ahead/behind counts opening a filterable switcher that uses the validated switch; decision dialogs become modal with a dimmed, click-absorbing backdrop while tool windows stay non-modal.
5. **Rebase engine · complete.** Extend the rewrite plan beyond pick and squash with reword, fixup, drop, and edit (reopen), with CLI support and Git-oracle tests before any UI.
6. **Interactive rebase dialog and review header · complete.** Fold the Review commit identity into one line. Replace Reorder and squash with the planned modal: newest-first plan, action chips and keys, consequence badges, the selected commit in the shared reader with inline reword, and Open in Review with return.
7. **Rebase stops · complete.** Reopened-commit stop in Local Changes with split commits, and the three-column conflict blocks with per-block choices, both in the operation area with Continue, Skip commit, and Abort and restore.
8. **Preferences · complete.** Repository › Preferences… window per the [Settings panel](#settings-panel--planned) plan, starting with clone folders and a preferred folder.

### Acceptance

| Scenario | Expected result | Method |
| --- | --- | --- |
| Window frame | Menu titles and toolbar buttons keep their order and position in both views and at 720 and 1120 widths; each toolbar button matches its menu item's enablement and reason | egui geometry and interaction, native review |
| Pull when behind | Pull is enabled only when the current branch is behind its upstream and names the count; otherwise it is disabled with the reason | egui interaction, Git oracle |
| Rebase actions | Reword, fixup, drop, and edit produce the same history as `git rebase -i` with the same todo; backups are created first | Git oracle, CLI |
| Rebase stops | Continue, Skip, and Abort appear only in the operation area and only when Git supports them | Git oracle, egui interaction |

### Gaps · reviewed 2026-10-30 · all fixed

Differences between the [final design](final-design.md) and what shipped, plus places that break the [stable layout and no-placeholder rules](design.md). Each fix keeps its own test.

Design not yet implemented:

1. **Shortcuts and menus · fixed.** A real 26 px menu bar; View lists Local Changes (`Ctrl+1`), History (`Ctrl+2`), Commit details (`Space`), and Refs sidebar (`Ctrl+\`), and the shortcuts work. Repository ends with Preferences and Quit; macOS has a gg menu with Settings… (`Cmd+,`) and Quit.
2. **Stashes picker · fixed.** Rows show ref, message, branch, age, and file count in fixed slots; the picker keeps a fixed size.
3. **Search · fixed.** A toolbar search field on wide windows (`Ctrl+K` focuses it), the icon on narrow ones, and a Files group that opens the latest commit changing the file in Review on that file.
4. **Conflict stop · fixed.** Resolve conflicts opens in the main workspace under the operation area; Close or `Esc` returns.
5. **Preferences · fixed.** Diff (reader presentation, image layout) and Commit (signing default) sections; the details pane and image layout are remembered; stash, merge, new-branch, and fetch-prune choices are remembered.
6. **Frame metrics · fixed.** Menu bar 26 px; compact diff rows 19 px.

Layout moves with content:

7. **Operation area · fixed.** Always present at two lines; idle it summarizes the branch, tracking, changes, and stashes; a rebase stop fits its note and step chips on the second line.
8. **Conflict workspace · fixed.** Fixed path-list width and fixed heights for blocks, result, and side panes; errors and the empty state share one reserved line.
9. **Interactive rebase dialog · fixed.** The changing result count comes last in the header; the message or reword field has one fixed slot.
10. **Review header · fixed.** The parent selector, file count, Retry, History, and Prev/Next file keep reserved slots and are hidden, not removed.
11. **Search and stash popups · fixed.** Results and the stash list scroll inside fixed heights; the stash actions reserve room for the drop confirmation.

Placeholder text in panes:

12. **· fixed.** Panes now keep their slots empty or keep the previous content instead. Previously, "Loading…" or "Reading…" text replaced content in: commit details ("Loading message..."), history rows, Review and Local Changes file rows, the reader ("Loading selected-file diff…", "Loading full file…", "Loading attribution…"), file history ("Loading file history…", "Loading versions…"), Review's empty list ("Reading comparison…"), and the Worktrees, submodule, and LFS windows. Endpoint labels show "resolving…". Status-line progress is allowed.

### Mockup alignment · reviewed 2026-10-30 · in progress

Second pass comparing every Final-page board with the app. Done: the Changes reader's Unstaged | Staged toggle (decided: follow the mockup) and line selection bar with Clear, menus from one command table, About gg, rebase plan and stop wording, Changes group headers with Stage all/Unstage all, commit box wording, toolbar tooltips with shortcuts, the search placeholder, Blame and the ‹ › ⋯ reader buttons, refs sidebar groups, history pill order and colors, the History footer, the window title, and `[`/`]` between conflict blocks.

Still to build (medium):

1. Review header: per-file +/− stats and the `FILES n +x −y` header (subject, ID pill, author · date, and **Compare with** are done) (parents and HEAD); `FILES n +x −y` with per-file stats; left-elided paths; a fixed 300 px file list.
2. Conflict workspace: files panel with resolved list and blocks left; result line numbers. **Open in editor** needs a way to launch the user's editor (a new dependency or a setting): decide in review.
3. Rebase stop: **Commits made at this stop** section in Local Changes.
4. Stash picker: two-line rows (Up/Down/Enter/Del keys are done).
5. Preferences: footer with the config path and Close; search field; the settings listed on the Final page that are not built yet.
6. Refs sidebar: **Worktrees** group and branch tracking counts.

Decisions for the review (see the canvas page **Review · as built**):

- Side-by-side diff, the bottom commit inspector, and the Unstaged/Staged reader toggle (all done).
- History: the filter row is gone; `Commit · N` and the active filter sit in the column header (done).
- Decided: follow the mockups. Refresh is automatic (file watcher and before commands), `Ctrl+R` is Reword, `Del` deletes, Preferences is modal, Open in editor uses an editor-command setting. F10 menu focus is not available in egui.

## Repository setup and branch integration · in review

Goal: make clone destinations quick to compose and branch integration discoverable without weakening Git safety.

Contracts: [repository clone and branch switching](architecture.md#branch-switching-and-creation), [clone](architecture.md#network-operations).

### Increments

1. **Clone setup and input editing.** Seed Clone's URL only from clipboard text recognized as a repository URL; preserve native paste in every text field. Replace the single destination path input with a root selector and project-slug field. Default to the marked development base folder plus URL slug, list the launch folder last, and persist the base folders and their default.
2. **Branch list clarity.** Make the refs sidebar user-resizable; bold the selected branch and use a stable per-ref color shared with its history lane for branch glyph and dot. Double-click a branch to request the ordinary safe switch, never to bypass validation.
3. **Recoverable integration.** When local changes block a requested switch/rebase, retain the target and offer explicitly submitted stash-and-switch or rebase autostash. Expose **Pull --rebase** for upstream integration and the existing ref-scoped rebase actions through operation and recovery paths.

### Acceptance

| Scenario | Expected result | Method |
| --- | --- | --- |
| Clone destination | URL can be pasted; recognized clipboard URL seeds the field; root plus slug defaults to the marked development base folder (else the first saved one, else the current directory) plus repository name; base folders and the default persist and can be selected | Clipboard/UI tests, settings tests, native review |
| Text entry | Paste works in every single-line and multiline text input without replacing normal editing | Input-event tests and native review |
| Resizable refs and Details | Divider drag changes widths within bounds; idle frames and normal interaction preserve both axes of history geometry | egui geometry/interaction and native review |
| Branch identity | Current branch is bold; branch glyph and dot match its stable history-lane color | egui paint/geometry and native review |
| Branch double-click | Double-click requests ordinary checked-out switch; current branch is inert; dirty state does not bypass validation | egui interaction and Git oracle |
| Dirty integration | Blocked switch/rebase offers explicit stash and retains target; no automatic stash/retry; post-switch pull-rebase and rebase-onto remain explicit and recoverable | Git oracle, operation worker tests, native review |

## Settings panel · planned

Goal: gather user preferences in one Settings window and keep them across launches.

### Increments

Status: complete. Increments 1–3 shipped with the final interface; 4 and 5 shipped with the Diff and Commit sections.

1. **Settings store · complete.** Persist preferences in `gg/settings.json` under the OS configuration directory; read `gg/roots.json` once and fold it in. Unknown or malformed values fall back to defaults without losing the rest.
2. **General · complete.** Theme (system, light, dark) and network avatars (on, or cached avatars and initials). Promote the `GG_THEME` and `GG_OFFLINE` debug switches to these settings; keep the variables as overrides.
3. **Repositories · complete.** Move Development base folders and the default clone folder into Settings; keep Repository → Clone's **Folders…** shortcut.
4. **Layout · complete.** Remember Commit details visibility and the binary preview layout.
5. **Operation defaults · complete.** Remember the last choices for stash (include untracked, keep staged), stash restore (restore staged, remove after restore), merge (always create a merge commit), new branch (switch after creation), and fetch prune. Dialogs open with these values; every operation stays explicitly submitted.

### Acceptance

| Scenario | Expected result | Method |
| --- | --- | --- |
| Round trip | Each setting survives relaunch; a malformed file resets only the affected values | Settings tests |
| Migration | An existing `gg/roots.json` appears in Settings with its default intact | Settings tests |
| Overrides | Debug environment variables win over saved values without rewriting them | Settings tests |
| Settings window | Sections fit at 720 and 1120 widths and apply changes immediately | egui geometry and native review |
| Operation defaults | Dialogs open with remembered values; nothing runs without explicit submission | egui interaction and Git oracle |

## UI-flow consolidation · pre-M7

Goal: improve continuity, explicit decisions, and recovery feedback in the existing native workflows before adding M7 features.

Contracts: [working status, committing, and amend](architecture.md#committing).

### Increments

1. **Commit drafts · complete.** Retain normal and amend commit text and the selected file when leaving Local Changes for history, review, or a refresh; restore them for the same canonical worktree, clear the submitted draft only after a successful commit, retain it on hook failure and conflict, and never transfer drafts between worktrees.
2. **Merge, rebase, and recovery workers · complete.** Run merge, rebase, and continue/skip/abort on one background operation worker; close the dialog immediately, name the pending operation in the status line, disable duplicate or incompatible mutation actions, and refresh mutation state, history, refs, status, reflog, and backups once the result is accepted.
3. **Conflict workflow · complete.** Name the operation, unresolved count, and operation commit above the conflict content; explain Ours/Theirs per operation, especially rebase; state that whole-file choices replace and stage while **Stage resolved** stages external edits; disable Continue while unresolved paths remain; run conflict listing, content reads, and side resolution on workers; select the next unresolved path after resolution; and use side tabs with bounded scrolling at narrow widths. Coherence increment 6 supersedes the immediate side/stage wording with a read-only preview followed by one validated **Stage resolution**.
4. **Comparison direction and scope · complete.** Rename the commit-row action to **Compare with current HEAD** and show a persistent Before/After header with the requested ref name and resolved commit IDs. Label whole-file staging actions **Stage file**, **Unstage file**, and **Discard file**, keep hunk and selected-line actions distinct, use **Discard** consistently, and state the exact file/scope plus unchanged index in the confirmation.
5. **Commit readiness and amend mode · complete.** Derive readiness from the completed, validated status scan: a normal commit requires a nonempty message and at least one staged entry, message-only amend stays valid, and disabled states name the reason. The amend banner names the affected commit and subject above the commit action. The message editor expands and collapses without permanently displacing the file list.
6. **Responsive staging · complete.** At narrow widths, show a **Staged**/**Unstaged** switch with change counts, focus the nonempty pane when the other is empty, and offer **Split** to stack both comparisons. Keep both panes discoverable and preserve each pane's selection and scroll across focus and layout changes. Superseded by Coherence increment 3, which removes the duplicate reader switch and Split in favor of the approved two-group layout.
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
| Responsive staging | Superseded by Coherence increment 3: narrow widths keep both named groups above one reader, with no Staged/Unstaged comparison switch or Split control; each group keeps independent selection and scroll | egui geometry, Git fixture, and native review |
| Action discoverability | The visible **Commit actions** menu beside the selected commit identity shares the context-menu definitions, groups inspect/integrate/rewrite/recovery commands, uses target-aware labels, explains disabled commands, and opens with Alt+A; the Help shortcut window remains, and keyboard-only users can inspect, compare, switch a branch explicitly from Refs, and reach recovery | egui interaction, Git fixture, and native review |
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
| Local Changes navigation | Local Changes and History share one stable navigation strip in that order at wide and narrow widths, the toolbar and refs stay visible, and switching preserves history selection/filter/offset and the Local Changes draft | egui layout and native review |
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
