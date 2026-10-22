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
git good refs
git good switch feature-x
git good switch --create feature-y
git good status
git good status --file src/main.rs --area unstaged
git good stage --file src/main.rs
git good stage --file src/main.rs --hunk '@@ -10,3 +10,4 @@'
git good show HEAD
git good show HEAD --file src/main.rs
git good show HEAD --file assets/logo.png --export after --output /tmp/logo.png
```

`git good` (or `git-good`) opens the containing repository, including from nested directories. Supply a path to open another repository.

Stream, filter, and inspect history; copy commit IDs. The graph includes commits reachable from HEAD and every local branch, remote branch, and tag. The history view draws commit lanes and Fork-style branch, remote, and tag pills before the subject. Click a pill to select its commit; double-click a branch pill to switch to that branch, or a tag/remote pill to jump to its commit. At wide widths a refs sidebar groups branches, remotes, and tags with filtering and a HEAD check; the **Refs** menu remains the narrow-window fallback. Both mark the current branch, switch local branches after a clean-worktree check, create a new branch from HEAD, and jump to tags or remotes. Read-only ref labels are also available as `git good refs [PATH]` JSON. Select a commit and press **Enter** to browse its changed files; **Back to history** restores your position. `git good show HEAD [--repo PATH] [--parent N]` streams the same file summaries as JSON. For merges, choose a parent in Review. Binary files show PNG/JPEG previews or a hexadecimal sample. Use **Export before/after** to save complete content to a new file. Select a file to read its diff; **Enter** opens a full-width reading view and **Escape** returns to files. Add `--file PATH` to output that diff as JSON. Parents are one-based; the default is the first parent, or the empty tree for a root commit.

Choose **Status** to inspect staged, unstaged, untracked, and conflicted paths. **Refresh** rescans the worktree; **Back** restores history. `git good status [PATH]` streams the same entries as JSON.

Open a row with **View diff** or Enter to review both sides of the path together. The **Staged** pane compares HEAD with the index; the **Unstaged** pane compares the index with the working file. They sit side by side at wide widths and stack on narrow screens. Each pane offers **Stage all** or **Unstage all**, and the unstaged pane also offers **Drop all**. Use the inline **Stage hunk**/**Unstage hunk** action to apply a whole hunk, click a hunk header to select all of its changed lines, or click individual added/deleted lines to select only those lines. **Drop hunk** or **Drop selected** discards working changes after confirmation, leaving the index untouched. Successful actions refresh both panes in place, and **Back to files** returns to a refreshed status list. The Status view includes a commit message field; **Commit** writes the index and runs the repository's Git hooks. Hook failures abort before HEAD changes and refresh the status so recovery is visible. Add `--file PATH --area staged|unstaged|untracked` for a selected-file comparison.

[Contributing](CONTRIBUTING.md) · [Development](docs/development.md) · [Architecture](docs/architecture.md) · [Design](docs/design.md) · [Plan](docs/plan.md) · [Roadmap](ROADMAP.md) · [Regression coverage](docs/regression.md) · [Verification](docs/verification.md)

Inspiration: [Fork](https://fork.dev/) workflows and PhotoCraft's [guide](https://github.com/storytold/photocraft/blob/main/AGENTS.md) and [architecture](https://github.com/storytold/photocraft/blob/main/docs/architecture.md).

Apache-2.0 · [LICENSE](LICENSE)

In Status, **Stage file** records the selected working path in the index at action time; **Unstage file** restores a staged row from HEAD. Both preserve working edits and unrelated staging. Inside **Review & stage**, the inline hunk and line actions stage or unstage only the selected changes; whole-file actions remain available for modes, type changes, symlinks, untracked additions, and deletions. **Drop** discards working changes only: tracked content is restored from the index, untracked files are removed, and the index is never modified. CLI: `git good stage --file PATH [--hunk HEADER]... [--repo PATH]`, `git good unstage --file PATH [--repo PATH]`, `git good commit -m MESSAGE [--repo PATH]`, and `git good switch [--create] BRANCH [--repo PATH]`. Commit runs `pre-commit`, `prepare-commit-msg`, `commit-msg`, and `post-commit` hooks; a pre-commit or message-hook failure leaves the index and working tree intact and keeps the message for retry. A failing post-commit hook reports a warning after the commit exists. Repeat `--hunk` with exact `@@` headers from `git good status --file PATH --area unstaged` to stage multiple hunks.

Use **Repository** for Open/Refresh and **View** or Space for commit details. Arrows and j/k navigate lists; in reading views they scroll content. Page keys move a viewport; Enter/Right opens the selection and Escape/Left returns. Text editors retain their keys.
