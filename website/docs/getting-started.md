---
title: Get started
description: Install Git Good, open a repository and learn the layout of the window.
---

# Get started

Git Good is a single binary, `git-good`. Put it on your `PATH` and Git runs it for `git good`.

## Install

=== "Linux and macOS"

    ```sh
    curl -fsSL https://raw.githubusercontent.com/baol/gg/main/scripts/install.sh | sh
    ```

    The script picks the build for your system and processor (`x86_64` or `arm64` on Linux, Apple silicon on macOS; on an Intel Mac it says to use `cargo install git-good --locked` instead), checks it against its published SHA-256 and puts `git-good` in `~/.local/bin`. If that folder is not on your `PATH` yet, it prints the line to add. Run it again to update.

=== "Windows"

    In PowerShell:

    ```powershell
    irm https://raw.githubusercontent.com/baol/gg/main/scripts/install.ps1 | iex
    ```

    The script picks the `x86_64` or `arm64` build, checks it against its SHA-256, puts `git-good.exe` in `%LOCALAPPDATA%\Programs\git-good` and adds that folder to your user `PATH`; open a new terminal afterwards. Run it again to update.

=== "Cargo"

    The prebuilt binary through [cargo-binstall](https://github.com/cargo-bins/cargo-binstall), or a build from source (Rust 1.95 or newer):

    ```sh
    cargo binstall git-good
    cargo install git-good --locked
    ```

=== "By hand"

    1. Download the archive for your platform from [GitHub Releases](https://github.com/baol/gg/releases). Builds cover Linux and Windows, each on `x86_64` and `arm64`, and macOS on Apple silicon; each has a `.sha256` file beside it.
    2. Extract `git-good` (`git-good.exe` on Windows) into a folder on your `PATH`, such as `~/.local/bin`.
    3. To update, replace the binary with the new release.

Both scripts install nothing when the download does not match its checksum. `GG_VERSION=0.1.90` installs a given release, and `GG_INSTALL_DIR` another folder. You can read them first: [install.sh](https://github.com/baol/gg/blob/main/scripts/install.sh), [install.ps1](https://github.com/baol/gg/blob/main/scripts/install.ps1).

Git Good runs the `git` on your `PATH`, so your configuration, hooks, credential helpers and signing setup all apply. Commit hooks need Git 2.36 or newer.

## Open a repository

!!! tip "First time with Git?"
    Git records your name and email in every commit. The welcome screen shows who you commit as; when Git does not know yet, click **Set name and email…** (or open **Preferences › Commit**), fill in **Name** and **Email** and click **Save to Git**. It is the same as `git config --global user.name` and `user.email`.

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

![The History tree of acme-api: the toolbar at the top, refs on the left, the commit list with lanes in the middle and the selected commit's details at the bottom](assets/screens/history.png){ .gg-shot }

From top to bottom:

Title and menu bar
:   **Repository**, **Branch**, **Commit**, **Changes**, **View** and **Help**. Every command lives in a menu and shows its key there.

Toolbar
:   The sidebar toggle, **Local Changes** (with the number of changed files) and **History tree**. Then Fetch, Pull and Push; the Pull button shows how many commits you are behind. Then New branch, Stash changes, the stash list, Interactive rebase and Recovery. On the right, the **current branch** button (`main ↑5 ↓1`: five commits to push, one to pull) opens a filterable branch switcher, and the search box (++ctrl+k++) finds commits, branches, files and commands.

Refs sidebar
:   Branches grouped by folder (`feature/`, `fix/`), remotes, tags and stashes. Type in **Filter refs** to narrow it. Double-click a branch to switch to it.

History tree
:   One row per commit with its lane, pills for the branches and tags on it, author, short hash and date.

Details
:   The selected commit's subject, author, parent and changed-line count, with **Review** (++enter++) and **Actions** (++alt+a++).

Footer
:   The working-tree summary on the left and the keys for the focused view. Results of commands ("Rewrote 3 commits · backup …") also appear here.

## Next

- [Workflows](workflows/index.md) walks through each task with screenshots.
- [Real use cases](use-cases.md) shows how the pieces fit together on a working day.
- [Scripting with the CLI](cli.md) covers `git good --cli` and its JSON output.
