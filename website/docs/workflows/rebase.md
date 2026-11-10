---
title: Clean up with interactive rebase
description: Reword, squash, fixup, drop and reorder commits with a visible plan.
---

# Clean up with interactive rebase

Before pushing, Linus wants to fold *WIP fix db typo* into the commit it fixes. Select the oldest commit to change, *Read the database URL from ACME_DB*, and press ++r++ (**Interactive rebase from here…** in the Commit menu). **Branch › Interactive rebase of current branch…** opens it for the commits your branch has that its upstream does not (or, when everything is pushed, that the default branch does not).

![The interactive rebase plan: three commits, newest first; "WIP fix db typo" is set to Fixup into 2d566e9, and the header reads 3 commits → 2](../assets/screens/rebase.png){ .gg-shot }

## Build the plan

The plan lists the commits newest first, like the History tree. For each one choose an action from its menu, or press its key:

| Key | Action | Result |
|---|---|---|
| ++p++ | Pick | Keep the commit as it is |
| ++r++ | Reword | Keep the changes, edit the message in place |
| ++e++ | Reopen | Stop after this commit so you can change or split it in Local Changes |
| ++s++ | Squash | Combine into the commit below and keep both messages |
| ++f++ | Fixup | Combine into the commit below and keep that commit's message |
| ++d++ | Drop | Remove the commit |

++alt+up++ and ++alt+down++ (or dragging the handle) move a commit. Badges say what each choice does: *fixup into 2d566e9* on the WIP commit, *absorbs b912e2c* on its target. The header counts the result: **3 commits → 2**.

The right side shows the selected commit's files and diff, so you can check what you are folding before you start. **Open in Review** reads it in full and brings you back to the same plan.

## Start

- **Stash working changes first** lets the rebase run with uncommitted work, and restores it afterwards.
- The notice says what happens to your old commits: a private backup ref is created before Git starts, and [Recovery](recovery.md) can restore it.
- Press **Start rebase** (++ctrl+enter++).

The band at the top shows each step while Git works. When it is done, the footer reads, for example, *Rewrote 3 commits · backup refs/gg/backup/rewrite/…/main*, with a **Push** button when only unpublished commits changed (**Undo** otherwise) and the details.

If a step stops (a **Reopen**, or a conflict), the band offers **Continue**, **Skip commit** and **Abort and restore**. On a conflict, **Resolve conflicts** in the band opens the [conflict view](conflicts.md).

!!! note "Linear ranges only"
    Interactive rebase works on a range without merge commits. Choose a starting commit after the last merge.

## Rebase onto another branch

To move the whole branch instead, choose **Branch › Rebase current onto…** and name the branch, remote branch or tag to rebase onto. A backup of the old tip is saved first.

**Leave out commits from** handles a branch built on top of another one. In Pro Git's example, `client` was branched from `server`, which was branched from `master`. Rebasing `client` onto `master` with **Leave out commits from** `server` moves only `client`'s own commits and leaves `server`'s where they are (`git rebase --onto master server`).

```sh
git good --cli rebase master --leave-out server
```
