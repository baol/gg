# Verification

Purpose: record measured results and outstanding acceptance checks.

## Coherence I1 · Stable navigation and scoped commands · 2026-10-25

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixtures.

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 367 tests, 0 failures, 3 ignored opt-in performance checks. This includes 176 `git-good` unit tests and 33 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- Independent V1 review found and the candidate fixed three gaps before acceptance: the History strip now leaves the narrow Details overlay, closing Review through the History strip restores the recorded caller location, and the Refs-menu switch readiness matches the sidebar (network and mutation block it). Repository open/clone now drops a stale Review so an old return location cannot be applied to new history.
- **Local Changes** and **History** render in one compact strip beneath the toolbar in that order at 1120×740 and 720×480. The wide refs sidebar no longer repeats navigation, the toolbar no longer carries a narrow-only destination fallback, and **History** replaces the old destination/filter title. Geometry and interaction tests switch destinations at both widths and verify that history selection, filter, pixel offset, and the Local Changes draft survive.
- Clicking a sidebar ref row, a toolbar Refs item, or a history graph pill inspects its target commit. A real Git fixture test uses egui pointer input on the sidebar and confirms HEAD, index bytes, worktree status, and branch refs are unchanged. The sidebar context menu and the toolbar Refs menu expose **Switch to \<branch\>** with the current branch disabled; backend branch fixtures continue to cover dirty rejection, checkout, and post-checkout warnings.
- The selected commit has one **Commit actions** menu beside its identity in the wide details pane and the narrow details overlay. It uses the existing `commit_action_menu` definition and `run_commit_action` dispatch; the global **Actions** menu, separate **Commit actions** window, and Help entry that opened it are gone. Alt+A opens the narrow details overlay when needed and then the same popup. Tests render all command groups at both widths and exercise the shared menu command result.
- Review records a return location with the caller's commit selection, filter, and pixel offset, plus whether narrow Details was the caller. Closing Review restores it exactly; presentation changes add no Back step. Recovery-specific sources remain unimplemented.
- Native X11 captures: [wide history with the stable strip and scoped menu](screenshots/coherence-i1-history-wide.png), [narrow history](screenshots/coherence-i1-history-narrow.png), and [narrow details with Commit actions](screenshots/coherence-i1-details-narrow.png).
- Limitations: the screenshot hook opens History directly, so no native Local Changes capture was automated; its strip is the same panel verified through geometry/interaction tests. Only the dark theme, Linux/X11, and llvmpipe rendering were inspected. Independent verification remains pending.

## Coherence I2 shared reader · 2026-10-25

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixtures.

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 375 tests, 0 failures, 3 ignored opt-in performance checks. This includes 184 `git-good` unit tests and 33 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- Independent V1 review found six candidate issues before acceptance; the candidate now fixes them: the restored place survives a settling full-file update, export readiness follows the displayed sides, identity adoption and side changes preserve/clear the right place and content, endpoint labels wait for matching content in history, and the reader API no longer exposes unused endpoint paths or a capability mismatch.
- `reader::ReaderContext` carries the repository root, exact `FileChange`, both endpoint labels, presentation, inspection side, cursor, anchor, and pixel offset. Unit tests save a context, change mode/side/place, restore it, assert every value and endpoint label, and confirm the place survives identity adoption and asynchronous updates.
- Commit review supplies resolved parent/reference and commit labels to the reader header; file history supplies the older/current content-version labels. The header keeps the exact modes and blob IDs, and endpoint labels only appear once displayed content matches the requested identity.
- `reader::Capabilities` derives Diff, full-side selection, and export availability from the actual comparison; export uses the displayed sides, absent and gitlink sides cannot export, and an unresolved history edge cannot open Diff. Existing 2 MiB/100,000-line, binary preview, and export budgets remain covered by their owning tests.
- Requested and displayed identities remain separate through generation, mode, and side checks; existing stale-result tests still pass.
- Native X11 at 1120×740: [shared reader with exact endpoints](screenshots/coherence-i2-reader-wide.png). At 720×480: [narrow shared reader](screenshots/coherence-i2-reader-narrow.png).
- Limitations: commit review and file history consume the shared reader; staging migration and blame/line-history remain I3/I4 and later work. Native evidence is Linux/X11 dark theme only.


## Coherence I3 Local Changes layout · 2026-10-25

Linux x86_64 · Rust 1.99.1 · Fira Sans/Phosphor · Xvfb/llvmpipe · disposable Git fixture (`/tmp/opencode/gg-i3-fixture`: partially staged `a.txt`, staged `b.txt`, unstaged `c.txt`, untracked `new.txt`).

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 380 tests, 0 failures, 3 ignored opt-in performance checks. This includes 188 `git-good` unit tests and 33 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- Independent V1 review found blocking gaps that the candidate now fixes: conflict selections read the unmerged summary and an area-specific route instead of “No unstaged changes”; a staged-away untracked entry keeps an explicit empty comparison with no stale actions; off-page refresh and draft reselection find the selected comparison through the worker’s selected-row lookup; the post-commit warning no longer claims the commit was aborted; and the staged-first worker assertion checks the real ordering invariant. Regression tests cover each fix.
- Local Changes now owns two independent lists. The **Unstaged** group is above **Staged** in the left column; each keeps its own selection, scroll offset, viewport, and bounded row cache. Staged/Unstaged rows show change kind and path; untracked and conflict rows carry explicit badges and colors, and the active list's selection is visually stronger than the inactive list's retained selection. Selecting either entry immediately selects its comparison; **View diff**, the separate **Review & stage** screen, the narrow Staged/Unstaged reader switch, and **Split** are gone.
- Wide layouts place the single reader right of both lists and the commit/amend form below the reader; short groups compact instead of leaving a fixed empty half. Narrow layouts keep bounded collapsible groups above the reader and the form below it. Geometry assertions at 1120×740 and 720×480 verify both groups, reader, and form positions.
- The status worker pages both list viewports independently and maps working pages into their own logical coordinates. The Git status oracle asserts every `Area::Staged` row precedes all unstaged, untracked, and conflict rows; the worker test asserts the boundary and independent pages. Hover-safe adoption, cancellation, bounded rows, and stale-result rejection remain covered. Refresh supersedes a parked previous scan instead of waiting for an update it may never publish; CI exposed and the candidate fixed this race.
- A real Git fixture with one partially staged path verifies both group entries and distinct comparisons (HEAD → index versus index → working file). A conflict fixture verifies the unmerged summary, route, and non-editable actions. Amend mode names HEAD's parent rule and the affected commit/base banner. Whole-file, hunk, and selected-line tests confirm unrelated index entries and working bytes survive; Discard confirmation leaves the index unchanged. A staged-away tracked or untracked entry retains its path with an explicit empty comparison and no stale actions; refresh and draft restoration resolve an off-page selection through the worker lookup.
- Commit/amend tests cover the lower-right/below-reader form at both widths, a visible readiness reason beside the Commit action, separate per-worktree normal/amend drafts, message-only amend, hook-failure draft retention, post-commit warnings, backup refs, and clearing only the submitted draft.
- Native X11 captures: [wide Local Changes](screenshots/coherence-i3-local-changes-wide.png) and [narrow Local Changes](screenshots/coherence-i3-local-changes-narrow.png), using `GG_LOCAL_CHANGES=1` for the debug screenshot hook.
- Limitations: only Linux/X11, the dark theme, and llvmpipe were inspected natively; the 720×480 narrow layout shows one row per group and requires scrolling for the remaining rows. Local Changes still uses the staging-specific diff pane rather than the shared reader's Diff/Full and Before/After controls; that remains a later coherence migration. Bundled-font CJK glyphs render as tofu in the native fixture, although exact path bytes are retained. A second independent V1/V2 pass after these fixes has not run.

## Coherence I4 · File history inside the reader · 2026-10-25

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixtures (`/tmp/opencode/gg-i4-fixture`: add, content change, rename with content change, later change; `/tmp/opencode/gg-i4-merge-fixture`: a two-parent merge whose parents carry different file states).

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 391 tests, 0 failures, 3 ignored opt-in performance checks. This includes 199 `git-good` unit tests and 33 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- File history is now a comparison-source mode in the existing Review reader. `file_history_view::FileHistory` owns the worker-backed timeline and cursor but no longer owns a `DiffView` or draws full-screen content panels. Review renders the compact version strip above the same reader: the wide layout keeps the file list in place beside it, and narrow reading keeps the same full-width reader path.
- `H` and the visible **History** button open history for the selected file and place. Opening captures the entry `ReaderContext` (comparison change, exact path, revision, endpoint labels, Diff/Full mode, inspection side, cursor/anchor, and pixel offset). The **Close history** button or `Back`/`Escape` restores it exactly once, including when the entry file-list row has left the bounded cache: the saved row identity is restored and revealed independently of the cache. `Alt+Left`/`Alt+Right` and **Older version**/**Newer version** change only the reader source and add no Back/return-location step. `H` while history is open does not stack another source.
- Selecting another file while history is active keeps history open, resets the version cursor for the new file, and preserves the original entry context; closing then restores the original file and reader place. Tests assert the new path, newest version, and saved context. Switching parents leaves history. `Escape` closes the chooser/history source before reading, the file list, and Review.
- The strip shows the version counter, kind, commit, subject, author, date, and old path. Both directions disable with visible reasons and never wrap. A unique predecessor follows automatically; a two-parent fixture renders the chooser in the same strip with every candidate. Diff is disabled until the earlier edge is resolved, while the current Full file remains readable and loads normally.
- Existing Git oracles still cover unchanged-commit skipping, rename/old-path, delete/re-add absence, one- and two-sided merges, non-UTF8 paths, and cancellation/retry with retained versions. UI tests additionally cover rapid version steps, a failed page retaining its timeline on retry, and absent/deleted edges. A worker snapshot test confirms opening and paging content versions preserves every repository file byte; the existing repository snapshot checks remain unchanged.
- Native X11 at 1120×740: [newest version in the wide reader](screenshots/coherence-i4-history-wide.png), [rename and old path](screenshots/coherence-i4-history-rename.png), [oldest boundary](screenshots/coherence-i4-history-oldest.png), [ambiguous predecessor chooser with Full only](screenshots/coherence-i4-history-chooser.png), and [the restored entry reader after closing history](screenshots/coherence-i4-history-restored.png).
- Native X11 at 720×480: [history above the narrow reader](screenshots/coherence-i4-history-narrow.png) and [the restored narrow reader](screenshots/coherence-i4-history-narrow-restored.png).
- Limitations: only Linux/X11, the dark theme, and llvmpipe rendering were inspected natively. In the wide split the strip reflows into compact rows above the reader without changing the file-list/reader widths; very long chooser labels can still wrap in the ~400-pixel reader. Attribution and line/range history remain deferred extension points; no such controls were added. This record supersedes the full-screen history sub-view and Old/New side wording in the M7 file-history evidence below.

## Coherence I5a · One file workspace · 2026-10-26

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixture (`/tmp/opencode/gg-i5a`: staged `src/lib.rs`, unstaged `src/app.rs`, untracked `note.txt`) plus the I4 history fixture.

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 400 tests, 0 failures, 3 ignored opt-in performance checks. This includes 205 `git-good` unit tests and 33 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- Local Changes is now a consumer of the shared reader. `status_view::StageActions` owns only the selected `WorkingRequest` and the mutation worker; `diff_view::DiffView` loads and renders Staged, Unstaged, Untracked, and Conflict comparisons with the same Diff/Full file, Before/After, endpoints, place, and change navigation as review and history. The staging-specific renderer, duplicate **View diff** button, narrow metadata/open panel, and permanent path/route/mode/size rows are removed.
- The reader header owns the file identity once: exact path, route (`HEAD → index`, `HEAD parent → index`, `index → working file`, `empty → working file`, or the unmerged summary), Diff/Full file, and compact **< Prev**/**Next >** change navigation. **File actions** exposes export, copy path, whole-file Stage/Unstage/Discard, and the File details toggle; File details shows area, kind, sizes, object modes/IDs, base/index identity, and the worker note on demand.
- Working full-file reads run on the shared selected-file worker. Git oracles assert exact bytes for Staged (base tree and index), Unstaged (index and normalized working file), Untracked (absent and working file), Deleted (explicit absent side), symlink target text, cancellation, and stale-index rejection. Existing 2 MiB/100,000-line, binary, gitlink, and snapshot coverage is unchanged.
- Inline **Stage hunk**/**Unstage hunk** and **Discard hunk**/selected-line actions now derive edits from the shared rows. Unit tests assert whole-hunk fallback, selected-line scope, discard block expansion, per-hunk membership, pending-mutation gating, and whole-file actions. Real Git fixtures still verify index/working/index-byte preservation, confirmed Discard, Escape/Cancel dismissal without publication, disappeared entries, and amend/restage behavior. `Tab` switches keyboard focus between the two groups without leaving the text editor's keys.
- Binary working comparisons keep an explicit **Binary content** summary and a no-inline-preview note instead of an empty reader and "No changes". Mode-only and symlink working comparisons keep their mode/worker notes in the header and File details. A Git oracle covers a real chmod-only comparison.
- Review now shows one compact subject strip and lets the reader own the path/endpoints. File history suppresses that strip while active, keeps the version strip above the same reader, moves **Close file history** into the strip, and uses compact change-navigation labels so controls do not wrap the wide split. Local Changes no longer repeats its heading or repository path above the workspace; Refresh/Retry lives once in the status line.
- Native X11 at 1120×740: [Local Changes on the shared reader](screenshots/coherence-i5a-local-wide.png), [staged comparison](screenshots/coherence-i5a-staged.png), [File actions](screenshots/coherence-i5a-file-actions.png), [File details](screenshots/coherence-i5a-file-details.png), [review reader](screenshots/coherence-i5a-review.png), [history in the same reader](screenshots/coherence-i5a-history.png), and [a binary working comparison with its explicit summary](screenshots/coherence-i5a-binary.png).
- Native X11 at 720×480: [narrow Local Changes](screenshots/coherence-i5a-narrow.png).
- Independent V1 review blocked on binary/mode-only blank readers, whole-file actions staying enabled during a pending mutation, missing working modes in File details, stale docs/screenshots, and test counts. The candidate fixed all six and a focused re-review accepted `ff79292` after re-running the gates and native checks.
- Remaining P3 observations: Tab can leave egui's latent focus on the Refresh button without painting a focus ring, and pointer selection dismisses an open discard confirmation without mutating anything. Neither affects safety or the restored reader.
- Limitations: only Linux/X11, the dark theme, and llvmpipe were inspected natively. The wide history split is still about 400 pixels, so the close control shares the version metadata line and a long subject truncates with the existing full-text disclosure. Attribution, line/range history, image-diff extensions, worktrees, submodules, LFS, and signing remain deferred.

