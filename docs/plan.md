# Plan

Purpose: sequence M3 delivery and acceptance.

## M3 · Complete · Graph, refs, and branches

Goal: understand repository topology and move confidently between branches, refs, and tags.

Contracts: [history graph and refs](architecture.md#history-graph-and-refs--m3). M2 is complete; [evidence](verification.md#m2-commit--2026-10-21).

### Increments

1. **Ref snapshot · implemented.** Read HEAD, local branches, remote branches, and tags with peeled commit targets through shared engine types and `git good refs [PATH]` JSON.
2. **Graph and labels · implemented.** Draw incremental commit lanes, merge nodes, and branch/remote/tag labels in the history view without changing repository state.
3. **Ref navigation · implemented.** Group refs in a toolbar menu, mark HEAD, clear filters when needed, and reveal the selected commit even after history completion.
4. **Branch switching and creation · implemented.** Add clean-worktree switching and creation with name/collision validation, Git post-checkout hooks, CLI receipts, modal UI, and coherent history/ref refresh.
5. **Fork-style ref sidebar · implemented.** Add grouped branch/remote/tag trees, filtering, HEAD marking, ref pills, and a narrow-window menu fallback while preserving the history table width.

### Acceptance

| Scenario | Expected result | Method |
| --- | --- | --- |
| HEAD, branches, remotes, tags | Exact names and peeled commit IDs; symbolic remote HEAD skipped | Git reference fixtures and CLI |
| Unborn and detached HEAD | Explicit HEAD state; no fabricated refs | Git fixtures |
| Linear history | One lane with stable vertical edges | Graph unit tests |
| Merge history | Distinct parent lanes, merge node, correct joins | Graph unit tests and native review |
| Filtered history | Graph rows remain aligned with their commits | Worker tests |
| Ref labels | Branches, remotes, and tags prefix the matching commit; overflow is explicit | egui and native review |
| Ref navigation | Sidebar and menu mark HEAD; selection clears filters and reveals deep commits | Worker and egui tests |
| Ref sidebar | Grouped trees, filtering, HEAD check, and branch switch/jump actions work at wide widths; menu remains below 1120 px | egui and native review |
| Ref pills | Current branch first, then tags and other refs; icons, overflow, and subject remain single-line; click selects and double-click switches/jumps | egui and native review |
| Read-only safety | Objects, refs, index, and working files unchanged | Byte/reference comparison |
| Branch switch/create | Dirty trees, missing/colliding refs, and invalid names reject without moving HEAD; success refreshes refs/history | Git oracle, CLI, and native review |
| Branch actions safety | Local branches only; tags/remotes remain read-only jumps; post-checkout failure is reported | Git fixtures and native review |

Record native wide/narrow captures and verification in [verification](verification.md).
