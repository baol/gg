# Roadmap

Purpose: define milestone outcomes, priorities, and release gates.

CLI-first usage: [README](README.md). Each milestone delivers terminal commands and a native view of the same engine behavior.

| Milestone | Workflow | Acceptance |
| --- | --- | --- |
| M0 · Current | Browse history | CLI-first commit browser; acceptance in [plan](docs/plan.md) |
| M1 | Review commits | File list, parent selection, text diffs, binary summaries, bounded loading |
| M2 | Commit work | Status, file/hunk staging, unstage, commit, mutation preconditions |
| M3 | Navigate branches | Graph, refs, tags, switch/create, pagination, cancellation |
| M4 | Synchronize | Clone, fetch, push, tracking, credentials, progress |
| M5 | Integrate | Merge, resolve conflicts, cherry-pick, revert, stash, recovery |
| M6 | Refine history | Rebase, reorder, squash, amend, reflog recovery |
| M7 | Expand workflows | Worktrees, submodules, LFS, signing, file history, blame, image diffs |

## Release gates

Target Linux (Wayland/X11), macOS, and Windows, with x86_64 and arm64 coverage added through native builds and hardware checks. Hosted CI starts with its available architectures.

Publish previews after M0 and platform runtime acceptance. Publish a daily-use release after M2, native packaging, and recovery documentation for mutations. Check keyboard access, accessibility, scaling, clipboard, and performance at each milestone. Evidence: [verification](docs/verification.md).
