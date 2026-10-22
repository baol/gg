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

Emacs bindings use physical Control. Ctrl+V retains native paste. Escape leaves filter focus. Text editors and open menus retain editing/navigation shortcuts; focused buttons allow list navigation. Reading views use Up/Down, j/k, Ctrl+P/N, and page keys to scroll their content. Enter opens a full-width diff from either file-list layout. History bindings apply while history is visible and editors are unfocused. Empty results keep selection empty.

Follow [bounded-reveal rules](design.md). Page by the rendered viewport with one row of overlap, clamp at history boundaries, and stream uncached destinations. Show cached selected metadata immediately; reserve the full-message slot while loading. Narrow details preserve hidden history selection and pixel offset.

Rationale: one bounded reveal preserves spatial context; overlapping pages retain reading context; fixed details slots keep loading visually stable.

## Presentation

Apply [design guidelines](design.md). Use a repository/action header, history/filter row, and fixed-height status line. Reserve counts/progress/errors. Show full paths on hover and edit paths explicitly.

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

Present status/path rows and selected before/after IDs and modes. Wide layouts use a resizable diff side pane; narrow layouts open a separate reading view with Back to files. History state and incoming history results wait while review is open; Back restores its pixel offset before applying current results.

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

Review opens a resizable file-list/diff workspace with commit identity and parent selection. Narrow windows show files and content successively. Back restores history selection, filter, and pixel anchor. Shortcuts belong to the active context; editors retain theirs.

### Binary previews and export

Decode PNG/JPEG on the selected-file worker. Bound each side to 16 MiB encoded, 8 megapixels, 4096 pixels per dimension, and 64 MiB decoder allocation. Fit previews to 1024 pixels; show original dimensions. Unknown, corrupt, and oversized images retain a labeled hexadecimal preview of the first 256 bytes when inside the encoded budget. Larger blobs retain summaries and export. Publish current-generation previews at the diff's interaction-safe boundary; retain one preview pair and two textures.

Export either present blob side to a new user-chosen path, preserving exact bytes. Write a temporary file beside the destination, then publish without overwriting. Keep Git metadata outside export destinations. Export symlink target bytes; gitlinks retain referenced commit IDs. Cancel before object decoding and between 64-KiB writes; remove temporary output on cancellation/failure. Run one export per review view, pin its path/object identity, and cancel on closing the export or review. Escape closes the export window before review navigation resumes. Gitoxide decoding uses memory proportional to the selected object and delta chain; output writes use fixed-size chunks.

### CLI

`git good show <commit> [--repo PATH] [--parent N]` streams JSON summaries. Add `--file PATH` for a repository-relative selected-file diff, including exact source-line bytes. Add `--export before|after --output PATH` to save that side and emit a JSON receipt. Use cwd discovery by default. Validate one-based parent numbers; roots use the empty tree automatically. Include repository/comparison identity and exact path bytes. Apply runtime error conventions; document usage when implemented.

Rationale: summaries provide early feedback; pinned, cancellable work keeps rapid selection accurate; a dedicated review view gives source readable width.

## History graph and refs · M3

Ref discovery is read-only and shared by CLI and native views. Emit HEAD state (symbolic name, detached flag, commit), local branches, remote branches, and tags with exact short and full names and peeled commit IDs. Skip symbolic remote `HEAD` refs and tags that cannot peel to commits. Keep HEAD explicit for unborn and detached repositories. `git good refs [PATH]` streams this snapshot as JSON; the history worker loads the same snapshot alongside a traversal and replaces it on refresh. History traversal starts from HEAD and every ref target, so branch, remote, and tag-only commits appear in the graph.

