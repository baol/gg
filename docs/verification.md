# Verification

Purpose: record measured results and outstanding acceptance checks.

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
- A controlled 10,000-file reader verifies first-page delivery, complete enumeration, deep/back page reload, retained errors, and shutdown. UI tests verify placeholder stability and review/history isolation with exact Back anchors.
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
