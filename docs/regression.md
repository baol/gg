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

- Geometry tests find every window toolbar icon, in a fixed left-to-right order and at identical positions in History and Local Changes, inside 720 and 1120 widths. Pull is ready only when the current branch has an upstream and is behind it; otherwise it names the reason. The Changes tab badge counts changed paths (a rename once). History, Review file, and Local Changes rows are 22 px with one-character change markers; the reader keeps path and endpoints on one line. `Ctrl+K` opens search; typing then Enter filters History, Down then Enter runs an action, and `Ctrl+K` again closes it. Search actions share the toolbar's readiness, refs inspect their commit, and the Search button stays clear of the toolbar icons at 720 and 1120 widths. A modal decision dialog absorbs clicks and navigation keys behind it (History selection is unchanged until it closes). The current-branch button opens a switcher at 720 and 1120 widths listing local branches only, current first and disabled; the sidebar's Stashes group opens the picker on the clicked entry and follows the refs filter. Stash changes is disabled on a clean worktree; the Stashes picker lists entries with Apply, Pop, Show changes, and Drop…. The toolbar rewrite starts at the branch's first own commit (Git fixture: unpushed commits, then commits beyond the default branch) and leaves the history selection unchanged.
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

## Coherence I3 Local Changes layout

- Geometry tests render at 1120×740 and 720×480 and assert Unstaged above Staged, a fixed 300 px wide file list, a Staged header that stays put when Unstaged empties, start-elided long paths that keep the file name, *No unstaged changes.* and *No staged changes.* only after a finished scan, both groups beside or above the reader, and the commit/amend form below the reader. Group headers name both lists and their counts; rows show change kind and path, with explicit Untracked/Conflict badges and a stronger selection fill for the active list. The Split control, View diff step, and **Review & stage** header are absent.
- Production-path worker tests page the Unstaged and Staged viewports independently. The Git scan oracle asserts every `Area::Staged` row precedes all unstaged, untracked, and conflict rows, and the worker maps working pages back to their own list coordinates. Bounded rows, cancellation, hover-safe adoption, stale-result rejection, and refresh superseding a parked previous scan remain covered.
- A real Git fixture stages one change and leaves another in the same path. The path appears in both groups; selecting the Staged entry requests HEAD → index and selecting the Unstaged entry requests index → working file. Amend selections name HEAD parent → index. Conflict selections read the unmerged summary with an area-specific route and no editable actions; untracked entries name empty → working file.
- Whole-file, hunk, and selected-line tests verify the expected index content while an unrelated staged path and working bytes stay unchanged. Discard tests confirm cancellation, unchanged index bytes, and the selected block/file scope. Conflict entries remain non-editable with an explicit unmerged summary.
- After stage, unstage, and discard, both lists and the active comparison refresh in place. A staged-away tracked or untracked entry keeps its path with an explicit empty comparison and no stale actions until another entry is selected. List selection and scroll offsets remain independent, and refresh or draft restoration resolves an off-page selection through the worker lookup.
- Commit/amend tests cover the lower-right form, a visible readiness reason, separate per-worktree drafts, message-only amend, the affected commit and base banner, hook-failure draft retention, post-commit warnings, backup refs, and clearing only the submitted draft.

## Coherence I4 file history inside the reader

- `H` and the visible **History** button enter history without leaving the Review reading layout. In the wide layout the file list stays in its place beside the same reader; in narrow reading the strip appears above the same full-width reader. History no longer renders a separate full-screen content view.
- Opening history captures the entry comparison, exact path, revision, endpoint labels, presentation, inspection side, cursor/range, and pixel place. Closing with its button or `Back`/`Escape` restores that reader context and file selection once, even when the entry row has left the bounded list cache. Version steps change only the reader source and add no Back/return-location step.
- The version strip exposes **Older version** and **Newer version**, the version counter, kind, commit, subject, author, date, and old path. Both directions disable with a visible reason and never wrap. Before/After remain inspection-side controls in Full file and never move versions.
- A unique predecessor is followed automatically. An ambiguous predecessor renders the chooser in the same strip; Diff remains disabled until the earlier edge is resolved while the current Full file stays readable. The chooser names every candidate without taking the first parent.
- Selecting another file while history is active keeps the strip open, resets the version cursor for that file, and saves the original entry context; the previous path's version cursor is never reused. Closing history restores the original entry file and reader place. Switching parents leaves history.
- `Escape`/`Back` dismisses the chooser/history source first, then the reading view, then the file list/review. `H` while history is open does not stack another history source.
- Rename/old-path, deletion/absence, unchanged-commit skipping, bounded preload/paging, trace limits, retry, cancellation, stale generation rejection, and read-only behavior remain covered by the existing Git oracles, worker tests, and repository snapshot checks.
- Geometry/interaction tests render the strip and reader at 1120×740 and 720×480, assert the wide file list and narrow reading layout, exercise exact context restoration and the another-file rule, and cover the chooser, boundaries, retry, and rapid version steps.

