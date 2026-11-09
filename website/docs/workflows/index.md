---
title: Workflows
description: Step-by-step guides for everyday Git tasks in Git Good.
---

# Workflows

Each guide follows one task from start to finish on the same example project: **acme-api**, a small Rust web service worked on by four people. It has feature branches, two release tags, a merge, a remote that has moved on, a stash and some work in progress. Every screenshot is rendered from the app itself on that repository.

| Task | Guide |
|---|---|
| Find a commit, a branch or who changed what | [Find your way in the History tree](history.md) |
| Read what a commit changed, file by file | [Review a commit](review.md) |
| Stage files, hunks or single lines and commit | [Commit part of your work](commit.md) |
| Switch and create branches, fetch, pull and push | [Branches, fetch, pull and push](sync.md) |
| Put work aside and bring it back | [Park work in a stash](stash.md) |
| Reword, squash, fixup, drop and reorder commits | [Clean up with interactive rebase](rebase.md) |
| Finish a merge or rebase that stopped | [Resolve a merge conflict](conflicts.md) |
| Get back a branch tip from before a rewrite | [Undo with Recovery](recovery.md) |
| Run any command from the keyboard, change keys | [Search, keys and preferences](keyboard.md) |

!!! tip "Every command, three ways"
    Each action is in a menu, most have a key, and all of them can be found by name in search (++ctrl+k++). The footer always lists the keys for the view you are in.
