# Domain language

Purpose: name each thing Git Good shows with one term, used the same way in the app, the specifications, the documentation site, the CLI, and conversations about the app.

Use these terms exactly. When a term is several words, write all of them: never shorten **History tree**, **File history**, **Line history**, or **Rewrite history** to "history". The tests `the_word_history_is_always_part_of_a_full_domain_term` and `no_shown_string_uses_history_outside_the_full_domain_terms` (apps/git-good/src/main.rs) fail when a shown string uses "history" on its own.

## Views

| Term | What it is | Git concept | Where |
|---|---|---|---|
| **History tree** | The commits of the repository as a list with lanes, branch and tag pills, author, short ID and date, newest first. | `git log --graph` | The **History tree** tab (`Ctrl+2`), View › History tree, the back button `‹ History tree`, Preferences › Appearance › History tree, `git good --cli history-tree` |
| **Local Changes** | The working tree and the index: unstaged, untracked and staged files, the reader, and the commit box. | `git status`, the index | The **Local Changes** tab (`Ctrl+1`), View › Local Changes |
| **Review** | Reading what one commit changed, file by file, against a parent or another commit. | `git show`, `git diff A B` | `Enter` on a commit, **Review** in Commit details |
| **Commit details** | The pane under the History tree for the selected commit: subject, author, parent, files. | `git show --stat` | View › Commit details, `Space` |
| **Refs sidebar** | Branches, remotes, tags, stashes and worktrees beside the History tree. | refs | View › Refs sidebar |
| **Reader** | The pane that shows one file: **Diff** or **Full file**, **Inline** or **Side by side**, **Blame**. | `git diff`, `git blame` | Review, Local Changes, File history |
| **Recovery** | The window that lists Git Good's backups and the reflog, and restores a branch tip. | `refs/gg/backup/…`, `git reflog` | Repository › Recovery…, the Recovery toolbar button |

## A file and its lines over time

| Term | What it is | Git concept | Where |
|---|---|---|---|
| **File history** | The versions of one file across commits, newest first, following it across renames. Opened with `H` in the reader; the list header reads `FILE HISTORY 7`. | `git log --follow -M25% -- <path>` | The **File history** button in the reader, `H` |
| **Version** | One state of a file in File history: the commit that introduced it, its path, and its content. | a blob at a path in a commit | A row of File history |
| **Absent version** | The state of a file before it existed or after it was deleted. Before a file first appears it is the commit just before the one that added it; after a deletion it is the commit that deleted it. | no blob at that path | The last row of File history, or a row between a deletion and a re-add |
| **Rename** | A file added in a commit that continues the most similar file the same commit deleted, when at least 25 % of the content matches. One row marked `R` in Review; a version reading `← old path` in File history. | `git diff -M25%` | Review, File history |
| **Line history** | The versions of one line or range of lines, from Blame. | `git log -L` | **Line history** in the Blame bar |
| **Blame** | For each line, the commit that last changed it. | `git blame` | The **Blame** button in the reader, `B` |

## Changing commits

| Term | What it is | Git concept |
|---|---|---|
| **Rewrite history** | Any operation that replaces commits instead of adding new ones: interactive rebase, rebase onto another branch, amend, reword, and reset. Git Good saves a backup before each one. | rewriting commits |
| **Interactive rebase** | The dialog that picks, rewords, reopens, squashes, fixes up, drops and reorders a linear range of commits. It keeps Git's name. Menus: Commit › Interactive rebase from here… (`R`), Branch › Interactive rebase of current branch…. | `git rebase -i` |
| **Backup** | A private ref Git Good writes before it rewrites history, under `refs/gg/backup/<operation>/<time>/<branch>`. Listed in Recovery; kept until deleted, or for the period set in Preferences › Rebase & recovery. | a ref |
| **Stash** | Changes put aside to bring back later. | `git stash` |
| **Worktree** | Another checkout of the same repository in its own folder. | `git worktree` |

## Words to avoid

| Avoid | Use instead |
|---|---|
| "History" on its own (tab, button, key hint, message) | History tree, File history, Line history, or Rewrite history |
| "Changes" for the view | Local Changes (the **Changes** menu keeps its title: it holds commands on local changes) |
| log, timeline (for the History tree) | History tree |
| inspector | Commit details |
| versions list, file log | File history |

## CLI

Command words follow the domain language. A word renamed for it keeps working as a hidden alias, left out of `--help`, so scripts do not break.

| Command | Prints | Hidden alias |
|---|---|---|
| `git good --cli history-tree` | The History tree as JSON | `history` |

JSON keys never change: `history-tree` prints the same document `history` did.
