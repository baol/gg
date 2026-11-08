---
title: Get started
description: Install Git Good, open a repository and learn the layout of the window.
---

# Get started

Git Good is a single binary, `git-good`. Put it on your `PATH` and Git runs it for `git good`.

## Install

=== "Prebuilt binary"

    1. Download the archive for your platform from [GitHub Releases](https://github.com/baol/gg/releases). Builds cover Linux, macOS and Windows, each on `x86_64` and `arm64`.
    2. Extract `git-good` (`git-good.exe` on Windows) into a folder on your `PATH`, such as `~/.local/bin`.
    3. To update, replace the binary with the new release.

    ```sh
    # Linux / macOS (replace TARGET with your platform's archive name)
    curl -L https://github.com/baol/gg/releases/latest/download/git-good-TARGET.tar.gz \
      | tar -xz -C ~/.local/bin git-good
    ```

=== "From source"

    You need Rust 1.95 or newer. Run the same command again to update.

    ```sh
    cargo install --git https://github.com/baol/gg git-good --locked --force
    ```

Git Good runs the `git` on your `PATH`, so your configuration, hooks, credential helpers and signing setup all apply. Commit hooks need Git 2.36 or newer.

## Open a repository

From a terminal inside a repository:

```sh
git good
```

Or pass a folder: `git good ~/src/acme-api`. Inside a subfolder of a repository, that repository opens. Anywhere else, Git Good opens on the welcome view:

![The welcome view: Open, Clone and New repository buttons above a list of recent repositories with their branch and last-opened date](assets/screens/welcome.png){ .gg-shot }

- **Open…** (++ctrl+o++) picks a folder. You can also drop a folder anywhere on the window.
- **Clone…** clones a URL into your preferred clone folder (set it in Preferences › General).
- **New repository…** runs `git init` in a folder.
- **Recent** lists the repositories you opened, newest first, with their current branch. Use the arrow keys and ++enter++; ++delete++ removes an entry from the list, not from disk.

## The window

![History of acme-api: the toolbar at the top, refs on the left, the commit list with lanes in the middle and the selected commit's details at the bottom](assets/screens/history.png){ .gg-shot }

From top to bottom:

Title and menu bar
:   **Repository**, **Branch**, **Commit**, **Changes**, **View** and **Help**. Every command lives in a menu and shows its key there.

Toolbar
:   The sidebar toggle, **Changes** (with the number of changed files) and **History**. Then Fetch, Pull and Push; the Pull button shows how many commits you are behind. Then New branch, Stash changes, the stash list, Interactive rebase and Recovery. On the right, the **current branch** button (`main ↑5 ↓1`: five commits to push, one to pull) opens a filterable branch switcher, and the search box (++ctrl+k++) finds commits, branches, files and commands.

Refs sidebar
:   Branches grouped by folder (`feature/`, `fix/`), remotes, tags and stashes. Type in **Filter refs** to narrow it. Double-click a branch to switch to it.

History
:   One row per commit with its lane, pills for the branches and tags on it, author, short hash and date.

Details
:   The selected commit's subject, author, parent and changed-line count, with **Review** (++enter++) and **Actions** (++alt+a++).

Footer
:   The working-tree summary on the left and the keys for the focused view. Results of commands ("Rewrote 3 commits · backup …") also appear here.

## Next

- [Workflows](workflows/index.md) walks through each task with screenshots.
- [Real use cases](use-cases.md) shows how the pieces fit together on a working day.
- [Scripting with the CLI](cli.md) covers `git good --cli` and its JSON output.
