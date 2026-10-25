# Browser regression baseline

Purpose: map browser and review behavior to executable protection and remaining coverage.

Contracts: [architecture](architecture.md). Results: [verification](verification.md). Extend coverage for each accepted interaction and defect.

## Automated protection

| Contract | Executable evidence | Coverage boundary |
| --- | --- | --- |
| CLI discovery, explicit path, JSON, errors, uncapped history | `apps/git-good/tests/cli.rs` | Process execution; native startup is separately reviewed |
| History scope, order, and parents; root, packed, detached, bare, worktree, shallow, all refs | `crates/git/tests/history.rs` | Synthetic repositories checked against Git; parent-after-child ordering asserted |
| Repository preservation | `browsing_preserves_repository_index_and_dirty_worktree` | CLI reads preserve all fixture files, including index, objects, refs, staged/unstaged/untracked content |
| First page before completion, whole-message search, deep/back pages | `first_page_precedes_completion_and_disk_pages_support_whole_history_search` | Real spool/worker with controlled reader; bounded output |
| Cache eviction and identity retention | `pages_evict_and_reload_without_losing_identity_or_selection` | UI row-count budget; peak process memory remains measured separately |
| Hover/press identity, inert placeholders, thumb drag and release, count growth | `interaction_tests.rs`; `incoming_count_waits_for_wheel_settling_and_preserves_hovered_content_and_offset` | Actual egui input and controlled arrivals; native momentum remains a platform check |
| Stale filters/details, refresh anchors, failed switch/retry, retained partial errors | `interaction_tests.rs` | Production mailbox and draw paths; generation/selection assertions |
| Filter selection rules | `state.rs`, `stream.rs`, `interaction_tests.rs` | Snapshot transitions and worker search covered separately; full streamed preserve/fallback sequence needs an integrated test |
| Native/Vim/Emacs navigation, empty/boundary states, copy output, editor focus | `navigation_bindings_*`; `keyboard_navigation_rejects_late_details_and_loads_an_evicted_row` | Native paste/clipboard delivery remains a platform check |
| Stable mid-list/last-page geometry and details loading | `keyboard_on_visible_rows_preserves_viewport`; `keyboard_last_page_has_stable_geometry_between_frames`; `keyboard_details_show_current_summary_until_message_arrives` | Frame geometry and selected data; GPU presentation reviewed separately |
| Narrow details/Back preserves hidden history | `narrow_details_navigation_preserves_hidden_history_and_back_anchor` | Production streaming state, selection, filter, exact pixel offset |
| Compact layout, overflow, shared avatar sizes and cache | `history_view.rs`, `bounded_text.rs`, `avatars.rs`, `compact_layout_preserves_table_width_and_disjoint_panes` | Geometry and controlled cache/network responses; screenshots and live service separately reviewed |
| Remote and tracking snapshot | `crates/git/tests/refs.rs`, `crates/git/tests/sync.rs`; refs sidebar and status-line tests | Local bare remotes and Git oracles; live credential helpers and SSH agents are native checks |
| Clone, fetch, push, and remote-branch checkout | `crates/git/tests/sync.rs`, `apps/git-good/tests/cli.rs`, `apps/git-good/src/main.rs` | Local bare-remote integration; real network latency and credential prompts remain native checks |
| Credentials, progress, and cancellation | `sync.rs` redaction/credential tests, worker cancellation tests, `network_cancel_button_stays_inside_the_toolbar` | Bounded progress and child reaping; live terminal credential prompts and slow networks are native checks |

## Remaining coverage

