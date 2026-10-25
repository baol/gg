# Implementation and verification prompts

Purpose: provide ready-to-dispatch subagent assignments for the coherent native gg interface, with dependencies and independent acceptance gates.

These prompts implement the [coherence proposal](ui-coherence.md). They are future assignments, not implementation evidence. The user's required Local Changes layout is **Unstaged upper left; Staged lower left; selected diff right; commit/amend lower right**.

## Dispatch order

Send the shared brief plus one assignment to each subagent. Do not dispatch the entire pack as one implementation task.

| Assignment | Dependency | Deliverable |
| --- | --- | --- |
| C · Coordinator | None | Baseline, sequencing, edit ownership, integration |
| I1 · Navigation and commands | Baseline established | Stable navigation, explicit switching, scoped actions, return locations |
| I2 · Shared reader | I1 accepted | Common comparison/presentation/place interface |
| I3 · Local Changes | I2 accepted | Required layout, staging and commit/amend integration |
| I4 · File history | I3 accepted | History changes the source inside the existing reader |
| I5 · Operations | I4 accepted | One operation area and responsive rewrite |
| I6 · Conflicts | I5 accepted | Read-only preview followed by validated Stage resolution |
| I7 · Repository tools | I6 accepted | Unified Recovery, stash restore, branch forms and pending intent |
| V1 · Behavior verifier | Each completed candidate | Independent Git, worker, and regression verification |
| V2 · Native UI verifier | Each completed UI candidate | Native layout, keyboard, coherence and journey verification |

Default to **one implementation writer at a time** in the current checkout. Several tasks edit main.rs, reader code, and owning specifications. V1/V2 can review a frozen candidate concurrently using separate fixture directories and isolated displays. Coordinate expensive builds; never let agents mutate the same fixture or shared source concurrently.

Deferred: blame/line-history algorithms, image-diff extensions, worktree management, submodules, LFS, signing, force push, and restart-persistent drafts. Preserve extension points without adding fake functional controls.

## Shared brief — prepend to every assignment

~~~text
Read AGENTS.md, CONTRIBUTING.md, docs/ui-coherence.md, the owning sections of docs/architecture.md, and docs/design.md, docs/plan.md, docs/regression.md, docs/development.md. Inspect the current diff before editing and preserve unrelated work.

Implement or verify ONLY your assigned increment. The agreed design is:
- Stable Local Changes / History navigation.
- Unstaged files upper left, Staged files lower left, one selected diff right, commit/amend lower right. A partially staged path appears in both lists. List selection determines comparison; no duplicate Staged/Unstaged reader switch.
- Shared reader controls and exact endpoints across review, staging, and file history. Older/Newer changes versions; Before/After chooses an inspection side.
- Ref selection is read-only; switching is explicit.
- One visible scoped command surface per object. Shortcuts/context menus invoke the same commands, target, preconditions, and confirmation.
- One operation area for progress and supported recovery controls.
- Shared Recovery, stash restore, branch-creation and return-navigation patterns.

The HTML prototype is illustrative, not an algorithm or pixel specification. Preserve native fonts, branding, density, themes, virtualized data, and bounded worker behavior. Reuse code for concrete responsibilities; avoid a wholesale Browser rewrite.

Update the owning architecture contract before/with behavior changes. Add a small plan increment and acceptance criteria; mark only delivered work complete. Update README and verification when relevant. Link shared requirements instead of duplicating them.

Preserve exact repository/path/revision identity, read-only browsing, hooks, backups, dirty-worktree checks, cancellable workers, stale-result rejection, pointer/scroll stability, worktree-isolated normal/amend drafts, special-file handling, and existing CLI commands/JSON semantics. Display strings and truncated previews are never mutation input.

The coordinator assigns exclusive ownership of shared files. Do not independently dispatch other agents, take another task, or edit another writer's files. Propose a concrete handoff if scope must expand. Never reset, stash, clean, or overwrite existing user work.

Start regression fixes with failing tests. Use real Git fixture oracles, production worker/input paths, and geometry assertions. Change old UI expectations only when the accepted contract changes, retaining their underlying safety and navigation coverage. Do not weaken tests to obtain a pass.

Run targeted checks during development, then the required increment gate:
  cargo fmt --all --check
  cargo test --workspace --locked
  cargo clippy --workspace --all-targets --locked -- -D warnings
Inspect native UI changes using docs/development.md and record evidence in docs/verification.md. Browser mockups do not count as native acceptance. Report unavailable checks honestly. Repeat passing checks only after changes or for unresolved concerns.

