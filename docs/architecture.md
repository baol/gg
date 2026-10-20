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

Traverse a deduplicated frontier from HEAD: emit the newest committer timestamp, break ties by descending object ID, then enqueue parents. Assign stable ordinals and retain continuation state. Every reachable commit remains accessible.

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
| Open details / return | Enter / Escape or Back |

Emacs bindings use physical Control. Ctrl+V retains native paste. Escape leaves filter focus. Text editors retain editing/navigation shortcuts; history bindings apply while history is visible and editors are unfocused. Empty results keep selection empty.

Follow [bounded-reveal rules](design.md). Page by the rendered viewport with one row of overlap, clamp at history boundaries, and stream uncached destinations. Show cached selected metadata immediately; reserve the full-message slot while loading. Narrow details preserve hidden history selection and pixel offset.

Rationale: one bounded reveal preserves spatial context; overlapping pages retain reading context; fixed details slots keep loading visually stable.

## Presentation

Apply [design guidelines](design.md). Use a repository/action header, history/filter row, and fixed-height status line. Reserve counts/progress/errors. Show full paths on hover and edit paths explicitly.

History columns: dot, subject, author image/name, short hash, date. Preserve minimum table width with horizontal scrolling. Use high-contrast selection. Group details portrait, author/date, subject, and copyable IDs; retain full IDs behind abbreviations.

Below 1040 logical pixels, details replace history reversibly; wider windows use a resizable side pane preserving table width.

### Avatars

Use a bounded Gravatar worker keyed by normalized email SHA-256. Cache one validated 128-pixel image per author; display at 18 pixels in history and 64 in details, with reserved initials fallbacks.

Persist images and 404 results in the OS cache. Read disk first; deduplicate author requests across texture eviction. Attempt transient failures once per session; retry next launch. Replace corrupt entries atomically. Bound bytes, decoded dimensions, timeouts, queues, and GPU cache. Publish/evict images after history/details hover and scrolling end. Cache location: [development](development.md).

## Commit review · M1 specification

### Comparison

Pin repository, commit, and parent IDs. Resolve revisions once. Default to first parent; roots compare with the empty tree. List merge parents in stored order with one-based numbers, short IDs, and full-ID tooltips. Parent changes reuse the pinned commit ID. Missing parent objects produce an identified incomplete-comparison state; retain metadata during loading/errors.

Keep exact Git path bytes and a one-line display string. Preserve Unicode; escape backslashes, controls, and invalid bytes (`\xNN`) so distinct paths have distinct labels. Copy Unicode paths verbatim; label the fallback **Copy escaped path**. CLI `--file` matches native path bytes. Changes carry old/new object IDs and modes, with added/deleted/modified/type-changed status. CLI and UI share deterministic Git tree traversal order. Represent moves as deletion/addition in M1, retaining both sides for future rename detection.

Parent switches retain the displayed comparison through loading/failure, label its identity, and offer retry. Adopt replacement files and diff together after pointer leave and scroll settling. Start the new comparison at its first file; empty comparisons clear selection. Independent worker mailboxes isolate superseded requests.

### Loading

Stream file summaries before blob reads. Virtualize rows using history's paging and interaction rules; counts progress to exact completion. Within a comparison, preserve selection by exact path, otherwise first result; clear it for empty comparisons.

Read selected-file content on a worker independent of history indexing. Key results by review generation, comparison, and file. Coalesce requests, cancel obsolete work between batches, and apply current results. Page diff rows and keep every file reachable through bounded caches.

### Changed-file implementation

Walk trees depth-first in Git tree order, skip identical subtrees, and defer blob contents to selected-file requests. The frontier scales with active tree entries. Yield between tree visits; individual tree decoding remains one operation.

Use an independent comparison worker with 32-step/4-ms slices, one request/result slot, temporary record/offset files, 256-row pages, and at most 768 UI rows. Retry replaces the worker and its mailbox for the same pinned comparison. Closing review cancels it; obsolete mailboxes remain isolated.

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
| Size/computation limit | Explicit summary, completion state, streaming export |

Review opens a resizable file-list/diff workspace with commit identity and parent selection. Narrow windows show files and content successively. Back restores history selection, filter, and pixel anchor. Shortcuts belong to the active context; editors retain theirs.

### Binary previews and export

Decode PNG/JPEG on the selected-file worker. Bound each side to 16 MiB encoded, 8 megapixels, 4096 pixels per dimension, and 64 MiB decoder allocation. Fit previews to 1024 pixels; show original dimensions. Unknown, corrupt, and oversized images retain a labeled hexadecimal preview of the first 256 bytes when inside the encoded budget. Larger blobs retain summaries and export. Publish current-generation previews at the diff's interaction-safe boundary; retain one preview pair and two textures.

Export either present blob side to a new user-chosen path, preserving exact bytes. Write a temporary file beside the destination, then publish without overwriting. Keep Git metadata outside export destinations. Export symlink target bytes; gitlinks retain referenced commit IDs. Cancel before object decoding and between 64-KiB writes; remove temporary output on cancellation/failure. Run one export per review view, pin its path/object identity, and cancel on closing the export or review. Escape closes the export window before review navigation resumes. Gitoxide decoding uses memory proportional to the selected object and delta chain; output writes use fixed-size chunks.

### CLI

`git good show <commit> [--repo PATH] [--parent N]` streams JSON summaries. Add `--file PATH` for a repository-relative selected-file diff, including exact source-line bytes. Add `--export before|after --output PATH` to save that side and emit a JSON receipt. Use cwd discovery by default. Validate one-based parent numbers; roots use the empty tree automatically. Include repository/comparison identity and exact path bytes. Apply runtime error conventions; document usage when implemented.

Rationale: summaries provide early feedback; pinned, cancellable work keeps rapid selection accurate; a dedicated review view gives source readable width.
