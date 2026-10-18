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

## Preview acceptance pending

- Measure cold/warm release startup and peak memory on the packed 100,000-commit fixture. Targets: first rows within 500 ms, input-to-frame below 50 ms, cached frame p95 within 16 ms.
- Verify native clipboard, high-DPI rendering, and live Gravatar delivery.
- Record remaining native build/runtime evidence for the [target platforms](../ROADMAP.md).
