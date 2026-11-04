# CLI

Purpose: list `git good` command-line operations.

 Git dispatches `git good` to `git-good` when the binary is on `PATH`. A bare argument is always a folder: `git good [FOLDER]` opens a window. Commands follow `--cli`, never open a window, write JSON on stdout, and report progress or errors on stderr. The repository is always `--repo PATH` (default: the working folder); positionals are only a command's own subject (a ref, a file, a URL, or the folder `clone` and `init` create), and `--` ends options, so `git good --cli blame -- -x.txt` names the file `-x.txt`.

A missing folder named like a command (`git good status`) opens no window: it fails with *No folder status. Did you mean git good --cli status?* An unknown word after `--cli` fails with *Unknown command …*, and an option before it with *Unknown option …*; `git good -- -dir` opens a folder whose name starts with a dash.

## Launch and inspect

```sh
git good [FOLDER]
git good -- FOLDER
git good --version
git good --help | -h
git good --cli history [--repo PATH]
git good --cli refs [--repo PATH]
git good --cli show <commit> [--repo PATH] [--parent N] [--against REF] [--file PATH] [--export before|after --output PATH]
git good --cli status [--repo PATH] [--file PATH --area staged|unstaged|untracked|conflict]
git good --cli reflog [REF] [--repo PATH]
git good --cli blame FILE [--revision REF] [--lines START,END] [--repo PATH]
git good --cli signature config [--repo PATH]
git good --cli signature verify REF [--repo PATH]
```

## Stage and commit

```sh
git good --cli stage --file PATH [--hunk HEADER]... [--repo PATH]
git good --cli unstage --file PATH [--repo PATH]
git good --cli commit [--amend] [--sign|--no-sign] -m MESSAGE [--repo PATH]
```

`-m` is also `--message` (in `commit`, `tag`, and `stash push`), and `--sign` is also `-s` (in `commit` and `tag`).

## Branches, tags, and checkout

```sh
git good --cli switch [--create | --stash [--include-untracked]] BRANCH [--repo PATH]
git good --cli branch --create NAME [--at REF] [--repo PATH]
git good --cli branch --delete BRANCH [--force] [--repo PATH]
git good --cli tag --create TAG --at REF -m MESSAGE [--sign|--no-sign] [--repo PATH]
git good --cli tag --delete TAG [--repo PATH]
git good --cli reset --soft|--mixed|--hard COMMIT [--repo PATH]
```

`switch --stash` stashes the tracked changes (and untracked files with `--include-untracked`) and then switches to an existing branch. `tag --create` signs as `tag.gpgSign` says unless `--sign` or `--no-sign` is given.

`git good --cli reset --hard` refuses, exiting nonzero with nothing changed, when it would overwrite untracked or ignored files whose paths are tracked in COMMIT; the error lists those paths, one per line, after `Move or delete them, then retry:`. The check reads each path as the bytes Git stores, so a name that is not UTF-8, or a working tree whose folder name ends with a space, is checked like any other (the error shows such a name lossily). Every reset first saves the tip it leaves as a private `refs/gg/backup/reset/…` reference and names it in the receipt's `backup`; when the backup cannot be written, the reset fails with nothing changed.

## Sync and stash

```sh
git good --cli fetch [REMOTE] [--prune] [--repo PATH]
git good --cli push [REMOTE] [BRANCH] [--set-upstream] [--force-with-lease] [--repo PATH]
git good --cli clone URL [PATH]
git good --cli init [--branch NAME] [--commit] [PATH]
git good --cli stash list [--repo PATH]
git good --cli stash push [-m MESSAGE] [--include-untracked] [--keep-index] [--repo PATH]
git good --cli stash apply|pop [STASH] [--index] [--repo PATH]
git good --cli stash drop [STASH] [--repo PATH]
```

`git good --cli clone` clones URL into PATH (default: a folder named after the repository) and prints the receipt `{"operation":"clone","path","head","branch","complete":true}`; it opens no window.

`git good --cli init` creates PATH (default: the current folder) when missing and runs `git init -b NAME` (default `main`); with `--commit` it adds every file not ignored and commits it as `Initial commit`. It refuses, exiting nonzero with nothing changed, when PATH is inside a repository. The receipt is `{"operation":"init","path","branch","head","files","complete":true}`; `head` is `null` and `files` 0 without a commit.

`git good --cli stash list` entries carry `index`, `reference`, `commit`, `message`, `branch` (`null` for a detached HEAD), and `age` (relative, such as `3 days ago`); `files` is `null` there because counting files reads each entry.

## Merge, rebase, rewrite, and conflicts

```sh
git good --cli merge REF [--no-ff] [--repo PATH]
git good --cli merge --continue|--abort [--repo PATH]
git good --cli rebase REF [--autostash|--no-autostash] [--repo PATH]
git good --cli rebase --continue|--skip|--abort [--repo PATH]
git good --cli rewrite --from COMMIT STEP... [--autostash|--no-autostash] [--repo PATH]
#   STEP: --pick|--edit|--squash|--fixup|--drop COMMIT, or --reword COMMIT MESSAGE
git good --cli rewrite --continue|--skip|--abort [--repo PATH]
git good --cli cherry-pick COMMIT... [--repo PATH]   # several: oldest first, one git cherry-pick
git good --cli revert COMMIT [--repo PATH]
git good --cli cherry-pick|revert --continue|--skip|--abort [--repo PATH]
git good --cli conflicts [--repo PATH]
git good --cli conflict --file PATH [--use ours|theirs|deletion|--stage] [--repo PATH]
```

`conflicts` lists each path escaped in `paths` and as its exact bytes in the matching entry of `path_bytes`. `--file` takes the exact path bytes (raw on Unix) and treats them as a literal path, never a pattern: `--file '*.txt'` resolves only the file named `*.txt`. Receipts report `path` escaped and `path_bytes` exact.

`--use deletion` resolves a delete/modify conflict by keeping the deletion: it removes the path from the index and the working tree (`git rm` for that path) and fails when neither side deleted the file. `--use ours|theirs` naming the side that deleted the file fails and points to `--use deletion`.

## Worktrees, submodules, and LFS

```sh
git good --cli worktree list [--repo PATH]
git good --cli worktree add DEST --branch BRANCH [--repo PATH]
git good --cli worktree add DEST --create BRANCH [--at REF] [--repo PATH]
git good --cli worktree remove PATH [--repo PATH]
git good --cli submodule status PATH [--recorded COMMIT] [--before COMMIT] [--repo PATH]
git good --cli submodule update PATH [--repo PATH]
git good --cli lfs <status|fetch> PATH [--area staged|unstaged|untracked|conflict] [--revision REF] [--before REF] [--repo PATH]
```