## Coherence I5a one file workspace

- Local Changes renders the selected Staged, Unstaged, Untracked, or Conflict comparison on the shared reader used by review and file history. Selecting either list entry selects its comparison immediately; the reader's Unstaged | Staged toggle switches the same path between both comparisons, selects its row in the other list, and disables a side without changes with its reason at 720 and 1120 widths; there is no **View diff** step, second content renderer, or narrow metadata panel with a duplicate open action.
- The file header appears once: exact path, one route or exact endpoints, Diff/Full file, and change navigation. Whole-file Stage/Unstage/Discard, export, copy path, and the File details toggle live behind **File actions**; modes, object IDs, byte sizes, and the worker note live in **File details** on demand. Permanently repeated path, route, mode, and size rows are absent, as is the duplicate Local Changes heading and repository path above the workspace.
- Working full file reads the same two sides on the shared selected-file worker: base tree and index for Staged, index and normalized working bytes for Unstaged, empty and working bytes for Untracked, and an explicit summary for Conflict. The 2 MiB/100,000-line budgets, symlink target text, gitlink summaries, binary summaries, cancellation, and stale repository rejection remain covered by Git oracles.
- Inline **Stage hunk**/**Unstage hunk** and **Discard hunk**/selected-line actions derive the same edits from the shared reader rows. Hunk actions apply the whole hunk; the selection bar below the reader applies picked lines across hunks as one mutation, counts them (**Stage N lines**, **Unstage N lines**, **Discard N lines**), offers **Discard** only on the unstaged side, and **Clear** forgets the picks without moving rows, at 720 and 1120 widths; a discard expands every touched change block. Whole-file, hunk, and selected-line mutations still preserve unrelated index entries and working bytes.
- A successful mutation refreshes both lists and the active comparison, keeps the reviewed path when it still exists, and disables duplicate whole-file and hunk actions while pending. Discard confirmation freezes keyboard list navigation; Escape or Cancel dismisses it without touching the index or working files, and pointer selection of another entry also dismisses it without publishing the pending discard. A disappeared path keeps its name and area with an explicit empty comparison and no stale actions. Normal/amend drafts and independent list scrolling remain unchanged.
- Review shows the comparison subject in one compact identity strip; the reader owns the path and exact endpoints. File history suppresses that review identity strip while active, shows its version strip above the same reader, and keeps the close control in the strip. The change navigation uses compact **< Prev**/**Next >** labels so the controls fit the wide split and narrow reader.
- `Tab` moves keyboard focus between the two groups and selects the other group's remembered row, while a focused text editor keeps normal keys. Binary working comparisons show a named summary and a no-inline-preview note instead of "No changes"; mode-only changes show the mode note in the header and details; File details names the working modes and worker note.
- Geometry and interaction tests render Local Changes at 1120×740 and 720×480, assert one reader header and the on-demand details/action surface, exercise both full-file working sides, and retain the whole-file, hunk, line, discard, refresh, amend, and draft scenarios.

## Coherence I5b operation area

- One operation area renders directly beneath the navigation strip, hidden when idle. It is the only place for running labels, progress, cancellation, and supported continuation controls; the status line does not duplicate them while it is active. Running continuations show their worker label instead of a stale needs-attention strip, and the needs-attention label names the operation commit.
- Merge shows Resolve conflicts, a gated Continue, and Abort without Skip. Cherry-pick, revert, and rebase show Skip. Continue stays disabled until no unmerged paths remain. Stash conflicts never add sequencer controls. The conflict window owns source inspection, read-only preview selection, and the single Stage resolution publication.
- Rewrite submission closes the plan. The area reports Rewriting commits, then Refreshing history until the replacement generation loads; browsing remains available, and the filter header labels the retained rows **Refreshing · showing previous history**. Replacement rows wait for the hover/scroll-safe boundary. Failures clear the area and report an error outcome whose **Backups…** action remains reachable.
- Network operations show phase/percent/message with Cancel in the area; branch operations show their label there. Duplicate submission is refused and incompatible mutations stay disabled for the duration.
- Opening a repository with an existing operation detects it on the first completed history load. Switching repositories (including a successful clone) drops the previous workers, network/stash/branch notices, outcome, conflict view, and refresh state; old operations are not re-detected while the requested history is still loading, late results are ignored, and the new repository is detected independently. Open is disabled while work owns the area, including the path-field Enter route.
- Local Changes disables staging, discard, commit, amend, and duplicate mutations while the area owns a rewrite, operation worker, network, or branch operation, but keeps list selection, scrolling, Back, and Refresh available for browsing. Network work cannot start during an in-progress merge/rebase; cancellation lives only in the area.
- Pending Local Changes mutations retain their view and block History, repository Open, and worktree Open until completion. Worktree switching is also rejected while a branch or network operation is active, so late results cannot cross repository contexts.
- Geometry/interaction tests cover the area at 720×480 and 1120×740, the merge-without-Skip rule, one-area ownership, rewrite lifecycle, network cancel, and outcome/recovery behavior.

## Coherence I6 conflict preview and Stage resolution

- Base, Ours, Theirs, and the working file select only the Result preview; no selection writes the working file or index. The working file is the default preview and external edits require **Refresh external edits**.
- **Stage resolution** is the only publication control. It validates the conflict stage IDs and exact working bytes captured at preview time, writes the chosen result or stages the unchanged working file, and stages the path. Stale stages or working bytes reject without publishing and surface the error in the conflict workspace and status line; a worktree write followed by a staging failure is reported as partial. It is disabled while another operation owns the area or the preview source has no content.
- Wide layouts keep Base/Ours/Theirs columns with a bounded Result preview; narrow layouts use Base/Ours/Theirs/Result tabs. The operation area owns Continue/Skip/Abort; the conflict workspace owns source inspection and publication.
- Publication runs on the operation worker and refreshes mutation state, conflict listing, history, and status; the next unresolved path is selected after success. Path traversal, absent sides, and mode/symlink/submodule publication fail explicitly without writing. Oversized regular working files use an exact streaming hash over every byte without a text preview; oversized side blobs publish and pass the same post-write verification. Symlink, directory, and submodule worktree paths keep side previews but disable a Working preview with a reason.
- Git oracles cover read-only preview, Base/Ours/Working publication, stale stage and working data, exact staged output, and path rejection. The egui integration opens a real conflicted merge, previews both sides without writing, then stages and verifies the file and index.

## Coherence I7 repository tools and pending intent

- One **Repository → Recovery…** window owns a Backups/Reflog source selector. The separate Reflog and Backups windows are not reachable from the product; each source keeps its identifiers, provenance, confirmation routes, and worker behavior.
- **Open commit** from either recovery source opens the ordinary commit review. Closing Review restores that recovery source, the Reflog reference filter, and the selected entry; it does not navigate to History. Reflog uses its selector and Backups uses its private reference as the entry identity, so duplicate backups at one commit stay distinct. Review names its return target **Back to recovery**, and the Recovery window is closed while Review is open.
- Returning from Review to a non-HEAD Reflog filter reloads that reference before reselecting the entry, so `main@{0}` is selected under **Reference: main** rather than silently falling back to HEAD.
- One shared branch form serves New branch and recovery Create branch here, titled for its caller. It names the start commit/selector, validates the full local ref, and exposes an explicit Switch after creation. New branch defaults on; recovery defaults off. Creation happens before any switch, reports success on the active recovery source, and names the existing branch when the follow-up switch is blocked.
- A blocked switch keeps its target as intent and shows a warning outcome with Retry switch and Dismiss. Retry revalidates by starting the switch again; the intent never fires automatically. Success clears the warning and intent, Dismiss clears them without switching, and a branch created before a blocked switch remains.
- **Repository → Stashes…** opens the stash window, which offers one Restore… form with Restore staged state and Remove stash after successful restore; Drop stays a separate confirmed action. Failed restores keep the form and entry in a bounded error region, successful restores report Restored, and stash conflicts add no sequencer controls. The Help window no longer advertises a top-level Stash menu.
- Tests render the Recovery source selector and caller-specific branch/stash forms at both widths, reselect source-specific entries (including duplicate backups at one commit and a non-HEAD Reflog filter), assert Review's recovery back label and return location, verify the unborn New-branch reason, keep a long restore error inside its scroll viewport, assert the Restored outcome, and reproduce blocked-switch intent through an idle frame before an explicit retry starts a new worker; success clears the warning and the created-branch partial state is reported.

## Coherence I8 closeout regressions

- A real-fixture egui geometry test opens Recovery at 720×480, asserts the window rectangle stays inside both axes, and asserts Open commit and Copy ref still render. The default width is the smaller of 760 pixels and the available content width minus 24 pixels.
- A reader regression reproduces the narrow file-list → reader transition with the resting pointer already over the reader. An explicit new selection publishes its diff once; the existing asynchronous-replacement test still waits for hover to clear.
- A status-line test asserts the wide hint includes paging and copy ID while the narrow hint keeps `j/k`, review, details, history, and filter without clipping.
- The Help window test rejects the retired `View, Stash, Help` menu sequence. The deferred-element list in [ui-coherence](ui-coherence.md#deferred-elements) covers LFS smudging/materialization, restart-persistent drafts, and the dedicated conflict workspace.

## M1 changed files

- Git-oracle fixtures cover root/first-parent changes, merge-parent selection, nested paths, additions/deletions, IDs/modes, file-to-directory replacement, empty comparisons, invalid parents/revisions, and pinned comparisons with unavailable blobs.
- Process tests cover nested discovery, explicit repository, exact path bytes, JSON, and error exits.
- Controlled reader tests cover early delivery, 10,000-file completion, deep/back page reload, retained partial errors, and worker shutdown.
- Input tests cover review/history isolation, late history results, Back anchors, fixed rows, and hovered placeholders at wide/narrow sizes. `unread_rows_stay_empty_and_loaded_rows_stay_put` checks that unread Review rows keep empty slots, loaded rows stay visible, and no loading or resolving text appears in the pane.

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

## Local Changes staging gestures

- Double-click an Unstaged or Untracked file row to stage the whole file; double-click a Staged row to unstage it. Conflict and intent-to-add rows do not toggle.
- The selected file header has its own Stage/Unstage button with a disabled reason until content settles or while a mutation is pending; File actions no longer duplicates Stage/Unstage file, while Discard file… and reader hunk/selected actions remain available.
- After a whole-file stage, select the next Unstaged row in display order, or the previous row when the staged row was last; with no Unstaged rows, keep the newly staged row selected. After a whole-file unstage, follow the path into Unstaged. Failures retain the current selection.
- An egui timing test covers three-file staging, double-click stage and unstage, auto-advance, last-file fallback, and both header buttons against a real Git index. Native captures cover wide Stage/Unstage and narrow Stage.

## M2 status and working diffs

- Git porcelain fixtures cover independent areas, ignores, conflicts, intent-to-add, discovery, and repository preservation. Bounded-worker tests cover cancellation and revisiting distant pages.
- Working comparisons reconstruct each side and match Git line counts. Fixtures verify normalization, symlink targets, binary/budget summaries, stale HEAD/index, and files changing during a read.
- In-memory input tests preserve hovered slots, last-page geometry, history anchors, and status selection/offset through working-diff navigation. Working comparisons publish the active comparison as soon as it is ready, even with the pointer over the view. CLI tests require an explicit area for selected paths and verify JSON and failure output.

File staging: compare staged entries and subsequent trees with `git add -A`; preserve working bytes, mode bits, symlink targets, and unrelated staging. Cover additions, modifications, deletions, type changes, intent-to-add, text attributes, emulated symlinks, unborn branches, SHA-1/SHA-256, stale HEAD/index/file observations, lock ownership, submodules, file/directory collisions, CLI receipts/errors, and worker refresh/failure.

Whole-file unstaging: compare restored entries and subsequent trees with Git; preserve working bytes and unrelated staging. Cover unborn branches, SHA-1/SHA-256, stale observations, lock ownership, file/directory collisions, CLI receipts/errors, and worker refresh/failure.

Hunk and line staging: compare selected index blobs with scripted `git add -p` in SHA-1/SHA-256 repositories. Cover whole-hunk and individual-line selections, paired old/new lines, partial selection with zeroed stat and `MM` status, all-change selection with real stat, multiple selections, CRLF attributes, missing final newlines, unrelated staged entries, binary/budget limits, symlink targets, mode changes, untracked/intent-to-add/deleted inputs, stale headers/lines/HEAD/index, lock ownership, and CLI `--hunk` receipts.

Partial unstaging: stage regular text additions, modifications, and deletions, then reconstruct selected lines from HEAD and index. Verify that restoring every line restores the HEAD entry, removing every line of an added file removes the index entry, and partial results write zeroed stat while working bytes stay untouched. Cover SHA-1/SHA-256, stale HEAD/index, lock ownership, CLI/worker errors, and mode/type/symlink fallbacks.

Dropping working changes: drop a whole hunk, selected block, and whole file from the Unstaged comparison; verify the index bytes are unchanged and other hunks survive. Restore modified, deleted, executable, and symlink content from the index, remove untracked files, and leave working bytes untouched on stale or unsupported input. Cover confirmation and cancellation, mode/type fallbacks, filtered line endings, lock ownership, and errors.

Local Changes layout: Unstaged and Staged lists keep independent selection, scroll offsets, viewports, and bounded row caches. Selecting either list selects its comparison immediately: index → working for Unstaged, empty → working for Untracked, the unmerged summary for Conflict, HEAD → index for Staged, and HEAD parent → index in amend mode. A partially staged path appears in both lists and its two entries select distinct comparisons. Verify the status worker pages both list viewports independently while preserving the staged-first boundary, bounded rows, cancellation, hover-safe adoption, and stale-result rejection. Verify whole-file **Stage file**/**Unstage file**/**Discard file**, inline **Stage hunk**/**Unstage hunk**, selected-line actions, and **Discard** confirmation and cancellation leave unrelated index entries, working bytes, and other staged paths unchanged. Mutations refresh both lists and the active comparison in place; a disappeared active tracked or untracked entry retains its path with an explicitly empty comparison and no stale actions, and names the other side with the toggle when the path is still changed there. Geometry tests prove Unstaged precedes Staged at the left/top, the reader is beside or below both groups, and the commit/amend form is below the reader at wide and narrow widths. The old side-by-side/stacked **Review & stage** view and View diff step are absent.

