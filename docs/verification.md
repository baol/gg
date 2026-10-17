# Verification

Purpose: record completed checks and observed results.

2026-10-17 · Linux x86_64 · Rust 1.94

## M0 baseline evidence

- Workspace build succeeds with the locked dependency graph.
- Formatting and Clippy pass with warnings treated as errors.
- Eight tests cover filtering, UI states, graphics backend configuration, discovery, unborn branches, invalid paths, missing objects, packed history, detached HEAD, merge parents, bare repositories, linked worktrees, and the baseline 500-commit cap (removed by the streaming increment).
- Adapter history IDs match `git rev-list HEAD` on the packed linear fixture.
- The native app opens on an isolated X11 display and exits successfully after a rendered screenshot.
- Visual inspection confirms the repository toolbar, selected commit row, full ID, author, formatted date, parents, message, and repository status fit the 1120 × 740 window.
- The JSON command returns the same three-commit fixture shown in the desktop capture.

Platform release gates: [roadmap](../ROADMAP.md).

## Browser state increment

- Twenty-nine workspace tests pass; format and Clippy checks pass.
- State tests cover filtering, zero matches, retained selection, failed refresh, retry, stale generations, and repository switching.
- In-memory egui input tests cover filter focus/Escape, keyboard selection beyond the viewport, rendering the selected row, and exact copy commands.
- Process tests exercise Git's `good` dispatch from repository roots and nested paths, explicit-path override, JSON parsing, stderr, and failure exits.
- Isolated X11 visual review confirms the revised toolbar, displayed repository identity, filter, left-aligned rows, and details panel at 1120 × 740.

## Tabular history increment

- One-line rows share non-overlapping column geometry with the header; narrow panes scroll horizontally.
- In-memory tests assert cell bounds at 660–2560 logical pixels and single-line long text at 100%/200% scale.
- List and details share a single cached 128-pixel texture; an in-memory test verifies both reserved display sizes. X11 visual review confirms the 64-pixel details portrait and initials fallback fit without overlap.
- Avatar tests cover normalized hash keys, deferred publication, persistent image/404 hits across worker restarts, corrupt entries, transient errors, and request deduplication without disk storage. Network access is disabled in UI tests.
- Isolated X11 visual review confirms dots, short hashes, initials, author names, dates, aligned headers, and selection. Live Gravatar delivery remains a network integration check.

## Compact layout review

- Repository identity and actions share one header; history count and filter share one row. Status messages keep a fixed height.
- Details group portrait/author/date, displays the subject once, and keeps copyable IDs beside short labels.
- In-memory checks cover 720×480, 1120×740, and 1600×900 at 100%/200% scale: toolbar visibility, table width, pane separation, and narrow Details/Back navigation with the selected row still visible.

- X11 captures reviewed at [1120×740](screenshots/m0-wide.png) and 720×480: [history](screenshots/m0-narrow.png), [details](screenshots/m0-details.png).

## Bounded text increment

- Shared fixed-height fields expose overflow through clickable `...`, full-text tooltips, and a separate selectable/copyable reading view.
- In-memory input verifies exact disclosure text and single-line bounds at 100%/200% scale; X11 review confirms bounded message and parent fields.
- Twenty-nine workspace tests pass; formatting and Clippy pass.

## Streaming increment

- The 499/500/501 boundary fixtures and CLI JSON test confirm complete history without a cap. Git fixtures verify pinned HEAD, exact merge parents, timestamp-skew ordering, and shallow boundaries.
- A controlled 100,000-commit reader proves first-page delivery before completion, bounded responses, deep-page reload, full-message filtering through the last commit, and exact zero-match completion.
- State/input tests verify cache eviction, stale-query rejection, selection intent, pinned placeholders, and frozen extent during wheel input, followed by stable growth under a resting pointer with preserved viewport offset.
- Native X11 [first-frame capture](screenshots/m0-streaming.png) shows usable rows and a progressive count while the real 100,000-commit fixture is still being indexed.
- Debug build on Linux x86_64, Intel i5-6600K: warm packed fixture: first rows 35.6 ms, complete indexing 18.8 s, in-memory frame p95 5.31 ms, 97 rows retained after visiting the oldest commit and returning. OS caches were warm; in-memory frames exclude GPU presentation.

- All 37 default workspace tests and the opt-in real-fixture timing check pass; formatting and Clippy pass.

## M0 acceptance pending

Cold/warm release measurements, peak-memory accounting, native thumb-drag/clipboard/high-DPI checks, and remaining platform evidence. Checklist: [plan](plan.md).
