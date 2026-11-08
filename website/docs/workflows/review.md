---
title: Review a commit
description: Read a commit file by file, inline or side by side, with blame and file history.
---

# Review a commit

Select a commit in History and press ++enter++ (or click **Review**). Here we review *Return 404 as JSON*, the commit tagged `v0.2.0`:

![Review of "Return 404 as JSON": the changed file on the left, the diff of src/routes.rs on the right with removed and added lines](../assets/screens/review.png){ .gg-shot }

## What you see

- **Files** on the left, with added and removed line counts for each. A renamed file is one row marked **R** at its new name; hovering it names the old one. ++up++ and ++down++ move between files.
- The **header** names the commit, its author and date, and what it is compared with: **Compare with** defaults to the first parent and can be any other parent of a merge.
- The **reader** shows the file's change as a **Diff** or the **Full file**, **Inline** or **Side by side**. Changed words are highlighted inside changed lines.
- ++bracket-left++ and ++bracket-right++ jump between changes; **Blame** (++b++) shows who last changed each line and **History** (++h++) lists the commits that touched the file.

## Typical uses

Check a teammate's change before you merge it
:   Review their branch tip, read each file side by side, then merge from the branch's actions.

Find when a line changed
:   Open Blame on the file, then follow the line to the commit that introduced it.

Follow a file across a rename
:   Review the commit that renamed it and press ++h++ on the **R** row: File history continues under the old name, back to the commit that added it.

Copy an old version
:   Switch to **Full file** to read the whole file as it was in that commit.

Press ++escape++ to go back to History, with the same commit still selected.