Committing: compare committed trees, parents, messages, and reflogs with Git for root, normal, detached-HEAD, and linked-worktree cases. Verify that only index content is committed, unstaged working bytes survive, and history selects the new commit after Back. Cover empty indexes, empty messages, conflicts, intent-to-add, sparse indexes, stale HEAD/index observations, identity resolution, and lock ownership.

Commit hooks: fixtures assert `pre-commit`, `prepare-commit-msg`, `commit-msg`, and `post-commit` execution order and arguments, including `core.hooksPath`. A pre-commit hook may stage content; a message hook may rewrite `COMMIT_EDITMSG`. Hook failures must preserve the index, working tree, and message, report the hook name/status/output with a retry path, and refresh status when hook staging occurred. A failing post-commit hook returns a warning with the created commit. CLI and egui tests cover receipts and draft retention; native review covers history refresh.

Navigation and diff continuity: Enter opens commits at both widths; text editors retain keys while focused buttons allow navigation. Reading keys scroll without changing files. History/review file selection retains displayed rows until current-generation adoption; hover defers replacement there. Local Changes list pages use hover-safe adoption and the active working comparison adopts each result immediately. Toolbar geometry covers Repository, View, and Status.

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
- egui tests render the operation area with Continue, Skip, and Abort at 720 and 1120 widths.

