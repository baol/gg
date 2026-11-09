---
title: Find your way in the History tree
description: Browse, filter and search the History tree, and inspect a commit.
---

# Find your way in the History tree

The History tree is the view Git Good opens on. It streams the log in pages: the first rows appear in about 150 ms even on a 100,000-commit repository, and you can scroll while the rest loads.

![The History tree of acme-api with the feature/rate-limit branch selected at the top](../assets/screens/history.png){ .gg-shot }

## Read the graph

- Each row is a commit. The lane on the left shows where branches split and merge: here `fix/timeout` was merged into `main` with a merge commit.
- Pills name the branches, remote branches and tags on a commit. `origin/main` sits on *Bump serde to 1.0.214*: your teammate's commit, which you fetched but have not pulled.
- The current branch is bold in the sidebar, with a check mark and its tracking counts (`↑5 ↓1`).

## Move around

| Key | Does |
|---|---|
| ++j++ / ++k++ or arrows | Select the next or previous commit |
| ++space++ | Show or hide the details |
| ++enter++ | [Review](review.md) the selected commit |
| ++alt+a++ | Open the commit's actions: cherry-pick, revert, reword, fixup, drop, rebase from here, reset… |
| ++ctrl+c++ | Copy the commit ID |
| ++ctrl+f++ | Filter the History tree by message, author or ID |

## Narrow it down

**Filter refs** at the top of the sidebar keeps only the branches, remotes and tags that match what you type. Branch folders such as `feature/` and `fix/` fold, and remotes and tags stay collapsed until you open them.

To filter the commits themselves, start typing in search (++ctrl+k++) and choose **Filter the History tree by "…"**. The filter matches the message, the author or the commit ID.

![Search with "rebase" typed: the first choice filters the History tree, the others are rebase commands with their menus and keys](../assets/screens/search.png){ .gg-shot }

## Any window size, either theme

At narrow widths the details fold under the list and the columns shrink; the light theme follows your system or Preferences › Appearance.

<div class="grid" markdown>

![The History tree in the light theme](../assets/screens/history-light.png){ .gg-shot }

![The History tree at 760 pixels wide](../assets/screens/history-narrow.png){ .gg-shot }

</div>
