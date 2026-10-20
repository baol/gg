# Roadmap

Purpose: define milestone outcomes and release gates.

Each milestone delivers CLI and native views of shared engine behavior. Current work: [plan](docs/plan.md).

| Milestone | Outcome |
| --- | --- |
| M0 · Baseline | Browse and search history; [evidence](docs/verification.md) |
| M1 · Complete | Review files and diffs against a chosen parent; [acceptance](docs/verification.md#m1-acceptance--2026-10-20) |
| M2 · Next | Status, file/hunk staging, commits, mutation preconditions |
| M3 | Graph, refs, tags, branch navigation and creation |
| M4 | Clone, fetch, push, tracking, credentials, progress |
| M5 | Merge, conflicts, cherry-pick, revert, stash, recovery |
| M6 | Rebase, reorder, squash, amend, reflog recovery |
| M7 | Worktrees, submodules, LFS, signing, file history, blame, image diffs |

Target Linux (Wayland/X11), macOS, and Windows on x86_64 and arm64, supported by native builds and hardware checks.

Passing main builds publish versioned releases; [delivery policy](docs/development.md). Supported preview releases require browser performance and platform runtime acceptance. Daily-use releases also require M2, native packaging, and mutation recovery documentation. Check keyboard access, accessibility, scaling, clipboard, and performance at each milestone. Outstanding gates: [verification](docs/verification.md#preview-acceptance-pending).