## M5 merge and conflicts

- Git oracle tests cover fast-forward, already-up-to-date, diverged merge commits, forced `--no-ff` merge commits, conflict detection with abort and continue, and option-shaped or missing references.
- CLI tests cover JSON receipts for merge, fast-forward and merge-commit kinds, conflict reporting, `--abort`, `--continue`, `--no-ff`, and unknown-option stderr.
- egui tests render the **Merge branch…** dialog and the merge operation area at 720 and 1120 widths; merges hide the unsupported Skip action.

## M5 conflict resolution

- Git oracle tests cover unmerged path listing, base/ours/theirs content, side selection with staging, manual staging, add/add conflicts without a base, and unknown-path or base-side rejection. A nested `--repo` fixture stages only the repository-relative conflict and preserves an identically named untracked nested file.
- CLI tests cover `conflicts` listing, `conflict --file` content, `--use ours|theirs`, `--stage`, and missing-path errors.
- egui tests render the **Resolve conflicts** window with path selection, operation-aware Base/Ours/Theirs labels, and side tabs at 720 and 1120 widths. Worker tests deliver conflict content to the open window without blocking, route side resolution through the operation worker, and prefer the next unresolved path after a resolution.

## M5 recovery and operation state

- Git and CLI tests cover continue, skip, and abort for cherry-pick, revert, and merge, including merge's unsupported skip and restored HEAD/worktree on abort. Abort without an operation and continue with a failing message hook return Git's error instead of a success receipt.
- The native operation area exposes Continue, Skip, and Abort, hides Skip for merges, and shows the conflict count.
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
- CLI tests verify the `amend` receipt fields and backup ref. egui tests load the HEAD message, show the commit contents as staged, unstage and restage working changes, rewrite the index, clear the toggle and amend base, and render the **Amended HEAD → \<id\> · N files** status line; a normal commit renders **Committed \<id\> · N files**.
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
- egui tests render the **Rebase branch** dialog with the autostash choice and backup warning, plus the operation area with Continue, Skip, and Abort at 720 and 1120 widths. Backups remains in Repository and in outcome recovery.