Compute graph rows in traversal order, including commits later hidden by a filter, so lane identities stay coherent. Each row carries its own lane, the set of lanes entering and leaving the row, and parent lanes. Reuse a parent's existing lane, allocate free lanes for additional merge parents, retire empty lanes, and cap layout at 64 lanes. Render stable per-lane colors, vertical continuations, parent joins, commit dots, and a ring for merge commits. A single-parent branch keeps its own lane color until it joins an existing lane; merge parents always use the parent lane color. Only the row's own lane starts a downward segment at the commit; newly created parent lanes are connected by their joins. Cache graph rows with the visible page and evict them with their commits.

Render matching refs as Fork-style pills before the subject, with the current branch first, then tags, other local branches, and remotes. Show up to four pills when the subject column has room (two when it is narrow) plus an explicit overflow count. Include the ref-kind icon in each pill and keep long labels single-line. Clicking a pill selects its commit. Double-clicking a local branch pill switches to that branch through the same clean-worktree path as the sidebar; double-clicking a tag or remote pill jumps to its commit without moving HEAD.

At content widths of 1120 px and above, show a fixed 190 px left refs sidebar with a filter, **New branch** action, collapsible **Branches**, **Remotes**, and **Tags** groups, slash-separated folder nodes, HEAD checkmark, and branch/remote/tag icons. Selecting a local branch switches to it after a clean-worktree check; selecting a remote or tag jumps to its commit. Keep the compact Refs menu as the narrow-window fallback. A View toggle hides the sidebar, and the action remains unavailable when there is no room for the history table. Selecting a ref clears a hiding filter, targets its commit by ID, and scans the completed spool to recover the row before revealing it. Ref navigation never moves HEAD, checks out content, writes refs, or modifies the index or working tree.

### Branch switching and creation

Switch and create operate on local branches only after a clean-worktree check. Open the observed status identity, scan for staged, unstaged, untracked, or conflicted paths, and reject the action before invoking Git if anything is dirty. Validate branch names as full local refs, reject collisions on creation and missing branches on switch, and run `git switch` or `git switch -c` from the worktree root so Git performs the atomic index/worktree/ref update and runs `post-checkout` hooks. Remove ambient `GIT_DIR` and `GIT_WORK_TREE` overrides. A missing Git executable, name error, dirty tree, collision, or checkout failure returns an actionable message and leaves the user on the current branch unless Git partially advanced the worktree; the message directs the user to inspect the current branch and hook output. If Git advanced to the target branch but returned nonzero, treat that as a successful switch with a `post-checkout` warning rather than a failed action.

The refs sidebar and the Refs menu treat local branches as switch actions, mark the current branch as HEAD, and keep tags and remotes as read-only jumps. **New branch** opens a modal whose Create action targets the current HEAD. CLI `git good switch [--create] BRANCH [--repo PATH]` returns a JSON receipt. On success, clear the history filter, select the new HEAD, reload history/refs, and refresh Status on its next open. Branch deletion, rename, remote tracking, and tag creation remain later work.

## Working status · M2

`git good status [PATH]` discovers the whole worktree from cwd or an explicit path and streams JSON. Bare repositories return an identified working-tree requirement. Unborn repositories compare the index with an empty tree.

Emit independent staged, unstaged, untracked, and conflict entries with exact path bytes and escaped one-line labels. A path can appear in both staged and unstaged categories. Honor ignore rules; enumerate individual untracked files. Represent renames as deletion/addition. Identify intent-to-add explicitly. Preserve repository contents, including index stat data.

Run scans on cancellable workers. Publish rows in discovery order and keep their ordinals stable for the scan. Stream through a 128-entry queue into disk-backed records; request visible pages of at most 256 rows and retain at most 768 rows in the UI. Index/tree metadata follows repository size; file comparison buffers follow gitoxide's status implementation.

Open Status from the repository header, including on unborn branches. Show category, change, and path in fixed-height rows; expose full paths through the shared overflow view. Count discovered entries until completion; show Clean only after a successful empty scan. Refresh retains the previous scan until replacement completes, then adopts it after hover and scrolling settle. Fresh scans begin at the first row. Keep the previous paging worker available throughout refresh. Back restores history's selection and offset. Failed scans retain identified results and offer retry. Status scans remain observational; file actions use the validated mutation contract below.

