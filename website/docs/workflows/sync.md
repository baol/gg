---
title: Branches, fetch, pull and push
description: Switch and create branches and keep them in sync with the remote.
---

# Branches, fetch, pull and push

## Where you stand

The current branch button in the toolbar reads `main ↑5 ↓1`: five local commits to push and one remote commit to pull. The Pull button carries the same *1*.

![The toolbar and sidebar: main is 5 ahead and 1 behind origin/main, the Pull button shows 1](../assets/screens/history.png){ .gg-shot }

## Switch and create branches

- Click the **current branch** button (or ++ctrl+shift+b++) to open the switcher; type to filter and press ++enter++.
- Double-click a branch in the sidebar or a pill in History to switch to it. A remote branch switches to its local branch, creating one that tracks it if needed.
- **New branch** (++ctrl+b++) creates a branch at the selected commit, and can switch to it.
- **Rename** (++f2++) and **Delete** (++delete++) work on the selected branch.

If your working changes would be overwritten, the switch stops before changing anything and offers **Stash and switch…**: the stash is kept, so you can bring the changes back on either branch.

## Fetch, pull and push

| Button | Key | Does |
|---|---|---|
| Fetch | ++ctrl+shift+f++ | Updates remote branches. Your branches and files are not touched. |
| Pull | ++ctrl+shift+l++ | Fetches and integrates the upstream: rebase by default; merge or fast-forward only in Preferences › General. |
| Push | ++ctrl+shift+p++ | Pushes the current branch, setting its upstream on the first push. |

Progress shows in the footer, and a long transfer can be cancelled. A rejected push explains why and offers **Fetch**, so you can look at the new commits before integrating them. Git Good never fetches, merges or rebases on its own.

**Force push with lease** is a separate command in the Repository menu. It names the branch, the remote and the commit it expects there, and fails without changing anything if the remote moved. A plain force push is never offered.

Credentials come from your Git setup: credential helpers, SSH agents and `~/.ssh/config`.