## M6 reorder, squash, and backups

- Git oracle tests list a linear range oldest first, reorder commits, squash a commit into the previous row while preserving the combined message, rewrite a root range through `--root`, and point a private `refs/gg/backup/rewrite/...` ref at the previous tip.
- Rewrite validation rejects an empty plan, a squash-first plan, duplicate commits, missing commits, commits outside the range, non-ancestor starts, option-shaped starts, and ranges containing merge commits. Merge ranges are rejected rather than flattened, and a shallow boundary is rejected instead of being rewritten as a root.
- Conflict fixtures verify the rewrite leaves Git rebase state, lists unmerged paths, aborts back to the previous tip and worktree, and skips the conflicting commit to complete the remaining range. Dirty tracked worktrees are rejected unless autostash is explicit, even with `rebase.autoStash=true`; autostash restores the working change afterward.
- CLI tests cover `git good rewrite --from COMMIT STEP...`, reorder, squash, reword, fixup, drop, and edit receipts, the backup ref, conflict abort, skip, and unknown-option errors. `git good rewrite --continue|--skip|--abort` targets the same Git rebase state.
- Backup tests assert `gg_git::backup::list` returns the private reference, kind, commit, subject, and commit time while the refs snapshot continues to exclude `refs/gg/`.
- egui tests render the **Interactive rebase** modal at 720 and 1120 widths: header with range and result count, newest-first plan with action chips, the selected commit with Open in Review, autostash, and the backup notice. Keys choose actions (a fixup with nothing below is refused with the reason), badges name the combination target, Esc cancels without starting, and Enter opens Review with **Back to rebase** and returns to the same plan. `rebase_plan` unit tests cover targets, badges, summaries, reword messages, the last kept commit, and step order. Review keeps Back, the commit identity, and the file count on one header line. At a reopened-commit stop, egui tests at 720 and 1120 widths check that Local Changes opens with the original message, the operation area names the stop and step (`Rebase stopped · step 2 of 3 · reopened <id>`), Continue waits with its reason and ignores `Ctrl+Enter` while changes remain, the identity reads `feature · rebase 2/3`, and commits made at the stop are listed. Git-fixture tests check that Reopen stages the commit's changes on its parent, that progress names done and remaining steps, that a split continues, that a root commit stays at HEAD, and that a conflict stop is reported. Conflict blocks: engine tests parse `diff3` output and compose each choice with markers kept for open blocks; Git-fixture tests split a two-block conflict regardless of `merge.conflictStyle`, stage composed text, refuse a changed working file, and name why a deleted side has no blocks; an egui test on a real conflict chooses with key 2, shows the block labels and result count, and leaves the file untouched until staging. Rebase dialog keys keep working after a button takes focus. The Stashes picker test checks ref, message without the branch prefix, branch or **detached**, age, and file count, and that the popup keeps its size when **Drop…** swaps in the confirmation; a Git-fixture test reads the branch, age, and file count (tracked, staged, and untracked) for a branch and a detached stash. Preferences: settings tests round-trip every preference, keep unknown keys, and reset only a malformed value; folder tests migrate `roots.json` with its default, reorder, count repositories, report a missing folder, refuse relative paths, and prefer the first folder after removing the preferred one; an egui test at 720 and 1120 widths lists the sections, a repository count, and a missing folder, names `Already in the list.`, and saves a theme change immediately while the window stays open and in place. A submitted rewrite closes the configuration; the operation area shows **Rewriting commits…**, then **Refreshing history…** until the replacement history generation is accepted, and Escape is no longer needed to dismiss a blocking modal. The Backups window renders the entry list, Open commit, Reset branch here, Create branch here…, Copy hash, and Copy ref at 1120 width. The operation area names an in-progress rebase; Backups stays in Repository and outcome recovery. Preference memory: settings tests round-trip the reader presentation, image layout, signing default, details pane, stash, merge, new-branch, and fetch-prune keys and keep defaults for absent keys; a reader opens in the preferred presentation once and keeps a later explicit choice; the image layout starts from and remembers the last choice per window; the Sign checkbox follows the Commit preference; and stash, merge, and new-branch forms open with remembered values, save nothing until submission, and save the submitted merge choice. The Preferences window shows the Diff and Commit sections at 720 width. Side-by-side diff: `split_rows` unit tests pair removed with added lines, pad the shorter side, keep an unpaired change on its own side, start a new run when a deletion follows an addition, map every unified row to the display row that shows it and back, and page both sides of the visible rows; an egui test at 720 and 1120 widths finds the Inline and Side by side toggle, both line number columns, a deletion beside its addition, context in both columns, a padded cell, and one hunk header; another keeps `[`/`]` change navigation across hunks; settings tests round-trip `diff_layout` with Inline as the default.

