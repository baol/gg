# Plan

Purpose: sequence M2 delivery and acceptance.

## M2 · In progress · Prepare commits

Goal: inspect working changes, stage deliberate edits, and create a commit.

Contracts: [architecture](architecture.md#working-status--m2). Preserve the [regression baseline](regression.md). M1 acceptance: [evidence](verification.md#m1-acceptance--2026-10-20).

### Increments

1. **Status · implemented.** Stream staged, unstaged, untracked, and conflicted paths into CLI JSON and a compact native view. Support refresh, cancellation, empty/error states, and Back to history.
2. **Working diffs · implemented.** Review index and working-tree content with explicit comparison identities and stale-result detection.
3. **File staging · implemented.** Stage working content or restore a staged path under the observed HEAD/index identity. Preserve working files and unrelated entries, then refresh status.
4. **Hunk staging · implemented.** Stage selected working-diff hunks, preserving omitted edits and unrelated index state, with stale-source validation.
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
| Stage add/modify/delete/type/ITA, symlinks, modes | Index matches `git add`; working bytes preserved | SHA-1/SHA-256 Git oracles |
| Stage conflicts, submodules, collisions, stale inputs | Actionable errors; index and lock preserved | Git fixtures |
| Stage/unstage UI and CLI | Receipts, single-flight worker, refreshed rows | CLI and worker tests |
| Stage selected hunks | Index matches `git add -p`; omitted edits stay unstaged | Git oracle and stat fields |
| Hunk staging limits and stale inputs | Actionable errors; index, lock, and working bytes preserved | Git fixtures and CLI |
| Hunk selection UI | Selected header highlighted; success refreshes; failure stays readable | egui worker test |
| Read-only inspection | Index, refs, objects, and files preserved | Byte comparison |

Record measurements and native wide/narrow captures in [verification](verification.md). M2 completes after all five increments meet their contracts.
