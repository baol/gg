---
title: Performance
description: How fast Git Good is on large repositories, and how that is measured and kept.
---

# Performance

Git Good is built so the window never waits for Git, and Git never waits for the window.

<div class="gg-stats" markdown>
<div class="gg-stat"><strong>130–180 ms</strong><span>first rows of the History tree</span></div>
<div class="gg-stat"><strong>1.7–1.9 s</strong><span>all 100,000 commits loaded</span></div>
<div class="gg-stat"><strong>17–21 ms</strong><span>jump to the last row</span></div>
<div class="gg-stat"><strong>70 MiB</strong><span>peak resident memory</span></div>
<div class="gg-stat"><strong>0.5 ms</strong><span>frame p95, sidebar with 10,000 refs</span></div>
</div>

## The benchmark

Measured on Linux x86_64 with 4 cores and a release build, on a repository written by `git fast-import`: 100,000 commits on `main` and 10,000 branches in 100 folders.

| Measure | Result |
|---|---|
| History tree: first rows | 130–180 ms |
| History tree: all commits loaded | 1.7–1.9 s |
| Frame time while loading | p95 17–18 ms, median 9–10 ms |
| Frame time when idle | p95 7–10 ms |
| Jump to the last row | 17–21 ms |
| Peak resident memory | 70 MiB |
| Refs sidebar frame | p95 0.5 ms |
| Rebuilding the sidebar after a new snapshot of 10,000 refs | 5 ms |

Frames are timed headless (layout and tessellation, without the GPU). The limits live in [`apps/git-good/perf/budgets.json`](https://github.com/baol/gg/blob/main/apps/git-good/perf/budgets.json) and a scheduled job fails when a number goes over its budget.

## Commands cost what Git costs

A command run from the window must take **no more than 1 % longer** than the same command run as `git good --cli …`. This is a project rule, and it is enforced in two ways:

- **The same Git processes.** Tests record every Git process the window starts for each operation and compare them with the CLI's: the same command lines, in the same order, and nothing before them. Refreshing the History tree afterwards happens in the background and never delays the result.
- **Measured nightly.** Each operation runs 30 times through the window and through the CLI on fresh repositories. The job fails if the window's median is over 1.01 × the CLI's median + 2 ms.

## Why it stays responsive

- **No Git on the UI thread.** Every operation runs on a worker; the window keeps drawing and shows the result when it arrives.
- **Streaming reads.** The History tree, status and diffs are read in pages and bounded, so a huge file or log never blocks a frame.
- **Idle means idle.** A window with nothing to do draws no frames. The file watcher ignores Git's object and log folders, so watching costs your Git commands nothing.
- **Stable targets.** When new content arrives, the scroll position, selection and focus follow the same commit or file, so a click never lands on a row that just moved.