## Force push with lease

- Normal push remains non-force; the separate Repository action is the only force route and is disabled without an upstream remote-tracking ref.
- The confirmation names the local branch, destination remote/branch, and lease commit and states that a changed remote rejects the rewrite. Bare `--force` is not parsed.
- The engine passes the exact lease commit captured by the confirmation as `--force-with-lease=refs/heads/<remote-branch>:<lease>`; the CLI resolves that lease from the remote-tracking ref immediately before pushing. Missing and stale leases fail without changing the remote, including when a fetch updated the tracking ref after the dialog captured an older lease. A stale lease does not offer Retry push (which would become non-force).
- Git fixtures cover stale rejection with the remote unchanged, fetch-updated lease success, receipt lease identity, and CLI rejection of bare `--force`. Native captures cover the dialog, a successful leased rewrite, a stale-lease failure, and the narrow dialog.

## M7 image onion skin

- **Onion skin** appears only in wide layouts alongside Side by side and Swipe; narrow layouts keep stacked before/after and do not expose the control.
- Before renders fully and After overlays with normal alpha at the default 50%; the labeled slider responds to keyboard arrows when focused, and the current percentage is shown beside the control and in the canvas metadata.
- Fan-out keeps each image at its own fitted or actual size without stretching; the checkerboard remains behind the canvas and per-side metadata/export are unchanged.
- Onion skin requires both images. A one-sided pair shows that both are required and keeps the absent-side state instead of blending.
- Alpha conversion clamps to 0–255 and native captures cover 50%, keyboard-adjusted 70%, and the narrow stacked layout.

