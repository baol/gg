# gg · good git

Purpose: introduce gg and explain installation and use.

A CLI-first Git browser with a native Rust interface.

Download binary archives from [GitHub Releases](https://github.com/baol/gg/releases) when available, or from the Rust workflow artifacts. Builds cover Linux, macOS, and Windows on x86_64 and arm64. Verify the SHA-256 sidecar, extract the archive, and place `git-good` (`git-good.exe` on Windows) on your PATH. See [platform requirements and release gates](docs/development.md).

Build from source:

```sh
cargo install --path apps/git-good --locked
cd /path/to/repository
git good
git good history
git good status
git good status --file src/main.rs --area unstaged
git good stage --file src/main.rs
git good stage --file src/main.rs --hunk '@@ -10,3 +10,4 @@'
git good show HEAD
git good show HEAD --file src/main.rs
git good show HEAD --file assets/logo.png --export after --output /tmp/logo.png
```

`git good` (or `git-good`) opens the containing repository, including from nested directories. Supply a path to open another repository.

Stream, filter, and inspect history; copy commit IDs. `git good history` streams JSON for scripts. Select a commit and press **Enter** to browse its changed files; **Back to history** restores your position. `git good show HEAD [--repo PATH] [--parent N]` streams the same file summaries as JSON. For merges, choose a parent in Review. Binary files show PNG/JPEG previews or a hexadecimal sample. Use **Export before/after** to save complete content to a new file. Select a file to read its diff; **Enter** opens a full-width reading view and **Escape** returns to files. Add `--file PATH` to output that diff as JSON. Parents are one-based; the default is the first parent, or the empty tree for a root commit.

Choose **Status** to inspect staged, unstaged, untracked, and conflicted paths. **Refresh** rescans the worktree; **Back** restores history. `git good status [PATH]` streams the same entries as JSON. Open a row with **View diff** or Enter; **Back to files** restores its position. Select a row and choose **Stage file** to record its current working content, including additions and deletions. In an unstaged text diff, click a hunk header and choose **Stage hunk** to record only that hunk; omitted hunks remain unstaged. Add `--file PATH --area staged|unstaged|untracked` for a selected-file comparison.

[Contributing](CONTRIBUTING.md) · [Development](docs/development.md) · [Architecture](docs/architecture.md) · [Design](docs/design.md) · [Plan](docs/plan.md) · [Roadmap](ROADMAP.md) · [Regression coverage](docs/regression.md) · [Verification](docs/verification.md)

Inspiration: [Fork](https://fork.dev/) workflows and PhotoCraft's [guide](https://github.com/storytold/photocraft/blob/main/AGENTS.md) and [architecture](https://github.com/storytold/photocraft/blob/main/docs/architecture.md).

Apache-2.0 · [LICENSE](LICENSE)

In Status, **Stage file** records the selected working path in the index at action time; **Unstage file** restores a staged row from HEAD. Both preserve working edits and unrelated staging. **Stage hunk** records selected hunks of an unstaged text modification and leaves other hunks visible. CLI: `git good stage --file PATH [--hunk HEADER]... [--repo PATH]` and `git good unstage --file PATH [--repo PATH]`. Repeat `--hunk` with exact `@@` headers from `git good status --file PATH --area unstaged` to stage multiple hunks.

Use **Repository** for Open/Refresh and **View** or Space for commit details. Arrows and j/k navigate lists; in reading views they scroll content. Page keys move a viewport; Enter/Right opens the selection and Escape/Left returns. Text editors retain their keys.