- Add integrated streamed filtering tests for preserved selection, first-match fallback, and zero results with delayed batches.
- Verify worker replacement/drop cancellation, temporary-file cleanup, and storage failures after partial delivery.
- Verify revision timestamp ties, non-UTF-8 display/native paths, and SHA-256 repositories.
- Implement and test the specified external-change refresh indication; explicit refresh and pinned history are already covered.
- Complete the [preview acceptance checks](verification.md#preview-acceptance-pending). Native integration and performance evidence complement in-memory tests.

## Increment gate

Run the full [development checks](development.md) for every increment. Use offline fixtures independent of user Git configuration. Exercise production worker/input paths; retain scenarios through refactoring and update expectations with explicit contract decisions.

Before connecting M1 controls, test Review/Back anchor restoration, active-view shortcuts, independent worker cancellation, and late-result isolation. Run these alongside the M0 suite.

## Coherence I1 navigation and scoped commands

- Geometry and interaction tests find **Local Changes** and **History** in one navigation strip in that order at 720 and 1120 widths, switch both destinations, and preserve history selection, filter, pixel offset, and the Local Changes draft. The wide sidebar no longer repeats navigation; the Refs toolbar menu keeps ref browsing reachable at narrow widths.
- Egui pointer tests click sidebar branch, remote, and current-branch rows and graph pills. They assert inspection only, no double-click mutation, and a real Git fixture confirms HEAD, index bytes, worktree status, and branch refs are unchanged.
- The sidebar context menu and Refs menu expose **Switch to \<branch\>**; current-branch and busy states disable it with a visible reason. Existing Git fixtures continue to cover dirty-worktree rejection, checkout, and post-checkout warnings.
- The selected commit has one **Commit actions** menu in wide details and narrow details, sharing `commit_action_menu`, `apply_commit_command`, and `run_commit_action` with the row context menu and Alt+A. The global Actions menu, separate Commit actions window, and their Help entry are absent; all commands and readiness rules remain.
- Review records the caller's selected commit, filter, pixel offset, and narrow-details state. Closing with Back, Escape, or the History destination restores it once; presentation changes add no Back step. Opening another repository clears the stale Review and caller location. Recovery's source variant is an explicit extension point for I7.

## Coherence I2 shared reader

- The shared reader context carries the requested file identity, exact `FileChange`, both endpoint labels, presentation, inspection side, cursor, anchor, and pixel offset. Tests save a context, change presentation and place, restore it, assert every value and endpoint label, and confirm the place survives identity adoption and asynchronous updates.
- Commit review supplies resolved parent/reference and commit labels to the reader header; file history supplies the older and current content-version labels. Endpoint labels appear only when displayed content matches the requested identity, and the header retains exact modes and blob IDs.
- Reader capabilities gate Diff, full-side selection, and export from the actual comparison. Export uses the displayed sides; absent and gitlink sides cannot export, and an unresolved history edge cannot open Diff.
- Requested and displayed identities remain separate: generation, mode, and side still reject stale worker results. Existing 2 MiB and 100,000-line limits, binary preview, and export budgets remain covered by their owning tests.
- Review captures the reader context when opening file history and restores it once on return; the same value is the staging/history integration API for I3/I4. No blame or line-history control is added.

## M1 changed files

- Git-oracle fixtures cover root/first-parent changes, merge-parent selection, nested paths, additions/deletions, IDs/modes, file-to-directory replacement, empty comparisons, invalid parents/revisions, and pinned comparisons with unavailable blobs.
- Process tests cover nested discovery, explicit repository, exact path bytes, JSON, and error exits.
- Controlled reader tests cover early delivery, 10,000-file completion, deep/back page reload, retained partial errors, and worker shutdown.
- Input tests cover review/history isolation, late history results, Back anchors, fixed rows, and hovered placeholders at wide/narrow sizes.

## M1 selected-file text

- Git-oracle hunks and byte reconstruction cover CRLF and final-newline changes; engine tests cover invalid UTF-8, empty sides, cancellation, and budgets.
- Process tests cover selected-file JSON and missing-path stderr/exit status.
- Controlled worker tests verify superseded work, 256-row pages, and document reuse. UI state tests reject stale pages and preserve hovered placeholders.
- Native layout and resource measurements remain part of M1 acceptance.

## M1 parent selection

- Git fixtures verify stored parent order, default selection, second-parent files, and unavailable parent objects.
- In-memory wide/narrow tests verify pinned revision requests, retained identity on hover/failure, atomic replacement, first-file selection, and retry mailboxes.

## M1 special objects and paths

- Portable object fixtures cover executable-bit changes, symlinks, type changes, missing gitlink targets, and byte-distinct path labels.
- Mode-only tests stop at header reads; cancellation tests interrupt before an unavailable second object header.
- Unix process tests select non-UTF-8 paths through `--file` and verify exact content bytes. Engine tests preserve Unicode and escape controls/backslashes.

## M1 export and binary preview

- CLI tests export packed blobs beyond the diff budget, preserve existing destinations, and reject absent sides and Git metadata destinations. The repository snapshot test covers history, review, diff, and export with staged/unstaged/untracked content.
- Cancellation tests remove partial output. PNG/JPEG tests verify dimensions, pixel budgets, malformed input, and bounded hexadecimal fallback.
- Preview requests share the selected-file worker's generation and mailbox tests; export retains its explicitly selected object identity.

## M1 resource acceptance

- `packed_review_latency_and_navigation` uses production workers and egui input on the packed [review fixture](development.md). It asserts first-summary/diff latency, cached-frame latency, deep/back reachability, and both UI cache limits.
- `retry_same_comparison_keeps_selection_anchor_and_diff` preserves selection, diff generation, offset, and count through partial retry delivery.
- `packed_diff_stress_and_rapid_selection` covers fully changed text, maximum-pixel PNGs, and superseded real worker requests. The X11 probe measures native selection response while these workloads run.
- Record release timings and peak process RSS in [verification](verification.md). Hardware and supported-platform runtime checks remain release gates.

## M2 status and working diffs

- Git porcelain fixtures cover independent areas, ignores, conflicts, intent-to-add, discovery, and repository preservation. Bounded-worker tests cover cancellation and revisiting distant pages.
- Working comparisons reconstruct each side and match Git line counts. Fixtures verify normalization, symlink targets, binary/budget summaries, stale HEAD/index, and files changing during a read.
- In-memory input tests preserve hovered slots, last-page geometry, history anchors, and status selection/offset through working-diff navigation. Working comparisons publish each pane as soon as it is ready, even with the pointer over the view. CLI tests require an explicit area for selected paths and verify JSON and failure output.

File staging: compare staged entries and subsequent trees with `git add -A`; preserve working bytes, mode bits, symlink targets, and unrelated staging. Cover additions, modifications, deletions, type changes, intent-to-add, text attributes, emulated symlinks, unborn branches, SHA-1/SHA-256, stale HEAD/index/file observations, lock ownership, submodules, file/directory collisions, CLI receipts/errors, and worker refresh/failure.

Whole-file unstaging: compare restored entries and subsequent trees with Git; preserve working bytes and unrelated staging. Cover unborn branches, SHA-1/SHA-256, stale observations, lock ownership, file/directory collisions, CLI receipts/errors, and worker refresh/failure.

Hunk and line staging: compare selected index blobs with scripted `git add -p` in SHA-1/SHA-256 repositories. Cover whole-hunk and individual-line selections, paired old/new lines, partial selection with zeroed stat and `MM` status, all-change selection with real stat, multiple selections, CRLF attributes, missing final newlines, unrelated staged entries, binary/budget limits, symlink targets, mode changes, untracked/intent-to-add/deleted inputs, stale headers/lines/HEAD/index, lock ownership, and CLI `--hunk` receipts.

Partial unstaging: stage regular text additions, modifications, and deletions, then reconstruct selected lines from HEAD and index. Verify that restoring every line restores the HEAD entry, removing every line of an added file removes the index entry, and partial results write zeroed stat while working bytes stay untouched. Cover SHA-1/SHA-256, stale HEAD/index, lock ownership, CLI/worker errors, and mode/type/symlink fallbacks.

Dropping working changes: drop whole hunk, selected block, and whole-file changes from the unstaged pane; verify the index bytes are unchanged and other hunks survive. Restore modified, deleted, executable, and symlink content from the index, remove untracked files, and leave working bytes untouched on stale or unsupported input. Cover confirmation and cancellation, mode/type fallbacks, filtered line endings, lock ownership, and errors.

Split stage view: open from either a staged or unstaged row. Wide widths show both panes side by side; narrow widths show a **Staged**/**Unstaged** switch with counts, focus the nonempty pane, and provide **Split** to stack both comparisons. Verify immediate adoption under hover, inline **Stage hunk**/**Unstage hunk** actions, hunk-header and individual-line selection, **Discard hunk**/**Discard selected**/**Discard file** with confirmation and cancellation, whole-file fallbacks, in-place refresh after success, status refresh on Back, preserved pane selection and scroll across focus/layout changes, and retained panes/error/index after failure.

Committing: compare committed trees, parents, messages, and reflogs with Git for root, normal, detached-HEAD, and linked-worktree cases. Verify that only index content is committed, unstaged working bytes survive, and history selects the new commit after Back. Cover empty indexes, empty messages, conflicts, intent-to-add, sparse indexes, stale HEAD/index observations, identity resolution, and lock ownership.

Commit hooks: fixtures assert `pre-commit`, `prepare-commit-msg`, `commit-msg`, and `post-commit` execution order and arguments, including `core.hooksPath`. A pre-commit hook may stage content; a message hook may rewrite `COMMIT_EDITMSG`. Hook failures must preserve the index, working tree, and message, report the hook name/status/output with a retry path, and refresh status when hook staging occurred. A failing post-commit hook returns a warning with the created commit. CLI and egui tests cover receipts and draft retention; native review covers history refresh.

Navigation and diff continuity: Enter opens commits at both widths; text editors retain keys while focused buttons allow navigation. Reading keys scroll without changing files. History/review file selection retains displayed rows until current-generation adoption; hover defers replacement there. Working stage panes adopt each result immediately. Toolbar geometry covers Repository, View, and Status.

## M3 graph and refs

Ref snapshots: compare HEAD, local branches, remote branches, and annotated/lightweight tags with Git for normal, unborn, and detached repositories. Verify peeled commit IDs, symbolic remote HEAD omission, exact names, and non-mutation of refs, index, and working files. CLI `refs` output must match the shared snapshot.

Graph layout: unit tests cover linear history, merge parents, lane reuse, joins, lane retirement, single-parent lane colors, and downward starts for only the row's own lane. Worker tests verify graph rows stay aligned with commits when the query filters rows, and egui tests require the graph column and ref labels at 720 and 1120 widths. Long or numerous refs must remain single-line with an explicit overflow count. History traversal includes commit targets from every local branch, remote branch, and tag; a `git rev-list --all` oracle and a parent-after-child assertion protect the all-ref set and topological order.

Ref navigation: the Refs sidebar groups branches, remotes, and tags as collapsible trees at wide widths; the Refs menu remains the narrow fallback. Both mark the symbolic HEAD branch, filter by ref name, inspect branch/remote/tag commits without mutating the repository, and expose an explicit **Switch to \<branch\>** command for local branches with the current branch disabled. Clicking any ref inspects its target; double-clicks have no mutating meaning. Jumping must clear a hiding filter, recover the correct row after traversal completion, preserve the selected anchor, and reveal deep targets. History labels render as pills with the current branch first, icons, width-aware overflow, and a readable subject. A pill click selects its commit without switching branches; tag and remote inspection never moves HEAD. Duplicate loose/packed refs must appear once. Native wide/narrow review verifies graph lanes, labels, sidebar, menus, and selection.

## M3 branch switching and creation

Branch safety: switch between local branches and create from the current HEAD only when no tracked change is present. Untracked files do not block switching or creation and survive the switch; Git rejects an untracked file that the target branch would overwrite, and that failure leaves the current branch and file intact. Cover unstaged, staged, and conflicted tracked inputs, branch-specific ignore rules, missing branches, duplicate names, invalid names, detached HEAD starts, and unborn repositories. Verify HEAD, index, and working files are unchanged on every rejection.

Branch actions: compare the checkout result with Git for worktree content, symbolic HEAD, and post-checkout hooks, including a failing hook that leaves the branch switched but returns a warning. Verify `git good switch [--create] BRANCH` receipts/errors, Refs-menu HEAD marking, modal creation, branch-switch refresh of history/refs, and dirty-tree recovery. Tags and remotes remain read-only jumps; branch deletion, rename, and remote tracking are outside this increment.

## M4 clone, fetch, push, tracking, and credentials

Tracking snapshot: compare configured remote names and fetch/push URLs with Git configuration, including a distinct push URL and missing/extra remotes. Verify local-branch upstream ref, remote, and remote branch against `git rev-parse` and `git config branch.*.remote`/`merge`; branches without an upstream stay explicit. Ahead/behind counts match `git rev-list --left-right --count` for diverged, equal, unrelated, shallow, and missing remote-tracking histories. Tracking reads never change refs, the index, or working files, and the previous snapshot stays displayed while a refresh is pending or failed. `git good refs [PATH]` carries the same remote and tracking data as the native sidebar.

Network operations: use local bare remotes and disposable clones. Fetch updates remote-tracking refs and leaves local branches, HEAD, the index, and the worktree unchanged. Push sends only the selected local branch, rejects non-fast-forward updates without force, and records upstream only after success. Clone rejects existing nonempty destinations and unsafe option-looking inputs, preserves the destination on failure, and opens only a successfully cloned repository. A missing Git executable returns installation guidance.

Credentials and redaction: fixture helpers and URL forms verify that transport delegates to Git configuration and the SSH agent. Missing credentials produce an actionable remote-name failure. Displayed URLs, receipts, progress, logs, and errors redact URL user information and never copy credentials or helper output. Ambient `GIT_DIR` and `GIT_WORK_TREE` overrides are removed.

Progress, cancellation, and serialization: bounded progress phases and percentages stay current and truncate long messages; the CLI writes one JSON receipt to stdout and progress to stderr. Cancelling the native operation stops and reaps the Git child, reports cancellation, and leaves the previous view refreshable. Duplicate Fetch, Push, Clone, or remote-branch checkout actions stay disabled until the worker finishes, and the toolbar cancel button remains inside the window at supported widths.

Native review: the refs sidebar and status line show the current branch upstream with Phosphor ahead/behind icons; Clone and Track remote branch dialogs render at 720 and 1120 widths; Fetch streams progress in the status line with the cancel action visible; remote-branch checkout creates a tracking branch only on a clean worktree and never silently detaches HEAD.

## M5 stash

- Git oracle tests cover push, list, apply, pop, and drop; messages; `--include-untracked`; `--keep-index`; newest-first ordering; conflicting apply and pop; invalid references; clean trees; and preservation of unrelated files.
- CLI tests cover JSON receipts for list/push/apply/pop/drop and actionable stderr for invalid references.
- egui tests render the **Stash changes…** dialog and **Manage stashes…** window at 720 and 1120 widths, including entry references and the Apply/Pop/Drop actions.

## M5 cherry-pick and revert

- Git oracle tests cover a clean cherry-pick with the expected parent, cherry-pick conflicts with abort and continue, revert with a conflict and skip, a clean revert, and option-shaped or missing revisions.
- CLI tests cover JSON receipts for cherry-pick and revert, conflict reporting, `--abort`, `--continue`, and unknown-option stderr.
- egui tests render the in-progress banner with Continue, Skip, and Abort at 720 and 1120 widths.

## M5 merge and conflicts

- Git oracle tests cover fast-forward, already-up-to-date, diverged merge commits, forced `--no-ff` merge commits, conflict detection with abort and continue, and option-shaped or missing references.
- CLI tests cover JSON receipts for merge, fast-forward and merge-commit kinds, conflict reporting, `--abort`, `--continue`, `--no-ff`, and unknown-option stderr.
- egui tests render the **Merge branch…** dialog and the merge in-progress banner at 720 and 1120 widths; merges hide the unsupported Skip action.

## M5 conflict resolution

- Git oracle tests cover unmerged path listing, base/ours/theirs content, side selection with staging, manual staging, add/add conflicts without a base, and unknown-path or base-side rejection.
- CLI tests cover `conflicts` listing, `conflict --file` content, `--use ours|theirs`, `--stage`, and missing-path errors.
- egui tests render the **Resolve conflicts** window with path selection, operation-aware Base/Ours/Theirs labels, and side tabs at 720 and 1120 widths. Worker tests deliver conflict content to the open window without blocking, route side resolution through the operation worker, and prefer the next unresolved path after a resolution.

## M5 recovery and operation state

- Git and CLI tests cover continue, skip, and abort for cherry-pick, revert, and merge, including merge's unsupported skip and restored HEAD/worktree on abort.
- The native in-progress banner exposes Continue, Skip, and Abort, hides Skip for merges, and shows the conflict count.
- Recovery refreshes history, refs, and an open Local Changes view at a safe boundary; the conflict window closes once no operation remains.
- Merge, rebase, and recovery use one background operation worker. The UI test blocks the worker behind a channel gate, proves the start call returns and a frame still renders and names the pending operation, rejects a duplicate submission, then releases the gate and accepts one result. Mutation actions are disabled while the worker is pending, and a failure or disconnection reports an actionable error.
- Operation outcomes render a severity-colored summary with **Details…** and one recovery button such as **Resolve conflicts…** or **Backups…** at 720 and 1120 widths. The details window shows selectable full text, and the recovery button opens the matching surface.

## Commit-row context menu and reset

- Git oracle tests cover soft, mixed, and hard reset and reject option-shaped or missing targets.
- CLI tests cover `git good reset --soft|--mixed|--hard COMMIT` receipts and missing-mode errors.
- egui tests render the reset confirmation, including the hard-reset warning, at 720 and 1120 widths.
- The visible **Commit actions** menu beside the selected commit identity shares the commit-row context-menu definitions, grouped as Inspect, Integrate, Rewrite and destructive, and Recovery. Tests cover grouped rendering at 720 and 1120 widths, the Alt+A shortcut opening the narrow details overlay when needed, the Help shortcut window, and the shared menu returning the clicked command. The global **Actions** menu and separate **Commit actions** window are gone.

## Compare to local

- Git oracle tests compare an arbitrary `against` base with the selected commit and assert the old/new sides.
- CLI tests cover `git good show HEAD --against HEAD^`, the generated comparison subject, changed files, and the `--against`/`--parent` conflict.
- The commit-row menu exposes **Compare with current HEAD**, which opens the review diff against the current branch tip. The review header names the Before ref and resolved commit and the After commit, including for parent, root, and merge comparisons.
- Whole-file staging actions read **Stage file**, **Unstage file**, and **Discard file**; hunk and selected-line actions stay distinct; **Discard** confirms the exact file and scope and states that the index is unchanged.

## Ref context menus

- Git oracle tests cover safe and forced branch deletion, current/missing-branch rejection, and tag deletion with invalid names.
- CLI tests cover `git good branch --delete BRANCH [--force]` and `git good tag --delete TAG` receipts and errors.
- egui tests render the branch and tag delete confirmations at 720 and 1120 widths.
- Branch, tag, and remote menus expose switch/jump, merge, rebase, track, delete, and copy actions.

## M6 amend and navigation

- egui tests find Local Changes and History in one navigation strip in that order at 1120 px and 720 px, keep the global toolbar and refs visible, switch destinations without losing history selection/filter/offset or the Local Changes draft, and confirm the old Status button is gone.
- Draft tests retain a multiline normal commit message and the selected path/area across history/review navigation, keep normal and amend drafts separate when toggling amend, and never transfer a draft between worktrees. Hook-failure tests verify a failed commit keeps the message.
- Git oracle tests rewrite HEAD with the staged index, preserve the original author and parents, allow a message-only amend with an unchanged tree, and keep the previous commit reachable through `refs/gg/backup/...`.
- Hook fixtures verify `pre-commit`, `prepare-commit-msg` with the `commit` source and old ID, `commit-msg`, and `post-rewrite amend` stdin. A failing `post-rewrite` returns a warning after the rewrite; other hook failures leave HEAD unchanged.
- Amend mode compares the index with HEAD's parent, including the empty tree for a root commit. Staged rows expose the commit contents, working changes remain unstaged, and unstage restores the parent side for whole files and hunks without changing working files.
- Amend rejects an unborn HEAD, an empty message, and a HEAD changed during hooks. The refs snapshot excludes backup refs.
- CLI tests verify the `amend` receipt fields and backup ref. egui tests load the HEAD message, show the commit contents as staged, unstage and restage working changes, rewrite the index, clear the toggle and amend base, and render the **Amended** status line.
- Commit readiness tests cover the disabled reasons for a running operation, a pending scan, an empty or whitespace message, and no staged files; message-only amend remains enabled. The amend banner names the affected commit and subject, and the message editor expands and collapses.

## M6 reflog

- Git oracle tests compare `HEAD` and branch reflog order, selectors, IDs, times, actions, and details with `git reflog show`; missing and option-shaped references are rejected.
- CLI tests verify `git good reflog [REF]` JSON, the default `HEAD`, entries after an amend, and missing-reference errors.
- Reflog tests verify read-only loading, cleared stale entries on failure, and the native window at 720 and 1120 widths with the reference selector, entry list, reset actions, branch creation, and copy.
- Branch creation at a reflog commit resolves the revision, rejects an existing branch, and does not switch the current branch. The reflog branch-creation dialog renders at both supported widths.

## M6 rebase

- Git oracle tests rebase a diverged local branch, verify the rewritten parent, preserved subject, and backup ref pointing at the previous tip, and accept local branch, tag, and remote-tracking references.
- Conflict fixtures verify `rebase-merge`/`rebase-apply` detection, `REBASE_HEAD` operation state, conflict paths, abort restoring the original tip and worktree, continue after staging, and skip completing without the conflicting commit.
- Dirty tracked worktrees are rejected unless autostash is explicitly enabled; `rebase.autoStash=true` does not override the backend's `--no-autostash` decision. Option-shaped and missing references are rejected.
- CLI tests cover `git good rebase REF [--autostash]`, receipt fields, the backup ref, dirty/autostash behavior, conflict abort/continue, and unknown-option errors.
- egui tests render the **Rebase branch** dialog with the autostash choice and backup warning, plus the in-progress banner with Continue, Skip, Abort, and Backups… at 720 and 1120 widths.

## M6 reorder, squash, and backups

- Git oracle tests list a linear range oldest first, reorder commits, squash a commit into the previous row while preserving the combined message, rewrite a root range through `--root`, and point a private `refs/gg/backup/rewrite/...` ref at the previous tip.
- Rewrite validation rejects an empty plan, a squash-first plan, duplicate commits, missing commits, commits outside the range, non-ancestor starts, option-shaped starts, and ranges containing merge commits. Merge ranges are rejected rather than flattened, and a shallow boundary is rejected instead of being rewritten as a root.
- Conflict fixtures verify the rewrite leaves Git rebase state, lists unmerged paths, aborts back to the previous tip and worktree, and skips the conflicting commit to complete the remaining range. Dirty tracked worktrees are rejected unless autostash is explicit, even with `rebase.autoStash=true`; autostash restores the working change afterward.
- CLI tests cover `git good rewrite --from COMMIT --pick COMMIT... --squash COMMIT...`, reorder and squash receipts, the backup ref, conflict abort, skip, and unknown-option errors. `git good rewrite --continue|--skip|--abort` targets the same Git rebase state.
- Backup tests assert `gg_git::backup::list` returns the private reference, kind, commit, subject, and commit time while the refs snapshot continues to exclude `refs/gg/`.
- egui tests render the **Reorder and squash** modal with ordered rows, up/down icon controls, Squash choices, autostash, and the backup warning at 720 and 1120 widths. A pending rewrite keeps the modal open with **Rewriting commits…** and ignores Escape; a pending refresh shows **Refreshing history…** and the modal closes only after the replacement history generation is accepted. The Backups window renders the entry list, Open commit, Reset branch here, Create branch here…, Copy hash, and Copy ref at 1120 width. The in-progress banner exposes Backups… and names an in-progress rebase.

## M7 file reading surface

- Engine file tests cover exact line splitting with CRLF and a missing final newline, changed markers for each side from unified rows, contiguous change groups, empty/binary/over-budget summaries, and one-based inclusive `Place` normalization with exact path bytes.
- Git fixture tests read the selected side, mark added and deleted lines, and return explicit absent and gitlink summaries without reading objects for a missing side. Cancellation is checked before the object read.
- Selected-file worker tests cover generation supersession, retained-document paging, line-centered restore pages, one full-file read per mode and side, changed markers derived from the diff rows, and hunk indices for change navigation.
- egui tests cover first-change cursor adoption, line retention across Diff/Full and Before/After, change position and jumps with clamped ranges, stale-page retention, and the rendered reading header at 720 and 1120 widths. Alt+Up/Down changes the selected file while remaining in reading mode.
- Change navigation starts from an explicit before-first position, so `Next change` reaches the first hunk or changed group even when it begins at the first row or line. Tests cover an unset cursor, the first and last changes, and a single change in both modes; the buttons are enabled only when a target exists in their direction, and the `]` key is driven through the rendered view to confirm an off-screen hunk scrolls into range.
- Native X11 captures in [verification](verification.md#m7-file-reading-surface--2026-10-24) cover the wide/narrow full file, both sides, change jumps, and next-file navigation. Clipboard delivery for double-click copy remains a platform check.

## M7 file history

- Engine tests cover the version-cursor boundaries (`Only version`, `Oldest version`, `Newest version`, `Loading earlier versions…`, limit, and pending choice), predecessor linkage to the adjacent older version only, and edge kinds for add, delete, rename, type change, and content change. Position marks an unloaded older version as loading rather than complete.
- Git oracle fixtures cover a linear history that skips commits carrying unchanged content, an unchanged commit resolving to the content-state introducer, rename detection with the old path, delete/re-add as an explicit absent version, a one-sided merge that follows the same-state carrier, a two-sided merge that offers every predecessor newest first and follows the chosen one, a root addition as the only version, non-UTF8 path bytes, and cancellation/retry keeping loaded versions and the pending boundary.
- File-history worker tests cover bounded preload to the oldest version and discarding the previous session when another file opens. The worker is generation-scoped, cancels superseded work, and reports busy and error updates.
- egui tests cover the version header and counter, Before/After boundaries that disable with a visible reason and never wrap, the predecessor chooser, Old/New labels on the embedded reading surface, Full-only rendering while the earlier side is unknown, `Alt+Left/Right`, and the rendered header naming the old path. Review integration opens history from reading with `H` and backs out one level with `Esc` while keeping the file list and comparison state.
- Native X11 captures in [verification](verification.md#m7-file-history--2026-10-25) cover wide and narrow history, the oldest/newest boundaries, the predecessor chooser, and Full/Diff on one edge.