## M7 commit and tag signing

- Raw verification parsing separates good, bad, unknown key, unsigned, and error; Git fixtures cover unsigned commits and annotated tags, non-commit/tag objects, and invalid references. Unsigned tags do not become errors just because Git prints “no signature found”.
- Signing configuration reads format, key, program, commit/tag signing, allowed signers, and tool availability without writing configuration; note and readiness follow the missing key/tool.
- A signed commit without a working signer fails with “Signing failed” and leaves HEAD, the index, and staged content unchanged; signed amend follows the same rule. A signed commit with the isolated OpenPGP fixture verifies as good.
- The Local Changes **Sign** choice starts from `commit.gpgsign`, is retained with the draft, blocks the commit when setup is unavailable, and does not silently produce an unsigned commit. Explicitly clearing it still permits an unsigned commit.
- CLI `git good commit --sign` signs; `--no-sign` opts out; without a flag the CLI follows `commit.gpgsign`. The receipt reports the signed state.
- Tag creation requires an explicit name, target, and message; unsigned creation is annotated, signed creation uses the configured signer, and a signing failure leaves no tag. CLI `tag --create` covers both outcomes.
- Signature windows render at 720×480 inside the viewport with state, signer, key, trust, format, configuration, and the read-only notice; tag verification uses the same window via the refs menu.
- Native captures cover good and unknown-key states, the Sign choice, and the tag dialog. SSH/X.509 formats and real agent prompts remain platform checks.

## M7 Git LFS

- Pointer parsing accepts the canonical v1 header, 64-digit SHA-256 OID, and size line; malformed OIDs/sizes and ordinary text are distinguished. Git fixtures cover before/after pointers and non-pointer content.
- Details separate OID, declared size, local LFS object-store availability, `git-lfs` availability, and remote; missing and available states render distinctly.
- Fetch is disabled with a reason when `git-lfs` is unavailable, when the object is already present, or when no pointer is selected. The confirmation names OID, size, and remote and states that fetch does not change working files.
- CLI `git good lfs status|fetch` returns the same details/receipt; the status test asserts pointer identity, size, and missing availability. A successful network fetch remains unverified because `git-lfs` is not installed.
- The LFS details window renders at 720×480 inside the viewport with path, Before/After identity, availability, installation state, remote, and Fetch object.

## M7 Git LFS working pointers and preview

- Local Changes LFS sides follow the selected area: Staged HEAD→index, Unstaged index→working, Untracked absent→working. Working-file pointer reads are bounded to 4 KiB; plain working text is not treated as LFS; conflicts keep an explicit resolve-first note.
- The selected available object produces a bounded preview (image when decodable, otherwise hexadecimal); missing, over-2-MiB, and unreadable objects stay explicit. Preview never writes working files or stages.
- CLI `lfs status|fetch --area` returns working-scope details and rejects combining `--area` with revision flags.
- Native captures cover working pointer details with a PNG preview and the narrow window. A successful network fetch and smudging/materialization remain unverified/planned.

## M7 submodules

