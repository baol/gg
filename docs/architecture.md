# Architecture

Purpose: define system boundaries and behavior contracts.

## Layers

| Crate | Responsibility |
| --- | --- |
| `gg-engine` | Typed commands, incremental readers, filtering |
| `gg-git` | Repository discovery and object access through gitoxide |
| `git-good` | CLI, egui/eframe on wgpu, view state, workers |

Dependencies flow from shell to adapter to engine. Keep the engine independent of Git libraries, UI, and platform services. CLI and UI share commands. Add abstractions for concrete responsibilities.

## Runtime

Use safe Rust and propagate errors with operation, path, and retry. Preserve objects, refs, index, and working files while browsing. Keep native paths in APIs and exact object identities; use Unicode for editable paths and replacement characters for display text.

Run Git reads/indexing on workers, publish immutable batches, and repaint for visible changes. Retain displayed results through loading and errors, labeled with their repository identity. Track requested and displayed repositories separately.

## Launch

Resolve the repository from cwd, including nested directories, or an explicit path. Interactive launch loads history immediately; the toolbar supports repository changes. CLI history streams JSON; failures use stderr and nonzero status. Represent bare repositories, worktrees, unborn branches, detached HEAD, and shallow boundaries explicitly.

## History sessions

Pin each session to repository identity and resolved HEAD. Carry generations through requests/results; apply current results. Refresh creates a replacement session, adopted when ready and interaction permits. Repository switches clear the old filter/selection; refresh preserves matching selection by ID, otherwise the first visible result.

Traverse the deduplicated union of commits reachable from HEAD and every local branch, remote branch, and tag. A single tip uses the incremental frontier: emit the newest committer timestamp, break ties by descending object ID, then enqueue parents. Divergent non-shallow tips use topological order so parents follow all children. Assign stable ordinals and retain continuation state; shallow traversal stops at recorded boundaries. Every reachable commit remains accessible.

Prioritize visible rows, overscan, then indexing/counting. Publish summaries, selected details, progress, completion, and errors. Summaries carry ID, subject, author, and date; selected details include full message and parents.

### Storage and budgets

| Resource | Contract |
| --- | --- |
| Traversal slice | At most 32 commits or 4 ms; foreground work between slices |
| Requests/responses | One coalesced slot each |
| Response page | Up to 256 summaries; cache the current worker page |
| UI cache | At most 768 rows and one selected full message |
| Session files | Commit records, fixed-size ordinal offsets, separate filter offsets; close with session |
| Graph state | Frontier/visited IDs scale with graph size; release visited IDs on completion |

Reuse records across viewport and filter requests. Retain rows and mark counts incomplete after storage/traversal errors.

### Count and interaction

Publish discovered count, optional approximate estimate, and completion. Use valid count information as a hint; otherwise show the growing discovered extent. Counting shares the ordered history index and ends with an exact total.

Anchor by commit ID and pixel offset. Preserve that anchor as the range grows. Freeze scrollbar geometry during scrolling, momentum, dragging, and scrollbar hover; preserve the thumb's grab offset. Apply pending extents after settling. Deep jumps show fixed-height loading slots and traversal progress.

Pin hovered content and hit targets until pointer leave. Bind press/release to the same commit. Fill off-pointer slots in place; make a hovered loading slot clickable after leave/re-entry. Preserve row identity, position, focus, and selection.

Queue reordering, session replacement, and filter-result replacement during hover or interaction. Release queued changes after the relevant interaction ends; a configurable quiet interval handles scroll settling, while pointer leave releases hovered targets. Scrolling may reveal/fill slots with stable geometry.

### Filtering

Each edit increments a revision, including repeated text. Preserve edits during opening/refresh and apply matching revisions. Search full messages progressively in traversal order; distinguish partial, complete, and empty results.

