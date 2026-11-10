---
title: Cherry-pick and revert
description: Copy a commit onto your branch, or undo one with a new commit.
---

# Cherry-pick and revert

Both start from a commit selected in the History tree and need a clean working tree (untracked files are fine). Both run Git itself, so hooks and conflicts behave exactly as on the command line.

## Copy commits onto your branch

Select a commit on another branch and choose **Commit › Cherry-pick onto HEAD**. Your branch gains a new commit with the same change and message; the footer reads *Cherry-picked …*.

To copy several, Ctrl-click (++cmd++-click on macOS) or Shift-click them: the Commit menu then reads **Cherry-pick 2 commits onto HEAD**, and Git Good applies them oldest first in one `git cherry-pick`.

A merge commit has no single change to copy, so Git Good refuses it with *Use the Merge action for merge operations.*

## Undo a commit

Select the commit and choose **Commit › Revert**. Git Good adds a new commit that reverses it, and the original stays in the History tree, so this is safe on branches others have pulled.

### Revert a merge

Reverting a merge commit undoes everything the merge brought in, against the branch it was merged into (`git revert -m 1`, as in Pro Git's *Undoing Merges*). The footer adds a warning: merging the same branch again will not bring those changes back. To bring them back, revert the revert first.

## When it conflicts

If the change does not apply cleanly, the amber band names the operation (*Cherry-pick conflict · applying …* or *Revert conflict · reverting …*) and lists what is left. Resolve each file as in [Resolve a merge conflict](conflicts.md), then choose:

- **Continue** to make the commit,
- **Skip commit** to leave this one out,
- **Abort and restore** to go back to where you started.

## From a script

```sh
git good --cli cherry-pick feature/rate-limit~1 feature/rate-limit
git good --cli revert v0.2.0
git good --cli revert --continue
```