- Git fixtures report parent index commit, nested HEAD, initialized/dirty state, URL, and recorded-object availability. Dirty nested content, a recorded commit that differs from the index, and a missing recorded object block Update.
- Update restores the parent-index gitlink commit in the nested repository without staging the parent; the receipt names the target and the next scan reports a clean nested checkout.
- An uninitialized clone retains the parent index identity with no nested HEAD or dirty state and does not implicitly initialize during inspection.
- CLI `git good submodule status|update` returns the same details and receipt; tests cover status, update after an older nested checkout, and a dirty update rejection.
- The egui Submodule details window renders at 720×480 inside the viewport with recorded IDs, index, checked-out, URL, Open nested, and Update; a dirty state disables Update with the reason. The diff view exposes gitlink Before/After IDs from the selected change.

## M7 worktrees

- Git fixtures list main and linked worktrees, branch/detached/bare labels, current/main markers, and dirty status. Add supports new and existing branches; checked-out branches and nonempty destinations are rejected with the exact path/reason.
- Remove blocks main, current, dirty, locked, missing/prunable, and status-unavailable worktrees. A clean linked worktree is removed without force and disappears from the next listing. A nested `--repo` fixture still identifies the containing linked worktree as current and refuses its removal.
- CLI `git good worktree list|add|remove` returns the same inventory and receipts; tests cover list counts, add, checked-out-branch rejection, dirty removal rejection, clean removal, and the resulting inventory.
- The egui window renders at 720×480 inside the viewport with branch/path, Current/Modified, Open, and Remove controls. A state test round-trips commit selection, filter, pixel place, narrow details, and details visibility per worktree; the existing draft test keeps Local Changes drafts isolated by canonical worktree.
- Create/Remove run on one worker; a failed add keeps the create dialog and a failed remove keeps the confirmation and visible error. Native captures cover the wide/narrow inventory and the create form.

## M7 image diffs

- PNG and JPEG decode with exact dimensions and preserved alpha; corrupt, oversized, unknown, binary, absent, and budget states keep metadata and the bounded hexadecimal fallback.
- Fit preserves each image's aspect ratio and never upscales; Actual size uses natural pixels and exposes a 100% zoom label.
- The view renders Side by side, Swipe, Fit, Actual size, Hexadecimal, per-side short ID, dimensions, and encoded bytes at 1120 width. At 720 width it stacks named Before/After previews.
- Swipe keeps Before beneath a clipped After image, aligns each image by its own dimensions without stretching, offers a labeled boundary slider and canvas drag, and paints a theme-aware checkerboard behind transparency.
- Export remains per side and uses the bounded exact-byte route; image presentation never changes the selected comparison or export identity.

## M7 blame attribution

- Git fixtures attribute the full file and a bounded line range, keep origin line numbers, and preserve author, email, author time, and subject through a content change. A nested `--repo` fixture keeps the requested repository-relative path instead of attributing an identically named nested file.
- Missing paths and revisions, out-of-range starts, cancellation, binary content, gitlinks, and symlinks return explicit errors rather than empty attribution.
- CLI `git good blame PATH [--revision REF] [--lines START,END] [--repo PATH]` returns JSON with the exact path, range, and line metadata; a missing path exits non-zero.
- The shared reader test renders the author/short-commit column and the selected line's author, email, relative time, commit, and subject. Attribution is available on either committed side, selects the After or Before revision accordingly, and disables working-tree content with a reason.
- Engine mapping tests cover context lines, introduced lines, deleted lines, a range containing either, and an unavailable line. Introduced or absent content is reported for the whole range rather than silently shrinking it.
- The file-history strip test names the selected line/range and its boundary reason. A worker maps each edge; Older/Newer stay disabled while mapping or after an introduced/absent boundary, so the place never jumps to unrelated content.
- Native captures cover wide/narrow attribution, a mapped line through two older versions, an introduced-line boundary, and Open commit handing the blamed commit to the ordinary review. Before-side attribution availability and the initial older-version index are asserted separately; an ambiguous file edge uses the existing predecessor chooser before any range mapping.

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
- egui tests cover the version strip and counter, Older/Newer boundaries that disable with a visible reason and never wrap, the predecessor chooser in the same strip, Before/After inspection labels on the shared reader, Full-only rendering while the earlier side is unknown, `Alt+Left/Right`, and the rendered strip naming the old path. Review integration opens history with `H` in the wide and narrow reader layouts, restores the entry context once on close, and applies the another-file rule recorded under Coherence I4.
- Native X11 captures in [verification](verification.md#m7-file-history--2026-10-25) cover wide and narrow history, the oldest/newest boundaries, the predecessor chooser, and Full/Diff on one edge.
