# Plan

Purpose: sequence M4 delivery and acceptance.

## M4 · Clone, fetch, push, tracking, credentials, and progress

Goal: move commits and branches between repositories with Git-compatible authentication and visible progress, without weakening read-only browsing.

Contracts: [remote tracking and network mutations](architecture.md#remote-tracking-and-network-mutations--m4). M3 is complete; [evidence](verification.md#m3-graph-refs-and-branches--2026-10-21).

### Increments

1. **Remote and tracking snapshot · complete.** Read configured remotes, each local branch's upstream, and ahead/behind counts through the shared snapshot. Include the data in `git good refs [PATH]` JSON and show the current branch's tracking in the native refs sidebar and status line.
2. **Fetch.** Add `git good fetch [REMOTE] [--prune] [--repo PATH]` and a native **Fetch** action. Stream bounded progress, refresh refs/history after success, and preserve the worktree and index.
3. **Push and upstream.** Add `git good push [REMOTE] [BRANCH] [--set-upstream] [--repo PATH]` and a native **Push** action. Push only the selected local branch without force, resolve the default remote explicitly, and record upstream only after a successful push.
4. **Clone.** Add `git good clone URL [PATH]` and a native **Clone…** action with URL and destination fields. Stream progress, reject unsafe destinations, and open the cloned repository after success.
5. **Remote branch tracking.** Switch to a remote branch by creating a local tracking branch after a clean-worktree check, and expose the same action from the refs sidebar and menu.
6. **Credentials, progress, and cancellation.** Delegate transport and credential-helper behavior to Git, remove ambient repository overrides, report actionable authentication failures without exposing secrets, bound progress text, and cancel the running network operation on request.

All M4 increments are implemented. The native app now runs on egui/eframe 0.37 and wgpu 30 with Rust 1.95+; the vendored Phosphor font keeps the tracking and cancel icons.

### Acceptance

| Scenario | Expected result | Method |
| --- | --- | --- |
| Configured remotes | Names and fetch/push URLs match Git configuration; URLs are display-only | Git fixtures and CLI |
| Local upstream | Upstream ref, remote, and remote branch are exact; branches without upstream stay explicit | Git fixtures and CLI |
| Ahead/behind | Counts match `git rev-list --left-right --count`; unrelated histories and missing upstream refs are explicit | Git oracle and shared snapshot tests |
| Tracking UI | Current branch shows upstream and ahead/behind in the sidebar and status line; long labels truncate explicitly | egui and native review |
| Fetch | Selected remote updates remote-tracking refs and history; worktree, index, local branches, and HEAD stay unchanged | Local bare-remote integration and CLI |
| Fetch failure | Missing remote, authentication failure, and cancellation return actionable errors; displayed state remains usable and refreshable | Command-line fixtures and native review |
| Push | Selected local branch reaches the remote; non-fast-forward rejection is reported without force; local repository remains intact | Local bare-remote integration and CLI |
| Push upstream | A successful `--set-upstream` push configures the branch upstream; failed pushes do not | Local bare-remote integration and CLI |
| Clone | New repository contains the expected refs and worktree, opens in the native app, and reports progress | Local bare-remote integration and native review |
| Credentials | Git credential helpers and SSH agent configuration are honored; missing credentials produce an actionable failure without secrets in output | Fixture helpers and native review |
| Progress and cancellation | Progress stays bounded and current; cancel stops the Git child and reports cancellation; a later refresh recovers completed remote work | Worker tests and native review |
| Read-only safety | Browsing, refs, tracking reads, and cancellation leave objects, refs, index, and working files unchanged unless an explicitly requested network mutation completed | Byte/reference comparison |

Record native wide/narrow captures and verification in [M4 verification](verification.md#m4-clone-fetch-push-tracking-and-credentials--2026-10-22).
