---
title: Git Good
description: A fast, native Git desktop app for people who live in Git.
hide:
  - navigation
  - toc
---

<div class="gg-hero" markdown>

# Git, without the guesswork

<p class="gg-tagline">Git Good is a fast, native desktop app for Git. Browse 100,000 commits without a stutter, stage single lines, tidy a branch with an interactive rebase, and undo any rewrite from Recovery. Every action runs the same Git commands as the command line.</p>

[Get started](getting-started.md){ .md-button .md-button--primary }
[See the workflows](workflows/index.md){ .md-button }

</div>

=== "Linux and macOS"

    ```sh
    curl -fsSL https://raw.githubusercontent.com/baol/gg/main/scripts/install.sh | sh
    ```

=== "Windows"

    ```powershell
    irm https://raw.githubusercontent.com/baol/gg/main/scripts/install.ps1 | iex
    ```


![Scrolling a 20,000-commit History tree with Page Down, then jumping to its first commit and back to the newest](assets/screens/demo-history.gif){ .gg-shot }

<div class="gg-stats" markdown>
<div class="gg-stat"><strong>~150 ms</strong><span>to the first rows of a 100,000-commit History tree</span></div>
<div class="gg-stat"><strong>1.8 s</strong><span>to load all 100,000 commits, while you scroll</span></div>
<div class="gg-stat"><strong>70 MiB</strong><span>peak memory for that History tree</span></div>
<div class="gg-stat"><strong>0.5 ms</strong><span>per frame for a sidebar of 10,000 branches</span></div>
<div class="gg-stat"><strong>1 %</strong><span>most a GUI command may add over the same CLI command</span></div>
</div>

## See it move

<div class="grid" markdown>

![Staging one hunk of src/routes.rs in Local Changes, writing the message and committing](assets/screens/demo-stage.gif){ .gg-shot }

![Folding a WIP commit into the one it fixes in Interactive rebase, starting it, then the backup of the old tip in Recovery](assets/screens/demo-rebase.gif){ .gg-shot }

</div>

## What it does

<div class="grid cards" markdown>

-   :material-source-branch: **A History tree that keeps up**

    ---

    Commit lanes, branch, remote and tag pills, and a refs sidebar that filters as you type. The History tree streams in pages, so the first rows show before the rest is read.

    [:octicons-arrow-right-24: Find your way in the History tree](workflows/history.md)

-   :material-file-compare: **Review like a reviewer**

    ---

    Press ++enter++ on a commit to read every file inline or side by side, with word-level highlights, blame, File history and image previews.

    [:octicons-arrow-right-24: Review a commit](workflows/review.md)

-   :material-checkbox-marked-outline: **Commit exactly what you mean**

    ---

    Stage a file, a hunk or one line, and ignore build output in two clicks. Unstaged and staged changes sit side by side, with the commit box under the diff.

    [:octicons-arrow-right-24: Commit part of your work](workflows/commit.md)

-   :material-format-list-numbered: **Interactive rebase you can see**

    ---

    Pick, reword, reopen, squash, fixup, drop and reorder, with each commit's diff beside the plan and the result count before you start.

    [:octicons-arrow-right-24: Clean up with interactive rebase](workflows/rebase.md)

-   :material-call-merge: **Conflicts, one block at a time**

    ---

    Base, ours and theirs side by side, with Ours, Theirs, Ours + theirs and Theirs + ours on each block, and a live result preview before you stage.

    [:octicons-arrow-right-24: Resolve a merge conflict](workflows/conflicts.md)

-   :material-backup-restore: **Every rewrite can be undone**

    ---

    Before each rebase, amend, rewrite or reset, Git Good saves the old tip as a private backup ref. Recovery lists them, with the reflog, and restores in one step.

    [:octicons-arrow-right-24: Undo with Recovery](workflows/recovery.md)

</div>

## Built for real work

Git Good is a native Rust application drawn with egui. It does not embed a browser, and it runs the `git` on your `PATH`, so your hooks, credential helpers, signing setup and configuration all apply. The same operations are available from scripts as `git good --cli …`, which prints JSON. [How it was built](how-it-was-built.md) says how it is specified and tested.

<div class="grid cards" markdown>

-   **Tidy a branch before review**

    Fold a "WIP" commit into the one it fixes, reword a subject, and push. One dialog, and a backup in case you change your mind.
    [:octicons-arrow-right-24: Use case](use-cases.md#tidy-a-feature-branch-before-review)

-   **Your teammate pushed first**

    The toolbar shows *ahead 5, behind 1*. Pull rebases your work onto theirs, and a conflict is resolved in the same window.
    [:octicons-arrow-right-24: Use case](use-cases.md#your-teammate-pushed-first)

-   **Ship half of your edits**

    A debug line and a fix live in the same file. Stage the fix by line, commit it, and keep the rest for later.
    [:octicons-arrow-right-24: Use case](use-cases.md#ship-part-of-a-file)

-   **Resolve a conflict without markers**

    Base, ours and theirs side by side, a key per choice, and a sentence that says which side is yours.
    [:octicons-arrow-right-24: Use case](use-cases.md#a-merge-conflict-on-the-same-line)

-   **Undo the wrong rewrite**

    The branch before the rewrite is one click away in Recovery: restore it, or branch from it.
    [:octicons-arrow-right-24: Use case](use-cases.md#i-rewrote-the-wrong-branch)

-   **Automate it**

    `git good --cli status`, `history-tree`, `rewrite` and others print JSON with stable keys and documented exit codes, for CI and editor integrations.
    [:octicons-arrow-right-24: Scripting with the CLI](cli.md)

</div>

## Light, dark, wide or narrow

The window follows your system theme and adapts to narrow windows: the sidebar folds away and the details stay readable.

<div class="grid" markdown>

![The History tree in the light theme](assets/screens/history-light.png){ .gg-shot }

![The History tree in a narrow window](assets/screens/history-narrow.png){ .gg-shot }

</div>

[Install Git Good](getting-started.md){ .md-button .md-button--primary }
[View on GitHub](https://github.com/baol/gg){ .md-button }