Return behavior, changed files/contracts, commands/results, native evidence paths, limitations, and interface/ownership handoff notes. Leave a reviewable diff; do not independently push, publish, or merge.
~~~

## C · Coordinator

~~~text
Apply the shared brief. Coordinate I1–I7 and independent verification.

1. Record baseline revision plus uncommitted files; preserve existing design documents/mockups. Establish the current check baseline and distinguish existing failures from new regressions.
2. Add staged coherence increments to docs/plan.md without changing unrelated M7 statuses. Adopt the proposal's changed contracts increment by increment.
3. Dispatch I1–I7 in order with the shared brief. Use one writer by default; main.rs, navigation, reader modules and docs are shared hotspots. Name the current owner in every handoff.
4. Prefer the current checkout for serialized work. If isolated worktrees are necessary, explicitly carry accepted code and uncommitted design sources: new worktrees do not automatically include this checkout's uncommitted files.
5. After each increment, freeze the candidate and dispatch V1/V2 with its exact revision/diff, changed contracts, checks already run, and evidence. Reviewers do not silently fix code while another agent owns it.
6. Return actionable findings to their implementer, get relevant regression checks, and have the verifier confirm the correction.
7. Verify the final combined tree and connected journeys. Separate passing branches do not prove integration. Retire superseded controls only after equivalent paths work.
8. Report accepted increments, remaining findings and verification limits. Do not claim deferred M7 features or untested platforms complete.

Resolve routine design/implementation choices within scope. If a real contract ambiguity remains, state a bounded proposal and its tradeoff. Do not repeatedly ask approval for already-authorized work.
~~~

## I1 · Stable navigation and scoped commands

~~~text
Apply the shared brief. Implement I1 only after the coordinator's baseline.

Starting points: apps/git-good/src/main.rs (Browser, commit_action_menu, run_commit_action, jump_to_ref), refs_view.rs, history_view.rs, navigation.rs, interaction_tests.rs. Read Architecture: Browser state and keyboard; Presentation; History graph and refs; Branch switching and creation.

Deliver stable Local Changes / History navigation at wide/narrow widths with reachable ref browsing. Remove competing navigation copies.

Make ref selection read-only everywhere, including sidebar, menus, and graph pills. Remove mutating single/double-click behavior. Explicit Switch to <branch> retains existing validation, checkout, and post-checkout warning behavior.

Give each selected object one visible scoped action menu. Context menus and keyboard routes reuse the same command definitions, targets, readiness reasons, and confirmations. Retire the separate Commit actions window and ambiguous global Actions route after replacements work. Route Alt+A to the applicable scoped command surface; preserve editor/menu keys.

Introduce a small return-location contract: opening a commit from a tool can return to that tool's selected item/filter/viewport. Presentation changes do not add Back entries. Do not implement Recovery yet.

Tests: actual egui selection/double-click input preserves HEAD/index/worktree/refs; explicit switching validates dirty state and reports successful checkout with hook warning correctly; keyboard/menu commands share targets; history anchors, filters, late-result isolation and narrow Back survive.

Handoff command and return-location interfaces to I2/I7. Do not refactor staging or operation workers here.
~~~

## I2 · Shared file workspace and reader

~~~text
Apply the shared brief. Implement I2 after I1 is accepted.

Starting points: apps/git-good/src/diff_view.rs, diff_worker.rs, review_view.rs, review_worker.rs, stage_view.rs, file_history_view.rs, navigation.rs; crates/engine/src/file.rs, diff.rs, review.rs, status.rs. Read Architecture: Commit review; Working diffs; File reading and history.

Extract a concrete shared reader from existing production components. Keep history enumeration, file enumeration and selected-file workers independent. Avoid replacing functioning workers solely to unify rendering.

The reader owns displayed comparison identity/endpoints, exact path, presentation, inspection side, cursor/range and reading place. Standardize Diff / Full file, Before / After, change navigation, bounded metadata and export access. Before/After selects a side only for full-file content.

Keep requested/displayed identities separate and adopt matching identity, metadata and rows together. Late requests must never put old content under a new label. Provide capability-based scoped actions; labels cannot grant mutation permissions.

Migrate commit review first. Full-width reading is a layout of this workspace, not another independent viewer. Define a saved reader-context value and integration API for staging/history. Remove obsolete duplicate rendering as consumers migrate.

Tests: path/side/place survives mode and width changes; root/merge/add/delete comparisons have exact endpoints; existing 2 MiB/100,000-line full-file limits and other diff/preview/export budgets remain; rapid path/side/mode changes reject stale results; browsing preserves repository snapshots.

Do not implement blame or line-history algorithms. Provide an honest extension point without a fake functional control. Handoff the API and preservation tests.
~~~