Preserve matching selection; otherwise select the first published match. Clear selection/details at zero matches. Apply result and selection changes together at an interaction-safe boundary. External changes produce a refresh indication; explicit refresh adopts them. Implementation gaps: [regression inventory](regression.md#remaining-coverage).

## Browser state and keyboard

| Action | Binding |
| --- | --- |
| Filter / refresh / copy full ID | Ctrl+F/R/C; Command on macOS |
| Previous / next commit | Up/Down, k/j, Ctrl+P/N |
| Previous / next page | Page Up/Down; Alt+V for previous |
| Open selected commit/file / return | Enter or Right / Escape, Left, or Back |
| Toggle commit details | Space |
| Selected commit actions | Alt+A or **Commit actions** beside its identity |

Emacs bindings use physical Control. Ctrl+V retains native paste. Escape leaves filter focus. Text editors and open menus retain editing/navigation shortcuts; focused buttons allow list navigation. Reading views use Up/Down, j/k, Ctrl+P/N, and page keys to scroll their content. Enter opens a full-width diff from either file-list layout. History bindings apply while history is visible and editors are unfocused. Empty results keep selection empty.

The **Commit actions** menu beside the selected commit identity uses the same grouped command definition as the commit-row context menu and the Alt+A route: Inspect, Integrate, Rewrite and destructive, and Recovery. Labels include the selected short commit, for example **Cherry-pick a1b2c3d** or **Compare a1b2c3d with current HEAD**. Mutation commands are disabled with an explanation while a mutation, operation worker, branch switch, or network operation is active. **Commit actions** appears in the wide details pane and the narrow details overlay; Alt+A opens the corresponding details surface first when needed and then the same popup. The **Help** menu opens a concise keyboard-shortcut window. The visible control, the context menu, and the shortcut invoke the same command for the same target, readiness reason, and confirmation.

Opening a commit or comparison from History or Details records a return location with the exact selected commit ID, filter query, and pixel offset, plus whether the narrow Details overlay was the caller. Closing Review restores that location once. Review presentation changes do not add return locations or Back steps. Recovery records its source and selected commit; closing Review returns to that Recovery entry, not to History.

One operation area sits directly beneath the navigation strip and is hidden when idle. It owns the running phase, progress, cancellation, and supported continuation controls across every view: operation-worker work, rewrite and history refresh, network progress with **Cancel**, branch work, and an in-progress merge, rebase, cherry-pick, or revert with **Resolve conflicts…**, **Continue**, **Skip**, and **Abort** as Git supports them. Merge has no **Skip**; stash-conflict recovery has no sequencer controls. Continue stays disabled while unmerged paths remain. Configuration closes on accepted submission, so a running rewrite does not block browsing; its area reports **Rewriting commits** and then **Refreshing history** until the replacement generation is accepted. Duplicate submission is refused and incompatible mutations stay disabled. The area is the only place for these controls; dialogs, the toolbar, and the status line do not repeat them.

Operation outcomes carry a severity, a concise summary, optional full details, and at most one recovery action. The status line keeps a fixed-height summary and exposes **Details…** when full text exists and a recovery button when one applies, such as **Resolve conflicts…**, **Recovery…**, **Retry push**, **Retry fetch**, or **Restore stash…**. A warning that retains intent also offers **Dismiss**, which clears the intent without executing it. While the operation area is active, the status line yields to it and does not duplicate progress or recovery controls; after completion it reports the outcome again. Merge, rebase, recovery, rewrite, push, fetch, and stash results populate the same outcome shape; errors outrank success notices, and a remaining conflict or failed worker keeps its own visible surface. The details window keeps selectable, copyable full text.

Follow [bounded-reveal rules](design.md). Page by the rendered viewport with one row of overlap, clamp at history boundaries, and stream uncached destinations. Show cached selected metadata immediately; reserve the full-message slot while loading. Narrow details preserve hidden history selection and pixel offset.

Rationale: one bounded reveal preserves spatial context; overlapping pages retain reading context; fixed details slots keep loading visually stable.

## Presentation

Apply [design guidelines](design.md). Use a repository/action header, one stable Local Changes / History navigation strip beneath it, one operation area when work is active, a history/filter row, and fixed-height status line. Reserve counts/progress/errors. Show full paths on hover and edit paths explicitly. The navigation strip keeps **Local Changes** before **History** at every width; ref browsing remains reachable through the Refs toolbar menu and the wide refs sidebar.

History columns: dot, subject, author image/name, short hash, date. Preserve minimum table width with horizontal scrolling. Use high-contrast selection. Group details portrait, author/date, subject, and copyable IDs; retain full IDs behind abbreviations.

Below 1040 logical pixels, details replace history reversibly; wider windows use a resizable side pane preserving table width.

### Avatars

Use a bounded Gravatar worker keyed by normalized email SHA-256. Cache one validated 128-pixel image per author; display at 18 pixels in history and 64 in details, with reserved initials fallbacks. Retain one decoded source and at most two display-sized textures per cached author; filter thumbnails for the current display scale.

Persist images and 404 results in the OS cache. Read disk first; deduplicate author requests across texture eviction. Attempt transient failures once per session; retry next launch. Replace corrupt entries atomically. Bound bytes, decoded dimensions, timeouts, queues, and GPU cache. Publish/evict images after history/details hover and scrolling end. Cache location: [development](development.md).

## Commit review · M1 specification

### Comparison

Pin repository, commit, and parent IDs. Resolve revisions once. Default to first parent; roots compare with the empty tree. List merge parents in stored order with one-based numbers, short IDs, and full-ID tooltips. Parent changes reuse the pinned commit ID. Missing parent objects produce an identified incomplete-comparison state; retain metadata during loading/errors.

Keep exact Git path bytes and a one-line display string. Preserve Unicode; escape backslashes, controls, and invalid bytes (`\xNN`) so distinct paths have distinct labels. Copy Unicode paths verbatim; label the fallback **Copy escaped path**. CLI `--file` matches native path bytes. Changes carry old/new object IDs and modes, with added/deleted/modified/type-changed status. CLI and UI share deterministic Git tree traversal order. Represent moves as deletion/addition in M1, retaining both sides for future rename detection.

Parent switches retain the displayed comparison through loading/failure, label its identity, and offer retry. Adopt replacement files and diff together after pointer leave and scroll settling. Start the new comparison at its first file; empty comparisons clear selection. Independent worker mailboxes isolate superseded requests.

### Loading

Stream file summaries before blob reads. Virtualize rows using history's paging and interaction rules; counts progress to exact completion. Within a comparison, preserve selection by exact path, otherwise first result; clear it for empty comparisons.

Keep requested and displayed file identities separate. Retain the displayed diff and its scroll offset while a replacement loads; publish its identity, metadata, preview, and rows together after hover and scrolling settle. Errors retain labeled previous content. Reset the new file to its first row on adoption.

Read selected-file content on a worker independent of history indexing. Key results by review generation, comparison, and file. Coalesce requests, cancel obsolete work between batches, and apply current results. Page diff rows and keep every file reachable through bounded caches.

### Changed-file implementation

Walk trees depth-first in Git tree order, skip identical subtrees, and defer blob contents to selected-file requests. The frontier scales with active tree entries. Yield between tree visits; individual tree decoding remains one operation.

Use an independent comparison worker with 32-step/4-ms slices, one request/result slot, temporary record/offset files, 256-row pages, and at most 768 UI rows. Retry replaces the worker and its mailbox for the same pinned comparison, retaining selected path, displayed diff, pixel anchor, and known count while enumeration catches up. Immutable comparison IDs preserve path order. Closing review cancels it; obsolete mailboxes remain isolated.

Present status/path rows and selected before/after IDs and modes. A persistent Before/After header names the resolved base and target commits; when the comparison is against a ref such as `HEAD`, the ref name is shown with the resolved commit. The commit-row action is **Compare with current HEAD**. The commit identity strip names the selected commit subject only; the shared reader names the selected path and exact endpoints once, so the path and endpoint labels are not repeated above it. Wide layouts use a resizable diff side pane; narrow layouts open a separate reading view with Back to files. History state and incoming history results wait while review is open; Back restores its caller location and pixel offset before applying current results.

### Diff and layout

Use one selected-file worker with one coalesced request/result slot. Retain one bounded document on the worker; publish at most 256 rows per page and cache at most 768 UI rows. Check blob headers before allocating content. Compare up to 2 MiB per side and 100,000 combined source lines with a 250-ms algorithm deadline. Return an explicit incomplete summary when a budget is exceeded. Cancel before each object header/content read and during row projection; object decoding and the deadline-bound algorithm finish their current operation. Reuse the document for viewport requests.

Use unified diffs with old/new line numbers, fixed-height rows, three context lines, and missing-final-newline markers. Label byte sizes Before and After. Preserve whitespace/line endings. Scroll long source lines horizontally in an explicit reading view. Distinguish context/additions/deletions with symbols and color.

| Content | Presentation |
| --- | --- |
| Binary blob | Before/after PNG/JPEG image previews; hexadecimal fallback; IDs and byte sizes |
| Symlink | Labeled target text, read directly from the blob |
| Gitlink | Referenced commit IDs, including unavailable submodule objects |
| Mode-only change | Metadata summary with before/after modes and unchanged content; read object headers only |
| Undecodable bytes | Escaped display; exact comparison bytes |
| Size/computation limit | Explicit summary, completion state, exact-side export |

Review opens a resizable file-list/diff workspace with commit identity and parent selection. Narrow windows show files and content successively. Back restores the recorded caller location: history selection, filter, and pixel anchor for History or Details callers. Shortcuts belong to the active context; editors retain theirs.

### Binary previews and export

Decode PNG/JPEG on the selected-file worker. Bound each side to 16 MiB encoded, 8 megapixels, 4096 pixels per dimension, and 64 MiB decoder allocation. Fit previews to 1024 pixels; show original dimensions. Unknown, corrupt, and oversized images retain a labeled hexadecimal preview of the first 256 bytes when inside the encoded budget. Larger blobs retain summaries and export. Publish current-generation previews at the diff's interaction-safe boundary; retain one preview pair and two textures.

Export either present blob side to a new user-chosen path, preserving exact bytes. Write a temporary file beside the destination, then publish without overwriting. Keep Git metadata outside export destinations. Export symlink target bytes; gitlinks retain referenced commit IDs. Cancel before object decoding and between 64-KiB writes; remove temporary output on cancellation/failure. Run one export per review view, pin its path/object identity, and cancel on closing the export or review. Escape closes the export window before review navigation resumes. Gitoxide decoding uses memory proportional to the selected object and delta chain; output writes use fixed-size chunks.

### CLI

`git good show <commit> [--repo PATH] [--parent N]` streams JSON summaries. Add `--file PATH` for a repository-relative selected-file diff, including exact source-line bytes. Add `--export before|after --output PATH` to save that side and emit a JSON receipt. Use cwd discovery by default. Validate one-based parent numbers; roots use the empty tree automatically. Include repository/comparison identity and exact path bytes. Apply runtime error conventions; document usage when implemented.

Rationale: summaries provide early feedback; pinned, cancellable work keeps rapid selection accurate; a dedicated review view gives source readable width.

## History graph and refs · M3

Ref discovery is read-only and shared by CLI and native views. Emit HEAD state (symbolic name, detached flag, commit), local branches, remote branches, and tags with exact short and full names and peeled commit IDs. Skip symbolic remote `HEAD` refs and tags that cannot peel to commits. Keep HEAD explicit for unborn and detached repositories. `git good refs [PATH]` streams this snapshot as JSON; the history worker loads the same snapshot alongside a traversal and replaces it on refresh. History traversal starts from HEAD and every ref target, so branch, remote, and tag-only commits appear in the graph.

Compute graph rows in traversal order, including commits later hidden by a filter, so lane identities stay coherent. Each row carries its own lane, the set of lanes entering and leaving the row, and parent lanes. Reuse a parent's existing lane, allocate free lanes for additional merge parents, retire empty lanes, and cap layout at 64 lanes. Render stable per-lane colors, vertical continuations, parent joins, commit dots, and a ring for merge commits. A single-parent branch keeps its own lane color until it joins an existing lane; merge parents always use the parent lane color. Only the row's own lane starts a downward segment at the commit; newly created parent lanes are connected by their joins. Cache graph rows with the visible page and evict them with their commits.

Render matching refs as pills before the subject, with the current branch first, then tags, other local branches, and remotes. Show up to four pills when the subject column has room (two when it is narrow) plus an explicit overflow count. Include the ref-kind icon in each pill and keep long labels single-line. Clicking any pill selects its target commit for inspection without moving HEAD. Ref selection never switches branches or creates tracking branches.

At every width, keep **Local Changes** and **History** in one compact navigation strip directly beneath the toolbar, in that order and at the same position. The wide layout additionally shows a fixed 190 px left refs sidebar with the refs filter, **New branch** action, collapsible **Branches**, **Remotes**, and **Tags** groups, slash-separated folder nodes, HEAD checkmark, and branch/remote/tag icons. Below 1120 px the Refs toolbar menu provides the same ref browsing. The same sidebar and toolbar stay visible while Local Changes is open; the main content area switches between the history table and the Local Changes workspace. Clicking any sidebar row, toolbar ref item, or graph pill inspects its target commit. The sidebar context menu and the toolbar Refs menu expose an explicit **Switch to <branch>** command for local branches; the current branch is disabled. Selecting a ref clears a hiding filter, targets its commit by ID, and scans the completed spool to recover the row before revealing it. Ref navigation never moves HEAD, checks out content, writes refs, or modifies the index or working tree.

### Branch switching and creation

Switch and create operate on local branches only after a clean-worktree check. Open the observed status identity, scan for staged, unstaged, or conflicted paths, and reject the action before invoking Git if any tracked change is present. Untracked files do not block switching or creation; Git still refuses a checkout that would overwrite them, and that failure is reported without deleting user files. Validate branch names as full local refs, reject collisions on creation and missing branches on switch, and run `git switch` or `git switch -c` from the worktree root so Git performs the atomic index/worktree/ref update and runs `post-checkout` hooks. Remove ambient `GIT_DIR` and `GIT_WORK_TREE` overrides. A missing Git executable, name error, dirty tree, collision, or checkout failure returns an actionable message and leaves the user on the current branch unless Git partially advanced the worktree; the message directs the user to inspect the current branch and hook output. If Git advanced to the target branch but returned nonzero, treat that as a successful switch with a `post-checkout` warning rather than a failed action.

The refs sidebar and the Refs menu treat clicking a local branch, tag, or remote as read-only inspection of its target commit. Both expose **Switch to <branch>** as an explicit action, with the current branch disabled; switching performs the clean-worktree check, validation, checkout, and post-checkout warning behavior above. A remote row keeps the explicit **Create tracking branch…** route. One shared branch-creation form serves **New branch** and every **Create branch here** entry: it names the starting commit and selector, validates the full local ref name, and exposes **Switch after creation**. New branch defaults the choice on; recovery creation defaults it off. The Create action always creates the branch first and switches only after success, so a blocked switch leaves the created branch in place. A blocked switch retains the target as **intent** and reports a warning outcome with **Retry switch**; the intent is revalidated only when the user invokes that action and never executes automatically. CLI `git good switch [--create] BRANCH [--repo PATH]` returns a JSON receipt. On success, clear the history filter, select the new HEAD, reload history/refs, and refresh an open Local Changes view. Branch deletion, rename, remote tracking, and tag creation remain later work.

## Working status · M2

`git good status [PATH]` discovers the whole worktree from cwd or an explicit path and streams JSON. Bare repositories return an identified working-tree requirement. Unborn repositories compare the index with an empty tree.

Emit independent staged, unstaged, untracked, and conflict entries with exact path bytes and escaped one-line labels. A path can appear in both staged and unstaged categories. Honor ignore rules; enumerate individual untracked files. Represent renames as deletion/addition. Identify intent-to-add explicitly. Preserve repository contents, including index stat data.

Run scans on cancellable workers. Publish rows in discovery order and keep their ordinals stable for the scan. Stream through a 128-entry queue into disk-backed records; request visible pages of at most 256 rows and retain at most 768 rows in the UI. Index/tree metadata follows repository size; file comparison buffers follow gitoxide's status implementation.

Open Local Changes from the stable navigation strip, including on unborn branches. The toolbar, navigation strip, and refs remain visible; the Local Changes workspace occupies the main content area. The workspace keeps two independent lists in one left column: **Unstaged** above, containing unstaged, untracked, and conflict entries with the last two marked by an explicit badge, and **Staged** below. Both lists stay visible at every width and keep their own selection, scroll offset, viewport, and bounded row cache. `Tab` moves keyboard focus between the two groups and selects the remembered or first row in the newly focused group; the commit editor keeps normal text-editing keys. Fixed-height rows show the change kind and exact path. A path with both index and working changes appears once in each list. Selecting an entry in either list immediately selects its comparison in the shared file reader; there is no separate **View diff** step, second Staged/Unstaged reader switch, full-screen **Review & stage** view, or staging-specific content renderer. The reader header names the path, the exact comparison endpoints, and Diff/Full file; whole-file staging commands and export/copy live in its **File actions** menu, while modes, object IDs, and sizes live in its optional **File details** disclosure instead of permanent rows. Wide layouts place the single reader to the right of both lists and the commit/amend form directly below the reader. Narrow layouts place bounded, collapsible groups above the reader and keep the form below it. The group header shows its count, and the status line shows both counts. Count discovered entries until completion; show Clean only after a successful empty scan. Refresh retains the previous scan until replacement completes, then adopts both list pages and the reader after hover and scrolling settle. Fresh scans begin at the first row of each list. **History** restores history's selection, filter, and offset. Failed scans retain identified results and offer retry. Status scans remain observational; file actions use the validated mutation contract below. Remove the former top **Commit** menu; cherry-pick, revert, reset, compare, and copy stay in the commit-row context menu.

### Working diffs

Selecting a Staged entry compares HEAD to index; selecting an Unstaged, Untracked, or Conflict entry compares index to the working file (empty to working file for untracked paths, an explicit unmerged summary for conflicts). In amend mode a Staged entry compares HEAD's parent tree to the index. The shared reader names both endpoints, renders only the active comparison, and offers Diff and Full file for the same entry; binary, mode-only, symlink, gitlink, budget, and conflict summaries stay explicit instead of becoming an empty reader; the workspace keeps the honest counterpart request available for the same path but never shows two comparison panes or a duplicate reader switch. Full file reads the same two sides with the same 2 MiB and 100,000-line budgets, including the normalized working snapshot.

Pin HEAD and the index fingerprint from the status scan. The status worker pages both list viewports independently: Staged rows use their own coordinates, and working rows are offset by the scan's staged boundary, so neither request walks the whole worktree. The scan emits every Staged row before working rows; `Update.staged` is that boundary, and the split is asserted. Capture working bytes on selection, apply Git text normalization, and validate repository inputs and working bytes again before publication. Changed inputs return an identified stale-result error with Refresh status. Request and cancel both comparisons on one cancellable worker and publish each pane as soon as it completes; adoption must not wait for pointer hover or scrolling to settle.

Reuse the bounded diff engine and fixed source rows. Give each list its own scroll area and preserve list offsets across refreshes; the active pane also keeps its offset. Preserve symlink targets and mode changes. Show explicit binary, gitlink, conflict, and budget summaries. Conflict resolution belongs to M5. Exact immutable-object export remains in commit review.

After a mutation, refresh both lists and the active comparison coherently without rebuilding the workspace. If the active entry disappears, retain its path and area with an explicitly empty comparison until another entry is selected; never leave stale rows looking like current results. Preserve unrelated list selection, independent scroll offsets, exact paths, and bounded virtualization.

### File staging

Stage file adds one observed working path to the index. Capture the bytes on disk at action time, like `git add`, rather than publishing a previously reviewed snapshot. Support unstaged modifications, untracked additions, deletions, type changes, intent-to-add, executables, and symlinks. Preserve working files, unrelated index entries, symlink targets, and executable modes. CLI `git good stage --file PATH [--repo PATH]` uses repository-relative paths and returns a JSON receipt after success.

Require an unstaged or untracked observation whose working path still matches its scanned kind: deletions stay absent and every other kind stays present. Normalize regular-file content through Git's attributes and conversion pipeline, then write the blob and its real stat data. Without symlink or executable-bit support, retain the indexed mode for emulated links and files. Reject already-staged paths, conflicts, sparse indexes, submodules, and file/directory collisions.

Acquire the index lock before checking the observed HEAD and index fingerprint. Re-capture the working path before publication and invalidate the tree cache; revalidate HEAD and publish the index atomically. Stale observations, absent or reappearing paths, lock ownership, and working changes during the action return actionable errors. Run UI mutations on a worker; disable duplicate actions and navigation while pending, then refresh status at an interaction-safe boundary.

### Hunk, line, and drop actions

Stage or unstage selected changes from the Staged or Unstaged comparison, or discard selected working changes from the Unstaged comparison. A hunk header carries inline **Stage hunk** or **Unstage hunk** actions and, for unstaged text edits, **Discard hunk**. Clicking the header selects or clears every changed line in that hunk; clicking an added or deleted row toggles that line. The hunk action applies the selected lines when any are selected, otherwise the whole hunk. The shared reader's **File actions** menu exposes whole-file **Stage file**, **Unstage file**, and **Discard file** for the active comparison; presentation, export, copy path, and whole-file commands appear once in that header. Identify each hunk by its exact `@@` header and each line by its old- or new-side line number; reject selections that no longer match. CLI `git good stage --file PATH --hunk HEADER` accepts repeated `--hunk` selections and adds `"hunks"` to the stage receipt.

Partial staging requires an unstaged, regular text modification with a 100644 or 100755 index entry. Reject untracked additions, intent-to-add, deletions, type or mode changes, symlinks, submodules, conflicts, sparse indexes, binary comparisons, and comparisons beyond the text budget. Acquire the index lock, then capture and normalize the working bytes through the same pipeline as the displayed diff and recompute index to working. Unknown headers or lines mean the diff is stale and publish nothing.

Rebuild the staged index blob by line ranges: keep unchanged gaps and unselected old-side lines, and take selected new-side lines. When every change is selected, write real working stat data, matching `git add -p` after a full selection. Otherwise write zeroed stat data so status compares content and reports the remaining unstaged edit (`MM`). Preserve unrelated index entries, modes, and working bytes; re-capture working bytes and revalidate HEAD before atomic publication.

Partial unstaging supports staged regular text additions, modifications, and deletions. Rebuild from the HEAD side and the index side: restore selected old-side lines and retain unselected new-side lines. If the result equals HEAD, restore the HEAD index entry; if HEAD is absent and the result is empty, remove the index entry. Otherwise write the partial blob with zeroed stat data. Mode, type, and symlink changes require whole-file actions.

After a successful mutation, refresh both lists, update both comparison requests, and reload the active comparison in place. Retain list scroll offsets, clear line selections, and keep the reviewed path when it still exists; otherwise retain it with an explicit empty comparison. Refresh the status list at the same boundary rather than waiting for Back. Failures keep both lists, the active comparison, and the index unchanged and show the error above the reader.

Rationale: hunk- and line-level actions follow `git add -p` while zeroed partial stat keeps omitted edits visible instead of silently clean. Keeping both list entries and both comparison requests lets a user move a change between staged and unstaged without losing the path context.

### Dropping working changes

Discard removes selected working changes without touching the index. For an unstaged regular text modification, rebuild the working file from the index and working sides: restore selected old-side lines and retain unselected new-side lines. Expand any selected row to its contiguous change block so a replacement cannot be half-discarded. Convert the normalized result back through Git filters before writing, validate the working snapshot again, and replace the file atomically. Partial discards reject mode, type, and symlink changes; use **Discard file** for those.

**Discard file** restores tracked modifications, deletions, mode changes, type changes, and symlinks from the index, and removes untracked files. Acquire the index lock for consistency, reject conflicts, intent-to-add, submodules, sparse indexes, and file/directory collisions, and leave every index byte unchanged. Require an explicit confirmation before any discard, state the exact file and scope, and state that the index is unchanged; keep the workspace visible, reload both lists and the active comparison in place after success, and retain a disappeared path with an explicit empty comparison. Failures keep the reviewed content, both lists, and the index unchanged.

Rationale: dropping is intentionally working-tree-only and reversible with the index still intact. Confirmation and change-block expansion prevent accidental loss while keeping the Local Changes workspace usable.

### Committing

Commit writes the current index, not working-tree content. Pin HEAD and the index when the action starts, reject commits with no staged changes, unmerged entries, intent-to-add entries, sparse indexes, and an unavailable worktree, and resolve author and committer identities from Git configuration or the documented environment variables. Normalize the message, write `COMMIT_EDITMSG`, and run hooks through `git hook run` so `core.hooksPath`, linked worktrees, and native Git execution rules are honored.

Run `pre-commit`, `prepare-commit-msg` with the message file and `message` source, and `commit-msg` before taking the index lock. Hooks may stage content or edit the message, so re-read the index and message afterwards and reject a HEAD change made during the hooks. Acquire the index lock only after hooks finish, rebuild the tree from the current index, verify it differs from HEAD, and create the commit object with the observed parent. Update `HEAD` atomically with the reference lock and write the reflog. Run `post-commit` after publication; its failure is a warning because the commit already exists.

Every other hook failure aborts before the commit is published. Preserve the index, working tree, and message file, and return the hook name, exit status, and captured output with a retry path. The native view keeps the draft message, refreshes status to expose any hook staging, and stays on Local Changes; **History** refreshes history and selects the new commit. CLI `git good commit -m MESSAGE` returns a JSON receipt with the commit, tree, parents, and any post-commit warnings. Amend is specified below; signing remains M7.

Commit text and the selected path/area are session state owned by the browser, keyed by canonical worktree identity, not by the visible Local Changes view. Leaving for history, review, or a refresh captures the active draft and selection; returning to the same worktree restores the exact text and reselects the path and area when they still exist. Normal and amend drafts are separate: enabling amend switches to the amend draft or HEAD's message, and disabling it restores the normal draft without overwriting either. A successful commit clears only the submitted draft. Hook failure, conflict, and ordinary navigation retain it, and switching worktrees never transfers a draft.

Commit readiness comes from the validated status identity, not a partial row cache. A normal commit enables only after the scan completes with a nonempty message and at least one staged entry; amend enables for a nonempty message even with no staged entries, preserving message-only amend. Disabled states name the reason: a running operation, a pending status scan, an empty message, or no staged files. The commit editor expands and collapses without permanently displacing the file list.

### Whole-file unstaging

Unstage file restores exactly one staged path from the observed HEAD, or removes its index entry on an unborn branch. Preserve working files and unrelated index entries. CLI `git good unstage --file PATH [--repo PATH]` uses repository-relative paths and returns a JSON receipt after success.

Acquire the index lock before checking the observed HEAD and index fingerprint. Restore the entry, invalidate the tree cache, revalidate, and publish the index atomically. Conflicts, sparse indexes, file/directory collisions, stale observations, and an existing lock return actionable errors. Run UI mutations on a worker; disable duplicate whole-file, hunk, and navigation actions while pending, then refresh status at an interaction-safe boundary. Whole-file unstaging remains the fallback for modes, types, symlinks, untracked additions, deletions, and conflict changes.

Rationale: restoring one entry gives a reversible operation while preserving the user's working edits and other staging decisions.

## Remote tracking and network mutations · M4

### Tracking snapshot

Read configured remotes and local-branch tracking alongside the ref snapshot. A remote carries its configured name and fetch URL, plus its push URL when distinct. Display URLs; never treat them as credentials. A local branch with an upstream carries the upstream's full ref name, remote name, remote branch name, and ahead/behind commit counts. Keep branches without a configured upstream explicit rather than inventing `origin` tracking. A configured upstream whose remote-tracking ref is absent remains visible with an unavailable count state.

Compute ahead and behind from the local branch tip and its remote-tracking tip. Count commits reachable from one side after excluding the merge base; when the histories are unrelated, count each full side. Shallow repositories report counts limited to available history. Compute counts for local branches with resolved upstreams and keep the snapshot read-only. Preserve the previous snapshot while a refresh is loading or fails.

`git good refs [PATH]` includes remotes and local-branch tracking in the same JSON snapshot as M3. The native refs sidebar shows the current branch's upstream and ahead/behind marker without hiding the ref tree. The status line names the upstream and counts when available. Long upstream labels truncate through the shared overflow view.

### Network operations

Clone, fetch, push, and remote-branch checkout are explicit mutations delegated to the Git executable for transport, credential helpers, SSH agents, hooks, configuration, and repository locking. This preserves Git-compatible authentication and protocol behavior while gix remains responsible for discovery and object reads. Locate Git through `PATH`; a missing executable returns installation guidance. Run with an explicit working directory, remove `GIT_DIR` and `GIT_WORK_TREE`, and never invoke a shell.

Fetch selects one configured remote, or the sole configured remote when omitted, and never updates local branches or the worktree. Push selects one local branch and one remote without force; a push that would overwrite remote work is rejected by Git and reported. The default remote is `origin` when configured, otherwise the sole configured remote, otherwise an error. `--set-upstream` records the pushed branch's upstream only after Git reports success. Clone creates a new destination; an existing nonempty destination is rejected by Git and its error is preserved without deleting user files.

After a successful fetch, reload refs and history while retaining the current selection where possible. After a successful push with upstream changes, reload tracking and refs. After clone, open the new repository only when Git succeeded. Failed or cancelled operations retain the previous displayed state and offer a retry path. Network mutations never run automatically during refresh, launch, or ref navigation.

### Credentials

Use Git's configured credential helpers, SSH agent, and protocol configuration. The native app runs network operations without an interactive terminal and reports a missing-credential failure with the remote name and a pointer to Git credential configuration. CLI operations may use the invoking terminal's normal Git credential prompt. Redact URL user information and never copy credentials, helper output, or raw environment values into receipts, logs, progress, or errors.

### Progress and cancellation

Parse Git's stderr progress as a bounded sequence of phases and percentages. Coalesce updates so the UI repaints at a steady rate, truncate long lines, and never buffer an unbounded transcript. CLI commands emit one final JSON receipt on stdout; progress belongs on stderr and may be suppressed when stderr is not a terminal. A native cancel request sends a kill to the Git child, reaps it on the worker, reports cancellation, and leaves the previous view usable. Network operations are serialized: duplicate Fetch, Push, Clone, or remote-branch checkout actions stay disabled until the worker finishes.

Remote-branch checkout creates a local branch that tracks the selected remote branch. It uses the same clean-worktree, validation, `--track`, hook, and post-checkout failure rules as local branch switching; it never detaches HEAD silently.

### CLI

`git good fetch [REMOTE] [--prune] [--repo PATH]` updates remote-tracking refs and prints a JSON receipt with the repository, remote, and fetched state. `git good push [REMOTE] [BRANCH] [--set-upstream] [--repo PATH]` prints the pushed branch, remote, upstream, and any warning. `git good clone URL [PATH]` prints the cloned path and resolved repository state. Commands stream progress on stderr and keep stdout parseable. Failures use stderr and a nonzero status without a partial receipt.

Rationale: Git owns protocol and credential compatibility, while gg owns explicit selection, bounded progress, cancellation, and a single refresh boundary.

## History mutations and recovery · M5

### Stash

List stash entries from the `refs/stash` reflog through the shared engine: position, full `stash@{n}` reference, recorded commit, and reflog subject. Newest entries come first. An empty stash list is explicit rather than an error. Stash listing is read-only and never changes refs, the index, or working files.

Delegating stash mutations to the Git executable keeps conflict detection, index restoration, and reflog semantics identical to Git. Remove ambient `GIT_DIR` and `GIT_WORK_TREE`, run with an explicit working directory, and never invoke a shell. A reference is accepted only as `stash@{n}` or a bare `n`; option-shaped or non-numeric references are rejected before Git runs.

Push records modified and staged changes, optionally including untracked files and keeping staged content in the index. A clean worktree is an actionable error, not an empty receipt. Apply and pop restore the selected entry; `--index` also restores staged state. When Git reports conflicts, the operation keeps the stash entry, lists the unmerged paths, and leaves the conflict markers in the worktree. Pop removes the entry only after a conflict-free apply. Drop removes the selected entry after an explicit confirmation and never touches working files.

`git good stash list [--repo PATH]` prints a JSON snapshot with the repository and entries. `git good stash push [-m MESSAGE] [--include-untracked] [--keep-index] [--repo PATH]`, `git good stash apply|pop [STASH] [--index] [--repo PATH]`, and `git good stash drop [STASH] [--repo PATH]` print JSON receipts with the reference, applied/created/dropped state, conflicts, and warnings. Failures use stderr and a nonzero status without a partial receipt.

The native **Stash changes…** dialog records a message and the untracked/keep-index options. The **Stashes** window lists entries with exact references and offers one **Restore…** form plus a separate confirmed **Drop**. The restore form exposes **Restore staged state** and **Remove stash after successful restore**; remove maps to pop only after a conflict-free apply, and a failed restore keeps the form, entry, and inputs. Successful stash mutations refresh the history, refs, and status boundary so the worktree state stays current. A conflict reports its paths in the status line and keeps the entry for recovery; stash conflicts never grow sequencer controls.

### Merge, cherry-pick, revert, and recovery

Merge, cherry-pick, and revert are explicit mutations delegated to Git so commit order, parent selection, hooks, and conflict behavior match the executable. Each operation validates a clean worktree, resolves one explicit commit or ref, and reports conflicts with unmerged paths instead of manufacturing a result. An in-progress merge, cherry-pick, or revert is detected from the repository state and exposed as Continue, Skip where supported, and Abort. Abort restores the pre-operation HEAD, index, and worktree as far as Git can; failures report detail for manual recovery without deleting user files. Conflict resolution shows base, ours, and theirs content for each unmerged path and marks a path resolved by staging it.

The conflict workflow names the operation, unresolved count, and operation commit above the content. It keeps Git's Ours/Theirs terms but explains what each side means for the operation, including rebase where Ours is the base being rebased onto and Theirs is the replayed commit. Base, Ours, Theirs, and the working file are preview sources: choosing one changes only the **Result** preview and never writes or stages anything. The working file is the default preview, so external edits are inspectable before publication. **Stage resolution** is the single publication step; it revalidates the conflict stage IDs and exact working bytes captured by the preview, then writes the chosen result and stages the path. A changed stage or working file returns an actionable stale-preview error in the conflict workspace and status line and requires **Refresh external edits**. If the worktree write succeeds but staging fails, the error states that the working file was already replaced; the review path, operation state, and index remain inspectable. Oversized regular working files are validated by exact streaming hash without a text preview; absent, symlink, directory, and submodule worktree paths keep Base/Ours/Theirs previews but disable a Working preview with an explicit reason. Stage resolution is disabled while another operation owns the area or the selected preview source has no content. Continue stays disabled while unmerged paths remain and explains why. Conflict listing, content reads, and resolution publication run on workers; after success the view selects the next unresolved path predictably. At wide widths the three side panes stay side by side with a bounded Result preview beneath them; at narrow widths Base/Ours/Theirs/Result are side tabs with one bounded scroll region, and the path list stays in its own bounded scroll.

Merge, rebase, and continue/skip/abort run on one background operation worker. The dialog closes immediately, the operation area names the running continuation, and duplicate or incompatible mutation actions stay disabled until the result arrives. Success refreshes mutation state, history, refs, status, reflog, and backups once; failure clears the pending label and reports the error while preserving Git's recovery state. UI mutations run on a worker, disable duplicate actions while pending, and refresh refs/history/status at an interaction-safe boundary.

## History rewrites and recovery · M6

### Amend

Amend rewrites the latest commit with the current index and an edited message. It requires an existing HEAD and a working tree, validates the repository status, pins the previous HEAD, and preserves the previous commit's author and parents. The committer identity and time come from the current configuration. An empty index is valid for a message-only amend; unlike a normal commit, the tree may equal the old tree.

Amend mode compares the index with the parent of HEAD rather than with HEAD. `StatusIdentity.base_tree` pins that parent tree (the empty tree for a root commit), and `Session::open_amend`, `working_diff`, and `unstage` use it as the before side. The staged rows therefore describe the proposed commit contents: existing changes in HEAD remain visible, newly staged working changes join them, and unstaging a file, hunk, or line restores the parent side in the index. Staging still compares the index with the working tree. Working files are never modified by staging or unstaging, and the usual index-lock, stale-observation, conflict, and whole-file mode checks apply.

Run `pre-commit`, `prepare-commit-msg` with the message file, `commit` source, and the old commit ID, then `commit-msg`. Re-read the message and index afterwards, reject a HEAD change made by a hook, acquire the index lock, rebuild the tree, and write the commit object with the original parents. Update `HEAD` atomically with the observed old ID as the expected value and write a `commit (amend): subject` reflog entry. Run `post-commit` and `post-rewrite amend` after publication; their failures are warnings because the rewrite already exists. `post-rewrite` receives the old and new IDs on stdin through `git hook run --to-stdin`, because the hook runner does not forward the parent process's stdin.

Before publication, create `refs/gg/backup/<label>/<unix-nanos>` pointing at the previous tip. A backup failure aborts the rewrite. Backup refs are private recovery anchors: the refs snapshot excludes `refs/gg/`, so they never appear in the sidebar or graph labels. The amend receipt carries `amended`, `previous`, and `backup` in addition to the commit, tree, parents, and warnings. CLI `git good commit --amend -m MESSAGE` prints the same receipt.

The Local Changes commit/amend form sits below the reader: at the lower right in wide layouts and below the reader at narrow widths, so it remains available while reading a comparison. It exposes an **Amend** toggle. Enabling it loads the current HEAD message without replacing a nonempty amend draft, keeps the normal draft separate, restarts status against HEAD's parent, and shows a banner above the form that names the affected commit and subject, names the amend base (its parent or a root/empty tree), and explains that staged changes are the commit contents. The action becomes **Amend commit**; the same reader provides **Stage file** and **Unstage file**, plus whole-file, hunk, and line stage/unstage actions on the active comparison. Unstage in amend mode removes the selected change from the amended commit and leaves working files unchanged. After success the toggle and amend base clear, the status line reads **Amended**, the remaining normal draft is retained, and the lists refresh against the new HEAD.

### Reflog

Read reflog entries through `git reflog show --date=unix --format=%H%x00%gD%x00%gs REF`, with `HEAD` as the default. Parse the timestamp from the dated selector, reconstruct stable `REF@{n}` index selectors from entry order, and split the subject at the first colon into an action and detail. Missing refs, option-shaped references, and Git failures return actionable errors; listing is read-only and never changes refs, the index, or working files. `git good reflog [REF] [--repo PATH]` prints the repository, reference, and entries as JSON.

The native **Repository → Recovery…** window has one source selector for **Backups** and **Reflog** and keeps their distinct provenance, identifiers, and readers. Reflog defaults to `HEAD` with a selector for `HEAD` and local branches. Selecting an entry shows its full ID, relative time, action, and detail, with **Open commit**, **Reset branch here** (Soft, Mixed, or Hard), **Create branch here…**, and **Copy hash**. **Open commit** runs the ordinary commit review and records the recovery source plus selected commit as its return location; Back returns there. Reset reuses the existing reset path and confirmation, including the Hard warning that staged and working changes are discarded. Create branch resolves the selected commit, validates the name, rejects an existing branch, and uses the shared form with **Switch after creation** off; the current branch and worktree stay put. Successful reset and branch creation refresh refs, history, status, and the open Recovery source.

### Rebase

`git good rebase REF [--autostash] [--repo PATH]` rebases the current branch onto one explicit local branch, remote-tracking ref, or tag. The reference is resolved to a commit before any mutation. A tracked working tree must be clean; `--autostash` is the explicit opt-in for working changes. The backend always passes `--autostash` or `--no-autostash`, so repository configuration cannot silently change the decision. Untracked files do not block the clean check; Git still refuses a rebase that would overwrite one. An operation already recorded by Git must be recovered before another rebase starts.

Before Git runs, pin the current HEAD and create `refs/gg/backup/rebase/<unix-nanos>` pointing at it. A backup failure aborts the rewrite. The rebase itself runs through Git with non-interactive editors so commit order, hook behavior, and conflict semantics match the executable. The receipt carries `reference`, resolved `commit`, `previous`, `backup`, `applied`, `in_progress`, `conflicts`, and `warnings`.

An in-progress rebase is detected through `rebase-merge` or `rebase-apply`, with the commit being applied read from `REBASE_HEAD`. The operation area names the rebase and exposes **Continue**, **Skip**, and **Abort**; the conflict window lists unmerged paths, and **Recovery…** remains in Repository and in the outcome recovery action. Continued and aborted rebases refresh history, refs, status, an open reflog, and open backups, and restore the pre-rewrite history row. `git good rebase --continue|--skip|--abort [--repo PATH]` prints the same receipt shape with null reference, commit, previous, and backup fields.

The native **Refs → Rebase branch…** action and each branch, remote, and tag context menu open a confirmed **Rebase branch** dialog with a reference field and an **Automatically stash working changes** option. The dialog states that the rewrite creates a private backup ref. Ref actions are disabled while a mutation is in progress.

### Reorder and squash

`git good rewrite --from COMMIT [--autostash] [--pick COMMIT]... [--squash COMMIT]... [--repo PATH]` rewrites the linear range from the named commit through HEAD. The start commit is resolved and checked as a HEAD ancestor. The range is listed oldest first and must be linear; merge commits are rejected rather than flattened. Every range commit must appear exactly once in the ordered steps, and the first step must be `pick`. A `squash` step combines that commit into the previous step, so the first commit cannot be squashed. The CLI receipt carries `from`, `previous`, `backup`, `applied`, `in_progress`, `conflicts`, `warnings`, and the step count.

The backend generates the interactive-rebase todo itself and supplies it through `GIT_SEQUENCE_EDITOR`; `GIT_EDITOR=true` accepts the combined squash message, and `--no-autosquash` and `--no-rebase-merges` keep repository configuration from changing the plan. A root range uses `--root`; a shallow boundary is rejected rather than rewritten as a root. A tracked working tree must be clean unless `--autostash` is explicit, and `--autostash` or `--no-autostash` is always passed. Before Git runs, create `refs/gg/backup/rewrite/<unix-nanos>` pointing at HEAD; a backup failure aborts the rewrite.

Reorder and squash use the same Git rebase state and operation area as rebase. Conflicts are read from the worktree and `REBASE_HEAD`, with **Continue**, **Skip**, and **Abort** backed by `git rebase --continue|--skip|--abort`. `git good rewrite --continue|--skip|--abort` exposes the same actions. Completion, skip, and abort refresh history, refs, status, an open reflog, and open backups, and restore the pre-rewrite history row. The commit-row context menu exposes **Reorder and squash to HEAD…**; the modal lists the range with Up/Down controls, **Squash** choices, an autostash option, and a warning that a private backup ref is created.

The native rewrite runs on a background worker. Accepted submission closes the configuration immediately; the operation area reports **Rewriting commits…**, then **Refreshing history…** until the replacement history update for the refresh generation is accepted. Browsing stays available, incompatible mutations remain disabled, and the retained pre-rewrite rows are labeled while refreshing. The replacement is adopted only at the existing hover/scroll-safe boundary, so the viewport does not jump under the pointer.

### Backups and recovery

`gg_git::backup::list` reads `refs/gg/backup/*` with `git for-each-ref`, newest first. The refs snapshot still excludes `refs/gg/`, so backups never appear as graph labels or ordinary ref rows. **Repository → Recovery…** (Backups source) and the outcome recovery action after a rewrite open the same Recovery window, listing the label, short commit, subject, full private ref, and commit time. Selected backups offer **Open commit**, **Reset branch here** (Soft, Mixed, or Hard), **Create branch here…**, **Copy hash**, and **Copy ref**. **Open commit** returns to the Backups source and selected entry. Reset reuses the existing reset confirmation, including the Hard warning; branch creation reuses the shared form with switching off. No recovery action deletes user files or backup refs.

## File reading and history · M7

### Reading surface

The selected-file view has one file identity and three navigation axes: change, file, and commit. **Diff** and **Full file** are two presentations of the same selected file, not separate viewers. Switching presentation keeps the exact path bytes, the Before/After side, and the reading position. The Before/After side applies to full-file reading; the unified diff always shows both sides.

Read one full-file side on the same selected-file worker as the diff, keyed by generation and side. Reuse the diff document for changed-line markers so a mode or side change never recomputes the comparison. Check the blob header before allocating, refuse sides above 2 MiB, and split at most 100,000 source lines. Missing sides, binary content, gitlinks, symlinks, and budget limits return explicit summaries instead of empty text. Cancel before object reads and between projection steps; a superseded generation or side is never published.

Full-file rows are fixed-height and virtualized with exact one-based line numbers. Added lines on the After side and deleted lines on the Before side carry a marker and the same semantic colors as the diff. Clicking a row sets the cursor; Shift-click extends the range. `Up`/`Down` and `j/k` move the cursor, `Shift+Up/Down` extends the range, `PgUp/PgDn` moves by a page, and every movement keeps the cursor visible. The cursor and range are the place used by history.

`[` and `]` move to the previous or next change. In full-file mode a change is a contiguous run of marked lines; in diff mode it is a hunk. Visible **Prev change**/**Next change** buttons perform the same actions, name the current change and total, and stay disabled when the file has no changes. Jumping to a hunk requests the page that contains it from the retained diff document rather than scanning loaded rows.

`Alt+Up` and `Alt+Down` change the selected file while staying in the reading view. The review header exposes **Prev file**/**Next file** for the same action. The selected file, presentation, and side survive the move; the line position is restored when the new file has that line and clamped otherwise. `Esc` still returns to the file list, and `Enter` opens the selected file.

The place is `{ path bytes, revision, start_line, end_line }`, with one-based inclusive lines and exact path bytes. Engine `Place` stores that identity so path, blame, and line history all use one anchor. A deleted range keeps the before side explicit; history must not silently discard it.

### Shared reader

One reader owns the displayed comparison identity, exact path, both endpoint labels, presentation, inspection side, cursor, anchor, and pixel offset across commit review, staging, and file history. The concrete renderer is the existing `DiffView`; `reader::ReaderContext` is the value consumers exchange. Endpoint labels name the resolved sides (`main (a1b2c3d)`, the parent commit, `Empty tree`, or an older content version) and are shown in the reader header together with the exact modes and blob IDs.

Requested and displayed identities stay separate. A new path, side, mode, or source request increments the generation and cancels superseded work; metadata and rows are adopted together only for the matching identity, and endpoint labels appear only when the displayed content matches that identity. `reader::Capabilities` reports what the current comparison actually supports for Diff, full-side selection, and export, so a label never grants an action. Presentation and side controls never change the comparison; changing the comparison never silently changes the file.

`DiffView::save_context` captures the requested identity, endpoint labels, and place, and `open_context` restores them without creating a second viewer. A restored place survives the asynchronous adoption of the matching content, including an identity change. Commit review saves the context when file history changes the comparison source and restores it exactly once when history closes. The same value is the integration point for staging and later history migration; blame and line-history algorithms remain a separate, unimplemented extension point.

### File and line history

History is a cursor over **content versions**, not over commits. A version is one content state of the followed path; commits that carry an unchanged state are skipped, so the timeline never shows a row that changed nothing. The newest version is the content state the selected revision carries, which may have been introduced by an older commit than the one the review opened. **Full file** shows the current version alone; **Diff** shows the edge `previous → current`. The Before/After inspection sides remain a separate reader control and never navigate versions.

History is a comparison-source mode inside the existing Review reader, not a second content view. Opening it from the reader keeps the file list and review state in place and places a compact version strip directly above the same reader in both the wide side panel and the narrow full-width reading layout. `H` and the visible **History** button open it for the selected path and place. The strip carries **Older version** and **Newer version**, the version counter, and the version's kind, commit, subject, author, date, and old path.

Opening history captures the entry comparison, exact path, revision, presentation, inspection side, cursor/range, and pixel place. Closing history with its button or `Back`/`Escape` restores that context exactly once. Version steps never add navigation or Back history: they change the reader source only. Selecting another file while history is active keeps history open but resets the version cursor to that file's newest content version; the original entry file, comparison, place, and revision stay saved for Close history. Closing returns to that saved entry context and selected file. The previous path's version cursor is never reused. Switching parents leaves history because the comparison revision changed.

Both version directions never wrap: at a boundary the button disables and names the reason — `Oldest version`, `Newest version`, `Only version`, `Loading earlier versions…`, a history limit, or a pending predecessor choice. Loading is a distinct reason from absent: a temporary wait must not read as the end of history. `Alt+Left` and `Alt+Right` perform the same moves.

Predecessor resolution follows carriers of the current content state backwards through parents. If exactly one parent carries the state, the edge is unique and history follows it silently. If several parents carry it and none is a same-state carrier of another, the predecessors are genuinely ambiguous: history stops and shows a chooser in the same version strip with each candidate's kind, commit, subject, author, date, and old path. It never silently takes the first parent. The choice is file-level and becomes the version's edge; choosing a place or hunk inside that edge is a separate question. An edge whose earlier side is not resolved renders the current Full file only, with Diff disabled rather than comparing a file with itself.

A rename is a path-changing edge: the newer version names the new path and the edge names the old path. An absent version is explicit — deletion is a version with no content, not an empty file, and re-adding starts a new added edge. Type changes, additions, deletions, renames, and content changes keep distinct labels. When the trace hits its commit budget, the boundary reports the limit instead of pretending the path starts there.

History preloads a bounded number of versions and pages older ones on demand. The timeline and every version's blob read run on workers keyed by generation; opening another file, backing out, or a newer request cancels the previous work, and a superseded result is never published. A failed page keeps the loaded versions and the pending boundary so a retry resumes rather than restarting. History and blame never move `HEAD`, write refs, or change the index or working files.

`Esc`/`Back` dismisses the active chooser or history source first, then the reading view, then the file list and review, while editor and menu keys retain ownership. `H` while history is open does not stack another history source. Line and range history reuse the same timeline anchored to `Place`. A single line maps through blame first; range boundaries map through diff hunks. When a range spans hunks that diverge into different predecessors, the range asks which predecessor to follow instead of inventing one boundary. A deleted range keeps the before side explicit, and absent versions are identified rather than replaced with empty content. `B` opens blame.

### Deferred large-repository features

Image diffs reuse the bounded preview and export rules. Worktrees keep drafts, selection, and filters isolated per canonical worktree. Submodules expose referenced commit IDs and require an explicit update action. Git LFS keeps pointer identity and fetch actions explicit. Signature verification reports the signer and status without changing repository configuration.
