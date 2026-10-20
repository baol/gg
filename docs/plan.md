# Plan

Purpose: sequence M1 delivery and acceptance.

## M1 · Current · Review commits

Goal: understand a selected commit's files and diff against a chosen parent.

Contracts: [architecture](architecture.md#commit-review--m1-specification). Preserve the [M0 regression baseline](regression.md); add review-context isolation tests before connecting M1 controls.

### Increments

1. **Changed files · implemented.** Typed comparison requests, incremental gitoxide summaries, JSON command, and native Review view. Deliver first-parent/root comparisons, loading/empty/retry states, and history restoration on Back.
2. **Selected-file text · implemented.** Cancellable blob reads, virtualized unified diffs, line numbers, context, horizontal scrolling, and final-newline markers. Bounded worker, paged rows, and CLI `--file`.
3. **Comparison completeness · implemented.** Parent selection, special-object fixtures, mode/type summaries, byte-safe paths, and cancellation between object reads implemented. Exact-side export and PNG/JPEG previews with hexadecimal fallback implemented.
4. **Acceptance · current.** Linux release latency, cache limits, export memory, and repository preservation measured. Complete native input/GPU and platform checks; retain captures and [evidence](verification.md).

### Tests

| Scenario | Expected result | Method |
| --- | --- | --- |
| Root, linear, empty, nested paths | Exact paths, statuses, IDs, modes | Git `diff-tree --raw -z --no-renames` oracle |
| Merge parents; missing parent | Chosen comparison or identified error | Git fixtures |
| Nested cwd, explicit repo, invalid revision/parent | Correct identity, JSON, stderr, exit status | Process tests |
| Many files, slow enumeration, deep navigation | Early rows, complete reachability, bounded caches | Controlled reader/input |
| Rapid commit/file/parent changes, retry | Current-generation results and coherent identity | Controlled worker |
| Review/Back, hover, press, paging | Stable targets, restored anchor, context-local shortcuts | egui input |
| CRLF, whitespace, final newline, Unicode | Correct hunks/line numbers and exact bytes | Git diff oracle and reconstruction |
| Binary, symlink, gitlink, mode/type changes | Appropriate summaries | Git fixtures |
| Large blobs, long lines, cancellation | Responsive UI, bounded memory, explicit completion | Resource measurements |
| Long paths, 720×480/1120×740, 100%/200% | Disjoint controls and stable geometry | Geometry and native review |
| Browse and export | Objects, refs, index, working files preserved | Filesystem comparison |

### Measurement targets

Record machine, fixture, cache state, and build profile; measure cold and warm release runs.

- First summaries: within 500 ms on a packed 10,000-file change.
- First diff: within 500 ms for UTF-8 files up to 1 MiB per side.
- Input-to-frame: below 50 ms during work; cached frame p95 within 16 ms.
- Record peak memory and verify cache/queue budgets.

Advance to M2 after CLI/native comparisons, behavioral tests, visual review, and measurements pass. Record [evidence](verification.md); retain separate [preview gates](../ROADMAP.md).