## I3 · Local Changes layout and commit/amend

~~~text
Apply the shared brief. Implement I3 after I2 is accepted.

Starting points: apps/git-good/src/status_view.rs, stage_view.rs, status_worker.rs, diff_view.rs, main.rs draft retention, interaction_tests.rs; crates/git/tests/status.rs and commit.rs. Read Architecture: Working status/diffs, staging, hunk/line/discard, committing, unstaging, and amend.

Implement the user's layout:
- Unstaged paths upper left, supported untracked entries clearly marked.
- Staged paths lower left; partially staged paths appear in both lists.
- One reader right. Unstaged selection means index -> worktree. Staged means HEAD -> index; amend means HEAD parent -> index.
- Commit/amend form lower right, below the reader and available while reading.

Remove the extra Staged/Unstaged reader switch and obsolete dual-diff/stacked-Split layouts. Preserve whole-file, hunk and selected-line scopes, their readiness rules, and confirmed Discard with unchanged index.

Refresh both lists and the reader coherently. If the selected list entry disappears, retain the reviewed path with an explicitly empty comparison until another entry is selected. Never leave stale rows looking current. Preserve independent list scroll, selection, exact paths, and bounded virtualization.

Narrow layouts keep both named groups reachable above the reader and the form below it. Bounded/collapsible lists may be used; do not introduce another comparison selector.

Normal/amend modes share the form but keep separate drafts per canonical worktree. Name the affected commit and changed base. Normal commit requires completed valid status, a message and staged changes; message-only amend works. Hook failures retain drafts; post-commit warnings state the commit exists. Success clears only the submitted draft and keeps remaining local changes visible.

Tests: wide/narrow geometry proves the layout; both entries of a partially staged file show distinct correct comparisons; file/hunk/line staging and discard preserve unrelated data; amend matches Git and keeps hooks/backups; drafts survive navigation, refresh, failures and worktree changes. Replace obsolete split-layout assertions while retaining their safety/async coverage.

Handoff path+area selection and reader/form integration to I4/I6.
~~~

## I4 · File history inside the reader

~~~text
Apply the shared brief. Implement I4 after I3 is accepted.

Starting points: apps/git-good/src/file_history_view.rs, file_history_worker.rs, review_view.rs, diff_view.rs; crates/engine/src/file_history.rs and file.rs; crates/git/src/file_history.rs and tests/file_history.rs. Read Architecture: File reading and history.

Make File history a comparison-source mode in the existing reader. Remove the second full-screen content renderer; retain the real worker-backed content-version cursor.

Capture entry comparison, exact path, revision, side, presentation, cursor/range and reading position. Show a version strip above the same content. Use Older version / Newer version; reserve Before / After for inspection sides. Version steps do not push Back entries.

Close file history restores the captured context once. Back/Escape dismisses the active chooser/history source/review in the correct order, preserving editor keys and the real caller.

Retain unchanged-commit skipping, renames/old paths, deletion/absence, no-wrap boundaries, bounded preload/paging, trace limits, retry, and cancellation. Unique predecessors follow automatically; ambiguity requires a chooser, with Diff unavailable until the earlier edge is resolved and current Full file still readable.

Choose and document one rule for selecting another file while history is active: correctly reset its history request with an unambiguous return snapshot, or require closing history first. Never reuse another path's version cursor.

Tests: exact reader restoration, boundaries, ambiguous predecessors, rename/deletion, rapid version/file changes, failed page/retry, nested return to a repository tool, Git-history oracles, and no repository writes.

Do not implement attribution or line/range tracing in this task. Preserve their Place extension contract.
~~~

## I5 · One operation area and responsive rewrite

~~~text
Apply the shared brief. Implement I5 after I4 is accepted.

Starting points: apps/git-good/src/main.rs (operation_busy, start_operation, finish_operation, begin_rewrite, finish_rewrite, RewriteJob, OperationOutcome), network.rs, and native worker tests. Read Architecture: outcomes, network progress/cancellation, M5 operation state, rebase, reorder/squash.

One operation area beneath navigation owns running/refreshing/needs-attention and supported continuation controls across views. The outcome line remains concise with Details and at most one next action; do not duplicate operation controls there or in dialogs.

Validated submission closes configuration. Rewrite loses its blocking progress modal: browsing remains responsive, incompatible mutations disable, and Rewriting becomes Refreshing until the matching replacement history is accepted. Explicitly label retained old history; preserve safe selection restoration.

