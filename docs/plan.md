# Plan

Purpose: sequence the implementation steps for the current and next milestone.

Product entry and usage: [README](../README.md). Command contracts: [architecture](architecture.md). Milestones and release gates: [roadmap](../ROADMAP.md).

## Delivery sequence

1. Define typed engine requests and results.
2. Implement Git access with synthetic fixtures and independent Git oracles.
3. Connect terminal commands and native views to the same behavior.
4. Verify errors, responsiveness, and rendering; record evidence.
5. Update milestone status.

## M0 · Current · Commit browser

Goal: run `git good` inside a repository and immediately understand its recent history.

### Delivery rule

Finish M0 before starting another milestone. Deliver small working increments; refine behavior and visual design together. [Architecture](architecture.md) owns the streaming and interaction contracts. [Roadmap](../ROADMAP.md) holds future outcomes.

### Increments

1. **Stable browser state.** Separate requested and displayed repository, selection, viewport anchor, and request generation. Define keyboard navigation and error/retry states.
2. **First screen.** Stream visible summaries and overscan from a pinned HEAD. Load selected details independently. Remove the 500-commit ceiling from desktop and headless history.
3. **Whole history.** Add progressive indexing, count estimates, exact completion, paged storage, cache eviction, and deep-scroll progress. Keep foreground requests ahead of counting.
4. **Stable interaction.** Preserve anchors, hover targets, click identity, focus, and scrollbar geometry as batches arrive. Stream whole-history filter results with explicit partial/completed states.
5. **Visual refinement.** Tune typography, density, spacing, selection, loading slots, and narrow-layout details. Review real rendered states at each increment; retain screenshots for final acceptance.
6. **Acceptance.** Verify installed CLI launch, interactive behavior, performance, and native preview targets; record results in [verification](verification.md).

### Progress

History now streams without a commit cap in the UI and CLI. Disk-backed indexing, whole-history filtering, bounded row caches, progressive counts, hover-safe publication, and refresh anchors are implemented. Remaining acceptance covers platform integration, broader interaction sequences, and performance/memory measurements. Current evidence: [verification](verification.md).

### Test plan

| Scenario | Expected result | Method |
| --- | --- | --- |
| Root, nested directory, explicit path, invalid path | Correct repository; useful error and exit status | Installed CLI/process tests |
| Linear, merge, skewed dates, packed, shallow, empty history | Stable unique order; exact merge-parent IDs; exact reachable count or explicit incomplete/error state | Git fixture oracles |
| 499, 500, 501, and 100,000 commits; first batch before traversal ends | Every commit accessible; first screen arrives progressively | Adapter integration tests |
| Slow count and rapid viewport changes | Visible requests win; queues and caches stay bounded | Controlled worker tests |
| Scroll forward/back after eviction; deep thumb jump | Same identities and ordering; loading progress; preserved anchor | State tests and native walkthrough |
| Count estimate grows or shrinks during wheel/momentum/thumb drag | Fixed gesture geometry; same anchored commit and pixel offset afterward | Input-sequence tests |
| Batch arrives during hover or pointer-down | Same hovered target; click activates the original commit; placeholder remains inert until re-entry | Input-sequence tests |
| Late results after filter, refresh, or repository switch | Only matching generation affects the view | State tests |
| Partial search, zero matches, hidden selection | Preserve matching selection; otherwise first match; clear details at zero matches; search reaches completion | State and integration tests |
| External HEAD update; refresh failure and retry | Explicit adoption; old session stays identified; selection retained where possible | Integration tests |
| Read-only browsing | Refs, index, objects, and working-file contents unchanged | Before/after fixture comparison |
| Long text, Unicode, 720×480 window, 100%/200% scale | Readable rows, accessible actions, stable details navigation | Render inspection and native walkthrough |
| Filter focus, row arrows, refresh, copy, return from details | Predictable focus and selection; exact copied ID | Input tests plus native clipboard checks |

Use egui `Context::run` with synthetic `RawInput` for in-memory interaction tests; inspect focus, rendered rows, geometry, and output commands. Use a controllable worker and clock for arrival timing. Reserve native runs for platform integration and visual review.

### Performance acceptance

On a recorded reference machine and a packed fixture of at least 100,000 commits, target first rows within 500 ms of session opening, input-to-frame latency below 50 ms during indexing, and cached scrolling within a 16 ms frame budget at the 95th percentile. Measure cold and warm runs separately. Record application cache/queue limits and peak memory; demonstrate eviction without losing history access. Treat these as acceptance targets to validate, not current measurements.

### Completion

All increments pass their behavioral checks and visual review. Installed `git good` opens its current repository immediately; history streams through completion without an application commit cap or interaction discontinuities. Each preview platform has native build and runtime evidence. M0 remains current until results are recorded.
