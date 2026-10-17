# Architecture

Purpose: define system boundaries, dependency direction, and runtime contracts.

## Layers

| Layer | Crate | Responsibility |
| --- | --- | --- |
| 0 | `gg-engine` | Typed commands, incremental reader, filtering |
| 1 | `gg-git` | Native repository discovery and object access through gitoxide |
| 2 | `git-good` | CLI dispatch, egui/eframe UI on wgpu, worker lifecycle |

Dependencies point downward. The engine stays independent of Git libraries, UI, and platform services. CLI and UI share engine commands; view state belongs to the shell.

## Launch contract

Both interactive and headless commands resolve the repository from the process working directory by default. Resolve an explicit path instead when supplied. Discover containing repositories from nested directories. Interactive launch loads history immediately; the path toolbar supports later navigation.

## History sessions

Open a session pinned to the repository identity and resolved HEAD object ID. Assign a generation to every request and result. A refresh creates a new generation; the visible session remains available until replacement is ready and interaction permits the switch.

Traverse from HEAD using a deduplicated frontier: emit the frontier commit with the newest committer timestamp, break ties by descending object ID, then enqueue its parents. This defines a deterministic discovery order that can stream immediately, including histories with skewed timestamps. Assign stable row ordinals as traversal advances. Publish final rows progressively; retain continuation state instead of restarting walks for each viewport.

Expose viewport-range requests and incremental results: row summaries, count progress, details, completion, and errors. Summaries contain ID, subject, author, and date; fetch full messages and parents for selected details. Bare repositories, linked worktrees, unborn branches, and detached HEAD remain explicit states.

Prioritize the visible range, then nearby overscan, then background indexing/counting. Yield background work between small batches. Coalesce obsolete viewport requests; reject results from older generations. Use bounded queues and a bounded summary/detail cache. Retain the ordinal-to-ID index in pages, spilling to temporary storage as needed. Cache limits constrain memory; every reachable commit remains accessible.

### Current session storage

Traverse in slices of at most 32 commits or 4 ms. Store commit records and fixed-size ordinal offsets in temporary files; build a separate offset index for whole-message filtering. Reuse the pinned traversal and stored records across viewport requests and filter edits. Files close with the session.

Coalesce requests and responses into one slot each. Serve up to 256 nearby summaries per response; retain at most 768 UI rows and one selected full message. Cache the worker's current page. Foreground requests run between traversal slices. The traversal frontier and visited-ID set scale with graph size; release visited IDs on completion. Surface storage and traversal errors with retained rows and an incomplete count.

The CLI writes the same traversal incrementally as JSON. Success includes every available reachable commit; failures return a nonzero status. Shallow repositories identify their available boundary explicitly.

### Count and scrollbar

Publish `discovered`, optional `estimate`, and `complete`. Reuse valid count information where available; treat repository-wide commit-graph counts as hints about reachable history. Show an estimate as approximate. With insufficient evidence, show a growing discovered extent rather than inventing a total. Counting advances the same ordered index used by viewport requests and eventually supplies an exact total.

Anchor the viewport by commit ID and pixel offset. Extend scroll range without changing that anchor. Freeze scrollbar geometry throughout scrolling, momentum, and thumb dragging; apply pending extent changes after interaction settles. Preserve the thumb's grab offset while dragging. A jump beyond the indexed frontier shows fixed-height loading slots and advances traversal with visible progress; arbitrary deep access may require indexing the preceding history.

### Interaction stability

Reserve fixed row heights and stable slots. Fill off-pointer loading slots in place; preserve existing row identity, position, focus, and selection. Pin a hovered row's content and hit target until the pointer leaves. Bind pointer-down to commit identity through pointer-up; a loading slot becomes clickable only after the pointer leaves and re-enters it.

Scrolling may reveal and fill requested slots while preserving the anchor and geometry. Queue reordering, session replacement, and filter-result replacement during hover, pointer-down, scrolling, momentum, or scrollbar dragging. Extend the discovered range after scrolling settles while preserving hovered row content and viewport offset; defer extent changes while the scrollbar control is hovered. Commit queued changes after the relevant interaction ends; use a short configurable quiet interval for scroll settling, never a timer alone to release a hovered target.

Explicit filter edits start a new result generation. Search the full pinned history progressively, label partial results, and finish with an exact match count. Publish matches in traversal order. Apply the same slot and interaction rules to arriving matches. Preserve selection when it matches the new filter; otherwise select the first published match. Clear selection and details while there are no matches, distinguishing searching from completed zero results. Apply selection changes with the corresponding result batch at an interaction-safe boundary. External repository changes produce a refresh indication; adopt them on explicit refresh.

### Browser state and keyboard

Track requested path separately from the displayed repository. Label retained results during loading and after errors. A successful repository switch clears the old filter and selection; a refresh within the same repository preserves matching selection by ID, otherwise selects the first visible result.

Ctrl on Linux/Windows and Command on macOS: F focuses filtering, R refreshes, C copies the selected full commit ID when a text editor has no focus. Escape leaves the filter; Up/Down moves through matching rows and reveals the selection. Text editors retain their normal navigation and copy behavior.

### Presentation

Apply the shared [design guidelines](design.md) to every text field and overflow disclosure.

Use one repository/action header, one history/filter row, and a fixed-height status line. Show the full repository path on hover and open its editor explicitly. Keep counts in reserved space and use high-contrast selected rows. Group the details portrait with author/date, show the subject once, and place copyable IDs in compact metadata rows. History uses fixed-height, single-line tabular rows: dot, subject, author image/name, short hash, and date. Share column geometry between headers and rows; truncate within clipped cells. Preserve a minimum table width with horizontal scrolling to keep columns disjoint. Assert geometry and one-line text at supported sizes/scales. Loading slots match final row geometry. Keep progress and errors in reserved space. At widths below 1040 logical pixels, show details as a reversible view while retaining list position; wider windows use a resizable side pane that preserves table width. Enter opens details; Escape or Back returns to narrow-screen history. Maintain full IDs in data and offer copy independently of displayed abbreviation.

Fetch [Gravatar](https://docs.gravatar.com/sdk/images/) images on a bounded background worker using normalized email SHA-256 keys. Cache a shared 128-pixel image per author; display it at 18 pixels in history and 64 pixels in details for sharp high-DPI rendering. Reserve both slots and show local initials while unavailable. Cache validated images and 404 results persistently by normalized author key in the OS cache directory. Read disk before querying; deduplicate requests per author even after texture eviction. Attempt transient failures once per worker session and retry on the next launch. Replace corrupt entries atomically. Bound response bytes, decoded dimensions, request duration, queues, and GPU cache size. Defer image publication and cache eviction while history or details is hovered, or scrolling is active.

## Runtime requirements

- Implement application behavior in safe Rust; propagate fallible operations.
- Preserve objects, refs, index, and working files while browsing.
- Perform Git reads and indexing on workers; publish immutable batches and repaint only for visible changes.
- Virtualize rows and retain the displayed session through read or refresh errors.
- Identify operation and path in errors; support retry.
- Preserve native paths in engine and adapter APIs; use Unicode text for editable UI paths.
- Decode display text with replacement characters where needed; retain exact object identity.
- Provide synchronous JSON output and failure exit status for headless commands.

## Growth

Implement the current milestone incrementally. Add abstractions when its concrete commands need them.