Use existing operation workers and serialization. Continue/Skip/Abort follow actual Git state: merge has no Skip; stash conflicts have no sequencer Continue/Abort. Network/export cancellation differs from sequencer abort; inspect outcomes rather than imply rollback.

Carry repository/worktree and generation identity through jobs. Navigation, repository switching, disconnection and delayed results must not update the wrong context. Duplicate submission starts one job. Refresh affected views at the accepted boundary and keep unresolved errors above later success notices.

Tests: delayed workers and real Git fixtures for success, conflicts, recovery, cancellation, disconnection, duplicate submission, browsing during rewrite, repository changes, and correct refresh-generation acceptance. Preserve backup reachability. Replace old modal-lifetime expectations with equivalent lifecycle/readiness guarantees.

Handoff the operation-state interface to I6/I7.
~~~

## I6 · Conflict preview followed by Stage resolution

~~~text
Apply the shared brief. Implement I6 after I5 is accepted.

Starting points: apps/git-good/src/main.rs ConflictView/ConflictUpdate and load/resolve paths; crates/engine/src/conflict.rs; crates/git/src/conflict.rs and tests/conflict.rs; apps/git-good/src/conflict_cli.rs and tests/cli.rs. Read Architecture: conflict resolution and M5/M6 recovery.

Open conflicts in the Local Changes/shared-file workspace. Name Base, current branch/replay state (ours), incoming commit (theirs), and result preview. Rebase labels must explain actual identities.

Choosing a source is READ-ONLY. Stage resolution is the single publication step for the selected side or externally edited content. Refresh external edits and validate path, repository, operation, unmerged-stage identities, and observed content before submission. Stale requests must not stage or overwrite newer unrelated changes.

Critical: gg_engine::conflict::Content contains display strings, binary summaries and truncation flags. NEVER write those strings or lossy decoded paths back. Retain a validated source identity/token and retrieve original complete bytes through the backend. External edits stage their validated actual content. Add a narrow typed request/snapshot if existing APIs cannot enforce this boundary; preserve existing CLI semantics.

Absent sides are not empty files. Handle deletion/binary/over-budget cases honestly; unsupported choices disable with reasons. Do not add an in-app editor. Do not promise atomicity across worktree/index: prevalidate, preserve unrelated data, and report actual partial state if staging fails after a worktree write.

Refresh state and select the next unresolved path after success. Continue lives only in the operation area and waits for zero unmerged paths. Stash conflicts retain the entry and have no fabricated Continue/Abort.

Tests: snapshot-preserving preview/source changes; original-byte application checked against Git; external edits; binary/large/add-delete cases and supported special paths; changed index/worktree/operation/path between preview and submit; duplicate submission; next-path selection; rebase/merge side identity; partial publication failure; compatible CLI tests.

Return exact unsupported cases rather than silently coercing them.
~~~

## I7 · Repository tools, recovery, and pending intent

~~~text
Apply the shared brief. Implement I7 after I6 is accepted. Complete these subincrements in order with checks between them.

Starting points: apps/git-good/src/backups_view.rs, reflog_view.rs, main.rs stash/recovery/create-branch dialogs and open_backup_review, refs_view.rs; crates/git/src/backup.rs, reflog.rs, stash.rs, branch.rs and their tests. Use I1 return locations, I2 reader, I5 operations.

A. Recovery:
Replace separate Backups/Reflog windows with Repository -> Recovery and a source selector. Preserve distinct provenance and identifiers. Share Open commit, Create branch here, Reset branch here and exact Copy commands. Ordinary review returns to the same recovery source/entry/filter/position. Keep private backups retained and out of refs. Do not collapse distinct entries pointing to one commit. Preserve reflog reference selection, failures, reset modes and Hard warnings.

B. Stashes:
Repository -> Stashes provides one Restore stash form with Restore staged state and Remove stash after successful restore. Map to existing apply/pop semantics. Conflicts always retain the entry and use the shared conflict workspace without sequencer controls. Delete stash confirms the exact entry and leaves working files untouched. Keep CLI apply/pop/drop and receipts compatible.

C. Branch forms and intent:
Use one branch-creation form with starting point and explicit Switch after creation. Recovery defaults to no switch. Switching validates current state. If branch creation succeeds and switching fails, report the existing branch and actual checkout without deleting it or reporting total failure.

Retain a blocked action's target/options/caller as an intent while the user reviews, commits or stashes. Offer the named action again and revalidate; never auto-execute it after a prerequisite succeeds. Cancel clears intent. Repository/worktree changes cannot silently retarget it.

Standardize Back/Cancel/Close and Delete/Discard wording. Remove old entry points only after shared paths work.

