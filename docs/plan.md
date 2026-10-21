# Plan

Purpose: sequence M2 delivery and acceptance.

## M2 · In progress · Prepare commits

Goal: inspect working changes, stage deliberate edits, and create a commit.

Contracts: [architecture](architecture.md#working-status--m2). Preserve the [regression baseline](regression.md). M1 acceptance: [evidence](verification.md#m1-acceptance--2026-10-20).

### Increments

1. **Status · implemented.** Stream staged, unstaged, untracked, and conflicted paths into CLI JSON and a compact native view. Support refresh, cancellation, empty/error states, and Back to history.
2. **Working diffs · implemented.** Review index and working-tree content with explicit comparison identities and stale-result detection.
3. **File staging · in progress.** Whole-file unstaging validates HEAD/index and refreshes status. Next: stage captured working content with the same preconditions.
4. **Hunk staging.** Select and validate patches against their observed source before applying them to the index.
5. **Commit.** Validate index and HEAD preconditions, create a commit, and refresh history/status coherently.

### Status acceptance

| Scenario | Expected result | Method |
| --- | --- | --- |
| Staged, unstaged, both, untracked, ignored | Exact paths and independent categories | Git porcelain oracle |
| Unborn, nested cwd, linked worktree, bare | Whole-worktree identity or explicit error | Git fixtures and CLI |
| Conflict, mode/type, intent-to-add, unusual paths | Explicit status and exact path bytes | Git fixtures |
| Large worktree, early exit, refresh | Early rows, bounded queues/cache, cancellation | Worker tests |
| Hover, scroll, refresh, Back | Stable targets and retained history anchor | egui input |
| Clean and failed scan | Clean only after completion; retry with retained results | State tests |
| Working diffs, CRLF, stale HEAD/index/file | Exact sides, normalized text, explicit refresh | Git fixtures and CLI |
| Read-only inspection | Index, refs, objects, and files preserved | Byte comparison |

Record measurements and native wide/narrow captures in [verification](verification.md). M2 completes after all five increments meet their contracts.