### Working diffs

Open a status row with Enter, double-click, or View diff to enter **Review & stage**. Show the staged comparison (HEAD to index) and the unstaged comparison (index to working file) for the same path together: side by side at 860 px and wider, stacked below that. Back restores the status selection and scroll position. `git good status [PATH] --file PATH --area staged|unstaged|untracked|conflict` reads one comparison as JSON.

Pin HEAD and the index fingerprint from the status scan. Compare HEAD to index for the staged pane, index to working file for the unstaged pane, and empty to working file for untracked paths. Capture working bytes on selection, apply Git text normalization, and validate repository inputs and working bytes again before publication. Changed inputs return an identified stale-result error with Refresh status. Compute both panes on one cancellable worker and publish each pane as soon as its comparison completes; adoption must not wait for pointer hover or scrolling to settle.

Reuse the bounded diff engine and fixed source rows. Give each pane its own scroll area and preserve its offset across refreshes. Preserve symlink targets and mode changes. Show explicit binary, gitlink, conflict, and budget summaries. Conflict resolution belongs to M5. Exact immutable-object export remains in commit review.

### File staging

Stage file adds one observed working path to the index. Capture the bytes on disk at action time, like `git add`, rather than publishing a previously reviewed snapshot. Support unstaged modifications, untracked additions, deletions, type changes, intent-to-add, executables, and symlinks. Preserve working files, unrelated index entries, symlink targets, and executable modes. CLI `git good stage --file PATH [--repo PATH]` uses repository-relative paths and returns a JSON receipt after success.

Require an unstaged or untracked observation whose working path still matches its scanned kind: deletions stay absent and every other kind stays present. Normalize regular-file content through Git's attributes and conversion pipeline, then write the blob and its real stat data. Without symlink or executable-bit support, retain the indexed mode for emulated links and files. Reject already-staged paths, conflicts, sparse indexes, submodules, and file/directory collisions.

Acquire the index lock before checking the observed HEAD and index fingerprint. Re-capture the working path before publication and invalidate the tree cache; revalidate HEAD and publish the index atomically. Stale observations, absent or reappearing paths, lock ownership, and working changes during the action return actionable errors. Run UI mutations on a worker; disable duplicate actions and navigation while pending, then refresh status at an interaction-safe boundary.

### Hunk, line, and drop actions

Stage or unstage selected changes from either pane, or drop selected working changes from the unstaged pane. A hunk header carries inline Stage hunk or Unstage hunk actions and, for unstaged text edits, Drop hunk. Clicking the header selects or clears every changed line in that hunk; clicking an added or deleted row toggles that line. The hunk action applies the selected lines when any are selected, otherwise the whole hunk. Pane headers expose whole-file Stage all and Unstage all, and Drop all on the unstaged pane. Identify each hunk by its exact `@@` header and each line by its old- or new-side line number; reject selections that no longer match. CLI `git good stage --file PATH --hunk HEADER` accepts repeated `--hunk` selections and adds `"hunks"` to the stage receipt.

Partial staging requires an unstaged, regular text modification with a 100644 or 100755 index entry. Reject untracked additions, intent-to-add, deletions, type or mode changes, symlinks, submodules, conflicts, sparse indexes, binary comparisons, and comparisons beyond the text budget. Acquire the index lock, then capture and normalize the working bytes through the same pipeline as the displayed diff and recompute index to working. Unknown headers or lines mean the diff is stale and publish nothing.

Rebuild the staged index blob by line ranges: keep unchanged gaps and unselected old-side lines, and take selected new-side lines. When every change is selected, write real working stat data, matching `git add -p` after a full selection. Otherwise write zeroed stat data so status compares content and reports the remaining unstaged edit (`MM`). Preserve unrelated index entries, modes, and working bytes; re-capture working bytes and revalidate HEAD before atomic publication.

