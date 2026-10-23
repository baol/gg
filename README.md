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
git good fetch [REMOTE] [--prune] [--repo PATH]
git good push [REMOTE] [BRANCH] [--set-upstream] [--repo PATH]
git good clone URL [PATH]
git good stash list
git good stash push -m MESSAGE [--include-untracked] [--keep-index]
git good stash apply|pop [STASH] [--index]
git good stash drop [STASH]
git good cherry-pick COMMIT
git good revert COMMIT
git good cherry-pick|revert --continue|--abort|--skip
git good merge REF [--no-ff]
git good merge --continue|--abort
git good conflicts
git good conflict --file PATH [--use ours|theirs|--stage]
git good commit --amend -m MESSAGE
git good reflog [REF]
git good reset --soft|--mixed|--hard COMMIT
git good branch --delete BRANCH [--force]
git good branch --create NAME [--at REF]
git good tag --delete TAG
```

`git good` (or `git-good`) opens the containing repository, including from nested directories. Supply a path to open another repository.

Stream, filter, and inspect history; copy commit IDs. The graph includes commits reachable from HEAD and every local branch, remote branch, and tag. The history view draws commit lanes and Fork-style branch, remote, and tag pills before the subject. Click a pill to select its commit; double-click a branch pill to switch to that branch, or a tag/remote pill to jump to its commit. At wide widths a refs sidebar groups branches, remotes, and tags with filtering and a HEAD check; the **Refs** menu remains the narrow-window fallback. Both mark the current branch, switch local branches after a clean-worktree check, create a new branch from HEAD, and jump to tags or remotes. Read-only ref, remote, and tracking labels are also available as `git good refs [PATH]` JSON. Select a commit and press **Enter** to browse its changed files; **Back to history** restores your position. `git good show HEAD [--repo PATH] [--parent N]` streams the same file summaries as JSON. For merges, choose a parent in Review. Binary files show PNG/JPEG previews or a hexadecimal sample. Use **Export before/after** to save complete content to a new file. Select a file to read its diff; **Enter** opens a full-width reading view and **Escape** returns to files. Add `--file PATH` to output that diff as JSON. Parents are one-based; the default is the first parent, or the empty tree for a root commit.

Tracking and network actions stay explicit. The refs sidebar and status line show the current branch's upstream and ahead/behind counts; choose **Track remote branch…** from **Refs** to create a local tracking branch after a clean-worktree check. The **Repository** menu offers **Fetch**, **Push**, and **Clone…**. Fetch updates remote-tracking refs only; Push sends the current branch without force and records upstream when it is absent; Clone opens the new repository only after Git succeeds. Operations run one at a time, stream bounded progress in the status line, and can be cancelled. Authentication and transport use your Git configuration, credential helpers, and SSH agent; failures are actionable and never print credentials. CLI receipts are JSON on stdout and progress goes to stderr.

Use **Stash** to park work without committing. **Stash changes…** records a message and can include untracked files or keep staged content in the index; **Manage stashes…** lists entries with Apply, Pop, and a confirmed Drop. Conflicting applies keep the entry, list the unmerged paths in the status line, and leave the conflict markers for recovery. The CLI exposes the same operations with JSON receipts: `git good stash list`, `git good stash push`, `git good stash apply`, `git good stash pop`, and `git good stash drop`.

Use the **Commit** menu to cherry-pick or revert the selected history commit. A conflicting operation leaves an in-progress banner with **Continue**, **Skip**, and **Abort**; the status line names the operation and conflict count. The CLI mirrors the same flow with `git good cherry-pick COMMIT`, `git good revert COMMIT`, and their `--continue`, `--skip`, and `--abort` recovery forms.

Use **Refs → Merge branch…** to merge a local branch, remote-tracking ref, or tag. Fast-forward and already-up-to-date merges complete directly; diverged histories create a merge commit, and **Always create a merge commit** forces one. A conflicting merge leaves the in-progress banner with **Continue** and **Abort** (Skip is hidden because Git does not support it), and the status line reports the conflict count. CLI: `git good merge REF [--no-ff]` and `git good merge --continue|--abort`.

While an operation is in progress, choose **Resolve conflicts…** to see each unmerged path with Base, Ours, and Theirs content. Select a side, stage manually resolved content, then Continue or Abort from the banner. Recovery refreshes history, refs, and an open Status view. CLI: `git good conflicts`, `git good conflict --file PATH`, `git good conflict --file PATH --use ours|theirs`, and `git good conflict --file PATH --stage`.

Right-click a history row for **Cherry-pick**, **Revert**, **Reset here** (Soft, Mixed, or Hard), **Compare to local**, **Copy hash**, and **Copy subject**. Reset here confirms first; Hard warns that staged and working changes are discarded. Compare to local opens the selected commit against the current branch tip. CLI: `git good reset --soft|--mixed|--hard COMMIT` and `git good show <commit> --against REF`.

Right-click a ref in the sidebar for its actions. Branches offer switch, merge into the current branch, delete (safe first, then a confirmed force delete), and copy name/hash. Tags offer jump, merge, delete, and copy. Remotes offer tracking-branch creation, merge, and copy. CLI: `git good branch --delete BRANCH [--force]`, `git good branch --create NAME [--at REF]`, and `git good tag --delete TAG`.

Rewrite the latest commit with the **Amend** toggle in the Status commit bar or **Commit → Amend last commit…**. Amend applies the staged index and the edited message; an empty index rewrites the message only. The original author is preserved, Git hooks run in order (including `post-rewrite`), and the previous tip is saved under a private `refs/gg/backup/...` ref that stays out of the refs sidebar. CLI: `git good commit --amend -m MESSAGE`.

Use **Repository → Reflog…** to inspect `HEAD` or a local branch newest-first. Each entry shows its selector, commit, time, action, and detail. **Reset branch here** offers Soft, Mixed, and Hard with confirmation and the Hard warning; **Create branch here…** recovers a tip without switching; **Copy hash** writes the exact ID. Listing is read-only. CLI: `git good reflog [REF]`.

Choose **Status** to inspect staged, unstaged, untracked, and conflicted paths. **Refresh** rescans the worktree; **Back** restores history. `git good status [PATH]` streams the same entries as JSON.

Open a row with **View diff** or Enter to review both sides of the path together. The **Staged** pane compares HEAD with the index; the **Unstaged** pane compares the index with the working file. They sit side by side at wide widths and stack on narrow screens. Each pane offers **Stage all** or **Unstage all**, and the unstaged pane also offers **Drop all**. Use the inline **Stage hunk**/**Unstage hunk** action to apply a whole hunk, click a hunk header to select all of its changed lines, or click individual added/deleted lines to select only those lines. **Drop hunk** or **Drop selected** discards working changes after confirmation, leaving the index untouched. Successful actions refresh both panes in place, and **Back to files** returns to a refreshed status list. The Status view includes a commit message field; **Commit** writes the index and runs the repository's Git hooks, while **Amend** rewrites HEAD with the staged index and message. Hook failures abort before HEAD changes and refresh the status so recovery is visible. Add `--file PATH --area staged|unstaged|untracked` for a selected-file comparison.

[Contributing](CONTRIBUTING.md) · [Development](docs/development.md) · [Architecture](docs/architecture.md) · [Design](docs/design.md) · [Plan](docs/plan.md) · [Roadmap](ROADMAP.md) · [Regression coverage](docs/regression.md) · [Verification](docs/verification.md)

Inspiration: [Fork](https://fork.dev/) workflows and PhotoCraft's [guide](https://github.com/storytold/photocraft/blob/main/AGENTS.md) and [architecture](https://github.com/storytold/photocraft/blob/main/docs/architecture.md).

Apache-2.0 · [LICENSE](LICENSE)

In Status, **Stage file** records the selected working path in the index at action time; **Unstage file** restores a staged row from HEAD. Both preserve working edits and unrelated staging. Inside **Review & stage**, the inline hunk and line actions stage or unstage only the selected changes; whole-file actions remain available for modes, type changes, symlinks, untracked additions, and deletions. **Drop** discards working changes only: tracked content is restored from the index, untracked files are removed, and the index is never modified. CLI: `git good stage --file PATH [--hunk HEADER]... [--repo PATH]`, `git good unstage --file PATH [--repo PATH]`, `git good commit -m MESSAGE [--repo PATH]`, `git good commit --amend -m MESSAGE [--repo PATH]`, `git good reflog [REF] [--repo PATH]`, `git good branch --create NAME [--at REF] [--repo PATH]`, and `git good switch [--create] BRANCH [--repo PATH]`. Commit runs `pre-commit`, `prepare-commit-msg`, `commit-msg`, and `post-commit` hooks; a pre-commit or message-hook failure leaves the index and working tree intact and keeps the message for retry. A failing post-commit hook reports a warning after the commit exists. Repeat `--hunk` with exact `@@` headers from `git good status --file PATH --area unstaged` to stage multiple hunks.

Use **Repository** for Open, Refresh, Reflog, Fetch, Push, and Clone, **Stash** for stash actions, **Commit** for amend, cherry-pick, and revert, **Refs** for branch actions and merge, and **View** or Space for commit details. Arrows and j/k navigate lists; in reading views they scroll content. Page keys move a viewport; Enter/Right opens the selection and Escape/Left returns. Text editors retain their keys.
