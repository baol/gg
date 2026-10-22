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

## Remaining coverage

- Add integrated streamed filtering tests for preserved selection, first-match fallback, and zero results with delayed batches.
- Verify worker replacement/drop cancellation, temporary-file cleanup, and storage failures after partial delivery.
- Verify revision timestamp ties, non-UTF-8 display/native paths, and SHA-256 repositories.
- Implement and test the specified external-change refresh indication; explicit refresh and pinned history are already covered.
- Complete the [preview acceptance checks](verification.md#preview-acceptance-pending). Native integration and performance evidence complement in-memory tests.

## Increment gate

Run the full [development checks](development.md) for every increment. Use offline fixtures independent of user Git configuration. Exercise production worker/input paths; retain scenarios through refactoring and update expectations with explicit contract decisions.

Before connecting M1 controls, test Review/Back anchor restoration, active-view shortcuts, independent worker cancellation, and late-result isolation. Run these alongside the M0 suite.

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

Split stage view: open from either a staged or unstaged row and require both panes at 720 and 1120 widths. Verify immediate adoption under hover, inline **Stage hunk**/**Unstage hunk** actions, hunk-header and individual-line selection, **Drop hunk**/**Drop selected**/**Drop all** with confirmation and cancellation, whole-file fallbacks, in-place refresh after success, status refresh on Back, and retained panes/error/index after failure.

Committing: compare committed trees, parents, messages, and reflogs with Git for root, normal, detached-HEAD, and linked-worktree cases. Verify that only index content is committed, unstaged working bytes survive, and history selects the new commit after Back. Cover empty indexes, empty messages, conflicts, intent-to-add, sparse indexes, stale HEAD/index observations, identity resolution, and lock ownership.

Commit hooks: fixtures assert `pre-commit`, `prepare-commit-msg`, `commit-msg`, and `post-commit` execution order and arguments, including `core.hooksPath`. A pre-commit hook may stage content; a message hook may rewrite `COMMIT_EDITMSG`. Hook failures must preserve the index, working tree, and message, report the hook name/status/output with a retry path, and refresh status when hook staging occurred. A failing post-commit hook returns a warning with the created commit. CLI and egui tests cover receipts and draft retention; native review covers history refresh.

Navigation and diff continuity: Enter opens commits at both widths; text editors retain keys while focused buttons allow navigation. Reading keys scroll without changing files. History/review file selection retains displayed rows until current-generation adoption; hover defers replacement there. Working stage panes adopt each result immediately. Toolbar geometry covers Repository, View, and Status.

## M3 graph and refs

Ref snapshots: compare HEAD, local branches, remote branches, and annotated/lightweight tags with Git for normal, unborn, and detached repositories. Verify peeled commit IDs, symbolic remote HEAD omission, exact names, and non-mutation of refs, index, and working files. CLI `refs` output must match the shared snapshot.

Graph layout: unit tests cover linear history, merge parents, lane reuse, joins, and lane retirement. Worker tests verify graph rows stay aligned with commits when the query filters rows, and egui tests require the graph column and ref labels at 720 and 1120 widths. Long or numerous refs must remain single-line with an explicit overflow count. History traversal includes commit targets from every local branch, remote branch, and tag; a `git rev-list --all` oracle and a parent-after-child assertion protect the all-ref set and topological order.

Ref navigation: the Refs sidebar groups branches, remotes, and tags as collapsible trees at wide widths; the Refs menu remains the narrow fallback. Both mark the symbolic HEAD branch, filter by ref name, switch local branches, and select a remote/tag commit without mutating the repository. Jumping must clear a hiding filter, recover the correct row after traversal completion, preserve the selected anchor, and reveal deep targets. History labels render as pills with the current branch first, icons, width-aware overflow, and a readable subject. A single pill click selects its commit; a double-click switches to a local branch or jumps to a tag/remote commit. Duplicate loose/packed refs must appear once. Native wide/narrow review verifies graph lanes, labels, sidebar, menus, and selection.

## M3 branch switching and creation

Branch safety: switch between local branches and create from the current HEAD only on a clean worktree. Cover unstaged, staged, and untracked inputs, missing branches, duplicate names, invalid names, detached HEAD starts, and unborn repositories. Verify HEAD, index, and working files are unchanged on every rejection.

Branch actions: compare the checkout result with Git for worktree content, symbolic HEAD, and post-checkout hooks, including a failing hook that leaves the branch switched but returns a warning. Verify `git good switch [--create] BRANCH` receipts/errors, Refs-menu HEAD marking, modal creation, branch-switch refresh of history/refs, and dirty-tree recovery. Tags and remotes remain read-only jumps; branch deletion, rename, and remote tracking are outside this increment.