## Coherence I5b · One operation area · 2026-10-26

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixtures (`/tmp/opencode/gg-i5b-merge`: a real two-parent merge conflict; `/tmp/opencode/gg-i5b-rewrite-slow`: 301 linear commits).

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 408 tests, 0 failures, 3 ignored opt-in performance checks. This includes 213 `git-good` unit tests and 33 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- One operation area now sits directly beneath the navigation strip and is hidden when idle. It owns operation-worker labels, rewrite/refresh progress, network progress with **Cancel**, branch work, and needs-attention conflicts. The old mutation banner, toolbar network cancel, and conflict-window recovery controls are gone; the status line yields to the area and does not duplicate progress or recovery controls.
- Merge shows **Resolve conflicts…**, a gated **Continue**, and **Abort** without **Skip**; cherry-pick, revert, and rebase show supported **Skip**. Continue stays disabled while conflicts remain. Stash conflicts do not create sequencer controls.
- Rewrite configuration closes on submission instead of staying as a blocking modal. The operation area reports **Rewriting commits** and then **Refreshing history** until the replacement generation is accepted; browsing stays available and the old history remains labeled through the refresh. A failed rewrite or refresh reports an error outcome with the backup route; duplicate submissions and incompatible mutations stay disabled.
- Opening a repository already in a merge, rebase, cherry-pick, or revert now detects the operation on the first history load and shows it in the area.
- Operations are repository-scoped. Starting an operation records its repository; results are applied only when that repository is still current. Switching repositories (including a successful clone) drops the old workers, network/stash/branch notices, outcome, conflict view, and refresh state, waits for the requested history to be adopted before re-detecting operations, and ignores late results. **Open…** is disabled while an operation worker, rewrite, or refresh is active, including the path-field Enter route.
- A running continuation replaces the needs-attention strip with a spinner and its worker label (for example **Continuing rebase…**), so the area always names the current phase. The needs-attention strip also names the operation commit.
- Rewrite and refresh failures populate the normal error outcome without setting the immediate `mutation_error` or leaving a masking history error, so **Backups…** remains reachable. During refresh the history header labels the rows **Refreshing · showing previous history**; replacement rows wait for the existing hover/scroll-safe boundary instead of overwriting an interacting table.
- Local Changes disables staging, discard, commit, amend, and duplicate mutations while a rewrite, operation worker, network, or branch operation owns the area, while list selection, scrolling, Back, and Refresh remain available for browsing. Its status line names the blocking operation.
- Network work cannot start during an in-progress merge/rebase, and the duplicate **Cancel network operation** menu alias is removed; the area is the only cancellation surface.
- Tests cover the one-area count at both widths, merge-without-Skip, conflict-window/area ownership, rewrite progress/refresh lifecycle and retained-history label, rewrite failure recovery visibility, operation repository scoping, continuation labels, external Local Changes gating, network cancel placement, duplicate submission, and the existing outcome/recovery behavior.
- Native X11 at 1120×740: [merge conflict in the operation area](screenshots/coherence-i5b-operation-merge-wide.png). At 720×480: [narrow operation area](screenshots/coherence-i5b-operation-merge-narrow.png). Also captured: [rewrite configuration](screenshots/coherence-i5b-rewrite-dialog.png) and [the closed dialog with the rewritten-history outcome](screenshots/coherence-i5b-rewrite-done.png).
- Independent V1 review blocked on repository-scoped results, refresh-failure recovery, continuation labels, retained-history labeling, safe adoption, and Local Changes gating. The candidate fixed them across `a6b2553` and `d8a06ef`; a focused re-review accepted `d8a06ef` after re-running the gates and native checks. The final P3 follow-up in `d967f2b` clears a stale outcome and notices after a successful clone and names the blocking rewrite/refresh in Local Changes.
- Limitations: only Linux/X11, the dark theme, and llvmpipe were inspected natively; a running rewrite/refresh was too short-lived to capture on the fixture, so its area is covered by the unit lifecycle test and the merge capture of the same component.

## Coherence I6 · Conflict preview and Stage resolution · 2026-10-26

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable conflict fixtures (`/tmp/opencode/gg-i5b-merge` and `/tmp/opencode/gg-i6-preview`: a real two-parent merge conflict).

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 417 tests, 0 failures, 3 ignored opt-in performance checks. This includes 214 `git-good` unit tests and 33 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- Conflict resolution is preview-first. Base, Ours, Theirs, and the working file are preview sources; selecting one changes only the **Result** preview and never writes or stages. The working file is the default preview so external edits are inspectable. Wide layouts keep the three side panes with a bounded Result preview beneath them; narrow layouts use Base/Ours/Theirs/Result tabs.
- **Stage resolution** is the single publication step. It revalidates the exact conflict stage object IDs and working bytes captured by the preview, then writes the chosen result (or stages the unchanged working file) and stages the path. A changed stage or working file returns an actionable stale-preview error in the conflict workspace and status line; **Refresh external edits** recaptures the content and snapshot. If the worktree write succeeds but staging fails, the error states that the file was already replaced. Publication is disabled while another operation owns the area or the selected source has no content.
- Publication runs on the operation worker. Success refreshes mutation state, conflicts, history, and working status, selects the next unresolved path, and updates the operation area. Invalid path geometry, absent sides, and mode/symlink/submodule publication return explicit errors without writing. Oversized regular working files are validated by an exact streaming hash over every byte (no sampling) without a text preview; oversized side blobs publish and pass the same post-write verification. Symlink, directory, and submodule worktree paths keep Base/Ours/Theirs previews but disable a Working preview with a reason. Publication is also disabled while network or branch work owns the area.
- Git oracles cover a read-only preview, Ours/Base/Working publication, stale stage IDs, stale working bytes, exact staged object output, partial worktree writes, oversized working files, non-regular working paths, and path traversal rejection. An egui integration test opens a real conflicted merge, confirms the default Working preview and unchanged working file, switches to Ours without writing, then stages the resolution and verifies the file and index.
- Native X11 at 1120×740: [wide conflict workspace with the working-file result preview](screenshots/coherence-i6-conflict-wide.png), [Theirs selected as a read-only preview while the file remains unmerged](screenshots/coherence-i6-conflict-preview.png), and [after Stage resolution with no unmerged paths](screenshots/coherence-i6-conflict-staged.png). At 720×480: [narrow conflict tabs and result preview](screenshots/coherence-i6-conflict-narrow.png).
- Independent V1 review blocked on invisible publication failures, unreported partial worktree writes, and oversized or non-regular working paths removing the side previews. The candidate now shows publication errors in the conflict workspace and status line, reports a worktree write that could not be staged, validates oversized working files by streaming hash, keeps side previews for symlink/directory/submodule worktree paths with a disabled Working preview and reason, clears stale previews when the selected path changes, and disables Stage resolution while another operation owns the area. A focused re-review remains pending.
- Independent V1 review blocked on invisible publication failures, unreported partial worktree writes, and non-exact oversized-file validation. The candidate fixed them across `f993f21` and `20126a7`; a focused re-review accepted `20126a7` after reproducing the stale-byte rejection, oversized-side publication, partial-write report, and all gates.
- Limitations: the conflict preview remains a dedicated resizable workspace rather than a panel inside the Local Changes two-list layout; it uses the same source/preview/publication grammar and operation area. At 720×480 the window can cover the operation area; it is movable and the area returns when the workspace closes. Only Linux/X11, the dark theme, and llvmpipe were inspected natively.

## Coherence I7 · Repository tools, recovery, and pending intent · 2026-10-26

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixture (`/tmp/opencode/gg-i7`: one private rewrite backup, one stash entry, a dirty tracked file, and two commits).

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 429 tests, 0 failures, 3 ignored opt-in performance checks. This includes 226 `git-good` unit tests and 33 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- **Repository → Recovery…** replaces the separate Reflog and Backups windows. One window exposes a **Source** selector for Backups or Reflog and keeps each source's provenance, identifiers, and readers. The idle operation, mutation, and fetch menus now point at the same entry.
- Reflog and Backups both offer **Open commit**. Opening one runs the ordinary commit review and records the recovery source, the Reflog reference filter when present, and that source's own entry identity — the reflog selector or the private backup reference — so closing Review reopens Recovery on the same source, filter, and entry, even when two backups share a commit. Review names its return target **Back to recovery**, and the Recovery window closes while Review is open instead of floating above it.
- Branch creation is one shared form for **New branch** and recovery **Create branch here…**. It is titled for its caller, names the starting commit and selector, validates the name, and exposes **Switch after creation**. New branch defaults the choice on; recovery creation defaults it off. Create always creates first and switches only after success, and success is reported on the active recovery source.
- A blocked switch now retains the target as intent and reports a warning outcome with **Retry switch** and **Dismiss**. The intent is revalidated only when the user invokes Retry and never executes automatically; a successful retry clears the warning and intent, Dismiss clears it without switching, and the created branch survives a blocked follow-up switch. When creation succeeds but the follow-up switch is blocked, the warning names the existing branch and the actual checkout state.
- **Repository → Stashes…** replaces the separate top-level Stash menu; the Help window no longer advertises it. The stash window replaces adjacent Apply and Pop controls with one **Restore…** form exposing **Restore staged state** and **Remove stash after successful restore**. Drop remains a separate confirmed destructive action. Failed restores keep the form, the entry, and its inputs in a bounded error region; successful restores report `Restored <reference>` rather than the retired Applied/Popped vocabulary.
- Tests cover the single Recovery window and source selector, source-specific entry reselection (including duplicate backups at one commit), restoration of a non-HEAD Reflog reference filter and its selected entry, Review's recovery back label and return location, the shared branch form title and switch choice at both widths, the unborn New-branch reason, the stash restore form and both options with a bounded error region, the Restored outcome, blocked-switch intent, Dismiss, and created-branch partial success.
- Native X11 at 1120×740: [Repository menu with one Recovery entry](screenshots/coherence-i7-menu.png), [Recovery · Backups with a private rewrite backup](screenshots/coherence-i7-recovery.png), [Recovery · Reflog after Back restored the non-HEAD `main` filter and `main@{0}` entry](screenshots/coherence-i7-reflog.png), [Stashes with Restore…](screenshots/coherence-i7-stash.png), [Restore stash form](screenshots/coherence-i7-restore.png), and [shared branch form with Switch after creation](screenshots/coherence-i7-branch.png).
- Independent V1 review accepted `7eb50f1` after the source-identity, blocked-intent, window-return, creation-reporting, form-naming, and coverage findings were fixed. Residual commit `54e34a2` restores the non-HEAD Reflog filter and selected entry, aligns restore vocabulary, adds the unborn-reason and bounded-error tests, removes the retired Stash menu from Help, and refreshes the two stale captures. A focused re-review of that residual commit was unavailable; its production changes are confined to the return-location/filter path, the restore outcome label, Dismiss extraction, and Help text.
- Limitations: only Linux/X11, the dark theme, and llvmpipe were inspected natively; captures are wide. The I8 closeout owns the 720×480 and keyboard-journey V2 pass. Recovery remains a resizable window rather than a primary destination; it is reachable from Repository and the outcome recovery action.