Partial unstaging supports staged regular text additions, modifications, and deletions. Rebuild from the HEAD side and the index side: restore selected old-side lines and retain unselected new-side lines. If the result equals HEAD, restore the HEAD index entry; if HEAD is absent and the result is empty, remove the index entry. Otherwise write the partial blob with zeroed stat data. Mode, type, and symlink changes require whole-file actions.

After a successful mutation, open a fresh status session for the new HEAD/index identity, update both pane requests, and reload them in place. Retain scroll offsets, clear line selections, and stay in **Review & stage**; refresh the status list when the user returns with Back. Failures keep the panes and index unchanged and show the error above them.

Rationale: hunk- and line-level actions follow `git add -p` while zeroed partial stat keeps omitted edits visible instead of silently clean. Showing both comparisons lets a user move a change between staged and unstaged without losing context.

### Dropping working changes

Drop discards selected working changes without touching the index. For an unstaged regular text modification, rebuild the working file from the index and working sides: restore selected old-side lines and retain unselected new-side lines. Expand any selected row to its contiguous change block so a replacement cannot be half-discarded. Convert the normalized result back through Git filters before writing, validate the working snapshot again, and replace the file atomically. Partial drops reject mode, type, and symlink changes; use Drop all for those.

Drop all restores tracked modifications, deletions, mode changes, type changes, and symlinks from the index, and removes untracked files. Acquire the index lock for consistency, reject conflicts, intent-to-add, submodules, sparse indexes, and file/directory collisions, and leave every index byte unchanged. Require an explicit confirmation before any drop; keep the panes visible, reload both comparisons in place after success, and refresh the status list on Back.

Rationale: dropping is intentionally working-tree-only and reversible with the index still intact. Confirmation and change-block expansion prevent accidental loss while keeping the split review usable.

### Committing

Commit writes the current index, not working-tree content. Pin HEAD and the index when the action starts, reject commits with no staged changes, unmerged entries, intent-to-add entries, sparse indexes, and an unavailable worktree, and resolve author and committer identities from Git configuration or the documented environment variables. Normalize the message, write `COMMIT_EDITMSG`, and run hooks through `git hook run` so `core.hooksPath`, linked worktrees, and native Git execution rules are honored.

Run `pre-commit`, `prepare-commit-msg` with the message file and `message` source, and `commit-msg` before taking the index lock. Hooks may stage content or edit the message, so re-read the index and message afterwards and reject a HEAD change made during the hooks. Acquire the index lock only after hooks finish, rebuild the tree from the current index, verify it differs from HEAD, and create the commit object with the observed parent. Update `HEAD` atomically with the reference lock and write the reflog. Run `post-commit` after publication; its failure is a warning because the commit already exists.

Every other hook failure aborts before the commit is published. Preserve the index, working tree, and message file, and return the hook name, exit status, and captured output with a retry path. The native view keeps the draft message, refreshes status to expose any hook staging, and stays on Status; Back refreshes history and selects the new commit. CLI `git good commit -m MESSAGE` returns a JSON receipt with the commit, tree, parents, and any post-commit warnings. Amend and signing remain later milestones.

### Whole-file unstaging

Unstage file restores exactly one staged path from the observed HEAD, or removes its index entry on an unborn branch. Preserve working files and unrelated index entries. CLI `git good unstage --file PATH [--repo PATH]` uses repository-relative paths and returns a JSON receipt after success.

Acquire the index lock before checking the observed HEAD and index fingerprint. Restore the entry, invalidate the tree cache, revalidate, and publish the index atomically. Conflicts, sparse indexes, file/directory collisions, stale observations, and an existing lock return actionable errors. Run UI mutations on a worker; disable duplicate actions and navigation while pending, then refresh status at an interaction-safe boundary. Whole-file unstaging remains the fallback for modes, types, symlinks, untracked additions, deletions, and conflict changes.

Rationale: restoring one entry gives a reversible operation while preserving the user's working edits and other staging decisions.
