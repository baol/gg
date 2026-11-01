# CLI

Purpose: list `git good` command-line operations.

 Git dispatches `git good` to `git-good` when the binary is on `PATH`. Commands output JSON on stdout and report progress or errors on stderr.

## Launch and inspect

```sh
git good [PATH]
git good --version
git good --help
git good history [PATH]
git good refs [PATH]
git good show <commit> [--repo PATH] [--parent N] [--against REF] [--file PATH] [--export before|after --output PATH]
git good status [PATH] [--file PATH --area staged|unstaged|untracked|conflict]
git good reflog [REF] [--repo PATH]
git good blame PATH [--revision REF] [--lines START,END] [--repo PATH]
git good signature <config|verify> [REF] [--repo PATH]
```

## Stage and commit

```sh
git good stage --file PATH [--hunk HEADER]... [--repo PATH]
git good unstage --file PATH [--repo PATH]
git good commit [--amend] [--sign|--no-sign] -m MESSAGE [--repo PATH]
```

## Branches, tags, and checkout

```sh
git good switch [--create] BRANCH [--repo PATH]
git good branch --create NAME [--at REF] [--repo PATH]
git good branch --delete BRANCH [--force] [--repo PATH]
git good tag --create TAG --at REF -m MESSAGE [--sign] [--repo PATH]
git good tag --delete TAG [--repo PATH]
git good reset --soft|--mixed|--hard COMMIT [--repo PATH]
```

`git good reset --hard` refuses, exiting nonzero with nothing changed, when it would overwrite untracked or ignored files whose paths are tracked in COMMIT; the error lists those paths, one per line, after `Move or delete them, then retry:`.

## Sync and stash

```sh
git good fetch [REMOTE] [--prune] [--repo PATH]
git good push [REMOTE] [BRANCH] [--set-upstream] [--force-with-lease] [--repo PATH]
git good clone URL [PATH]
git good init [--branch NAME] [--commit] [PATH]
git good stash list [--repo PATH]
git good stash push -m MESSAGE [--include-untracked] [--keep-index] [--repo PATH]
git good stash apply|pop [STASH] [--index] [--repo PATH]
git good stash drop [STASH] [--repo PATH]
```

`git good init` creates PATH (default: the current folder) when missing and runs `git init -b NAME` (default `main`); with `--commit` it adds every file not ignored and commits it as `Initial commit`. It refuses, exiting nonzero with nothing changed, when PATH is inside a repository. The receipt is `{"operation":"init","path","branch","head","files","complete":true}`; `head` is `null` and `files` 0 without a commit.

`git good stash list` entries carry `index`, `reference`, `commit`, `message`, `branch` (`null` for a detached HEAD), and `age` (relative, such as `3 days ago`); `files` is `null` there because counting files reads each entry.

## Merge, rebase, rewrite, and conflicts

```sh
git good merge REF [--no-ff] [--repo PATH]
git good merge --continue|--abort [--repo PATH]
git good rebase REF [--autostash] [--repo PATH]
git good rebase --continue|--skip|--abort [--repo PATH]
git good rewrite --from COMMIT STEP... [--autostash] [--repo PATH]
#   STEP: --pick|--edit|--squash|--fixup|--drop COMMIT, or --reword COMMIT MESSAGE
git good rewrite --continue|--skip|--abort [--repo PATH]
git good cherry-pick COMMIT [--repo PATH]
git good revert COMMIT [--repo PATH]
git good cherry-pick|revert --continue|--skip|--abort [--repo PATH]
git good conflicts [--repo PATH]
git good conflict --file PATH [--use ours|theirs|deletion|--stage] [--repo PATH]
```

`--use deletion` resolves a delete/modify conflict by keeping the deletion: it removes the path from the index and the working tree (`git rm` for that path) and fails when neither side deleted the file. `--use ours|theirs` naming the side that deleted the file fails and points to `--use deletion`.

## Worktrees, submodules, and LFS

```sh
git good worktree <list|add|remove> [OPTIONS] [--repo PATH]
git good submodule <status|update> PATH [OPTIONS] [--repo PATH]
git good lfs <status|fetch> PATH [--area staged|unstaged|untracked|conflict] [--revision REF] [--before REF] [--repo PATH]
```
