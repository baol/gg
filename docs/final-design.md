# Final interface design

Purpose: define gg's window frame, command placement, interactive rebase, rebase stops, and Preferences as agreed in the 2026-10-30 design review.

Status: agreed design. Delivery order and acceptance live in the [plan](plan.md#final-interface--staged); shipped behavior lives in [architecture](architecture.md). This document extends the [coherent interaction model](ui-coherence.md) and keeps its rules: one command definition per action, read-only ref selection, one operation area, one file workspace.

The reviewed mockups are the **Final design** page of the gg App Flows design canvas (claude.ai artifact); its **Exploration** page keeps the rejected variants.

## Window frame

The frame has two rows above every view. Both rows keep the same order and position in every view and at every width.

1. **Menu bar**, 26 px: **Repository · Branch · Commit · Changes · View · Help**, then the window identity right-aligned (repository, path, and operation state while one runs).
2. **Toolbar**, 34 px, left to right:
   - Refs sidebar toggle (`Ctrl+\`), pressed while the sidebar is shown.
   - **Changes | History** tabs. Changes is always present; its count badge appears only when there are local changes.
   - Fetch · Pull (rebase) · Push. Pull is enabled only when the current branch is behind its upstream and shows the behind count as a badge; otherwise it is disabled with the reason.
   - New branch · New tag · Merge into current.
   - Stash changes (disabled on a clean worktree) · Stashes (picker with count and caret; disabled when there are none).
   - Interactive rebase of the current branch (the commits its upstream lacks, else the commits the default branch lacks; independent of the selection) · Recovery. **Commit › Interactive rebase from here…** starts at the selected commit.
   - Right-aligned: current-branch button (`name ↑ahead ↓behind`, opens the branch switcher) and the search field (`Ctrl+K`).

Dialogs that ask for a decision or start a change are modal; tool and information windows are not. Toolbar buttons are icon-only Phosphor glyphs with tooltips naming the action and its shortcut. Disabled buttons stay in place with the same disable reason as their menu item.

The old navigation strip is merged into the toolbar; no other row repeats the tabs.

Menus are egui's in-window menu bar on every platform. gg does not create native OS menus (the macOS menu bar and KDE's global menu stay empty) and uses no Tauri or Tauri-ecosystem crates such as `muda`.

## Command placement

Every command has exactly one menu home. The toolbar, shortcuts, context menus, and search point back to it and share its readiness reason and confirmation.

| Menu | Commands |
| --- | --- |
| gg (macOS only) | Settings… (`Cmd+,`) · Quit (`Cmd+Q`) |
| Repository | Open… (`Ctrl+O`) · Clone… · Open recent ▸ · Fetch (`Ctrl+Shift+F`) · Pull (rebase) (`Ctrl+Shift+L`) · Push (`Ctrl+Shift+P`) · Push with force-with-lease… · Worktrees… · Submodules… · LFS… · Backups… · Reflog… · Preferences… (`Ctrl+,`) · Quit (`Ctrl+Q`) |
| Branch | New branch… (`Ctrl+B`) · New tag… · Switch to… (`Ctrl+Shift+B`) · Merge into current… · Rebase current onto… · Set upstream… · Track remote branch… · Rename… (`F2`) · Delete… (`Del`) |
| Commit | Review changes (`Enter`) · Compare with HEAD · Copy ID (`Ctrl+C`) · Cherry-pick onto HEAD · Revert · Interactive rebase from here… (`R`) · Reword… (`Ctrl+R`) · Reopen for editing… (`E`) · Fixup into… (`F`) · Drop… (`D`) · Reset current branch to here… — acts on the selected commit |
| Changes | Stage all (`Ctrl+Shift+S`) · Unstage all (`Ctrl+Shift+U`) · Discard all… · Commit (`Ctrl+Enter`) · Amend HEAD (`Ctrl+Shift+Enter`) · Stash changes (`Ctrl+Shift+Z`) · Stashes… · Resolve conflicts… |
| View | Local Changes (`Ctrl+1`) · History (`Ctrl+2`) · Refs sidebar (`Ctrl+\`) · Commit inspector (`Space`) · Inline diff · Side-by-side diff · Compact density · Theme ▸ |
| Help | Keyboard shortcuts (`?`) · Documentation · About gg |

The menus come from one command table that search also lists, so a command has one label, shortcut, and disabled reason everywhere. Reword, Reopen, Fixup, and Drop open the interactive rebase with the selected commit preset. There is no Refresh command: the window refreshes when files change and before each command (see the architecture). Still to decide (see the canvas page **Review · as built**): Track remote branch is kept from before the mockups; Submodules…, LFS…, and Side-by-side diff are listed but not built; F10 menu focus is not available in egui.

About gg shows the logo, version, author (people only, from the workspace `authors`), license with a link, source link, and a note that parts of gg were written with AI coding assistants and come with no warranty.

Rules:

- The toolbar holds only frequent commands that need no selection. Commands on a selected commit, file, or hunk stay beside that selection.
- Single-letter shortcuts act on the focused list and never fire while typing. Ctrl and Ctrl+Shift chords are global.
- Unavailable items stay visible and disabled with a reason.

## Search

`Ctrl+K` or the toolbar's Search button opens one search field; `Ctrl+F` opens the same search starting from the current History filter; there is no separate filter row. Results are grouped: **Filter** (filter history by the text, first and default; this is also how commits are found), **Actions** (named with their menu home), **Branches**, **Remotes**, **Tags**, and later **Files**. Search runs commands through the same definitions and readiness as the toolbar and menus.

## Density

Compact density is the default: history and file rows 22 px, diff rows 19 px, path and endpoints on one reader line, one-line commit identity, one-character change markers (`+ ~ − T ? !`) with the full kind in the tooltip. Comfortable density remains a preference.

## Stashes

The Stashes toolbar button opens a picker listing each entry's ref, message, branch, age, and file count. The selected entry offers **Apply**, **Pop**, **Show changes**, and **Drop…** (confirmed). The sidebar lists stash entries; selecting one opens the picker on it.

## Interactive rebase

A large modal dialog over dimmed History, laid out like the file workspace.

- **Header (operation bar):** branch, range (`<from>…HEAD onto <base>`), result count (`6 commits → 4 · 1 dropped`), **Cancel** (`Esc`), **Start rebase** (`Ctrl+Enter`).
- **Left, the plan:** newest first, matching History. Each row has a drag handle, move up/down, hash, message, ref pills, a consequence badge (`↓ fixup into <hash>`, `absorbs <hash>`, `⏸ stops here · reopens staged in Changes`, `dropped · recoverable from the backup`), and an action chip. The chip shows the current action; clicking opens a menu of all six actions with a plain hint and key. Keys on the selected row: **P** pick, **R** reword, **E** reopen, **S** squash, **F** fixup, **D** drop.
- **Squash and Fixup** combine into the nearest kept commit **below** (older). Squash combines both messages; Fixup keeps only the older message. They are disabled with a reason when no kept commit lies below.
- **Right, the selected commit:** hash, action, author, date, files, and diff in the shared reader. Reword edits the message here, showing the original. Drop dims the diff. **Open in Review** (`Enter`) opens the normal review; `Esc` returns to the plan unchanged.
- **Footer:** stash working changes first, which refs stay on the original commits, and the private-backup notice.

Merge commits remain unsupported and are named as the reason.

## Rebase stops

Both stops use the operation area under the toolbar. Its first line names the stop and holds **Continue** (`Ctrl+Enter`), **Skip commit**, and **Abort and restore**; the second line carries the explanation and the step chips (done ✓, current, next). The branch button and window identity show the rebase step.

- **Reopened commit (E):** Local Changes opens with the commit's changes staged and its message in the commit box. Unstaging files or lines and committing splits the commit; each part is listed under **Commits made at this stop**. Continue is highlighted once nothing remains.
- **Conflict:** the conflicted files list and a three-column block per conflict: **Base**, **Ours · rewritten branch**, **Theirs · replayed commit**. Choices per block: Ours, Theirs, Ours + theirs, Theirs + ours (keys 1–4). The result preview shows unresolved markers. **Stage resolution** is enabled when every block in the file has a choice. Continue stays disabled with a reason until every file is staged.

## Preferences

**Repository › Preferences…** opens a non-blocking window; changes apply immediately and persist under the OS configuration directory. Sections: General, Appearance, Diff, Commit, Rebase & recovery, Keyboard.

- **Clone folders** (General): an ordered list of parent folders. One radio marks the **preferred** folder used by Clone by default. Each row shows its repository count or a missing-folder state; rows reorder and remove. Adding validates the path (`Folder is empty.`, `Folder must be an absolute path.`, `Already in the list.`). Removing the preferred folder makes the first one preferred.
- The remaining sections follow the [Settings panel](plan.md#settings-panel--planned) plan.
