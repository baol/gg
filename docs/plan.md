# Plan

Purpose: sequence M1 implementation and define its acceptance checks.

Contracts: [architecture](architecture.md#commit-review--m1-specification). Future outcomes: [roadmap](../ROADMAP.md). Baseline and release checks: [verification](verification.md).

## M1 · Current · Review commits

Goal: select a commit, understand its changed files, and read a file's diff against the chosen parent.

Preserve the [M0 regression baseline](regression.md) in every increment. Add review-context isolation tests before wiring M1 controls.

Deliver one working increment at a time. Refine interaction, appearance, and tests together. M0 remains the browser baseline; preview acceptance remains tracked separately.

### Increments

1. **Changed files · next.** Add typed comparison requests and incremental file summaries. Resolve first-parent/root comparisons through gitoxide. Expose the same results through `git good show` and a native Review view. Show fixed-height status/path rows, progressive counts, loading, empty, and retry states. Preserve history on Back. Verify the full CLI-to-native workflow before proceeding.
2. **Selected-file text.** Read selected blobs on a cancellable worker. Add unified hunks, line numbers, context, horizontal scrolling, and missing-newline markers. Virtualize diff rows. Keep selected-file identity explicit during loading and failure.
3. **Comparison completeness.** Add merge-parent selection, binary summaries, symlinks, gitlinks, mode/type changes, byte-safe paths, and explicit large-file summaries/export. Resolve resource budgets and test cancellation at each expensive boundary.
4. **Polish and acceptance.** Exercise rapid selection, paging, keyboard focus, narrow navigation, scaling, errors, and read-only guarantees. Measure first-file and first-diff latency, frame times, and peak memory; retain native captures.

### First increment boundary

A selected commit opens its changed files without reading blob contents. Roots compare against the empty tree; other commits compare against their first parent. The UI and JSON expose matching identities, modes, statuses, and paths. Closing review restores the exact history position. File contents arrive in increment 2.

### Test plan

| Scenario | Expected result | Method |
| --- | --- | --- |
| Root, linear, empty commit, nested paths | Exact changed paths, statuses, IDs, modes; explicit empty result | Git `diff-tree --raw -z --no-renames` oracle |
| Merge with distinct parents; missing parent | Correct chosen comparison or identified read error | Synthetic Git fixtures |
| CLI from nested cwd, explicit repo, invalid revision/parent | Correct discovery/identity; JSON success; stderr and nonzero failure | Process tests |
| Many files; slow enumeration; deep navigation | First page before completion; complete reachability; bounded caches | Controlled reader and input tests |
| Rapid commit/file/parent changes and retry | Matching-generation results only; coherent identity throughout | Controlled worker tests |
| Review/Back, hover, pointer-down, keyboard paging | Stable targets and restored history anchor; context-local shortcuts | In-memory egui input |
| Text edits, CRLF, whitespace, final newline, Unicode | Correct hunks and line numbers; exact changed bytes | Git diff oracle and reconstruction |
| Binary, symlink, gitlink, mode/type changes | Explicit appropriate summaries | Git fixtures |
| Large blobs, long lines, cancellation | Responsive UI, bounded memory, explicit completion state | Resource/latency measurements |
| Long paths, 720×480 and 1120×740, 100%/200% scale | Disjoint controls, readable source, stable loading geometry | Geometry tests and native review |
| Browse and export | Repository objects, refs, index, and working files preserved | Before/after fixture comparison |

### Measurement targets

Record machine, fixture, cache state, and build profile. On a local packed fixture with 10,000 changed files, target first summaries within 500 ms. For a selected UTF-8 file of at most 1 MiB per side, target first diff within 500 ms. Keep input-to-frame below 50 ms during worker activity and cached frame p95 within 16 ms. Set and test memory/queue budgets before adding blob loading; measure cold and warm release runs separately.

### Completion

CLI and native views review the same immutable comparisons. Every changed file is reachable; text and non-text cases have explicit results. Interaction tests, visual review, and measured resource budgets pass. Record evidence before advancing to M2.
