---
title: Search, keys and preferences
description: Run any command from search, change keyboard shortcuts and set preferences.
---

# Search, keys and preferences

## One search box for everything

Press ++ctrl+k++ and type. Search finds commits, branches, files and **every menu command**, with the menu it lives in and its key:

![Search for "rebase": filter the History tree, rebase onto, pull with rebase, interactive rebase of the current branch, interactive rebase from here](../assets/screens/search.png){ .gg-shot }

++up++ / ++down++ choose, ++enter++ runs, ++escape++ closes.

## Keyboard first

- ++f6++ moves between search, the sidebar and the content; ++shift+f6++ goes back.
- ++f10++ opens the menu bar; ++shift+f10++ or the context menu key opens the selected item's actions.
- Single letters (++j++, ++k++, ++r++, ++s++…) act on the focused list and never fire while you type in a text box.
- **Help › Keyboard shortcuts** (++question++) lists them all.

## Change the keys

**Repository › Preferences…** (++ctrl+comma++) › **Keyboard**:

![Preferences › Keyboard: a list of commands with their keys, such as Fetch Ctrl+Shift+F and Pull Ctrl+Shift+L, and below it the fixed keys](../assets/screens/preferences.png){ .gg-shot }

Click a command's key and press the new one. A key that is already taken is refused, naming the command that has it, and ++escape++ cancels. The menus show your keys at once. **Fixed keys** at the bottom are the ones that cannot change, such as ++enter++ to review.

## Other preferences

| Section | Settings |
|---|---|
| General | Clone folders and the preferred one, prune deleted remote branches on fetch, the editor to open files in, dialog defaults (stash untracked files, merge with `--no-ff`, switch to a new branch) |
| Appearance | Theme (system, dark, light), compact or comfortable rows, zoom, code font size, title bar style, avatars in the History tree |
| Diff | Default layout (inline or side by side), context lines, whitespace, wrapping |
| Commit | Identity, subject length guide, run hooks, sign commits and tags |
| Rebase & recovery | Stash before a rebase, confirm Drop and Reset, pull strategy (rebase, merge, fast-forward only), offer force-with-lease after a rewrite, how long to keep private backups |

Changes apply immediately and are saved to Git Good's settings file in your configuration folder (`~/.config/gg/` on Linux).
