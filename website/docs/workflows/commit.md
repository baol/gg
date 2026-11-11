---
title: Commit part of your work
description: Stage files, hunks or single lines and write the commit.
---

# Commit part of your work

Click **Local Changes** in the toolbar. The badge counts the changed files; here there are three.

![Local Changes: two unstaged files and one staged file on the left, the diff of src/routes.rs with Stage hunk and Discard hunk on each hunk, and the commit box at the bottom](../assets/screens/local-changes.png){ .gg-shot }

## The lists

- **Unstaged** holds changes to tracked files (`~`) and new, untracked files (`?`).
- **Staged** holds what the next commit will contain.
- **Stage all** and **Unstage all** move everything; the **+** on a row stages that file.
- Images (PNG and JPEG) show as pictures, new or changed: side by side, swipe or onion skin, as in [Review](review.md).

## Stage only what belongs together

In the example, `src/routes.rs` has two hunks: a new `use crate::orders;` import and the change that makes `list_orders` call it. Both belong to the same commit, but you could commit them separately:

1. Click **Stage hunk** on a hunk header to stage that hunk.
2. Or tick the boxes on single changed lines and press **Stage N lines** below the diff.
3. With a file list focused, ++s++ and ++u++ stage and unstage the selected file; ++bracket-left++ and ++bracket-right++ move between hunks.

**Discard hunk** throws a hunk's working changes away after asking you to confirm.

## Ignore what should never be committed

Build output, logs and editor files show up as untracked (`?`) until Git is told to ignore them. Right-click it, or select it and open ⋯ (File actions):

![File actions on the untracked logs/requests.log: Ignore this file, Ignore all *.log files and Ignore folder logs/ above Discard file](../assets/screens/ignore.png){ .gg-shot }

| Choice | Pattern added | Ignores |
|---|---|---|
| **Ignore this file** | `/logs/requests.log` | only this file |
| **Ignore all \*.log files** | `*.log` | every `.log` file, in any folder |
| **Ignore folder logs/** | `/logs/` | the folder and everything in it |

- The pattern goes into `.gitignore` at the repository root. Git Good creates the file if there is none, keeps every line already in it and never adds a pattern twice. Hover a choice to see its pattern first.
- The footer names what was added (*Ignored \*.log · added to .gitignore*), and the ignored files leave the Unstaged list.
- Patterns are anchored to the root and escape characters Git treats specially (`*`, `?`, `[`, `\`), so they ignore exactly what you picked, even for a file named `report [draft].txt`.
- Rows that do not apply are left out: a file without an extension has no *Ignore all* row, and a file at the root has no folder row.
- Ignoring never touches tracked files: Git keeps tracking a file it already has. To stop tracking one, delete it (or run `git rm --cached` on it) and commit.

`.gitignore` is now a changed file itself. Stage and commit it so the rest of the team ignores the same files.

From a script: `git good --cli ignore logs/requests.log --extension` (see [Scripting with the CLI](../cli.md)).

## Commit

Write the message in the box under the diff (the first line is the subject) and press ++ctrl+enter++ or **Commit**.

- **Amend HEAD** replaces the last commit instead. Git Good saves the old commit as a [backup](recovery.md) first.
- With nothing changed, **Amend HEAD** on the *Working tree clean* screen (or **Changes › Amend HEAD**) opens the last commit: reword it, or unstage a file to take it out of the commit.
- Right-click a file in Unstaged or Staged for its actions: stage or unstage, open, copy its path, ignore it, or discard it.
- **Sign** signs the commit with your Git signing setup (GPG or SSH); its default follows `commit.gpgSign`.
- **Expand** gives the message more room.

Your draft message is kept if you switch views before committing.