## Coherence I8 · Closeout and narrow native journeys · 2026-10-26

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixtures (`/tmp/opencode/gg-i7`: one private rewrite backup, one stash entry, a dirty tracked file, and two commits; `/tmp/opencode/gg-i8-conflict`: a real two-parent merge conflict in `file.txt`).

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 432 tests, 0 failures, 3 ignored opt-in performance checks. This includes 229 `git-good` unit tests and 33 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- **Interaction-parity walk.** The four connected V2 journeys were walked on the integrated tree at 1120×740 and 720×480: History → review → reader → Back; Local Changes with both groups and commit/amend; conflict preview and Stage resolution; Recovery → Open commit → Back; Stashes → Restore form; and the shared branch form. Labels, placement, disable reasons, and return rules matched the owning contracts. The separate Reflog/Backups windows remain test-only wrappers; the retired top-level Stash menu is absent from the toolbar, Repository menu, and Help.
- **Narrow repository tools.** Recovery previously requested a 760-pixel width and hung over the right edge of a 720-pixel viewport. It now defaults to the smaller of 760 pixels and the available content width minus 24 pixels, and caps resizing at that bound. A real-fixture geometry test asserts both axes stay inside 720×480 while Open commit and Copy ref remain rendered. Recovery, Reflog, Stashes, Restore, and New branch all fit with margins at 720×480.
- **Narrow reader reveal.** Opening a file from the narrow review file list could stay on **Loading selected-file diff…** forever because the resting pointer was already over the reader and the hover-safe publication boundary waited for it to leave. A new explicit selection now publishes once under the pointer; asynchronous page replacements still wait for hover to clear. A regression test covers the reveal and the existing hover-wait test remains green.
- **Status hints.** The history status line clipped mid-shortcut at 720 pixels. Narrow widths now show the core bindings (`j/k`, review, details, history, filter) and leave the full list, including paging and copy ID, to wide widths and Help. A rendering test asserts both variants.
- **Deferred elements.** [ui-coherence.md](ui-coherence.md#deferred-elements) now names every design-exploration control that is not shipping: LFS smudging/materialization, restart-persistent drafts, a conflict panel inside Local Changes, and cross-platform/accessibility acceptance. None is represented by a placeholder control.
- Native X11 at 1120×740: [Local Changes](screenshots/coherence-i8-local-wide.png), [review reader](screenshots/coherence-i8-review-wide.png), and [conflict preview](screenshots/coherence-i8-conflict-wide.png); repository tools use the I7 wide captures above.
- Native X11 at 720×480: [History with the shortened status hint](screenshots/coherence-i8-history-narrow.png), [Local Changes](screenshots/coherence-i8-local-narrow.png), [review reader with the diff loaded under the resting pointer](screenshots/coherence-i8-review-narrow.png), [conflict preview](screenshots/coherence-i8-conflict-narrow.png), [Recovery · Backups](screenshots/coherence-i8-recovery-narrow.png), [Recovery · Reflog](screenshots/coherence-i8-reflog-narrow.png), [Stashes](screenshots/coherence-i8-stashes-narrow.png), [Restore form](screenshots/coherence-i8-restore-narrow.png), and [New branch with Switch after creation](screenshots/coherence-i8-branch-narrow.png).
- Limitations: only Linux/X11, the dark theme, and llvmpipe were inspected natively. The conflict workspace still covers the operation area at 720×480; it is movable and the area returns when it closes. Keyboard-only traversal of Recovery and Stashes, live rewrite refresh during a reset, very large reflog performance, blocked-switch intent at 720×480, macOS/Windows, other display scales, and assistive technology remain unverified. Independent V1/V2 sign-off of this closeout commit is pending.

## Coherence proposal and shared workspace · 2026-10-25

Reviewed [cross-view interaction consistency](ui-coherence.md) and created a [shared-workspace prototype](ui-coherence.html). Local Changes incorporates the user's requested arrangement: Unstaged above Staged on the left, selected changes on the right, commit/amend at lower right. File selection determines the comparison; no second Staged/Unstaged switch is present.

- Headless Chromium exercised all five starting scenarios at viewport widths 1120, 720, 390, and 320 pixels without root horizontal overflow or JavaScript runtime errors.
- Geometry checks confirmed the two left-hand groups and the right-hand diff/commit alignment. Interaction checks covered a partially staged file in both groups, stage/unstage readiness, commit success, separate normal/amend drafts and amend base identity, in-place file history with bounded version navigation, restoration of side/attribution, return from commit inspection to the selected Reflog source, conflict-preview versus staging boundaries, continuation across navigation, and stash restore options.
- Visually inspected light and dark layouts. Captures: [Local Changes, wide](screenshots/coherence-local-changes-wide.png), [file history in the shared reader, narrow](screenshots/coherence-file-history-narrow.png).
- JavaScript syntax, local document file links, inline/source equality, and `git diff --check` passed. The editable prototype source is [ui-coherence.fragment.html](ui-coherence.fragment.html).

These checks validate simulated browser interactions only. Production Rust, native Git behavior, accessibility certification, and release acceptance are unchanged; Cargo checks were not run for this design-only change.

## User-flow design pack · 2026-10-25

Reviewed the [documented features and roadmap](user-flows.md) and prepared 17 flows with [20 interactive mockup screens](ui-mockups.html). Existing behavior and proposed M7 interfaces are labeled separately. These are browser wireframes with fictional data and local state, not native application captures.

- Headless Chromium opened the standalone mockups and exercised all 20 screens at viewport widths 1120, 390, and 320 pixels. No root horizontal overflow or JavaScript runtime errors were observed.
- Simulated hunk staging, message editing, commit success, conflict Continue gating/resolution/completion, ambiguous-predecessor selection, push retry/success, rewrite/refresh completion, LFS object availability, and narrow conflict-side selection passed browser interaction checks.
- Inspected captures: [light staging, wide](screenshots/flows-stage-wide.png), [file history, narrow](screenshots/flows-history-narrow.png), and [dark conflicts, narrow](screenshots/flows-conflicts-dark-narrow.png).
- JavaScript syntax, local document file links, source/inline-copy equality, and `git diff --check` passed. Production Rust code was unchanged; Cargo checks were not run for this documentation and mockup change.

Native widget behavior, Git mutations, assistive technology, display scaling, exhaustive contrast, performance, and cross-platform acceptance remain outside this mockup review. M7 delivery and release-gate status are unchanged.

## M0 baseline · 2026-10-18

Linux x86_64 · Rust 1.94. M0 is the accepted browser baseline; M1 is planned. [Coverage and gaps](regression.md).

- 54 default workspace tests, formatting, Clippy, and debug build pass.
- Git fixture oracles verify order, merge parents, shallow boundaries, pinned HEAD, and 499/500/501 histories. CLI tests verify discovery, JSON, errors, uncapped output, and repository preservation.
- Controlled-worker/input tests cover delayed results, refresh/retry, filtering, cache eviction, dragging, keyboard boundaries, and narrow-view restoration.
- Geometry tests cover long text, Unicode, and 720×480, 1120×740, 1600×900 at 100%/200% scale. Avatar tests cover shared sizes, persistence, deduplication, corruption, failures, and deferred publication.
- Regressions reproduce the 242-pixel last-page jump, release-followed-by-motion jump, and hidden-history navigation. Fixes preserve row geometry, release offset, and selection. The streaming test accepts time-sliced first batches before its reader gate.
- CI is configured for offline checks on Linux, macOS, and Windows; this record establishes local results.

## Native X11 review

| Scenario | Captures |
| --- | --- |
| Layout | [Wide](screenshots/m0-wide.png), [narrow](screenshots/m0-narrow.png), [details](screenshots/m0-details.png) |
| Progressive history | [First frame](screenshots/m0-streaming.png) |
| Scrollbar continuity | [Drag](screenshots/m0-interaction-dragging.png), [release](screenshots/m0-interaction-released.png) |
| Full-message search | [Oldest match](screenshots/m0-interaction-oldest-filter.png) |
| Keyboard navigation | [Last page](screenshots/m0-interaction-keyboard-previous.png), [bindings](screenshots/m0-navigation-bindings.png) |

## Performance observation

Linux x86_64 · Intel i5-6600K · debug build · warm OS cache · packed 100,000 commits.

| Measurement | Result |
| --- | --- |
| First rows | 35.6 ms |
| Complete index | 18.8 s |
| In-memory frame p95 | 5.31 ms |
| Cached rows after deep/back navigation | 97 |

The opt-in timing check passed. Frame timings exclude GPU presentation.

## M1 changed files · 2026-10-18

- All 61 default workspace tests, formatting, Clippy, and debug build pass on Linux.
- Git-oracle tests verify root/first-parent comparisons, nested paths, statuses, object IDs/modes, file-to-directory replacement, empty commits, merge-parent selection, and pinned comparisons with unavailable blobs.
- CLI tests verify discovery, explicit repository, exact path bytes, JSON, and error exits.
- A controlled 10,000-file reader verifies first-page delivery, complete enumeration, deep/back page reload, retained errors, and shutdown. UI tests verify placeholder stability and review/history isolation with exact Back anchors. Unread rows keep empty slots with no loading text; loaded rows stay visible.
- Native X11 review confirms status/path rows, metadata, paging, and Back at [1120×740](screenshots/m1-files-wide.png) and [720×480](screenshots/m1-files-narrow.png).
- Large-tree latency/peak-memory measurements and selected-file diffs remain in the [M1 plan](plan.md).

## M1 selected-file text · 2026-10-18

- All 70 default workspace tests, formatting, Clippy, and debug build pass on Linux.
- Git-oracle hunks and byte reconstruction verify CRLF, final newlines, and exact content. Tests cover invalid UTF-8, binary/size/line budgets, CLI errors, worker cancellation, paging/reuse, stale results, and hovered placeholders.
- In-memory input verifies narrow diff/Back selection and pixel anchors. A 100,000-character Unicode line verifies bounded horizontal shaping.
- Native X11 review confirms the [wide workspace](screenshots/m1-diff-wide.png), [narrow reading view](screenshots/m1-diff-narrow.png), paging, and Back.
- Complete export, special-object fixtures, and release resource measurements in the remaining [M1 increments](plan.md).

## M1 parent selection · 2026-10-18

- All 71 default workspace tests, formatting, Clippy, and debug build pass on Linux.
- Git fixtures verify ordered parent IDs, exact first/second-parent files, and missing-parent errors.
- Wide/narrow input tests verify pinned commits, hover-safe replacement, retained results on failure, retry, and empty-comparison clearing.
- Native X11 review verifies selecting parent 2 in the [wide workspace](screenshots/m1-parent-wide.png) and [narrow reading view](screenshots/m1-parent-narrow.png).

## Preview acceptance pending

- Measure cold/warm release startup and peak memory on the packed 100,000-commit fixture. Targets: first rows within 500 ms, input-to-frame below 50 ms, cached frame p95 within 16 ms.
- Verify native clipboard, high-DPI rendering, and live Gravatar delivery.
- Record remaining native build/runtime evidence for the [target platforms](../ROADMAP.md).

## CI and binary delivery · 2026-10-18

- Fixed the floating-point literal warning that failed PR #1's Clippy checks on newer Rust.
- Linux x86_64: formatting, Clippy on Rust 1.94 and 1.97, and 54 workspace tests pass; the packed-history performance test remains opt-in. Test execution takes about 7 seconds after compilation (fresh test compilation: 3m 59s).
- Linux release build and CLI JSON smoke test pass; the tarball contains the binary, README, and license, and its SHA-256 sidecar verifies.
- actionlint 1.7.12 validates the six-target workflow, packaging steps, and gated draft release job.
- Native CI covers Linux, macOS, and Windows on x86_64 and arm64, caches builds, retains independent platform results, and uploads archives with checksums. Version tags create a draft release after all six jobs pass.
- Cross-platform CI results and native runtime acceptance remain pending. Unsigned binary archives do not establish installer, signing, or hardware acceptance.

## Automatic commit releases · 2026-10-19

- Every passing `main` push publishes a development prerelease for its exact commit with all six native archives and SHA-256 sidecars. Version tags publish directly; pull requests and manual runs keep artifacts without publishing.
- Main runs remain independent across commits. Pull-request runs cancel superseded work. Reruns update the same release.
- actionlint 1.7.12 passes. Local execution of the workflow steps verifies main/tag creation, reruns, checksum validation, and rejection of missing, corrupt, or unexpected assets using a recording GitHub CLI stub.
- Rust code is unchanged. The merged PR #2 run passed tests, release builds, and CLI smoke tests on all six targets. Native graphics acceptance remains separate.

## Automatic versioned releases · 2026-10-19

- 55 Rust tests and seven release-version tests pass; formatting and Clippy pass.
- Version tests cover the floor, numeric patch increments, series changes, retries, explicit-tag validation, and consistent manifest/lockfile updates.
- Cargo accepts a generated `0.3.7` workspace with the locked Linux dependency graph. Workflow YAML and shell syntax validate locally; CI verifies binary versions on all six platforms.
- The updated workflow publishes regular releases when merged into `main`.

## M1 special objects and paths · 2026-10-19

- 77 default workspace tests, formatting, Clippy, and debug build pass on Linux.
- Three failing fixtures established the missing mode/symlink summaries and ambiguous invalid-byte path labels; all pass after correction. Additional tests verify cancellation before the next header and exact CLI path/content bytes.
- Native X11 review confirms [wide mode summaries](screenshots/m1-special-wide-mode.png), [narrow mode summaries](screenshots/m1-special-narrow-mode.png), and [symlink targets](screenshots/m1-special-wide-symlink.png). Escaped paths stay on one line.
- Large-file export and release resource measurements remain in the [M1 plan](plan.md).

## M1 export and binary previews · 2026-10-19

- 83 default workspace tests, formatting, Clippy, and debug build pass on Linux.
- Process tests export a packed 3-MiB blob byte-for-byte, preserve existing destinations and source content, and reject absent sides and Git metadata destinations. Cancellation removes partial output; Escape closes the export context.
- PNG/JPEG tests cover dimensions, small-image scale, malformed input, and preview budgets. An egui geometry test preserves 16 hexadecimal rows at narrow widths.
- Native X11 review confirms [before/after images](screenshots/m1-binary-wide.png), [narrow previews](screenshots/m1-binary-narrow.png), [fixed-width hex](screenshots/m1-binary-hex.png), and the [export window](screenshots/m1-export.png).
- Cold/warm release latency and peak-memory measurements remain in M1 acceptance.

## M1 resource acceptance · 2026-10-20

Linux x86_64 · Intel i5-6600K · 16 GiB RAM · Rust 1.94 · release profile. Packed 10,000-file change, first file 1 MiB per side with two edited lines. Fresh process per run; cold-advised requests fixture-cache eviction with `POSIX_FADV_DONTNEED`, followed by three warm runs. Timings use production workers and in-memory egui at 1120×740; GPU presentation is a native gate.

| Measurement | Cold-advised | Warm range |
| --- | --- | --- |
| First summaries | 13.8 ms | 12.0–13.2 ms |
| First diff | 23.4 ms | 19.9–21.1 ms |
| Complete enumeration | 207.7 ms | 198.2–204.9 ms |
| Input frame during enumeration | 0.25 ms | 0.20–0.22 ms |
| Cached frame p95 | 0.17 ms | 0.11–0.14 ms |
| Peak process RSS | 17.4 MiB | 17.4–17.6 MiB |

- All timing/cache assertions pass. Last-page delivery takes 2.5–2.7 ms; returning takes under 0.3 ms. Retained file rows: 95–111; both UI caches stay within 768 rows. Workers retain single-slot mailboxes and bounded pages.
- Packed 64-MiB export: 0.12 s cold-advised / 0.09 s warm, peak 135.6 MiB RSS. The delta-encoded side takes 0.23 / 0.15 s, peak 199.5 MiB. Both exports match source SHA-256. These costs include object decoding and mapped pack pages; output writes remain 64-KiB chunks.
- Repository snapshots remain unchanged after review/export. The process regression also checks staged, unstaged, untracked, index, object, and ref bytes.
- 84 default tests, formatting, Clippy, and debug build pass. The opt-in release review check passes four runs; the history benchmark remains separate.
- Native 200% X11 captures verify [wide previews](screenshots/m1-binary-wide-2x.png) and [narrow hexadecimal rows](screenshots/m1-binary-hex-2x.png). A failing rendered-header regression led to explicit Before/After byte labels, replacing an unsupported arrow glyph.
- Complete native input/GPU measurements and the remaining [platform gates](#preview-acceptance-pending) before closing acceptance.

## M1 retry and portability · 2026-10-20

- A failing input regression reproduced retry resetting selection. Same-comparison retries now retain the selected path, diff generation, pixel anchor, and known count through partial delivery. Parent-switch tests retain their first-file reset.
- CI exposed a newer Clippy chunk-iteration lint and an APFS filename restriction in a fixture. Use typed chunks and import raw paths directly into Git objects; the Unix process test still selects the exact invalid-byte path.
- 86 default Rust tests, formatting, Clippy, and debug/release builds pass locally. The release stress check covers fully changed 1-MiB text, two 8-megapixel PNGs, and rapid selection; platform reruns follow.

## M1 acceptance · 2026-10-20

M1 is complete: commit review, selected-parent diffs, binary previews, exact export, and interaction preservation. [Six native CI jobs](https://github.com/baol/gg/actions/runs/39384178444) pass tests, Clippy, release builds, version/history smoke checks, and packaging on Linux, macOS, and Windows, each on x86_64 and arm64. Local verification passes 86 Rust tests, seven release-version tests, formatting, Clippy, and debug/release builds.

- Same-comparison retry preserves selection, diff, scroll anchor, and known count. Parent changes retain their separate reset behavior.
- Release stress fixture: fully changed 1-MiB text reaches the explicit computation-budget summary in 262–269 ms; an 8-megapixel PNG pair previews in 117–122 ms. Rapid image/text switching settles on the final selection in 257–260 ms. Three fresh-process runs, one cold-advised and two warm, peak at 43.5–43.9 MiB RSS; in-memory frame p95 stays below 0.05 ms.
- The packed 10,000-file check still passes after the retry fix: first rows 13.0 ms, first diff 21.0 ms, cached frame p95 0.11 ms, 95 retained file rows.
- Native release app on Xvfb/X11: 80 keyboard selections with preview publication enabled, measured through single-pixel readback. First visible response: p50 15.8 ms, p95 23.5 ms, maximum 33.0 ms. The [repeatable probe](development.md) excludes full-screen capture overhead and keeps the pointer clear of deferred-publication regions.
- Native review confirms the [explicit text-budget fallback](screenshots/m1-stress-budget.png), readable hex, and available exact export. Earlier wide/narrow and 200% captures remain valid.
- Supported-preview hardware, monitor presentation, accessibility, clipboard, and platform graphics acceptance remain in the [release gates](#preview-acceptance-pending).

## Light-theme contrast · 2026-10-20

- A failing contrast test reproduced pastel status text on light backgrounds. Theme-aware semantic colors now give light diff/status text at least 4.5:1 contrast, including selected rows. Secondary text, links, warning/error colors, and history selection accents use deeper light-theme colors.
- 88 default tests, formatting, Clippy, and debug build pass. Tests check semantic text/fill pairs, shared accents, and unchanged dark visuals through theme switches.
- Native X11 review confirms [light history](screenshots/light-history.png), [wide diffs](screenshots/light-diff-wide.png), [narrow diffs](screenshots/light-diff-narrow.png), and the preserved [dark palette](screenshots/dark-diff-contrast.png).

## M1 visual polish · 2026-10-20

- Embed the supplied `gg.png` for the native window and fixed-size toolbar brand.
- Reproduced oversized avatar textures in an in-memory regression. List/detail textures now match physical display sizes and share one cached source; filtered-detail tests cover 100%, 125%, and 200% scale.
- 89 workspace tests, seven release-script tests, formatting, Clippy, and debug build pass. Native light 1120×740 and dark 720×480 layouts preserve header geometry.

## M2 status · 2026-10-21

- 99 workspace tests, formatting, Clippy, and debug build pass. Git porcelain fixtures cover independent staged/unstaged entries, ignores, intent-to-add, conflicts, unborn branches, nested discovery, linked worktrees, and repository preservation.
- Worker and egui checks cover bounded delivery, cancellation, deep/back paging, hovered loading slots, refresh failure/retry, last-page stability, and history restoration.
- Linux x86_64, debug build, fresh process, warm 10,000-file fixture: first CLI entry 7.3 ms; complete JSON 185.7 ms. Native X11 light 1120×740 and dark 720×480 layouts inspected.

## M2 working diffs · 2026-10-21

- 108 workspace tests, formatting, Clippy, and debug build pass. Git oracles verify reconstructed sides and line counts; fixtures cover stale HEAD/index, changing working files, CRLF normalization, symlinks, emulated symlinks, and binary/budget summaries.
- A failing production-worker regression reproduced cached working bytes after refresh with unchanged HEAD/index. Explicit refresh now invalidates that snapshot; reopening reads the new bytes.
- Native X11 review confirms compact [wide status](screenshots/m2-status-wide.png), [narrow status](screenshots/m2-status-narrow.png), [light staged diffs](screenshots/m2-staged-diff.png), and [dark unstaged diffs](screenshots/m2-unstaged-diff.png).

## M2 whole-file unstaging · 2026-10-21

- 113 workspace tests, formatting, Clippy, and debug build pass. SHA-1/SHA-256 Git oracles cover additions, deletions, staged/working edits, unborn branches, tree-cache invalidation, unrelated staging, stale HEAD/index, lock ownership, and file/directory collisions. CLI checks verify receipts, stderr, and exit status.
- A failing worker regression reproduced paging clearing an action error. Mutation failures now remain visible until explicit retry; successful actions refresh status through its existing interaction boundary.
- Native X11 clicks in disposable repositories verify unstaging at 1120×740 and 720×480. Git confirms the selected path leaves staged changes and working bytes remain intact. [Narrow result](screenshots/m2-unstage-result.png).

## M2 navigation and diff continuity · 2026-10-21

- 119 workspace tests, formatting, Clippy, and debug build pass. Six added tests cover Enter/Right opening a commit at 720 and 1120 widths, Esc/Left returning to files with selection retained, reading keys scrolling without changing files, hover-deferred replacement, and input ownership for focused buttons versus text editors.
- Shared navigation keys treat text editors and open menus as owners of their input; focused buttons still allow list movement, and activation releases focus before opening a selection.
- Diffs retain the displayed path, metadata, preview, rows, and scroll offset while a replacement loads. The current generation publishes together after hover and scrolling settle; failures keep labeled previous content, and an adopted replacement starts at its first row.
- Repository actions live under **Repository** (Open…, Refresh) and presentation under **View** (Commit details · Space). Enter opens commits at both widths, the full-width reading view scrolls with the shared keys, and Status actions sit beside the file list.
- Native X11 review at 1120×740 and 720×480 confirms the reorganized toolbar and Enter-opened views: [wide](screenshots/m2-navigation-wide.png), [narrow](screenshots/m2-navigation-narrow.png), [wide reading](screenshots/m2-reading-wide.png), [narrow reading](screenshots/m2-reading-narrow.png), [status actions](screenshots/m2-status-actions.png), [narrow status actions](screenshots/m2-status-actions-narrow.png).

## M2 file staging · 2026-10-21

- 127 workspace tests, formatting, Clippy, and debug build pass. SHA-1/SHA-256 Git oracles stage modifications, deletions, untracked additions, type changes, intent-to-add, executable modes, and unrelated entries, then compare `ls-files` with `git add -A`. Attribute normalization, symlink targets, and emulated symlinks match Git while working bytes stay intact.
- Fixtures cover unborn branches, lock ownership, stale HEAD/index/file observations, reappearing deletions, conflicts, submodules, and file/directory collisions. CLI checks verify the stage receipt, stderr, exit status, and repeated-action errors; worker tests verify single-flight staging, retained rows on failure, and refreshed areas after success.
- Native X11 clicks stage `README.md` in a disposable repository at 1120×740 and 720×480; Git confirms the path moves to staged changes and its working bytes remain intact. [Wide action](screenshots/m2-stage-wide.png), [narrow action](screenshots/m2-stage-narrow.png), [wide result](screenshots/m2-stage-result.png), [narrow result](screenshots/m2-stage-narrow-result.png).

## M2 hunk staging · 2026-10-21

- 134 workspace tests, formatting, Clippy, and debug build pass. Five engine tests compare selected index blobs with scripted `git add -p` in SHA-1 and SHA-256 repositories. Partial selection writes zeroed stat and reports `MM`; selecting every hunk writes real stat and leaves only staged changes. Multiple selections, CRLF attributes, missing final newlines, and unrelated staged entries match Git while working bytes and lock ownership stay intact.
- Rejections cover untracked additions, intent-to-add, deletions, mode and symlink-target changes, binary comparisons, the 2-MiB text budget, empty selections, unknown headers, stale HEAD/index observations, and an existing index lock. Every rejection leaves the index unchanged.
- CLI `--hunk` receipts report the selected count; unknown headers and a missing value exit with stderr and publish nothing. The egui test clicks a hunk header, verifies the selected highlight and enabled **Stage hunk** action at 720 and 1120 widths, stages successfully back to a refreshed list with staged and unstaged rows, then fails under a held lock and retains the reading view, error, and index.
- Native X11 clicks stage the first of two hunks in a disposable repository at 1120×740 and 720×480. Git reports `MM notes.txt`, one staged and one unstaged hunk, and zeroed index stat. [Wide selection](screenshots/m2-hunk-wide.png), [wide result](screenshots/m2-hunk-result-wide.png), [narrow selection](screenshots/m2-hunk-narrow.png), [narrow result](screenshots/m2-hunk-result-narrow.png).

## M2 split stage view · 2026-10-21

- All workspace tests pass: 149 passed and 3 stress tests ignored. Formatting, Clippy with `-D warnings`, and the debug build pass.
- Engine oracles cover individual-line staging and unstaging, paired old/new line replacement, zeroed stat for partial results, real stat when every change is selected, and partial unstaging of staged additions, modifications, and deletions. Restoring every line restores the HEAD entry; removing the last staged addition removes the index entry. Working bytes, unrelated entries, modes, and locks remain intact.
- Drop oracles select one hunk or one change block, restore filtered CRLF content and missing final newlines, restore modified/deleted/executable/symlink paths from the index, and remove untracked files. The index file bytes stay identical throughout. Partial mode changes and staged inputs are rejected.
- The egui tests open both panes at 720 and 1120 widths, adopt results with the pointer resting over the view, stage a whole hunk from the inline header action, stage one selected line, unstage an inline hunk and the whole file, and confirm the view stays open. Drop tests cancel and then confirm a hunk drop, verify that a selected line reverts its whole change block, drop the remaining file, and confirm the index is byte-identical and the view stays open. Back refreshes the status list; failures retain the panes, error, and unchanged index.
- Native X11 review at 1120×740 uses a disposable repository with one staged and one unstaged hunk. Clicking the unstaged added line changes the inline actions to **Stage 1** and **Drop selected**; staging refreshes both panes in place while Git reports `MM` and the expected mixed index content. Inline **Unstage hunk** and **Unstage all** also stay in the view and refresh both panes. **Drop hunk** requires confirmation, then restores the working side from the index while leaving the staged hunk untouched. At 720×740 the panes stack, the inline actions and line selection remain usable, and the same staging flow works.
- Captures: [wide split](screenshots/m2-split-wide.png), [wide line selected](screenshots/m2-split-line-selected-wide.png), [wide in-place result](screenshots/m2-split-line-result-wide.png), [wide full unstage](screenshots/m2-split-unstage-wide.png), [wide drop confirmation](screenshots/m2-split-drop-confirm-wide.png), [wide drop result](screenshots/m2-split-drop-result-wide.png), [narrow split](screenshots/m2-split-narrow.png), [narrow line selected](screenshots/m2-split-line-selected-narrow.png).

## M2 commit · 2026-10-21

- All workspace tests pass: 163 passed and 3 stress tests ignored. Formatting, Clippy with `-D warnings`, and the debug build pass.
- Commit oracles cover root, normal, detached-HEAD, and linked-worktree commits; only index content is committed, unstaged working bytes survive, the parent and reflog match Git, and history selects the new commit after Back.
- Hook fixtures cover execution order and arguments for `pre-commit`, `prepare-commit-msg`, `commit-msg`, and `post-commit`, custom `core.hooksPath`, index changes made by `pre-commit`, message edits by `commit-msg`, and an empty message left by a hook. A failing post-commit hook returns a warning with the created commit.
- Rejections cover empty messages, empty staged changes, conflicts, intent-to-add, stale HEAD/index observations, and an existing index lock. Hook failures abort before HEAD changes, preserve the index and working tree, include the hook name/status/output, and leave an actionable retry path.
- CLI checks verify JSON receipts and hook-failure stderr. Egui checks verify the commit panel at 720 and 1120 widths, successful commit adoption, refresh, draft retention, and status recovery after a failing hook. Native X11 review confirms commit-panel layout, a clean post-commit status, and the new commit selected in history.
- Captures: [commit message wide](screenshots/m2-commit-message-wide.png), [commit result wide](screenshots/m2-commit-result-wide.png), [history selection wide](screenshots/m2-commit-history-wide.png), [commit panel narrow](screenshots/m2-commit-narrow.png).

## M3 graph, refs, and branches · 2026-10-21

- All workspace tests pass: 189 passed and 3 stress tests ignored. Formatting, Clippy with `-D warnings`, and the debug build pass.
- [Six native CI jobs](https://github.com/baol/gg/actions/runs/39785755711) pass tests, Clippy, release builds, CLI smoke checks, and packaging on Linux, macOS, and Windows on x86_64 and arm64. Windows fixtures pin `core.autocrlf=false`, and graph/ref strokes use explicit `_f32` widths.
- Ref oracles cover HEAD, local branches, remote branches, lightweight tags, annotated tag peeling, symbolic remote HEAD omission, unborn refs, detached HEAD, and duplicate loose/packed refs. `git good refs [PATH]` emits the shared snapshot without changing refs, the index, or working files.
- Graph unit tests cover linear lanes, merge parents, joins, lane reuse, lane retirement, single-parent color continuity, and downward starts for only the row’s own lane. Stream tests keep graph rows aligned with their commits when filtering, and deep selected-ID recovery finds completed rows by identity. An all-ref oracle compares the commit set with `git rev-list --all`, includes branch-, remote-, and tag-only commits, and requires every parent to follow its children.
- Branch oracles cover local switching, creation from HEAD, dirty unstaged/staged/conflicted rejection, duplicate and missing names, invalid names, detached starts, unborn repositories, and `post-checkout` hooks; a failing post-checkout hook leaves the branch switched and returns a warning. Untracked files no longer block switching or creation and survive the switch, while Git still rejects an untracked file that the target branch would overwrite. Successful switches preserve the expected worktree content and symbolic HEAD; failures leave the old branch and working files intact.
- CLI checks verify `git good switch [--create]` JSON receipts and dirty-tree stderr. egui checks verify graph-column geometry, current-first ref labels and overflow, single-click selection, double-click branch switching, tag/remote jumping, and current-branch no-ops, sidebar visibility at 1120 px versus 1119 px, grouped branch/remote/tag trees, filtering, and the branch creation window at supported widths.
- Native X11 review at 1120×740 and 720×740 shows all five disposable-repository commits from every branch and tag with continuous lanes, refined merges, a green third-branch tip that keeps its color while joining an existing orange lane, an orange branch that stays orange through its final join, merge branches that retain their parent-lane color with no duplicate downward stub, ref pills with icons, the grouped refs sidebar with branch folders and HEAD check, branch switching through the sidebar and a pill double-click, tag jumping through a pill double-click, branch creation at HEAD, and the narrow refs-menu fallback. The worktree was clean before each mutation.
- Captures: [sidebar and history](screenshots/m3-refs-sidebar-wide.png), [all-ref history](screenshots/m3-all-refs-wide.png), [branch color join](screenshots/m3-branch-color.png), [branch switched](screenshots/m3-refs-sidebar-switch.png), [new branch modal](screenshots/m3-refs-sidebar-new-branch.png), [narrow history](screenshots/m3-graph-narrow.png).

M3 is complete. Branch deletion/rename, remote tracking, and tag creation remain future work.

## M4 clone, fetch, push, tracking, and credentials · 2026-10-22

Linux x86_64 · Rust 1.99.1 · debug and release builds · local bare remotes.

- 211 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug and release builds pass.
- egui/eframe moved from 0.33 to 0.37 with wgpu 27 to 30 and the workspace MSRV from 1.94 to 1.95. The native app now uses `App::ui`, root-`Ui` panels, `clamped_offset`, `smooth_scroll_delta`, role-based widget info, `Context::request_screenshot`, and theme-specific styles. Tests run through `Context::run_ui` with a helper that clears unapplied texture deltas.
- `Cargo.lock` pins a single `windows` 0.62.2 for `wgpu-hal` and `gpu-allocator`, fixing the Windows DX12 build that failed with two `ID3D12Device` types. `cargo check -p wgpu-hal --target x86_64-pc-windows-msvc` passes locally.
- Sync tests normalize the cloned worktree to `core.autocrlf=false`, compare cloned blob content through `git show`, and compare working bytes before and after fetch, so they pass on Windows with `core.autocrlf=true`.
- The `egui_phosphor_icons` dependency was removed because its latest release targets egui 0.36. The Phosphor regular font is vendored at `apps/git-good/assets/Phosphor.ttf` with its license; app-owned constants and a dedicated font family keep the tracking arrows and the cancel `X`. The vendored font is also a fallback for the default proportional and monospace families, so the toolbar and status line render the tracking arrows without depending on system font coverage. Tests assert the explicit family, the default-family fallback, and that the tracking label uses the Phosphor arrows.
- Tracking tests compare configured remotes, upstreams, and ahead/behind counts with Git for diverged, equal, unrelated, shallow, and missing remote-tracking refs. `git good refs` carries the same remote and tracking snapshot.
- Local bare-remote tests cover clone, fetch, push, non-fast-forward rejection, `--set-upstream`, option injection, URL redaction, credential-failure detection, cancellation, and repository preservation. CLI tests verify JSON receipts, stderr, and exit status.
- Branch switching now ignores untracked files when checking cleanliness, so branch-specific ignore rules no longer block a return switch; Git still rejects an untracked file that the target branch would overwrite and the failure leaves the current branch and file intact. Regression tests cover forward/back switching, creation, and the conflicting-file rejection.
- Native X11 review at 1120×740 and 720×480 confirms upstream and ahead/behind in the sidebar and status line, the Clone and Track remote branch dialogs, and Fetch progress with the cancel action. The first capture exposed the cancel button clipped at the right edge; the toolbar now reserves space while a network operation runs, and `network_cancel_button_stays_inside_the_toolbar` protects both widths.
- The debug `GG_SCREENSHOT` path uses `Context::request_screenshot`; it produced these captures.
- Captures: [tracking wide](screenshots/m4-tracking-wide.png), [tracking narrow](screenshots/m4-tracking-narrow.png), [clone dialog](screenshots/m4-clone-dialog.png), [track dialog](screenshots/m4-track-dialog.png), [fetch progress and cancel](screenshots/m4-fetch-progress.png).
- [Six native CI jobs](https://github.com/baol/gg/actions/runs/39808879624) pass tests, Clippy with `-D warnings`, release builds, CLI smoke checks, and packaging on Linux, macOS, and Windows on x86_64 and arm64 for `beccd6e`. The [release gates](#preview-acceptance-pending) remain unchanged.
- M4 is complete: tracking snapshots, Git-backed clone/fetch/push, remote-branch checkout, credentials, bounded progress, cancellation, branch switching with untracked files, and the egui/eframe 0.37 upgrade are verified.
- Follow-up: `git good --help` lists `fetch`, `push`, and `clone`, [ROADMAP](../ROADMAP.md) marks M4 complete, and `help_lists_every_cli_command` guards the usage banner.

## M5 stash · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 219 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests compare stash push, list, apply, pop, and drop with Git for modified, staged, and untracked changes; messages; `--include-untracked`; `--keep-index`; newest-first ordering; conflicting apply and pop; invalid references; clean trees; and preservation of unrelated files.
- CLI tests verify JSON receipts for `stash list`, `push`, `apply`, `pop`, and `drop`, plus actionable stderr for invalid references.
- egui tests render the **Stash changes…** dialog and **Manage stashes…** window at 720 and 1120 widths, including entry references and the Apply, Pop, and Drop actions.
- [Six native CI jobs](https://github.com/baol/gg/actions/runs/39824257394) pass tests, Clippy with `-D warnings`, release builds, CLI smoke checks, and packaging on Linux, macOS, and Windows on x86_64 and arm64 for `f1d2262`.

## M5 cherry-pick and revert · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 226 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests cover a clean cherry-pick and its parent, cherry-pick conflicts with abort and continue, a conflicting revert with skip, a clean revert, and option-shaped or missing revisions.
- CLI tests verify JSON receipts for `cherry-pick` and `revert`, conflict reporting, `--abort`, `--continue`, and unknown-option stderr.
- egui tests render the in-progress banner with Continue, Skip, and Abort at 720 and 1120 widths.
- [Six native CI jobs](https://github.com/baol/gg/actions/runs/39828199511) pass tests, Clippy with `-D warnings`, release builds, CLI smoke checks, and packaging on Linux, macOS, and Windows on x86_64 and arm64 for `c7a582d`.

## M5 merge and conflicts · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 234 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests cover fast-forward, already-up-to-date, diverged merge commits, forced `--no-ff` merge commits, conflicts with abort and continue, and option-shaped or missing references.
- CLI tests verify JSON receipts for merge, `fast_forward` and `merge_commit` kinds, conflict reporting, `--abort`, `--continue`, `--no-ff`, and unknown-option stderr.
- egui tests render the **Merge branch…** dialog and the merge in-progress banner at 720 and 1120 widths; merges hide the unsupported Skip action.

## M5 conflict resolution · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 241 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests cover unmerged path listing, base/ours/theirs content, side selection with staging, manual staging, add/add conflicts without a base, and unknown-path or base-side rejection.
- CLI tests verify `conflicts` listing, `conflict --file` content, `--use ours|theirs`, `--stage`, and missing-path errors.
- egui tests render the **Resolve conflicts** window with path selection and Base, Ours, and Theirs panes at 720 and 1120 widths.

## M5 recovery and operation state · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 241 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Continue, skip, and abort are covered for cherry-pick, revert, and merge, including merge's unsupported skip and restored HEAD/worktree on abort.
- The in-progress banner exposes Continue, Skip, and Abort and hides Skip for merges.
- Stash, cherry-pick, revert, merge, and conflict resolution refresh an open Local Changes view after success, and the conflict window closes when no operation remains.

## Commit-row context menu and reset · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 247 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- The history-row context menu exposes Cherry-pick, Revert, Reset here (Soft/Mixed/Hard), Copy hash, and Copy subject.
- Git oracle tests cover soft, mixed, and hard reset and reject option-shaped or missing targets; CLI tests verify JSON receipts and missing-mode errors.
- egui tests render the reset confirmation, including the hard-reset warning, at 720 and 1120 widths.

## Compare to local · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 249 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests compare an arbitrary `against` base with the selected commit and assert the old/new sides.
- CLI tests cover `git good show HEAD --against HEAD^`, the generated comparison subject, changed files, and the `--against`/`--parent` conflict.
- The commit-row menu adds **Compare to local**, opening the review diff against the current branch tip.

## Ref context menus · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 253 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests cover safe and forced branch deletion, current/missing-branch rejection, and tag deletion with invalid names.
- CLI tests cover `git good branch --delete BRANCH [--force]` and `git good tag --delete TAG` receipts and errors.
- egui tests render the branch and tag delete confirmations at 720 and 1120 widths.
- Branch, tag, and remote menus expose switch/jump, merge, track, delete, and copy actions.

## M6 amend and reflog · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 272 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests amend with the staged index, preserve the original author and parents, keep a message-only tree, and verify the private backup ref. Amend-base tests show HEAD's contents as staged, use the empty tree for a root commit, and unstage whole files and hunks against the parent without changing working files. Hook fixtures cover `prepare-commit-msg` arguments, `commit-msg`, and `post-rewrite amend` stdin plus warnings.
- CLI tests cover `git good commit --amend`, `git good reflog [REF]`, and `git good branch --create NAME [--at REF]`, including receipts, backup refs, missing references, and branch creation without switching.
- egui tests switch between Local Changes and All Commits from the sidebar and toolbar fallback, keep the toolbar and refs visible, and assert the old Status button is gone. They load the HEAD message into the amend toggle, show the commit's contents as staged, unstage and restage a working change through the unified view, rewrite the commit, clear the amend base, render the **Amended** status line, and render the reflog window and branch-creation dialog at 720 and 1120 widths.
- Reflog read-only tests cover `HEAD` and branch selectors, newest-first entries, times, actions, details, stale-entry clearing, and missing or option-shaped references.
- [Six native CI jobs](https://github.com/baol/gg/actions/runs/39938179241) pass tests, Clippy with `-D warnings`, release builds, CLI smoke checks, and packaging on Linux, macOS, and Windows on x86_64 and arm64 for `3761a1f`.

## M6 rebase · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 280 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests rebase a diverged branch onto another local branch and verify the rewritten parent, preserved subject, single-parent history, and a private `refs/gg/backup/rebase/...` ref pointing at the previous tip. Local branches, tags, and remote-tracking refs all resolve.
- Clean-worktree tests reject tracked changes, including with `rebase.autoStash=true`, unless autostash is explicitly enabled; `--no-autostash` keeps repository configuration from overriding the decision. Option-shaped and missing references are rejected without changing HEAD.
- Conflict fixtures verify rebase state detection, `REBASE_HEAD`, conflict paths, abort restoring the original tip and worktree, continue after staging, and skip completing without the conflicting commit.
- CLI tests cover `git good rebase REF [--autostash]`, receipt fields, backup refs, dirty failures, conflict abort/continue, and unknown-option errors.
- egui tests render the **Rebase branch** dialog with the autostash choice and private-backup warning, plus the in-progress banner with **Continue**, **Skip**, **Abort**, and **Backups…** at 720 and 1120 widths. The Refs menu and branch, remote, and tag context menus expose rebase actions.

## M6 reorder, squash, and rewrite recovery · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 296 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- [Six native CI jobs](https://github.com/baol/gg/actions/runs/39957559424) pass tests, Clippy with `-D warnings`, release builds, CLI smoke checks, and packaging on Linux, macOS, and Windows on x86_64 and arm64 for `171d2f8`.
- Git oracle tests list a linear range oldest first, reorder commits, squash a commit into the previous row with both messages preserved, rewrite a root range through `--root`, and verify a private `refs/gg/backup/rewrite/...` ref points at the previous tip.
- Rewrite validation rejects empty, squash-first, duplicate, missing, and out-of-range steps; non-ancestor and option-shaped starts; and ranges containing merge commits. Merge commits are rejected rather than flattened, and a shallow boundary is rejected instead of being rewritten as a root.
- Conflict fixtures verify the rewrite leaves Git rebase state, lists unmerged paths, aborts to the previous tip and worktree, and skips the conflicting commit to finish the remaining range. Dirty tracked worktrees are rejected unless autostash is explicit, even with `rebase.autoStash=true`; autostash restores the working change.
- CLI tests cover `git good rewrite --from COMMIT STEP...`, reorder, squash, reword, fixup, drop, and edit receipts, backup refs, conflict abort, skip, and unknown-option errors. `git good rewrite --continue|--skip|--abort` targets the same Git rebase state.
- Backup tests verify `gg_git::backup::list` returns the private reference, kind, commit, subject, and commit time, while the refs snapshot continues to exclude `refs/gg/`.
- egui tests render the **Interactive rebase** modal at 720 and 1120 widths: header with range and result count, newest-first plan with action chips, the selected commit with Open in Review, autostash, and the private-backup notice. Keys choose actions (a fixup with nothing below is refused with the reason), badges name the combination target, Esc cancels without starting, and Enter opens Review with **Back to rebase** and returns to the same plan. `rebase_plan` unit tests cover targets, badges, summaries, reword messages, the last kept commit, and step order. Review keeps Back, the commit identity, and the file count on one header line. At a reopened-commit stop, egui tests at 720 and 1120 widths check that Local Changes opens with the original message, the operation area names the stop and step (`Rebase stopped · step 2 of 3 · reopened <id>`), Continue waits with its reason and ignores `Ctrl+Enter` while changes remain, the identity reads `feature · rebase 2/3`, and commits made at the stop are listed. Git-fixture tests check that Reopen stages the commit's changes on its parent, that progress names done and remaining steps, that a split continues, that a root commit stays at HEAD, and that a conflict stop is reported. Conflict blocks: engine tests parse `diff3` output and compose each choice with markers kept for open blocks; Git-fixture tests split a two-block conflict regardless of `merge.conflictStyle`, stage composed text, refuse a changed working file, and name why a deleted side has no blocks; an egui test on a real conflict chooses with key 2, shows the block labels and result count, and leaves the file untouched until staging. Rebase dialog keys keep working after a button takes focus. The Stashes picker test checks ref, message without the branch prefix, branch or **detached**, age, and file count, and that the popup keeps its size when **Drop…** swaps in the confirmation; a Git-fixture test reads the branch, age, and file count (tracked, staged, and untracked) for a branch and a detached stash. Preferences: settings tests round-trip every preference, keep unknown keys, and reset only a malformed value; folder tests migrate `roots.json` with its default, reorder, count repositories, report a missing folder, refuse relative paths, and prefer the first folder after removing the preferred one; an egui test at 720 and 1120 widths lists the sections, a repository count, and a missing folder, names `Already in the list.`, and saves a theme change immediately while the window stays open and in place. A pending rewrite keeps the modal open with **Rewriting commits…** and ignores Escape; a pending refresh shows **Refreshing history…** and closes only after the replacement history generation is accepted. The Backups window renders the entry list, **Open commit**, **Reset branch here**, **Create branch here…**, **Copy hash**, and **Copy ref**. The in-progress banner exposes **Backups…** and names an in-progress rebase. Preference memory: settings tests round-trip the reader presentation, image layout, signing default, details pane, stash, merge, new-branch, and fetch-prune keys and keep defaults for absent keys; a reader opens in the preferred presentation once and keeps a later explicit choice; the image layout starts from and remembers the last choice per window; the Sign checkbox follows the Commit preference; and stash, merge, and new-branch forms open with remembered values, save nothing until submission, and save the submitted merge choice. The Preferences window shows the Diff and Commit sections at 720 width.
- The native rewrite runs Git on a background worker. The modal disables its controls during the run, overrides the history hover-safety gate for the rewrite refresh, and stays open until the replacement history has loaded instead of exposing the previous commit list.

## Pre-M7 commit draft retention · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 300 workspace tests pass (3 ignored); formatting and Clippy with `-D warnings` pass.
- A Browser-level regression reproduces the old loss: a multiline draft set in Local Changes, then navigation to history and back through All Commits restores the exact text.
- Drafts are keyed by canonical worktree identity: a second worktree starts with an empty draft, and returning to the first worktree restores its draft.
- Normal and amend drafts stay separate across an amend toggle round trip; enabling amend keeps the normal draft, and disabling it restores the normal text while retaining the amend text.
- The selected path and area are captured with the draft and restored when the same entry is present in the refreshed status.
- Hook-failure coverage verifies a failed commit keeps the submitted message; successful-commit coverage verifies the submitted draft clears.
- Native wide/narrow interaction review remains pending for this increment.

## Pre-M7 operation workers · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 302 workspace tests pass (3 ignored); formatting and Clippy with `-D warnings` pass.
- Merge, rebase, and continue/skip/abort share one background operation worker. The dialog returns immediately, the pending operation is named in the status line, duplicate submission is rejected, and the accepted result refreshes mutation state, history, refs, status, reflog, and backups once.
- The event-driven regression holds a worker behind a channel, proves a frame still renders and names the pending operation while the worker is blocked, rejects a second submission, then releases the worker and verifies one accepted result.
- A fixture `pre-rebase` hook blocks on a marker file. The rebase start returns immediately, a frame renders and names **Rebasing onto feature…** while the hook waits, and releasing the marker completes the rebase with the accepted notice.
- Worker failure or disconnection clears the pending label and reports an actionable error while preserving Git's recovery state.
- Native wide/narrow interaction review remains pending for this increment.

## Pre-M7 conflict workflow · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 305 workspace tests pass (3 ignored); formatting and Clippy with `-D warnings` pass.
- The conflict window names the operation, unresolved count, and operation commit; explains Ours/Theirs for merge, rebase, cherry-pick, and revert; states that side choices replace and stage the whole file; and states that **Stage resolved** stages external or manually edited content.
- Continue is disabled while unresolved paths remain and explains why. Skip and Abort remain deliberate actions with explicit consequences.
- Conflict listing, content reads, and side resolution run on workers. The content test tolerates a blocked reader while frames render and adopts content when it arrives; side resolution starts and completes through the shared operation worker.
- Resolving a path reloads the list and selects the next unresolved path predictably; a pure selection test covers keep-found, keep-missing, and empty lists.
- At narrow widths the Base/Ours/Theirs panes become side tabs with one bounded scroll region; the path list keeps its own bounded scroll.
- Native wide/narrow interaction review remains pending for this increment.

## Pre-M7 comparison direction and scope · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 306 workspace tests pass (3 ignored); formatting and Clippy with `-D warnings` pass.
- The commit-row action is **Compare with current HEAD**. A review test renders an `--against HEAD` comparison and checks the persistent Before line for the `HEAD` ref name plus the resolved base ID and the After line for the resolved target ID.
- Whole-file staging actions are **Stage file**, **Unstage file**, and **Discard file**. Hunk and selected-line actions remain distinct, and the confirmation names the exact file and scope and states that the index is unchanged.
- Native wide/narrow interaction review remains pending for this increment.

## Pre-M7 commit readiness and amend mode · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 308 workspace tests pass (3 ignored); formatting and Clippy with `-D warnings` pass.
- Commit readiness is derived from the completed status scan and validated identity. A normal commit requires a nonempty message and at least one staged entry; message-only amend stays valid; disabled reasons cover a running operation, a pending scan, an empty message, and no staged files.
- The amend banner names the affected commit and its subject directly above the commit action.
- The message editor expands and collapses, and the existing commit/hook/amend fixtures still pass.
- Native wide/narrow interaction review remains pending for this increment.

## Pre-M7 responsive staging · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 309 workspace tests pass (3 ignored); formatting and Clippy with `-D warnings` pass.
- Wide widths keep both comparisons side by side. Narrow widths show a **Staged**/**Unstaged** switch with change counts; the nonempty pane is focused when the other is empty, and **Split** stacks both comparisons.
- The combined-view test exercises both wide and narrow modes, switches tabs, verifies the focused and inactive hunk actions, and toggles **Split** to confirm both panes remain available.
- A narrow-only test opens an unstaged-only file and confirms the unstaged pane is focused by default with **Stage hunk** visible and **Unstage hunk** absent.
- Pane selection and scroll state remain on their own panes across focus and layout changes.
- Native wide/narrow interaction review remains pending for this increment.

## Pre-M7 action discoverability · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 313 workspace tests pass (3 ignored); formatting and Clippy with `-D warnings` pass.
- The toolbar **Actions** menu and the **Commit actions** window use the same grouped command definition as the commit-row context menu: Inspect, Integrate, Rewrite and destructive, and Recovery. Labels carry the selected short commit.
- Mutation commands are disabled with an explanation while a mutation, operation worker, branch switch, or network operation is active.
- Alt+A toggles the commit-actions window. The **Help** menu opens a keyboard-shortcut window that names navigation, repository, and action bindings.
- Tests render the grouped window at 720 and 1120 widths, exercise the Alt+A shortcut, verify the shared menu returns the clicked command, and render the Help window.
- Native wide/narrow interaction review remains pending for this increment.

## Pre-M7 readable operation outcomes · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 316 workspace tests pass (3 ignored); formatting and Clippy with `-D warnings` pass.
- Merge, rebase, recovery, rewrite, push, fetch, and stash results populate a severity, summary, optional full details, and at most one recovery action. Errors outrank success notices in the status line.
- Tests render a conflict outcome summary with **Details…** and **Resolve conflicts…** at 720 and 1120 widths, open the selectable **Operation details** window, and click **Backups…** to open the recovery surface.
- Rewrite outcomes include the saved backup ref and conflict paths in the details text.
- Native wide/narrow interaction review remains pending for this increment.

## Native screenshot hook · 2026-10-24

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe.

- The tracked pre-commit hook builds the debug app, captures 1120×740, converts the PPM to PNG with the standard library, and stages `docs/screenshots/gg-native-wide.png` when UI sources are staged.
- The screenshot path requests one capture after a settle delay, so repeated repaints cannot interleave writes.
- `python3 -m unittest discover -s scripts -p 'test_*.py'` covers PPM header and pixel parsing, comments, CRLF headers, and invalid input.
- The hook skips during merge, rebase, cherry-pick, and revert operations and when staged UI files also have unstaged or untracked edits.

## M7 file reading surface · 2026-10-24

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 334 workspace tests pass (3 ignored); formatting and Clippy with `-D warnings` pass.
- **Full file** reads one side on the selected-file worker, keyed by generation and side, bounded to 2 MiB and 100,000 source lines. Changed-line markers reuse the retained diff document, so toggling presentation or side never recomputes the comparison. Absent, binary, gitlink, symlink, and over-budget sides return explicit summaries.
- Switching Diff/Full keeps the file, side, and approximate line. When no diff line is loaded yet, full mode adopts the first change; the mode row names the current or next change and the footer names the line/range/change/file bindings. Returning to diff or moving to another file requests the page centered on the reading line, so a distant place is not lost.
- `[`/`]`, **Prev change**, and **Next change** move between changed blocks in full mode and hunks in diff mode. `Alt+Up/Down` and **Prev file**/**Next file** change the selected file without leaving reading mode.
- Change navigation distinguishes "before the first change" from a cursor on a change. **Next change** reaches the first hunk or changed group even when it starts at the first row or line, a single change is terminal after that jump, and each button is enabled only while a target exists in its direction. The `]` key is exercised through the rendered view, including scrolling a far hunk into range.
- Native X11 at 1120×740: [diff](screenshots/m7-file-wide-diff.png), [full file at the first change](screenshots/m7-file-wide-full.png), [second change](screenshots/m7-file-wide-change.png), [before side](screenshots/m7-file-wide-before.png), and [Alt+Down to the next file](screenshots/m7-file-wide-file.png).
- Native X11 at 720×480: [full file](screenshots/m7-file-narrow-full.png) and [second change](screenshots/m7-file-narrow-change.png).
- Full-file reads and reading navigation never write objects, refs, the index, or working files.

## Real-repository performance · VS Code · 2026-10-27

Linux x86_64 · Rust 1.99.1 · release build · `/home/mirko/dev/vscode` (194,924 commits reachable from all refs, 6,205 refs: 1 local branch, 5,804 remote branches, 400 tags; 1.7 GiB packed objects). Filesystem cache was warm after repeated fixture inspection; these are not cold-start numbers.

| Check | Result |
| --- | --- |
| `git good history` CLI (streams 66 MiB JSON) | 3.92 s wall, 222 MiB peak RSS |
| `git good refs` CLI | 1.01 s wall, 125 MiB peak RSS |
| `git good status` CLI | 2.10 s wall, 118 MiB peak RSS |
| Native history first rows | 2.3–2.8 s |
| Native history complete | 17.1–18.6 s |
| In-memory history frame p95 | 38.7–41.2 ms |
| Deep jumps to oldest and back | 69–111 ms |
| Retained history rows | 94 (within the 768-row cache) |
| Native test peak RSS | 270–293 MiB |

The production worker test `real_repository_history_first_frame_and_deep_navigation` now records this shape for any `GG_TEST_REPOSITORY`. The initial measurement was 6.70 s to first rows and 27.0 s to complete. **Optimization:** when the union has more than 512 divergent tips, history now delegates topological ordering to `git rev-list --topo-order --stdin` and reads commit metadata through gitoxide; the refs snapshot is reused instead of scanned twice. The two measured runs after the change reached first rows in 2.3–2.8 s and completion in 17.1–18.6 s, with the same 194,924 commits and the same first 20 commit IDs as `git rev-list --topo-order --all`. CLI history dropped from 13.91 s to 3.92 s. **Remaining gap:** first rows still miss the 500 ms M0 target because full topological order over 194,924 commits and 6,205 refs takes about 1.7 s in Git itself before its first line. Meeting 500 ms would require a provisional HEAD-only page or a persisted ordering cache, which would change the current all-refs topo contract. Input-to-frame p95 stays below the 50 ms interaction target but above the 16 ms cached-frame target; one back jump reached 111 ms. Deep jumps and retained-row bounds otherwise pass, and the result is recorded for the next decision.

## Local Changes staging gestures · 2026-10-28

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixture (`/tmp/opencode/gg-commit-exp`: three modified tracked files, with one staged to show both groups).

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 490 tests, 0 failures, 4 ignored opt-in performance checks. This includes 248 `git-good` unit tests and 42 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- A single click selects the file and its comparison; a double-click on an Unstaged/Untracked row stages the whole file, and a double-click on a Staged row unstages it. Conflict and intent-to-add rows are inert rather than broadening to a partial action.
- The selected file header now carries its own **Stage** or **Unstage** button beside File actions (replacing the duplicate menu entry). It is disabled with a reason while the comparison is loading, a mutation is pending, or the row cannot be staged as a whole file; hunk and selected-line staging remain in the reader.
- After staging a whole file, the next Unstaged row in display order is selected (the previous row when the staged row was last). If no Unstaged rows remain, the newly staged row stays selected in Staged. After unstaging, the same path is selected in Unstaged. A failed mutation keeps the current selection.
- The egui interaction test uses real double-click timing over three files and covers staging a/b/c in turn, the last-file fallback into Staged, double-click unstaging, and both header buttons. Repository assertions confirm only the intended index entries changed.
- Native X11 at 1120×740: [the selected unstaged file with its own Stage button](screenshots/m7-commit-stage-button.png) and [a staged file with its own Unstage button](screenshots/m7-commit-unstage-button.png). At 720×480: [the Stage button in the narrow Local Changes layout](screenshots/m7-commit-stage-narrow.png).
- Limitations: only Linux/X11, the dark theme, and llvmpipe were inspected natively. Touch double-tap and platform-specific double-click timing remain platform checks.

## Force push with lease · 2026-10-28

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable bare remote (`/tmp/opencode/force-remote.git`) with a local branch tracking `origin/main`.

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 489 tests, 0 failures, 4 ignored opt-in performance checks. This includes 247 `git-good` unit tests and 42 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- **Force push with lease** is a separate Repository action, enabled only when the current branch has an upstream and a remote-tracking ref. The confirmation names `origin/main`, the local branch, and the lease commit and states that a changed remote rejects the rewrite and that bare `--force` is never used.
- The confirmation captures the lease from `refs/remotes/<remote>/<branch>` and the worker passes that exact commit as `--force-with-lease=refs/heads/<remote-branch>:<lease>`, so a later fetch cannot broaden what the user confirmed. A stale lease failed with Git's `stale info`, left the remote and local branch unchanged, and offered only **Dismiss** rather than a retry that would become non-force. After `fetch` updated the lease, the same rewrite succeeded; an attempt using the previously confirmed lease still failed, and the receipt named the new lease. Refs refreshed to a clean `main...origin/main`.
- Normal push still rejects a diverged branch. CLI `git good push ... --force-with-lease` prints `force_with_lease` and `lease`; `--force` is rejected as an unknown option.
- Git fixtures cover stale rejection, fetch-updated success, receipt lease identity, and the non-force reminder. Native X11 at 1120×740: [the lease confirmation](screenshots/m7-force-lease-dialog.png), [a successful leased rewrite](screenshots/m7-force-lease-success.png), and [a stale lease rejected without changing the remote](screenshots/m7-force-lease-stale.png). At 720×480: [the confirmation inside the viewport](screenshots/m7-force-lease-narrow.png).
- Limitations: only local file remotes were exercised; live credentials, protected branches, and server-side force-push policy are native/remote checks. Only Linux/X11, the dark theme, and llvmpipe were inspected natively.

## M7 image onion skin · 2026-10-28

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixture (`/tmp/opencode/gg-images`: a 120×80 RGBA PNG replaced by a 160×100 RGBA PNG across two commits).

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 480 tests, 0 failures, 4 ignored opt-in performance checks.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- **Onion skin** is a third wide presentation beside Side by side and Swipe. Before is drawn at full opacity and After is overlaid with normal alpha blending, initially at 50%. The labeled opacity slider is keyboard-adjustable when focused; five Right-arrow presses changed the native capture from 50% to 70%.
- Each image keeps its own dimensions when centered: the 120×80 Before and 160×100 After are not stretched to a common size. Fit never upscales, Actual size remains available with scrolling, the theme-aware checkerboard stays visible around the canvas, and per-side metadata and export are unchanged.
- At widths below 800 px the control row keeps the stacked before/after presentation and does not offer Onion skin. Selecting Onion skin when either side is absent shows that both images are required and retains the explicit absent-side state instead of blending.
- Unit tests cover the wide-only control, the default 50%/adjusted opacity label, overlay metadata, alpha conversion, and the one-sided fallback. Native X11 at 1120×740: [Onion skin at 50%](screenshots/m7-onion-wide.png) and [after keyboard adjustment to 70%](screenshots/m7-onion-adjusted.png). At 720×480: [stacked preview with no Onion skin control](screenshots/m7-onion-narrow.png).
- Limitations: only Linux/X11, the dark theme, and llvmpipe were inspected natively. Difference/heatmap and other blend modes remain separate work, and this increment adds no pan/zoom beyond Fit and Actual size.

## M7 Git LFS working pointers and preview · 2026-10-28

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixture (`/tmp/opencode/gg-lfs-local`: an index pointer and a different working pointer for a 120×80 RGBA PNG, with both objects present in the local LFS store).

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 486 tests, 0 failures, 4 ignored opt-in performance checks. This includes 246 `git-good` unit tests and 41 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- Local Changes exposes **File actions → LFS details…** for a pointer comparison. Staged uses HEAD→index, Unstaged uses index→working file, and Untracked uses absent→working file; working-file reads stop at 4 KiB. Conflicts report that the path must be resolved instead of presenting an index stage. Git fixtures cover all four areas and plain working text.
- The details window loads a bounded preview of the selected locally available object. A 120×80 PNG rendered with the checkerboard and fit rules; objects over 2 MiB, unreadable objects, missing objects, and non-image content keep an explicit note. Preview does not smudge, materialize, stage, or write working files.
- CLI `git good lfs status|fetch PATH --area staged|unstaged|untracked|conflict` returns the same working-scope details and rejects combining `--area` with `--revision`/`--before`. The existing revision-scope CLI and receipt behavior is unchanged.
- Native X11 at 1120×740: [Local Changes pointer with an available working-side object and PNG preview](screenshots/m7-lfs-working.png), and [the working-side object missing with an explicit fetch-to-preview note](screenshots/m7-lfs-working-missing.png). At 720×480: [the same details window and preview](screenshots/m7-lfs-working-narrow.png).
- Limitations: `git-lfs` is not installed in this environment, so a successful network fetch remains unverified; the fabricated local object store exercises availability and preview only. Smudging/materializing fetched content into working files remains planned. Only Linux/X11, the dark theme, and llvmpipe were inspected natively.

## M7 commit and tag signing · 2026-10-28

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixture (`/tmp/opencode/gg-sign`) · isolated OpenPGP home (`/tmp/opencode/gnupg`, test key only) · empty keyring (`/tmp/opencode/emptygnupg`) for the unknown-key state.

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 479 tests, 0 failures, 4 ignored opt-in performance checks. This includes 243 `git-good` unit tests and 40 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- Commit details → **Signature details…** and a tag ref menu → **Signature details…** report the object kind, state, signer, key fingerprint/ID, trust label, declared format, and read-only signing configuration. Good, unsigned, bad, unknown-key, unavailable, and error are distinct states. Git raw verification parsers cover `GOODSIG`, `VALIDSIG`, `BADSIG`, `ERRSIG`/`NO_PUBKEY`, `TRUST_*`, and Git's unsigned-tag “no signature found” message.
- The window states that verification is read-only and does not change trust stores or configuration. An unknown public key rendered “Unknown key” while the signing configuration still showed the key and enabled commit/tag signing, keeping validity, trust, and setup separate.
- **Sign** in Local Changes is initialized from `commit.gpgsign`, is captured with the draft, and is enabled only when a key is configured and the signing tool is available. A blocked setup disables the commit with a reason instead of starting an unsigned commit; the unit test confirms the message, HEAD, and index are unchanged. A deliberate opt-out remains visible and produces an unsigned commit.
- Signed commits run the existing hooks, then `git commit-tree -S` with the observed tree, parents, message, and pinned identities; HEAD is updated only after signing succeeds. A forced missing-signer fixture fails with “Signing failed”, leaves HEAD and index unchanged, and never creates an unsigned fallback. The amended path follows the same rule.
- CLI `git good commit --sign` successfully created a good OpenPGP-signed commit against the isolated test key; the receipt reported `"signed": true` and `git verify-commit --raw` returned `GOODSIG`/`VALIDSIG`/`TRUST_ULTIMATE`. CLI `git good commit` without a flag follows `commit.gpgsign`; `--no-sign` is an explicit opt-out.
- **Refs → New tag…** (wide sidebar, Refs menu, and narrow width) requires a name, explicit target, and message. **Sign tag** follows `tag.gpgsign` and is disabled with a reason when the setup is unavailable. CLI `git good tag --create v2 --at HEAD -m MESSAGE --sign` created a good OpenPGP-signed tag verified by `git verify-tag --raw`; an unavailable signer left no tag. Unsigned annotated tags are also covered.
- `git good signature config|verify [REF] [--repo PATH]` returns the same details JSON; `git good signature verify` resolved both a commit and a tag to `kind` and `state`.
- Native X11 at 1120×740: [good commit signature with enabled signing configuration](screenshots/m7-signature-good.png), [unknown public key kept distinct from the enabled configuration](screenshots/m7-signature-unknown-key.png), [commit form with the Sign choice](screenshots/m7-commit-sign.png), and [the signed-tag dialog](screenshots/m7-tag-sign.png). At 720×480: [the signature window](screenshots/m7-signature-narrow.png) and [the tag dialog](screenshots/m7-tag-sign-narrow.png).
- Limitations: only OpenPGP was exercised; SSH and X.509 signing/verification rely on Git's configured tool and the same raw-status parser but were not exercised natively. Key generation and signing used an isolated test keyring, never the user's keyring or trust database. Configuration readiness verifies that a key string is set and the tool runs; it does not prove the key exists or the agent can unlock it, so that failure remains an explicit signing error. Only Linux/X11, the dark theme, and llvmpipe were inspected natively.

## M7 Git LFS · 2026-10-28

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixture (`/tmp/opencode/gg-lfs`: a canonical pointer for a 12,345-byte SHA-256 object, tested with the local object missing and present).

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 469 tests, 0 failures, 4 ignored opt-in performance checks. This includes 241 `git-good` unit tests and 37 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- The reader detects canonical LFS v1 pointers from the selected file text and exposes **File actions → LFS details…**. The window names the exact path, Before/After OID and declared size, whether each object exists in the repository LFS object store, whether `git-lfs` is installed, and the remote. Missing, available, malformed, and non-pointer states stay explicit.
- **Fetch object…** is enabled only for a missing target with `git-lfs` available. The confirmation names OID, size, and remote and states that fetching may download data and does not change or smudge working files. Fetch runs on a worker via `git lfs fetch [REMOTE] --include=<path>` and rescans availability. No automatic network, smudge/materialization, staging, or working-tree write is added.
- `git good lfs status|fetch PATH [--revision REF] [--before REF] [--repo PATH]` returns the same bounded pointer details and receipt. Git fixtures cover canonical/malformed parsing, before/after pointers, plain text, missing objects, and locally available objects.
- Native X11: [missing LFS object with fetch unavailable because git-lfs is not installed](screenshots/m7-lfs-missing.png), [object available in the local LFS store](screenshots/m7-lfs-available.png), and [the narrow LFS window](screenshots/m7-lfs-narrow.png).
- Limitations: `git-lfs` is not installed in this environment, so a successful fetch was not exercised natively or in CI; the disabled state is reproducible. Local Changes/working-tree pointers and bounded previews are recorded in the later M7 Git LFS working pointers and preview section. Only Linux/X11, the dark theme, and llvmpipe were inspected natively.

## M7 submodules · 2026-10-27

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixtures (`/tmp/opencode/gg-submodule` with `/tmp/opencode/gg-submodule-nested`: a parent gitlink plus a nested repository, exercised clean, dirty, and at an older nested commit).

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 461 tests, 0 failures, 4 ignored opt-in performance checks. This includes 239 `git-good` unit tests and 36 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- The reader's **File actions → Submodule details…** names the exact path, Before/After recorded gitlinks, parent index commit, nested HEAD, initialization and dirty state, URL/branch, configured-URL differences, object availability, and explicit missing-entry/status notes. Opening details never initializes, checks out, stages, or fetches anything.
- **Update…** targets the parent index gitlink. Dirty nested content, a recorded commit different from the index, and a missing recorded object keep Update disabled with the reason. The confirmation names the target and URL, warns that Git may fetch, and states that the parent index is not staged. The worker runs `git submodule update --init -- <path>` and rescans details; the parent gitlink remains unstaged.
- **Open nested repository** switches to the initialized nested repository as its own context while the parent selection and draft stay in the worktree state map. Update does not add recursive, force, or parent-staging behavior.
- CLI `git good submodule status|update PATH [--repo PATH]` returns the same details and receipt. Git fixtures cover clean/nested state, dirty blocking, index mismatch, update restoring the index commit, and an uninitialized clone retaining the parent index identity.
- Native X11: [details at an older nested commit with Update enabled](screenshots/m7-submodule-wide.png), [dirty nested content blocking Update](screenshots/m7-submodule-dirty.png), [the explicit update confirmation](screenshots/m7-submodule-confirm.png), [after the nested checkout is restored to the index commit](screenshots/m7-submodule-updated.png), and [the narrow details window](screenshots/m7-submodule-narrow.png).
- Limitations: only Linux/X11, the dark theme, and llvmpipe were inspected natively. Offline/auth failures, missing recorded objects, and changed URL/configuration are covered by state logic/tests rather than native network captures. Nested Open is exercised through the state/worker paths; the parent has no dedicated Back button after switching, so return uses Repository → Open or the worktree state map.

## M7 worktrees · 2026-10-27

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixture (`/tmp/opencode/gg-worktrees`: main plus a linked `feature` worktree with a modified `a.txt`).

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 453 tests, 0 failures, 3 ignored opt-in performance checks. This includes 237 `git-good` unit tests and 35 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- **Repository → Worktrees…** lists the main and linked worktrees with exact paths, branch/detached/bare labels, current/main markers, lock/prunable reasons, and clean/modified/unavailable status. The window clamps inside 720×480 and keeps Open and Remove controls reachable.
- **Create worktree…** accepts an absent or empty destination and either an existing local branch or a new branch with a start revision. A branch checked out elsewhere is named with its path instead of forced. Add runs on a worker, keeps the dialog on failure, and lists the worktree only after success.
- **Open** saves the current worktree's selection, filter, pixel place, narrow-details state, and Local Changes draft before loading the target. Each worktree restores its own state; a new worktree starts empty.
- **Remove…** re-lists and blocks main, current, dirty, locked, missing/prunable, and status-unavailable worktrees with explicit reasons. Confirmation names the exact path and branch, and `git worktree remove` runs without force. Missing registrations are not pruned here.
- CLI `git good worktree list|add|remove` returns the same inventory and receipts as JSON. Tests cover new/existing branches, checked-out branches, nonempty destinations, dirty/main removal, and clean removal.
- Native X11: [wide worktree inventory with a modified linked worktree](screenshots/m7-worktrees-wide.png), [narrow inventory inside 720×480](screenshots/m7-worktrees-narrow.png), and [the shared create form](screenshots/m7-worktree-create.png).
- Limitations: only Linux/X11, the dark theme, and llvmpipe were inspected natively. Native Open and Remove confirmation flows were exercised through egui state tests rather than separate captures. Locked/prunable states and large worktree inventories were not natively reproduced. Prune is intentionally not offered.

## M7 image diffs · 2026-10-27

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixture (`/tmp/opencode/gg-images`: a 120×80 RGBA PNG replaced by a 160×100 RGBA PNG across two commits).

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 443 tests, 0 failures, 3 ignored opt-in performance checks. This includes 234 `git-good` unit tests and 34 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- The reader names each image side's short blob ID, exact dimensions, encoded bytes, and absent/budget/gitlink state. PNG and JPEG decode through the bounded worker; corrupt, oversized, unknown, and over-budget content keeps metadata, hexadecimal fallback where available, and per-side export.
- **Side by side** is the wide default. **Swipe** aligns each image by its own dimensions without stretching, clips After over Before at a labeled boundary, supports the slider's keyboard arrows and canvas dragging, and paints a theme-aware checkerboard behind transparency. **Fit** preserves aspect ratio and never upscales; **Actual size** shows natural pixels with a 100% label and scrolling. At 720 width the view stacks named Before/After previews instead of forcing a narrow swipe.
- Preserved alpha is asserted from a decoded RGBA image. Aspect-ratio fitting, non-upscaling, presentation controls, dimensions, and swipe labels are covered by egui tests; export continues through the existing exact-byte route.
- Native X11: [wide side-by-side with dimensions and byte sizes](screenshots/m7-image-wide.png), [wide swipe with the labeled boundary and checkerboard](screenshots/m7-image-swipe.png), and [narrow stacked previews](screenshots/m7-image-narrow.png).
- Limitations: only Linux/X11, the dark theme, and llvmpipe were inspected natively. There is no pixel-difference heatmap, free pan/zoom beyond Actual size, or animation; those were not part of this increment. Onion skin is recorded separately.

## M7 blame attribution · 2026-10-27

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixture (`/tmp/opencode/gg-blame`: six-line `src/app.rs` with three authors and three commits).

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked` passed: 441 tests, 0 failures, 3 ignored opt-in performance checks. This includes 232 `git-good` unit tests and 34 CLI tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- `gg_git::blame::read` resolves the committed path and revision, checks the regular-blob budget (2 MiB and 100,000 lines), refuses binary content, gitlinks, and symlinks, clamps a requested range to the file, and parses `git blame --line-porcelain` into author, email, author time, subject, current line, and origin line. A cancelled request returns an explicit cancellation.
- `git good blame PATH [--revision REF] [--lines START,END] [--repo PATH]` returns the same bounded result as JSON. The CLI test attributes a two-commit file, requests a two-line range, and exits non-zero for a missing path.
- In the shared reader, **Show attribution** appears in Full file on either committed side; `B` toggles it. The After side blames the selected commit and the Before side blames the parent or against revision. The column names the author and short commit for each source line, and the selected line names author, email, relative time, short commit, and subject. Working-tree content disables attribution with an explicit reason. The worker is generation-keyed and cancellable, so a new file, side, or revision supersedes older attribution.
- **Line history** opens the existing version timeline anchored to the selected line or range. Opening from the Before side starts at that older content version, so a deleted line keeps its Before anchor and its Newer edge reports the deletion. Each Older/Newer step maps through the edge on a cancellable worker: context lines map, an introduced line or range stops with **Line introduced in this version**, a deleted line or range stops with **Line absent in this version**, and the range is never silently shrunk. An ambiguous file edge uses the existing predecessor chooser before mapping; a range is never split across candidates or given an invented boundary.
- **Open commit** in the attribution strip hands the blamed commit to the ordinary review and keeps the caller return location, so Back still returns to History or the recovery source rather than to blame.
- Native X11: [wide attribution with three authors](screenshots/m7-blame-wide.png), [narrow attribution with the selected line named](screenshots/m7-blame-narrow.png), [line history mapped through two older versions](screenshots/m7-line-history-wide.png), [narrow line history](screenshots/m7-line-history-narrow.png), [an introduced-line boundary that disables Older version](screenshots/m7-line-history-boundary.png), and [Open commit opening the blamed base commit](screenshots/m7-open-commit.png).
- Tests separately cover Before-side attribution availability and the initial Before version index for line history. Native captures show the After-side journeys; the Before-side path is covered by those tests rather than a separate native capture.
- Limitations: attribution does not infer uncommitted content. Ambiguous merge edges use the file-level predecessor chooser; there is no per-hunk split of one range across different candidates. Only Linux/X11, the dark theme, and llvmpipe were inspected natively.

## M7 file history · 2026-10-25

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 358 workspace tests pass (3 ignored); formatting and Clippy with `-D warnings` pass.
- History is a cursor over content versions. Git oracle fixtures confirm that commits carrying an unchanged state are skipped, an unchanged commit resolves to the commit that introduced the content, a rename names the old path, delete/re-add produces an explicit absent version, a root addition is the only version, and non-UTF8 path bytes survive. Cancellation and a failed step keep the loaded versions and the pending boundary so a retry resumes.
- A one-sided merge follows the parent that carries the content silently. A two-sided merge with no same-state carrier offers both predecessors newest first and follows the chosen one; the engine exposes the choice so the view can stop and ask instead of taking the first parent. The chosen edge then shows the chosen base with Old/New labels.
- Boundaries are explicit and never wrap: the newest version disables After with `Newest version`, the oldest disables Before with `Oldest version` or `Only version`, and an in-flight page disables Before with `Loading earlier versions…` until the result arrives. A limit or a pending choice names its own reason.
- The file-history worker preloads a bounded page and pages older versions on demand, is generation-scoped, cancels superseded work, and reports busy and error updates. Opening another file discards the previous session.
- The view renders the version header (kind, commit, subject, author, date, old path), the version counter, boundaries, and the predecessor chooser. The embedded reading surface keeps Diff/Full and the line place, uses Old/New labels, and falls back to Full-only with Diff disabled while the earlier side is unknown.
- `H` opens history from reading and `Esc` backs out one level: history to the file, then the file list, while the comparison and reading state stay intact behind the history sub-view.
- Native X11 at 1120×740: [newest version with Diff](screenshots/m7-history-wide-diff.png), [Full file with Old/New](screenshots/m7-history-wide-full.png), [oldest boundary](screenshots/m7-history-oldest.png), [predecessor chooser](screenshots/m7-history-chooser.png), and [the chosen predecessor edge](screenshots/m7-history-chosen.png).
- Native X11 at 720×480: [narrow history](screenshots/m7-history-narrow.png).
- History navigation reads objects and never writes objects, refs, the index, or working files.

## Nested paths and operation ownership · 2026-10-28

Linux x86_64 · Rust 1.99.1 · Xvfb/llvmpipe · disposable Git fixtures · `origin/main` at `7c26d68`.

- Six regression tests covering seven failure scenarios failed before the fixes and pass afterward: nested current-worktree removal, nested conflict staging, nested blame, abort without an operation, continue with a failing message hook, pending Local Changes navigation, and worktree switching during an active operation.
- `cargo fmt --all --check` passed.
- `cargo test --workspace --locked` passed: 499 tests, 0 failures, 4 ignored opt-in performance checks.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- Nested `--repo` fixtures preserve unrelated nested files, identify the containing worktree as current, and keep blame on the requested repository-relative path. Recovery failures return Git's error instead of a success receipt.
- Native X11 at 1120×740 with a five-second `pre-commit` hook showed **Committing…** and disabled History. Clicking History left Local Changes open with the pending worker until completion.
