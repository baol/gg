---
title: Scripting with the CLI
description: Use git good --cli for scripts, CI and editor integrations, with JSON output.
---

# Scripting with the CLI

Every operation in the window is also a command: `git good --cli <command>`. Commands never open a window, print JSON on stdout and report progress and errors on stderr. They run the same Git processes as the window does.

```sh
git good --cli status --repo ~/src/acme-api
```

```json
{"repository":{"path":"/home/ada/src/acme-api",
  "head":"fa148783e22720b0b77d0900fa1e105551141e13","head_ref":"refs/heads/main", "…": "…"},
 "entries":[
  {"area":"staged","kind":"modified","path":"README.md", "…": "…"},
  {"area":"unstaged","kind":"modified","path":"src/routes.rs", "…": "…"},
  {"area":"untracked","kind":"added","path":"src/orders.rs", "…": "…"}],
 "complete":true,"status":"completed"}
```

Paths come as `path` (readable) and `path_bytes` (exact bytes, for names that are not UTF-8). Keys are only ever added, never renamed or removed, so scripts keep working across releases.

## Examples

List stashes with their branch and age:

```sh
git good --cli stash list
```

```json
{"operation":"stash-list","stashes":[
  {"index":0,"reference":"stash@{0}","message":"On feature/rate-limit: tune the burst size",
   "branch":"feature/rate-limit","age":"7 days ago", "…": "…"}],
 "complete":true,"status":"completed"}
```

Fold a fixup commit, keeping uncommitted work:

```sh
git good --cli rewrite --from 2d566e9 \
  --pick 2d566e9 --fixup b912e2c --pick fa14878 --autostash
```

The receipt names the backup, which `git good --cli reset --hard <backup>` or the Recovery window can restore:

```json
{"operation":"rewrite","status":"completed","steps":3,
 "backup":"refs/gg/backup/rewrite/1793901278176003211/main", "…": "…"}
```

Resolve a conflict from a script:

```sh
git good --cli conflicts
git good --cli conflict --file src/routes.rs --use theirs
git good --cli merge --continue
```

Stage one hunk and commit:

```sh
git good --cli stage --file src/routes.rs --hunk '@@ -1,6 +1,8 @@'
git good --cli commit -m "Import the orders module"
```

## Exit codes

| Exit | Meaning |
|---|---|
| 0 | Completed, or stopped as planned (such as a rewrite `--edit` stop) |
| 1 | Failure: `Error: <message>` on stderr, nothing on stdout |
| 2 | Usage error |
| 4 | Stopped on conflicts; the receipt lists them |
| 5 | Partial: the receipt says `complete: false` |

## Commands

| Area | Commands |
|---|---|
| Inspect | `history-tree`, `refs`, `show`, `status`, `reflog`, `blame`, `signature config`, `signature verify` |
| Stage and commit | `stage`, `unstage`, `commit [--amend] [--sign]` |
| Branches and tags | `switch [--create \| --stash]`, `branch --create \| --delete`, `tag --create \| --delete`, `reset --soft \| --mixed \| --hard` |
| Sync | `fetch`, `push [--set-upstream] [--force-with-lease]`, `clone`, `init` |
| Stash | `stash list \| push \| apply \| pop \| drop` |
| Integrate | `merge`, `rebase`, `rewrite`, `cherry-pick`, `revert`, each with `--continue`, `--skip` and `--abort` where they apply |
| Conflicts | `conflicts`, `conflict --file PATH --use ours \| theirs \| deletion \| --stage` |
| More | `worktree list \| add \| remove`, `submodule status \| update`, `lfs status \| fetch` |

Every command except `clone` and `init` takes `--repo PATH` (default: the current folder). The full reference, with every option, is [docs/cli.md](https://github.com/baol/gg/blob/main/docs/cli.md) in the repository.
