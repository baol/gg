# gg · good git

![gg native history view](docs/screenshots/gg-native-wide.png)

Purpose: introduce gg and explain installation and use.

A fast, native Git desktop app. Browse history, inspect diffs, stage hunks or lines, commit, and recover rewrites without leaving the UI.

## Install or update

Git runs `git-good` whenever you invoke `git good`, as long as the binary is on your `PATH`.

### Prebuilt binaries

1. Download the archive for your platform (`x86_64` or `arm64` on Linux, macOS, or Windows) from [GitHub Releases](https://github.com/baol/gg/releases).
2. Extract `git-good` (`git-good.exe` on Windows) into a directory on your `PATH` (such as `~/.local/bin` or `/usr/local/bin`).
3. To update, replace the binary in place with the new release.

```sh
# Linux / macOS example (replace TARGET with your platform archive)
curl -L https://github.com/baol/gg/releases/latest/download/git-good-TARGET.tar.gz | tar -xz -C ~/.local/bin git-good
```

### From source

Requires Rust 1.95+. Re-run the same command to update:

```sh
cargo install --git https://github.com/baol/gg git-good --locked --force
```

Or from a local checkout:

```sh
cargo install --path apps/git-good --locked --force
```

## Run

Open the current repository (or pass a path):

```sh
git good
```

## UI overview

- **History and Refs** — Stream and filter commit history with commit lanes and branch, remote, and tag pills. Switch branches, create tracking branches, merge, or rebase from the **Refs** sidebar or menu.
- **Commit review** — Press **Enter** on a commit to inspect changed files, side-by-side or inline diffs, image previews, blame, and file history.
- **Local Changes** — Inspect staged, unstaged, untracked, and conflicted files. Stage, unstage, or discard whole files, hunks, or individual lines, then commit or amend `HEAD`.
- **Rewrites and recovery** — Cherry-pick, revert, reset, or reorder and squash commits to `HEAD`. In-progress operations show a banner with **Resolve conflicts…**, **Continue**, **Skip**, and **Abort**. Automatic private backups (`Repository → Backups…`) and `Repository → Reflog…` let you restore previous branch tips.
- **Sync and Stash** — Fetch, push, and clone from **Repository** with live progress and cancellation, or park work from **Stash**.
- **Keyboard navigation** — Navigate with arrows or `j`/`k`, toggle details with `Space`, open commit actions with `Alt+A`, and view all shortcuts under **Help**.

## Documentation

[CLI reference](docs/cli.md) · [Contributing](CONTRIBUTING.md) · [Development](docs/development.md) · [Architecture](docs/architecture.md) · [Design](docs/design.md)

Apache-2.0 · [LICENSE](LICENSE)
