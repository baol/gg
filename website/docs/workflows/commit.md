---
title: Commit part of your work
description: Stage files, hunks or single lines and write the commit.
---

# Commit part of your work

Click **Changes** in the toolbar. The badge counts the changed files; here there are three.

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

## Commit

Write the message in the box under the diff (the first line is the subject) and press ++ctrl+enter++ or **Commit**.

- **Amend HEAD** replaces the last commit instead. Git Good saves the old commit as a [backup](recovery.md) first.
- **Sign** signs the commit with your Git signing setup (GPG or SSH); its default follows `commit.gpgSign`.
- **Expand** gives the message more room.

Your draft message is kept if you switch views before committing.
