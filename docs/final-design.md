# Final interface design

Purpose: define Git Good's window frame, command placement, interactive rebase, rebase stops, and Preferences as agreed in the 2026-10-30 design review.

Status: agreed design. Delivery order and acceptance live in the [plan](plan.md#final-interface--staged); shipped behavior lives in [architecture](architecture.md). This document extends the [coherent interaction model](ui-coherence.md) and keeps its rules: one command definition per action, read-only ref selection, one operation area, one file workspace.

The reviewed mockups are the **Final design** page of the gg App Flows design canvas (claude.ai artifact); its **Exploration** page keeps the rejected variants. The canvas mockups still show the earlier toolbar with branch, tag, merge, stash, rebase, and recovery buttons; the toolbar described here supersedes them.

## Window frame

The frame has two rows above every view. Both rows keep the same order and position in every view and at every width.

1. **Menu bar**, 26 px: **Repository · Branch · Commit · Changes · View · Help**, then the window identity (repository, path, and operation state while one runs). On Windows and Linux it is the window's title bar (see *Title bar* below); on macOS the menus are in the screen's menu bar and the window has no menu bar row.
2. **Toolbar**, 34 px, left to right:
   - Refs sidebar toggle (`Ctrl+\`), pressed while the sidebar is shown.
   - **Local Changes | History tree** tabs. Local Changes is always present; its count badge appears only when there are local changes.
   - Fetch · Pull (rebase) · Push. Pull is enabled whenever the current branch has an upstream and shows the cached behind count as a badge; without an upstream it is disabled with the reason. Pull fetches first and reports `Already up to date with origin/main.` when there is nothing to integrate.
   - New branch (`Ctrl+B`).
   - Stash changes (disabled on a clean worktree) · Stashes (count and caret; opens the stash picker under the button; disabled with *No stashes*).
   - Interactive rebase from here (`R`): starts at the selected History tree commit; disabled with the reason when no commit is selected or it cannot be rebased · Recovery (backups and reflog).
   - Right-aligned: current-branch button (`name ↑ahead ↓behind`, opens the branch switcher) and the search field (`Ctrl+K`).

The toolbar holds navigation, sync, and the owner's everyday actions; New tag, Merge into current, and Interactive rebase of the current branch live in their menus and in search. Every toolbar command also has a menu home and a search entry, so it stays reachable at every width, including 720×480.

Dialogs that ask for a decision or start a change are modal; tool and information windows are not. A dialog's buttons show the key that presses them: **Cancel** shows `Esc`, the primary shows `Enter` when Enter runs it (also while it is disabled), and a destructive primary shows no key, because its dialog opens on Cancel. Toolbar buttons are icon-only Phosphor glyphs with tooltips naming the action and its shortcut. Disabled buttons stay in place with the same disable reason as their menu item.

The old navigation strip is merged into the toolbar; no other row repeats the tabs.

Menus are egui's in-window menu bar on Windows and Linux. Every menu and dropdown (menu bar menus, submenus, the branch switcher, the rebase action chip, Commit actions, and every choice list) draws the same 24 px rows: a 14 px check column, the label, then the shortcut. The chosen or active item shows a checkmark in that column; no item gets a selected fill. Rows have no frame until hovered. KDE's global menu stays empty.

**Title bar (Windows and Linux).** The window has no OS decorations: the menu bar is the title bar. The menus start at the left (after any caption buttons there), no logo beside them; the window identity is centred on the window in the free gap between the menus and the caption buttons, slid toward the menus when it does not fit centred and shortened from its end when it does not fit at all. Only that free gap (identity included) moves the window: a primary-button drag starts an OS move and a double-click maximizes or restores; menu titles keep their presses. Minimize, maximize/restore and close caption buttons sit at the end, and in the order, the desktop's layout names: Windows' 46 px full-height buttons at the right with a red Close on hover; GNOME's libadwaita round buttons in its `button-layout` order (default Close only); KDE's Breeze buttons in KWin's order; other desktops' plain round buttons in `gtk-decoration-layout` order. Round buttons also drag the window. Close runs **Quit**. Invisible 8 px zones along the edges (12 px at the corners) resize the window, except while it is maximized or full screen. **Preferences › Appearance › Use the system title bar** (off by default, saved in `settings.json`, applied at the next launch) gives the window its OS title bar and frame back; the menu bar then shows the identity right-aligned and has no caption buttons, drag gap or resize zones.

**macOS menu bar.** macOS keeps its own title bar and window frame. The menus are native, in the screen's menu bar, built from the same command table as the in-window bar (same items, labels, shortcuts, enablement and checks): an app menu **Git Good** (About Git Good · Settings… `⌘,` · Services · Hide Git Good `⌘H` · Hide Others · Show All · Quit Git Good `⌘Q`), then Repository · Branch · Commit · Changes · View, **Window** (Minimize · Zoom · Bring All to Front) and **Help** with the system search. A key equivalent reaches Git Good as the key press it was, so the window's key rules decide (text fields keep ⌘C, ⌘X, ⌘V, ⌘A and ⌘Z, which are not key equivalents; bare keys such as `R` or `Del` are not shown as key equivalents); a click runs the command as an in-window menu click does. A new Open recent entry rebuilds the menu; other changes update it in place. A system key Git Good binds keeps Git Good's meaning: Hide moves to ⌃⌘H and Minimize to ⌃⌘M. The in-window menu bar is hidden; the toolbar is the window's top row. The native menu uses `muda`, a standalone AppKit menu crate; Tauri itself, webviews and other Tauri-ecosystem crates stay out.

## Narrow windows

Accepted 2026-11-01 (canvas **Proposals** page, *Narrow 720 px · proposal*). 720 px is the minimum supported width; 1040 px starts the wide layouts.

- **Toolbar**: every button stays at 720 px; below 1040 px the branch button is an icon (branch and ↑↓ in its tooltip and switcher) and search is a `Ctrl+K` icon when the field does not fit.
- **History tree**: the Author column drops first, then Date below a 600 px list, so subjects never truncate before them; a branch in sync with its upstream shows one pill (`main ⇄ origin`). The inspector stays under the list with **Details** (`Space`), which replaces the list with the commit details: **‹ History tree** (`Esc`), j/k step through commits, a label grid, the message, and FILES (Enter opens Review on the chosen file); **Review** is the primary action.
- **Refs**: below the docked sidebar's 1120 px the sidebar toggle (`Ctrl+\`) opens a 248 px drawer over the dimmed view, focus in its filter; Enter shows the ref in the History tree and closes, Esc closes.
- **Review**: the file list folds behind **‹ Files** `1/3` and the reader takes the full width; its header stays one row (endpoints in the identity row's `vs` picker, Blame and File history in ⋯, `B`/`H` still work); Inline is the default, Side by side stays available; Enter reads a file, `F` or ‹ Files returns, and the list's free space shows the commit message.
- **Local Changes**: groups stay above the reader, fold with ▾/▸ (empty ones start folded), and show at most six rows together; the commit form stays at the bottom with a resizable message box; counts are in the footer; Sign lives in Preferences › Commit.

## Command placement

Every command has exactly one menu home. The toolbar, shortcuts, context menus, and search point back to it and share its readiness reason and confirmation.

| Menu | Commands |
| --- | --- |
| Git Good (macOS only) | Settings… (`Cmd+,`) · Quit (`Cmd+Q`) |
| Repository | Open… (`Ctrl+O`) · Clone… · New repository… · Open recent ▸ · Fetch (`Ctrl+Shift+F`) · Pull (rebase) (`Ctrl+Shift+L`) · Push (`Ctrl+Shift+P`) · Push with force-with-lease… · Worktrees… · Submodules… · LFS… · Recovery… · Preferences… (`Ctrl+,`) · Quit (`Ctrl+Q`) |
| Branch | New branch… (`Ctrl+B`) · New tag… · Switch to… (`Ctrl+Shift+B`) · Merge into current… · Rebase current onto… · Interactive rebase of current branch… · Set upstream… · Track remote branch… · Rename… (`F2`) · Delete… (`Del`) |
| Commit | Review changes (`Enter`) · Compare with HEAD · Copy ID (`Ctrl+C`) · Cherry-pick onto HEAD · Revert · Interactive rebase from here… (`R`) · Reword… (`Ctrl+R`) · Reopen for editing… (`E`) · Fixup into… (`F`) · Drop… (`D`) · Reset current branch to here… — acts on the selected commit |
| Changes | Stage all (`Ctrl+Shift+S`) · Unstage all (`Ctrl+Shift+U`) · Discard all tracked changes… · Commit (`Ctrl+Enter`) · Amend HEAD (`Ctrl+Shift+Enter`) · Stash changes… (`Ctrl+Shift+Z`) · Stashes… · Resolve conflicts… |
| View | Local Changes (`Ctrl+1`) · History tree (`Ctrl+2`) · Refs sidebar (`Ctrl+\`) · Commit details (`Space`) · Inline diff · Side-by-side diff · Compact density · Theme ▸ |
| Help | Keyboard shortcuts (`?`) · Documentation · About Git Good |

The menus come from one command table that search also lists, so a command has one label, shortcut, and disabled reason everywhere. Reword, Reopen, Fixup, and Drop open the interactive rebase with the selected commit preset. The Commit actions popup (inspector **Actions**, row context menu, `Alt+A`) is the same menu with **Copy subject** and **Signature details…** under Copy ID, **Reset current branch to here ▸** as Soft / Mixed / Hard, and one band above disabled mutations giving their shared reason. **Interactive rebase of current branch…** covers the commits its upstream lacks, else the commits the default branch lacks, independent of the selection; **Commit › Interactive rebase from here…** starts at the selected commit. **Recovery…** opens one window whose source selector holds **Backups** and **Reflog**; outcome buttons that offer backups open it on Backups. **Stashes…** opens the stash picker centered under the toolbar; the refs sidebar's stash rows open it on their entry. **Discard all tracked changes…** restores every tracked file, staged and unstaged, to HEAD and keeps untracked files; the per-file **Discard unstaged changes…**, **Discard hunk**, and **Discard N lines** touch only unstaged working changes and leave the index unchanged. There is no Refresh command: the window refreshes when files change and before each command (see the architecture). Still to decide (see the canvas page **Review · as built**): Track remote branch is kept from before the mockups. The in-window menus work from the keyboard (Alt or F10 to the bar, Alt+letter, underlined mnemonics, arrows, Enter, Esc) and scroll when taller than the window, as the [architecture](architecture.md#browser-state-and-keyboard) specifies.

Keyboard shortcuts is one filterable key | action table generated from the command table, grouped by menu and then by list scope (History tree, Local Changes list, Review, Rebase list, Conflicts, Stashes); it is non-modal, `Esc` closes it, and **Edit keys…** opens Preferences › Keyboard. A footer **Details…** window is titled by the operation, shows one plain summary line, then Git's output in monospace, with the footer's same recovery action as its primary.

About Git Good uses the dialog frame and shows the logo, version, commit, build date and target, author (people only, from the workspace `authors`), license with a link, source link, and a note that parts of Git Good were written with AI coding assistants and come with no warranty; **Copy version info** copies the four build lines as plain text.

Rules:

- The toolbar holds only everyday navigation and sync: the refs sidebar toggle, Local Changes, History tree, Fetch, Pull, Push, the branch button, and search. Commands on a selected commit, file, or hunk stay beside that selection.
- Single-letter shortcuts act on the focused list and never fire while typing. Ctrl and Ctrl+Shift chords are global.
- Unavailable items stay visible and disabled with a reason that names the repository state. Commands that are not built are not listed.

## Search

`Ctrl+K` or the toolbar's Search button opens one search field; `Ctrl+F` opens the same search starting from the current History tree filter; there is no separate filter row. Results are grouped: **Filter** (filter the History tree by the text, first and default; this is also how commits are found), **Actions** (named with their menu home), **Branches**, **Remotes**, **Tags**, and **Files** (tracked paths; choosing one opens its latest change in Review). Search runs commands through the same definitions and readiness as the toolbar and menus.

## Density

Compact density is the default: History tree and file rows 22 px, diff rows 19 px, one reader header row (path, endpoints, and the mode controls), one-line commit identity, one-character change markers (`+ ~ − T ? !`) with the full kind in the tooltip. Comfortable density remains a preference.

## Stashes

**Changes › Stashes…** opens a picker, centered under the toolbar, listing each entry's ref, message, branch, age, and file count. The selected entry offers **Apply**, **Pop**, **Show changes**, and **Drop…** (confirmed). The sidebar lists stash entries; selecting one opens the picker on it.

## Interactive rebase

A large modal dialog over the dimmed History tree, laid out like the file workspace. It opens 12 px below the toolbar, centered, so the menu bar and toolbar stay in view.

- **Header (operation bar):** branch, range (`<from>…HEAD onto <base>`), result count (`6 commits → 4 · 1 dropped`), **Cancel** (`Esc`), **Start rebase** (`Ctrl+Enter`).
- **Left, the plan:** newest first, matching the History tree. Rows are 40 px with two lines: hash, message, and ref pills, then the consequence badge in its action's color (none for Pick). Each row has a drag handle, move up/down, hash, message, ref pills, a consequence badge (`↓ fixup into <hash>`, `absorbs <hash>`, `⏸ stops here · reopens staged in Local Changes`, `dropped · recoverable from the backup`), and an action chip: 24 px tall, rounded, with the mockup's glyph and colors (Pick neutral with a border, Reword blue, Reopen purple, Squash and Fixup amber, Drop red). The chip shows the current action; clicking opens a menu of all six actions with a plain hint and key. **Start rebase** is the filled primary button with its `Ctrl+Enter` hint; **Cancel** shows `Esc`. Keys on the selected row: **P** pick, **R** reword, **E** reopen, **S** squash, **F** fixup, **D** drop.
- **Squash and Fixup** combine into the nearest kept commit **below** (older). Squash combines both messages; Fixup keeps only the older message. They are disabled with a reason when no kept commit lies below.
- **Right, the selected commit:** hash, action, author, date, files, and diff in the shared reader. Reword edits the message here, showing the original. Drop dims the diff. **Open in Review** (`Enter`) opens the normal review; `Esc` returns to the plan unchanged.
- **Footer:** stash working changes first, which refs stay on the original commits, and the private-backup notice.

Merge commits remain unsupported and are named as the reason.

## Rebase stops

Both stops use the operation band under the toolbar, the template merge, cherry-pick, and revert follow. Its first line holds the icon, the title naming the stop, the status right after it at normal size (what is left, or why Continue waits; the title truncates first), and **Resolve conflicts** (primary, hidden while the workspace is open), **Continue** (`Ctrl+Enter`), **Skip commit**, and red **Abort and restore**; the second line carries the explanation and the step chips (`✓ n done`, the current chip, the next chips that fit, `+N next`, with the full plan on hover; no scroll area). Amber with a warning glyph while conflicts remain, purple with a pause glyph otherwise; Continue is the filled primary once it can run. A running Continue, Skip, or Abort keeps the band: spinner, worker label, buttons disabled in place. The branch button and window identity show the rebase step.

- **Reopened commit (E):** Local Changes opens with the commit's changes staged and its message in the commit box. Unstaging files or lines and committing splits the commit; each part is listed under **Commits made at this stop**. Continue becomes the primary once nothing remains.
- **Conflict:** the conflict workspace is the Changes view (no Close; `Esc` returns to the plain list): CONFLICTED with what is left per file, RESOLVED, STAGED with − to unstage, ALSO CHANGED BY THIS REBASE, and the legend in a 300 px pane; the file header with **Block by block** | **Whole file**, **Open in editor**, and **Refresh external edits**; a bordered block per conflict with **Base**, **Ours · rewritten branch (onto <base>)**, and **Theirs · <short id> (replayed commit)** in equal columns that wrap long lines inside themselves (stage numbers in the label tooltips). Choices per block: Ours, Theirs, Ours + theirs, Theirs + ours (keys 1–4). The blocks scroll in up to 60% of the height; the line-numbered, editable **Result** takes the rest, tinted by origin (Ours blue, Theirs amber, unresolved markers in the warning tint, hand edits neutral). **Stage resolution** (`Ctrl+S`) stages the Result once it has no markers left. Continue stays disabled with the reason as the band's status until every file is staged.

## Inspect and recovery windows

Submodules, LFS, Signature details, and Recovery follow the inspect proposal: non-modal windows with a title header (status pill and ×), label grids of 22 px rows with 8-character IDs and copy buttons, and a footer with the result or the disabled reason at the left and the filled primary rightmost; their confirmations sit in the modal frame with Cancel `Esc` and the primary `Enter` side by side (a destructive red primary, such as Delete backup, shows no key).

- **Repository › Submodules…** lists every gitlink with a status pill (amber differs, red broken, green clean, grey otherwise); `Enter` opens Submodule details.
- **Repository › LFS…** lists the pointers in HEAD, missing objects first and checked, with one **Fetch <n> objects** that names the count and size.
- **Signature details** shows the result as a coloured banner titled by the object; the signing configuration is in Preferences › Commit.
- **Repository › Recovery…** switches Backups | Reflog in its header, offers one **Restore <branch>…** (a hard reset of a clean tree that first saves the current tip), keeps Reset ▸ Soft/Mixed/Hard and the copy actions under ⋯, adds **Delete backup…**, and reports in its 22 px status strip. It has no Refresh: it re-reads on open, on focus, and after operations.

See [architecture](architecture.md#reflog) for the contracts.

## Preferences

**Repository › Preferences…** opens a non-blocking window; changes apply immediately and persist under the OS configuration directory. Sections: General, Appearance, Diff, Commit, Rebase & recovery, Keyboard.

- **Clone folders** (General): an ordered list of parent folders. One radio marks the **preferred** folder used by Clone by default. Each row shows its repository count or a missing-folder state; rows reorder and remove. Adding validates the path (`Folder is empty.`, `Folder must be an absolute path.`, `Already in the list.`). Removing the preferred folder makes the first one preferred.
- The remaining sections follow the [Settings panel](plan.md#settings-panel--planned) plan.
