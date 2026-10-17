# Verification

Purpose: record completed checks and observed results.

2026-10-17 · Linux x86_64 · Rust 1.94

## M0 evidence

- Workspace build succeeds with the locked dependency graph.
- Formatting and Clippy pass with warnings treated as errors.
- Eight tests cover filtering, UI states, graphics backend configuration, discovery, unborn branches, invalid paths, missing objects, packed history, detached HEAD, merge parents, bare repositories, linked worktrees, and the 500-commit cap.
- Adapter history IDs match `git rev-list HEAD` on the packed linear fixture.
- The native app opens on an isolated X11 display and exits successfully after a rendered screenshot.
- Visual inspection confirms the repository toolbar, selected commit row, full ID, author, formatted date, parents, message, and repository status fit the 1120 × 740 window.
- The JSON command returns the same three-commit fixture shown in the desktop capture.

Platform release gates: [roadmap](../ROADMAP.md).
