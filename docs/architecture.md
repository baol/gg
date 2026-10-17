# Architecture

Purpose: define system boundaries, dependency direction, and runtime contracts.

## Layers

| Layer | Crate | Responsibility |
| --- | --- | --- |
| 0 | `gg-engine` | Typed commands, serializable snapshots, filtering |
| 1 | `gg-git` | Native repository discovery and object access through gitoxide |
| 2 | `git-good` | CLI dispatch, egui/eframe UI on wgpu, worker lifecycle |

Dependencies point downward. The engine stays independent of Git libraries, UI, and platform services. CLI and UI share engine commands; view state belongs to the shell.

## Launch contract

Both interactive and headless commands resolve the repository from the process working directory by default. Resolve an explicit path instead when supplied. Discover containing repositories from nested directories. Interactive launch loads history immediately; the path toolbar supports later navigation.

## History contract

`repository.history`: `HistoryRequest { path }` → `History` or an actionable error.

Snapshots carry resolved path, HEAD label, complete commit IDs, parents, author identity, timestamp with offset, and full message. Support ordinary, bare, and linked-worktree repositories. Represent unborn branches as empty history and detached HEAD explicitly.

Traverse all parents reachable from HEAD in gitoxide's default ancestor order. Return up to 500 commits and a `truncated` flag. Filter loaded messages, authors, emails, and IDs; select by complete ID.

## Runtime requirements

- Implement application behavior in safe Rust; propagate fallible operations.
- Preserve objects, refs, index, and working files while browsing.
- Read repositories on a background worker; pass owned snapshots through a channel and request repaint on completion.
- Virtualize commit rows. Retain the last successful snapshot through refresh errors.
- Identify operation and path in errors; support retry.
- Preserve native paths in engine and adapter APIs; use Unicode text for editable UI paths.
- Decode display text with replacement characters where needed; retain exact object identity.
- Provide synchronous JSON output and failure exit status for headless commands.

## Growth

Add typed requests for each workflow. Introduce a command enum with the second repository command. Add pagination and cancellation with expanded traversal, mutation preconditions with staging, and toolkit-independent graph data with branch visualization.