Tests: Git fixture oracles for stash options/conflicts, resets/backups, branch creation/switch partial success; real input paths for nested review return, source/selection retention, keyboard parity, canceled intent and worktree isolation. Copy exact identifiers.

Do not expand into force push, worktree management, restart persistence or other deferred features.
~~~

## V1 · Independent behavior verifier

~~~text
Apply the shared brief. Independently review the coordinator's named candidate and increment. Receive its exact revision/diff including uncommitted files, changed contracts and claimed checks. This task verifies; it does not authorize a parallel production rewrite.

Inspect code and owning specs, not just the implementation summary. Use disposable offline Git fixtures with isolated configuration. Keep candidate/fixtures stable and coordinate build ownership.

Challenge the relevant claims:
- Read-only ref selection and explicit checkout; visible/context/keyboard command parity.
- Exact comparison identity and rejected stale worker results across path/side/source/repository changes.
- Both entries of partially staged paths; exact mutation scope; amend base, separate drafts, hooks and backup failures.
- History context restoration, chooser, bounds, rename/deletion and read-only behavior.
- Serialized responsive operations, refresh-boundary completion, valid recovery buttons and cancellation semantics.
- Read-only conflict preview, original-byte application, stale rejection, external edits and partial failures.
- Stash option mapping, retained conflicts, distinct Recovery provenance, branch partial success, and intents never auto-executing.

Use production-path tests and independent CLI/Git snapshots. A correct label is not proof of a Git effect. If a regression test is missing, return a minimal failing fixture/input sequence to the implementer. Do not edit shared production files during review. In an explicitly assigned isolated verification checkout, verifier-only tests are allowed; include their patch and results.

Check required fmt/test/clippy results for this exact candidate, reusing recorded identical-candidate results where appropriate. Distinguish new failures, baseline failures and unavailable environment checks. Never mark an unexecuted check passed.

Return PASS/FAIL/NOT VERIFIED for each scoped requirement, exact commands and candidate identity, severity-ranked findings with file/line and reproduction, and an accept/block recommendation. Have the implementer fix findings, then verify corrections.
~~~

## V2 · Independent native UI and coherence verifier

~~~text
Apply the shared brief. Verify the real git-good binary from the coordinator's identified candidate. Mockups explain intent but are not native evidence. Use disposable repositories and the documented isolated-display capture workflow.

Exercise established native sizes (at least 1120x740 and 720x480), a larger resizable layout, light/dark themes and supported display scales. Use long Unicode paths, many files, empty/loading/error states. State actual platform/display conditions tested.

Walk these connected journeys with keyboard and pointer:
1. History -> inspect branch without checkout -> explicit switch; commit -> file -> Diff/Full file -> history versions -> close -> Back. Verify actual caller, comparison, file, side/range/filter/scroll restoration.
2. Local Changes has both left groups and diff/form on the right. Select a partially staged path from each group; stage/unstage hunks/lines; inspect an empty counterpart; toggle amend; navigate away/back; trigger hook failure. Check drafts, endpoints and stable action placement.
3. Start rewrite, browse during execution/refresh, resolve conflicts through preview then Stage resolution, continue. Operation area owns recovery; source labels and supported controls match the Git operation.
4. Recovery -> source/entry -> Open commit -> Back; stash restore options; resume/cancel blocked branch-switch intent. The same labels, confirmations and return rules apply.

Check keyboard discoverability and focus, editor/menu key ownership, both bundled fonts, readable selected/diff text, disabled reasons, bounded progress/metadata, and reachable controls. Use geometry evidence alongside screenshots.

Report concrete inconsistencies: name the two views and the differing label/location/effect/navigation for the same intent. Preserve legitimate distinctions such as stage versus discard and cancel versus abort. A superseded control left beside its replacement is a finding when it recreates competing flows.

Keep production files unchanged. Return capture paths, build/candidate/platform information, exact steps, observed results, severity-ranked issues and untested conditions. The coordinator/implementer records accepted evidence and fixes findings. One Linux capture session does not establish cross-platform or full accessibility acceptance.
~~~

## Handoff format and completion

~~~text
Assignment and candidate:
Implemented behavior / verified requirements:
Changed files and owning contracts:
Preserved invariants / intentional expectation changes:
Checks: command -> result, including failures and unavailable checks:
Native captures / evidence:
Remaining findings and limitations:
Interfaces and edit ownership handed to the next agent:
~~~

An increment passes when scoped production behavior meets its contract, required checks pass, and independent review has no unresolved blocking findings. Pre-existing failures and environment limitations remain explicitly unverified, never counted as acceptance. Final completion requires the integrated native journeys, not the number of prompts executed.
