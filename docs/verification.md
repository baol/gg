# Verification

Purpose: record measured results and outstanding acceptance checks.

## M0 baseline · 2026-10-18

Linux x86_64 · Rust 1.94. M0 is the accepted browser baseline; M1 is planned. [Coverage and gaps](regression.md).

- 54 default workspace tests, formatting, Clippy, and debug build pass.
- Git fixture oracles verify order, merge parents, shallow boundaries, pinned HEAD, and 499/500/501 histories. CLI tests verify discovery, JSON, errors, uncapped output, and repository preservation.
- Controlled-worker/input tests cover delayed results, refresh/retry, filtering, cache eviction, dragging, keyboard boundaries, and narrow-view restoration.
- Geometry tests cover long text, Unicode, and 720×480, 1120×740, 1600×900 at 100%/200% scale. Avatar tests cover shared sizes, persistence, deduplication, corruption, failures, and deferred publication.
- Regressions reproduce the 242-pixel last-page jump, release-followed-by-motion jump, and hidden-history navigation. Fixes preserve row geometry, release offset, and selection. The streaming test accepts time-sliced first batches before its reader gate.
- CI is configured for offline checks on Linux, macOS, and Windows; this record establishes local results.

## Native X11 review

| Scenario | Captures |
| --- | --- |
| Layout | [Wide](screenshots/m0-wide.png), [narrow](screenshots/m0-narrow.png), [details](screenshots/m0-details.png) |
| Progressive history | [First frame](screenshots/m0-streaming.png) |
| Scrollbar continuity | [Drag](screenshots/m0-interaction-dragging.png), [release](screenshots/m0-interaction-released.png) |
| Full-message search | [Oldest match](screenshots/m0-interaction-oldest-filter.png) |
| Keyboard navigation | [Last page](screenshots/m0-interaction-keyboard-previous.png), [bindings](screenshots/m0-navigation-bindings.png) |

## Performance observation

Linux x86_64 · Intel i5-6600K · debug build · warm OS cache · packed 100,000 commits.

| Measurement | Result |
| --- | --- |
| First rows | 35.6 ms |
| Complete index | 18.8 s |
| In-memory frame p95 | 5.31 ms |
| Cached rows after deep/back navigation | 97 |

The opt-in timing check passed. Frame timings exclude GPU presentation.

## M1 changed files · 2026-10-18

- All 61 default workspace tests, formatting, Clippy, and debug build pass on Linux.
- Git-oracle tests verify root/first-parent comparisons, nested paths, statuses, object IDs/modes, file-to-directory replacement, empty commits, merge-parent selection, and pinned comparisons with unavailable blobs.
- CLI tests verify discovery, explicit repository, exact path bytes, JSON, and error exits.
- A controlled 10,000-file reader verifies first-page delivery, complete enumeration, deep/back page reload, retained errors, and shutdown. UI tests verify placeholder stability and review/history isolation with exact Back anchors.
- Native X11 review confirms status/path rows, metadata, paging, and Back at [1120×740](screenshots/m1-files-wide.png) and [720×480](screenshots/m1-files-narrow.png).
- Large-tree latency/peak-memory measurements and selected-file diffs remain in the [M1 plan](plan.md).

## M1 selected-file text · 2026-10-18

- All 70 default workspace tests, formatting, Clippy, and debug build pass on Linux.
- Git-oracle hunks and byte reconstruction verify CRLF, final newlines, and exact content. Tests cover invalid UTF-8, binary/size/line budgets, CLI errors, worker cancellation, paging/reuse, stale results, and hovered placeholders.
- In-memory input verifies narrow diff/Back selection and pixel anchors. A 100,000-character Unicode line verifies bounded horizontal shaping.
- Native X11 review confirms the [wide workspace](screenshots/m1-diff-wide.png), [narrow reading view](screenshots/m1-diff-narrow.png), paging, and Back.
- Complete export, special-object fixtures, and release resource measurements in the remaining [M1 increments](plan.md).

## M1 parent selection · 2026-10-18

- All 71 default workspace tests, formatting, Clippy, and debug build pass on Linux.
- Git fixtures verify ordered parent IDs, exact first/second-parent files, and missing-parent errors.
- Wide/narrow input tests verify pinned commits, hover-safe replacement, retained results on failure, retry, and empty-comparison clearing.
- Native X11 review verifies selecting parent 2 in the [wide workspace](screenshots/m1-parent-wide.png) and [narrow reading view](screenshots/m1-parent-narrow.png).

## Preview acceptance pending

- Measure cold/warm release startup and peak memory on the packed 100,000-commit fixture. Targets: first rows within 500 ms, input-to-frame below 50 ms, cached frame p95 within 16 ms.
- Verify native clipboard, high-DPI rendering, and live Gravatar delivery.
- Record remaining native build/runtime evidence for the [target platforms](../ROADMAP.md).

## CI and binary delivery · 2026-10-18

- Fixed the floating-point literal warning that failed PR #1's Clippy checks on newer Rust.
- Linux x86_64: formatting, Clippy on Rust 1.94 and 1.97, and 54 workspace tests pass; the packed-history performance test remains opt-in. Test execution takes about 7 seconds after compilation (fresh test compilation: 3m 59s).
- Linux release build and CLI JSON smoke test pass; the tarball contains the binary, README, and license, and its SHA-256 sidecar verifies.
- actionlint 1.7.12 validates the six-target workflow, packaging steps, and gated draft release job.
- Native CI covers Linux, macOS, and Windows on x86_64 and arm64, caches builds, retains independent platform results, and uploads archives with checksums. Version tags create a draft release after all six jobs pass.
- Cross-platform CI results and native runtime acceptance remain pending. Unsigned binary archives do not establish installer, signing, or hardware acceptance.

## Automatic commit releases · 2026-10-19

- Every passing `main` push publishes a development prerelease for its exact commit with all six native archives and SHA-256 sidecars. Version tags publish directly; pull requests and manual runs keep artifacts without publishing.
- Main runs remain independent across commits. Pull-request runs cancel superseded work. Reruns update the same release.
- actionlint 1.7.12 passes. Local execution of the workflow steps verifies main/tag creation, reruns, checksum validation, and rejection of missing, corrupt, or unexpected assets using a recording GitHub CLI stub.
- Rust code is unchanged. The merged PR #2 run passed tests, release builds, and CLI smoke tests on all six targets. Native graphics acceptance remains separate.

## Automatic versioned releases · 2026-10-19

- 55 Rust tests and seven release-version tests pass; formatting and Clippy pass.
- Version tests cover the floor, numeric patch increments, series changes, retries, explicit-tag validation, and consistent manifest/lockfile updates.
- Cargo accepts a generated `0.3.7` workspace with the locked Linux dependency graph. Workflow YAML and shell syntax validate locally; CI verifies binary versions on all six platforms.
- The updated workflow publishes regular releases when merged into `main`.

## M1 special objects and paths · 2026-10-19

- 77 default workspace tests, formatting, Clippy, and debug build pass on Linux.
- Three failing fixtures established the missing mode/symlink summaries and ambiguous invalid-byte path labels; all pass after correction. Additional tests verify cancellation before the next header and exact CLI path/content bytes.
- Native X11 review confirms [wide mode summaries](screenshots/m1-special-wide-mode.png), [narrow mode summaries](screenshots/m1-special-narrow-mode.png), and [symlink targets](screenshots/m1-special-wide-symlink.png). Escaped paths stay on one line.
- Large-file export and release resource measurements remain in the [M1 plan](plan.md).

## M1 export and binary previews · 2026-10-19

- 83 default workspace tests, formatting, Clippy, and debug build pass on Linux.
- Process tests export a packed 3-MiB blob byte-for-byte, preserve existing destinations and source content, and reject absent sides and Git metadata destinations. Cancellation removes partial output; Escape closes the export context.
- PNG/JPEG tests cover dimensions, small-image scale, malformed input, and preview budgets. An egui geometry test preserves 16 hexadecimal rows at narrow widths.
- Native X11 review confirms [before/after images](screenshots/m1-binary-wide.png), [narrow previews](screenshots/m1-binary-narrow.png), [fixed-width hex](screenshots/m1-binary-hex.png), and the [export window](screenshots/m1-export.png).
- Cold/warm release latency and peak-memory measurements remain in M1 acceptance.

## M1 resource acceptance · 2026-10-20

Linux x86_64 · Intel i5-6600K · 16 GiB RAM · Rust 1.94 · release profile. Packed 10,000-file change, first file 1 MiB per side with two edited lines. Fresh process per run; cold-advised requests fixture-cache eviction with `POSIX_FADV_DONTNEED`, followed by three warm runs. Timings use production workers and in-memory egui at 1120×740; GPU presentation is a native gate.

| Measurement | Cold-advised | Warm range |
| --- | --- | --- |
| First summaries | 13.8 ms | 12.0–13.2 ms |
| First diff | 23.4 ms | 19.9–21.1 ms |
| Complete enumeration | 207.7 ms | 198.2–204.9 ms |
| Input frame during enumeration | 0.25 ms | 0.20–0.22 ms |
| Cached frame p95 | 0.17 ms | 0.11–0.14 ms |
| Peak process RSS | 17.4 MiB | 17.4–17.6 MiB |

- All timing/cache assertions pass. Last-page delivery takes 2.5–2.7 ms; returning takes under 0.3 ms. Retained file rows: 95–111; both UI caches stay within 768 rows. Workers retain single-slot mailboxes and bounded pages.
- Packed 64-MiB export: 0.12 s cold-advised / 0.09 s warm, peak 135.6 MiB RSS. The delta-encoded side takes 0.23 / 0.15 s, peak 199.5 MiB. Both exports match source SHA-256. These costs include object decoding and mapped pack pages; output writes remain 64-KiB chunks.
- Repository snapshots remain unchanged after review/export. The process regression also checks staged, unstaged, untracked, index, object, and ref bytes.
- 84 default tests, formatting, Clippy, and debug build pass. The opt-in release review check passes four runs; the history benchmark remains separate.
- Native 200% X11 captures verify [wide previews](screenshots/m1-binary-wide-2x.png) and [narrow hexadecimal rows](screenshots/m1-binary-hex-2x.png). A failing rendered-header regression led to explicit Before/After byte labels, replacing an unsupported arrow glyph.
- Complete native input/GPU measurements and the remaining [platform gates](#preview-acceptance-pending) before closing acceptance.

## M1 retry and portability · 2026-10-20

- A failing input regression reproduced retry resetting selection. Same-comparison retries now retain the selected path, diff generation, pixel anchor, and known count through partial delivery. Parent-switch tests retain their first-file reset.
- CI exposed a newer Clippy chunk-iteration lint and an APFS filename restriction in a fixture. Use typed chunks and import raw paths directly into Git objects; the Unix process test still selects the exact invalid-byte path.
- 86 default Rust tests, formatting, Clippy, and debug/release builds pass locally. The release stress check covers fully changed 1-MiB text, two 8-megapixel PNGs, and rapid selection; platform reruns follow.

## M1 acceptance · 2026-10-20

M1 is complete: commit review, selected-parent diffs, binary previews, exact export, and interaction preservation. [Six native CI jobs](https://github.com/baol/gg/actions/runs/39384178444) pass tests, Clippy, release builds, version/history smoke checks, and packaging on Linux, macOS, and Windows, each on x86_64 and arm64. Local verification passes 86 Rust tests, seven release-version tests, formatting, Clippy, and debug/release builds.

- Same-comparison retry preserves selection, diff, scroll anchor, and known count. Parent changes retain their separate reset behavior.
- Release stress fixture: fully changed 1-MiB text reaches the explicit computation-budget summary in 262–269 ms; an 8-megapixel PNG pair previews in 117–122 ms. Rapid image/text switching settles on the final selection in 257–260 ms. Three fresh-process runs, one cold-advised and two warm, peak at 43.5–43.9 MiB RSS; in-memory frame p95 stays below 0.05 ms.
- The packed 10,000-file check still passes after the retry fix: first rows 13.0 ms, first diff 21.0 ms, cached frame p95 0.11 ms, 95 retained file rows.
- Native release app on Xvfb/X11: 80 keyboard selections with preview publication enabled, measured through single-pixel readback. First visible response: p50 15.8 ms, p95 23.5 ms, maximum 33.0 ms. The [repeatable probe](development.md) excludes full-screen capture overhead and keeps the pointer clear of deferred-publication regions.
- Native review confirms the [explicit text-budget fallback](screenshots/m1-stress-budget.png), readable hex, and available exact export. Earlier wide/narrow and 200% captures remain valid.
- Supported-preview hardware, monitor presentation, accessibility, clipboard, and platform graphics acceptance remain in the [release gates](#preview-acceptance-pending).

## Light-theme contrast · 2026-10-20

- A failing contrast test reproduced pastel status text on light backgrounds. Theme-aware semantic colors now give light diff/status text at least 4.5:1 contrast, including selected rows. Secondary text, links, warning/error colors, and history selection accents use deeper light-theme colors.
- 88 default tests, formatting, Clippy, and debug build pass. Tests check semantic text/fill pairs, shared accents, and unchanged dark visuals through theme switches.
- Native X11 review confirms [light history](screenshots/light-history.png), [wide diffs](screenshots/light-diff-wide.png), [narrow diffs](screenshots/light-diff-narrow.png), and the preserved [dark palette](screenshots/dark-diff-contrast.png).

## M1 visual polish · 2026-10-20

- Embed the supplied `gg.png` for the native window and fixed-size toolbar brand.
- Reproduced oversized avatar textures in an in-memory regression. List/detail textures now match physical display sizes and share one cached source; filtered-detail tests cover 100%, 125%, and 200% scale.
- 89 workspace tests, seven release-script tests, formatting, Clippy, and debug build pass. Native light 1120×740 and dark 720×480 layouts preserve header geometry.

## M2 status · 2026-10-21

- 99 workspace tests, formatting, Clippy, and debug build pass. Git porcelain fixtures cover independent staged/unstaged entries, ignores, intent-to-add, conflicts, unborn branches, nested discovery, linked worktrees, and repository preservation.
- Worker and egui checks cover bounded delivery, cancellation, deep/back paging, hovered loading slots, refresh failure/retry, last-page stability, and history restoration.
- Linux x86_64, debug build, fresh process, warm 10,000-file fixture: first CLI entry 7.3 ms; complete JSON 185.7 ms. Native X11 light 1120×740 and dark 720×480 layouts inspected.

## M2 working diffs · 2026-10-21

- 108 workspace tests, formatting, Clippy, and debug build pass. Git oracles verify reconstructed sides and line counts; fixtures cover stale HEAD/index, changing working files, CRLF normalization, symlinks, emulated symlinks, and binary/budget summaries.
- A failing production-worker regression reproduced cached working bytes after refresh with unchanged HEAD/index. Explicit refresh now invalidates that snapshot; reopening reads the new bytes.
- Native X11 review confirms compact [wide status](screenshots/m2-status-wide.png), [narrow status](screenshots/m2-status-narrow.png), [light staged diffs](screenshots/m2-staged-diff.png), and [dark unstaged diffs](screenshots/m2-unstaged-diff.png).

## M2 whole-file unstaging · 2026-10-21

- 113 workspace tests, formatting, Clippy, and debug build pass. SHA-1/SHA-256 Git oracles cover additions, deletions, staged/working edits, unborn branches, tree-cache invalidation, unrelated staging, stale HEAD/index, lock ownership, and file/directory collisions. CLI checks verify receipts, stderr, and exit status.
- A failing worker regression reproduced paging clearing an action error. Mutation failures now remain visible until explicit retry; successful actions refresh status through its existing interaction boundary.
- Native X11 clicks in disposable repositories verify unstaging at 1120×740 and 720×480. Git confirms the selected path leaves staged changes and working bytes remain intact. [Narrow result](screenshots/m2-unstage-result.png).

## M2 navigation and diff continuity · 2026-10-21

- 119 workspace tests, formatting, Clippy, and debug build pass. Six added tests cover Enter/Right opening a commit at 720 and 1120 widths, Esc/Left returning to files with selection retained, reading keys scrolling without changing files, hover-deferred replacement, and input ownership for focused buttons versus text editors.
- Shared navigation keys treat text editors and open menus as owners of their input; focused buttons still allow list movement, and activation releases focus before opening a selection.
- Diffs retain the displayed path, metadata, preview, rows, and scroll offset while a replacement loads. The current generation publishes together after hover and scrolling settle; failures keep labeled previous content, and an adopted replacement starts at its first row.
- Repository actions live under **Repository** (Open…, Refresh) and presentation under **View** (Commit details · Space). Enter opens commits at both widths, the full-width reading view scrolls with the shared keys, and Status actions sit beside the file list.
- Native X11 review at 1120×740 and 720×480 confirms the reorganized toolbar and Enter-opened views: [wide](screenshots/m2-navigation-wide.png), [narrow](screenshots/m2-navigation-narrow.png), [wide reading](screenshots/m2-reading-wide.png), [narrow reading](screenshots/m2-reading-narrow.png), [status actions](screenshots/m2-status-actions.png), [narrow status actions](screenshots/m2-status-actions-narrow.png).

## M2 file staging · 2026-10-21

- 127 workspace tests, formatting, Clippy, and debug build pass. SHA-1/SHA-256 Git oracles stage modifications, deletions, untracked additions, type changes, intent-to-add, executable modes, and unrelated entries, then compare `ls-files` with `git add -A`. Attribute normalization, symlink targets, and emulated symlinks match Git while working bytes stay intact.
- Fixtures cover unborn branches, lock ownership, stale HEAD/index/file observations, reappearing deletions, conflicts, submodules, and file/directory collisions. CLI checks verify the stage receipt, stderr, exit status, and repeated-action errors; worker tests verify single-flight staging, retained rows on failure, and refreshed areas after success.
- Native X11 clicks stage `README.md` in a disposable repository at 1120×740 and 720×480; Git confirms the path moves to staged changes and its working bytes remain intact. [Wide action](screenshots/m2-stage-wide.png), [narrow action](screenshots/m2-stage-narrow.png), [wide result](screenshots/m2-stage-result.png), [narrow result](screenshots/m2-stage-narrow-result.png).

## M2 hunk staging · 2026-10-21

- 134 workspace tests, formatting, Clippy, and debug build pass. Five engine tests compare selected index blobs with scripted `git add -p` in SHA-1 and SHA-256 repositories. Partial selection writes zeroed stat and reports `MM`; selecting every hunk writes real stat and leaves only staged changes. Multiple selections, CRLF attributes, missing final newlines, and unrelated staged entries match Git while working bytes and lock ownership stay intact.
- Rejections cover untracked additions, intent-to-add, deletions, mode and symlink-target changes, binary comparisons, the 2-MiB text budget, empty selections, unknown headers, stale HEAD/index observations, and an existing index lock. Every rejection leaves the index unchanged.
- CLI `--hunk` receipts report the selected count; unknown headers and a missing value exit with stderr and publish nothing. The egui test clicks a hunk header, verifies the selected highlight and enabled **Stage hunk** action at 720 and 1120 widths, stages successfully back to a refreshed list with staged and unstaged rows, then fails under a held lock and retains the reading view, error, and index.
- Native X11 clicks stage the first of two hunks in a disposable repository at 1120×740 and 720×480. Git reports `MM notes.txt`, one staged and one unstaged hunk, and zeroed index stat. [Wide selection](screenshots/m2-hunk-wide.png), [wide result](screenshots/m2-hunk-result-wide.png), [narrow selection](screenshots/m2-hunk-narrow.png), [narrow result](screenshots/m2-hunk-result-narrow.png).

## M2 split stage view · 2026-10-21

- All workspace tests pass: 149 passed and 3 stress tests ignored. Formatting, Clippy with `-D warnings`, and the debug build pass.
- Engine oracles cover individual-line staging and unstaging, paired old/new line replacement, zeroed stat for partial results, real stat when every change is selected, and partial unstaging of staged additions, modifications, and deletions. Restoring every line restores the HEAD entry; removing the last staged addition removes the index entry. Working bytes, unrelated entries, modes, and locks remain intact.
- Drop oracles select one hunk or one change block, restore filtered CRLF content and missing final newlines, restore modified/deleted/executable/symlink paths from the index, and remove untracked files. The index file bytes stay identical throughout. Partial mode changes and staged inputs are rejected.
- The egui tests open both panes at 720 and 1120 widths, adopt results with the pointer resting over the view, stage a whole hunk from the inline header action, stage one selected line, unstage an inline hunk and the whole file, and confirm the view stays open. Drop tests cancel and then confirm a hunk drop, verify that a selected line reverts its whole change block, drop the remaining file, and confirm the index is byte-identical and the view stays open. Back refreshes the status list; failures retain the panes, error, and unchanged index.
- Native X11 review at 1120×740 uses a disposable repository with one staged and one unstaged hunk. Clicking the unstaged added line changes the inline actions to **Stage 1** and **Drop selected**; staging refreshes both panes in place while Git reports `MM` and the expected mixed index content. Inline **Unstage hunk** and **Unstage all** also stay in the view and refresh both panes. **Drop hunk** requires confirmation, then restores the working side from the index while leaving the staged hunk untouched. At 720×740 the panes stack, the inline actions and line selection remain usable, and the same staging flow works.
- Captures: [wide split](screenshots/m2-split-wide.png), [wide line selected](screenshots/m2-split-line-selected-wide.png), [wide in-place result](screenshots/m2-split-line-result-wide.png), [wide full unstage](screenshots/m2-split-unstage-wide.png), [wide drop confirmation](screenshots/m2-split-drop-confirm-wide.png), [wide drop result](screenshots/m2-split-drop-result-wide.png), [narrow split](screenshots/m2-split-narrow.png), [narrow line selected](screenshots/m2-split-line-selected-narrow.png).

## M2 commit · 2026-10-21

- All workspace tests pass: 163 passed and 3 stress tests ignored. Formatting, Clippy with `-D warnings`, and the debug build pass.
- Commit oracles cover root, normal, detached-HEAD, and linked-worktree commits; only index content is committed, unstaged working bytes survive, the parent and reflog match Git, and history selects the new commit after Back.
- Hook fixtures cover execution order and arguments for `pre-commit`, `prepare-commit-msg`, `commit-msg`, and `post-commit`, custom `core.hooksPath`, index changes made by `pre-commit`, message edits by `commit-msg`, and an empty message left by a hook. A failing post-commit hook returns a warning with the created commit.
- Rejections cover empty messages, empty staged changes, conflicts, intent-to-add, stale HEAD/index observations, and an existing index lock. Hook failures abort before HEAD changes, preserve the index and working tree, include the hook name/status/output, and leave an actionable retry path.
- CLI checks verify JSON receipts and hook-failure stderr. Egui checks verify the commit panel at 720 and 1120 widths, successful commit adoption, refresh, draft retention, and status recovery after a failing hook. Native X11 review confirms commit-panel layout, a clean post-commit status, and the new commit selected in history.
- Captures: [commit message wide](screenshots/m2-commit-message-wide.png), [commit result wide](screenshots/m2-commit-result-wide.png), [history selection wide](screenshots/m2-commit-history-wide.png), [commit panel narrow](screenshots/m2-commit-narrow.png).

## M3 graph, refs, and branches · 2026-10-21

- All workspace tests pass: 189 passed and 3 stress tests ignored. Formatting, Clippy with `-D warnings`, and the debug build pass.
- [Six native CI jobs](https://github.com/baol/gg/actions/runs/39785755711) pass tests, Clippy, release builds, CLI smoke checks, and packaging on Linux, macOS, and Windows on x86_64 and arm64. Windows fixtures pin `core.autocrlf=false`, and graph/ref strokes use explicit `_f32` widths.
- Ref oracles cover HEAD, local branches, remote branches, lightweight tags, annotated tag peeling, symbolic remote HEAD omission, unborn refs, detached HEAD, and duplicate loose/packed refs. `git good refs [PATH]` emits the shared snapshot without changing refs, the index, or working files.
- Graph unit tests cover linear lanes, merge parents, joins, lane reuse, lane retirement, single-parent color continuity, and downward starts for only the row’s own lane. Stream tests keep graph rows aligned with their commits when filtering, and deep selected-ID recovery finds completed rows by identity. An all-ref oracle compares the commit set with `git rev-list --all`, includes branch-, remote-, and tag-only commits, and requires every parent to follow its children.
- Branch oracles cover local switching, creation from HEAD, dirty unstaged/staged/conflicted rejection, duplicate and missing names, invalid names, detached starts, unborn repositories, and `post-checkout` hooks; a failing post-checkout hook leaves the branch switched and returns a warning. Untracked files no longer block switching or creation and survive the switch, while Git still rejects an untracked file that the target branch would overwrite. Successful switches preserve the expected worktree content and symbolic HEAD; failures leave the old branch and working files intact.
- CLI checks verify `git good switch [--create]` JSON receipts and dirty-tree stderr. egui checks verify graph-column geometry, current-first ref labels and overflow, single-click selection, double-click branch switching, tag/remote jumping, and current-branch no-ops, sidebar visibility at 1120 px versus 1119 px, grouped branch/remote/tag trees, filtering, and the branch creation window at supported widths.
- Native X11 review at 1120×740 and 720×740 shows all five disposable-repository commits from every branch and tag with continuous lanes, refined merges, a green third-branch tip that keeps its color while joining an existing orange lane, an orange branch that stays orange through its final join, merge branches that retain their parent-lane color with no duplicate downward stub, Fork-style ref pills with icons, the grouped refs sidebar with branch folders and HEAD check, branch switching through the sidebar and a pill double-click, tag jumping through a pill double-click, branch creation at HEAD, and the narrow refs-menu fallback. The worktree was clean before each mutation.
- Captures: [sidebar and history](screenshots/m3-refs-sidebar-wide.png), [all-ref history](screenshots/m3-all-refs-wide.png), [branch color join](screenshots/m3-branch-color.png), [branch switched](screenshots/m3-refs-sidebar-switch.png), [new branch modal](screenshots/m3-refs-sidebar-new-branch.png), [narrow history](screenshots/m3-graph-narrow.png).

M3 is complete. Branch deletion/rename, remote tracking, and tag creation remain future work.

## M4 clone, fetch, push, tracking, and credentials · 2026-10-22

Linux x86_64 · Rust 1.99.1 · debug and release builds · local bare remotes.

- 211 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug and release builds pass.
- egui/eframe moved from 0.33 to 0.37 with wgpu 27 to 30 and the workspace MSRV from 1.94 to 1.95. The native app now uses `App::ui`, root-`Ui` panels, `clamped_offset`, `smooth_scroll_delta`, role-based widget info, `Context::request_screenshot`, and theme-specific styles. Tests run through `Context::run_ui` with a helper that clears unapplied texture deltas.
- `Cargo.lock` pins a single `windows` 0.62.2 for `wgpu-hal` and `gpu-allocator`, fixing the Windows DX12 build that failed with two `ID3D12Device` types. `cargo check -p wgpu-hal --target x86_64-pc-windows-msvc` passes locally.
- Sync tests normalize the cloned worktree to `core.autocrlf=false`, compare cloned blob content through `git show`, and compare working bytes before and after fetch, so they pass on Windows with `core.autocrlf=true`.
- The `egui_phosphor_icons` dependency was removed because its latest release targets egui 0.36. The Phosphor regular font is vendored at `apps/git-good/assets/Phosphor.ttf` with its license; app-owned constants and a dedicated font family keep the tracking arrows and the cancel `X`. The vendored font is also a fallback for the default proportional and monospace families, so the toolbar and status line render the tracking arrows without depending on system font coverage. Tests assert the explicit family, the default-family fallback, and that the tracking label uses the Phosphor arrows.
- Tracking tests compare configured remotes, upstreams, and ahead/behind counts with Git for diverged, equal, unrelated, shallow, and missing remote-tracking refs. `git good refs` carries the same remote and tracking snapshot.
- Local bare-remote tests cover clone, fetch, push, non-fast-forward rejection, `--set-upstream`, option injection, URL redaction, credential-failure detection, cancellation, and repository preservation. CLI tests verify JSON receipts, stderr, and exit status.
- Branch switching now ignores untracked files when checking cleanliness, so branch-specific ignore rules no longer block a return switch; Git still rejects an untracked file that the target branch would overwrite and the failure leaves the current branch and file intact. Regression tests cover forward/back switching, creation, and the conflicting-file rejection.
- Native X11 review at 1120×740 and 720×480 confirms upstream and ahead/behind in the sidebar and status line, the Clone and Track remote branch dialogs, and Fetch progress with the cancel action. The first capture exposed the cancel button clipped at the right edge; the toolbar now reserves space while a network operation runs, and `network_cancel_button_stays_inside_the_toolbar` protects both widths.
- The debug `GG_SCREENSHOT` path uses `Context::request_screenshot`; it produced these captures.
- Captures: [tracking wide](screenshots/m4-tracking-wide.png), [tracking narrow](screenshots/m4-tracking-narrow.png), [clone dialog](screenshots/m4-clone-dialog.png), [track dialog](screenshots/m4-track-dialog.png), [fetch progress and cancel](screenshots/m4-fetch-progress.png).
- [Six native CI jobs](https://github.com/baol/gg/actions/runs/39808879624) pass tests, Clippy with `-D warnings`, release builds, CLI smoke checks, and packaging on Linux, macOS, and Windows on x86_64 and arm64 for `beccd6e`. The [release gates](#preview-acceptance-pending) remain unchanged.
- M4 is complete: tracking snapshots, Git-backed clone/fetch/push, remote-branch checkout, credentials, bounded progress, cancellation, branch switching with untracked files, and the egui/eframe 0.37 upgrade are verified.
- Follow-up: `git good --help` lists `fetch`, `push`, and `clone`, [ROADMAP](../ROADMAP.md) marks M4 complete, and `help_lists_every_cli_command` guards the usage banner.

## M5 stash · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 219 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests compare stash push, list, apply, pop, and drop with Git for modified, staged, and untracked changes; messages; `--include-untracked`; `--keep-index`; newest-first ordering; conflicting apply and pop; invalid references; clean trees; and preservation of unrelated files.
- CLI tests verify JSON receipts for `stash list`, `push`, `apply`, `pop`, and `drop`, plus actionable stderr for invalid references.
- egui tests render the **Stash changes…** dialog and **Manage stashes…** window at 720 and 1120 widths, including entry references and the Apply, Pop, and Drop actions.
- [Six native CI jobs](https://github.com/baol/gg/actions/runs/39824257394) pass tests, Clippy with `-D warnings`, release builds, CLI smoke checks, and packaging on Linux, macOS, and Windows on x86_64 and arm64 for `f1d2262`.

## M5 cherry-pick and revert · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 226 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests cover a clean cherry-pick and its parent, cherry-pick conflicts with abort and continue, a conflicting revert with skip, a clean revert, and option-shaped or missing revisions.
- CLI tests verify JSON receipts for `cherry-pick` and `revert`, conflict reporting, `--abort`, `--continue`, and unknown-option stderr.
- egui tests render the in-progress banner with Continue, Skip, and Abort at 720 and 1120 widths.
- [Six native CI jobs](https://github.com/baol/gg/actions/runs/39828199511) pass tests, Clippy with `-D warnings`, release builds, CLI smoke checks, and packaging on Linux, macOS, and Windows on x86_64 and arm64 for `c7a582d`.

## M5 merge and conflicts · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 234 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests cover fast-forward, already-up-to-date, diverged merge commits, forced `--no-ff` merge commits, conflicts with abort and continue, and option-shaped or missing references.
- CLI tests verify JSON receipts for merge, `fast_forward` and `merge_commit` kinds, conflict reporting, `--abort`, `--continue`, `--no-ff`, and unknown-option stderr.
- egui tests render the **Merge branch…** dialog and the merge in-progress banner at 720 and 1120 widths; merges hide the unsupported Skip action.

## M5 conflict resolution · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 241 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests cover unmerged path listing, base/ours/theirs content, side selection with staging, manual staging, add/add conflicts without a base, and unknown-path or base-side rejection.
- CLI tests verify `conflicts` listing, `conflict --file` content, `--use ours|theirs`, `--stage`, and missing-path errors.
- egui tests render the **Resolve conflicts** window with path selection and Base, Ours, and Theirs panes at 720 and 1120 widths.

## M5 recovery and operation state · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 241 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Continue, skip, and abort are covered for cherry-pick, revert, and merge, including merge's unsupported skip and restored HEAD/worktree on abort.
- The in-progress banner exposes Continue, Skip, and Abort and hides Skip for merges.
- Stash, cherry-pick, revert, merge, and conflict resolution refresh an open Local Changes view after success, and the conflict window closes when no operation remains.

## Commit-row context menu and reset · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 247 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- The history-row context menu exposes Cherry-pick, Revert, Reset here (Soft/Mixed/Hard), Copy hash, and Copy subject.
- Git oracle tests cover soft, mixed, and hard reset and reject option-shaped or missing targets; CLI tests verify JSON receipts and missing-mode errors.
- egui tests render the reset confirmation, including the hard-reset warning, at 720 and 1120 widths.

## Compare to local · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 249 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests compare an arbitrary `against` base with the selected commit and assert the old/new sides.
- CLI tests cover `git good show HEAD --against HEAD^`, the generated comparison subject, changed files, and the `--against`/`--parent` conflict.
- The commit-row menu adds **Compare to local**, opening the review diff against the current branch tip.

## Ref context menus · 2026-10-22

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 253 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests cover safe and forced branch deletion, current/missing-branch rejection, and tag deletion with invalid names.
- CLI tests cover `git good branch --delete BRANCH [--force]` and `git good tag --delete TAG` receipts and errors.
- egui tests render the branch and tag delete confirmations at 720 and 1120 widths.
- Branch, tag, and remote menus expose switch/jump, merge, track, delete, and copy actions.

## M6 amend and reflog · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 272 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests amend with the staged index, preserve the original author and parents, keep a message-only tree, and verify the private backup ref. Amend-base tests show HEAD's contents as staged, use the empty tree for a root commit, and unstage whole files and hunks against the parent without changing working files. Hook fixtures cover `prepare-commit-msg` arguments, `commit-msg`, and `post-rewrite amend` stdin plus warnings.
- CLI tests cover `git good commit --amend`, `git good reflog [REF]`, and `git good branch --create NAME [--at REF]`, including receipts, backup refs, missing references, and branch creation without switching.
- egui tests switch between Local Changes and All Commits from the sidebar and toolbar fallback, keep the toolbar and refs visible, and assert the old Status button is gone. They load the HEAD message into the amend toggle, show the commit's contents as staged, unstage and restage a working change through the unified view, rewrite the commit, clear the amend base, render the **Amended** status line, and render the reflog window and branch-creation dialog at 720 and 1120 widths.
- Reflog read-only tests cover `HEAD` and branch selectors, newest-first entries, times, actions, details, stale-entry clearing, and missing or option-shaped references.
- [Six native CI jobs](https://github.com/baol/gg/actions/runs/39938179241) pass tests, Clippy with `-D warnings`, release builds, CLI smoke checks, and packaging on Linux, macOS, and Windows on x86_64 and arm64 for `3761a1f`.

## M6 rebase · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 280 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- Git oracle tests rebase a diverged branch onto another local branch and verify the rewritten parent, preserved subject, single-parent history, and a private `refs/gg/backup/rebase/...` ref pointing at the previous tip. Local branches, tags, and remote-tracking refs all resolve.
- Clean-worktree tests reject tracked changes, including with `rebase.autoStash=true`, unless autostash is explicitly enabled; `--no-autostash` keeps repository configuration from overriding the decision. Option-shaped and missing references are rejected without changing HEAD.
- Conflict fixtures verify rebase state detection, `REBASE_HEAD`, conflict paths, abort restoring the original tip and worktree, continue after staging, and skip completing without the conflicting commit.
- CLI tests cover `git good rebase REF [--autostash]`, receipt fields, backup refs, dirty failures, conflict abort/continue, and unknown-option errors.
- egui tests render the **Rebase branch** dialog with the autostash choice and private-backup warning, plus the in-progress banner with **Continue**, **Skip**, **Abort**, and **Backups…** at 720 and 1120 widths. The Refs menu and branch, remote, and tag context menus expose rebase actions.

## M6 reorder, squash, and rewrite recovery · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 296 workspace tests pass (3 ignored); formatting, Clippy with `-D warnings`, and the debug build pass.
- [Six native CI jobs](https://github.com/baol/gg/actions/runs/39957559424) pass tests, Clippy with `-D warnings`, release builds, CLI smoke checks, and packaging on Linux, macOS, and Windows on x86_64 and arm64 for `171d2f8`.
- Git oracle tests list a linear range oldest first, reorder commits, squash a commit into the previous row with both messages preserved, rewrite a root range through `--root`, and verify a private `refs/gg/backup/rewrite/...` ref points at the previous tip.
- Rewrite validation rejects empty, squash-first, duplicate, missing, and out-of-range steps; non-ancestor and option-shaped starts; and ranges containing merge commits. Merge commits are rejected rather than flattened, and a shallow boundary is rejected instead of being rewritten as a root.
- Conflict fixtures verify the rewrite leaves Git rebase state, lists unmerged paths, aborts to the previous tip and worktree, and skips the conflicting commit to finish the remaining range. Dirty tracked worktrees are rejected unless autostash is explicit, even with `rebase.autoStash=true`; autostash restores the working change.
- CLI tests cover `git good rewrite --from COMMIT --pick COMMIT... --squash COMMIT...`, reorder and squash receipts, backup refs, conflict abort, skip, and unknown-option errors. `git good rewrite --continue|--skip|--abort` targets the same Git rebase state.
- Backup tests verify `gg_git::backup::list` returns the private reference, kind, commit, subject, and commit time, while the refs snapshot continues to exclude `refs/gg/`.
- egui tests render the **Reorder and squash** modal with ordered rows, up/down icon controls, Squash choices, autostash, and the private-backup warning at 720 and 1120 widths. A pending rewrite keeps the modal open with **Rewriting commits…** and ignores Escape; a pending refresh shows **Refreshing history…** and closes only after the replacement history generation is accepted. The Backups window renders the entry list, **Open commit**, **Reset branch here**, **Create branch here…**, **Copy hash**, and **Copy ref**. The in-progress banner exposes **Backups…** and names an in-progress rebase.
- The native rewrite runs Git on a background worker. The modal disables its controls during the run, overrides the history hover-safety gate for the rewrite refresh, and stays open until the replacement history has loaded instead of exposing the previous commit list.

## Pre-M7 commit draft retention · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 300 workspace tests pass (3 ignored); formatting and Clippy with `-D warnings` pass.
- A Browser-level regression reproduces the old loss: a multiline draft set in Local Changes, then navigation to history and back through All Commits restores the exact text.
- Drafts are keyed by canonical worktree identity: a second worktree starts with an empty draft, and returning to the first worktree restores its draft.
- Normal and amend drafts stay separate across an amend toggle round trip; enabling amend keeps the normal draft, and disabling it restores the normal text while retaining the amend text.
- The selected path and area are captured with the draft and restored when the same entry is present in the refreshed status.
- Hook-failure coverage verifies a failed commit keeps the submitted message; successful-commit coverage verifies the submitted draft clears.
- Native wide/narrow interaction review remains pending for this increment.

## Pre-M7 operation workers · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 302 workspace tests pass (3 ignored); formatting and Clippy with `-D warnings` pass.
- Merge, rebase, and continue/skip/abort share one background operation worker. The dialog returns immediately, the pending operation is named in the status line, duplicate submission is rejected, and the accepted result refreshes mutation state, history, refs, status, reflog, and backups once.
- The event-driven regression holds a worker behind a channel, proves a frame still renders and names the pending operation while the worker is blocked, rejects a second submission, then releases the worker and verifies one accepted result.
- A fixture `pre-rebase` hook blocks on a marker file. The rebase start returns immediately, a frame renders and names **Rebasing onto feature…** while the hook waits, and releasing the marker completes the rebase with the accepted notice.
- Worker failure or disconnection clears the pending label and reports an actionable error while preserving Git's recovery state.
- Native wide/narrow interaction review remains pending for this increment.

## Pre-M7 conflict workflow · 2026-10-23

Linux x86_64 · Rust 1.99.1 · local Git fixtures.

- 305 workspace tests pass (3 ignored); formatting and Clippy with `-D warnings` pass.
- The conflict window names the operation, unresolved count, and operation commit; explains Ours/Theirs for merge, rebase, cherry-pick, and revert; states that side choices replace and stage the whole file; and states that **Stage resolved** stages external or manually edited content.
- Continue is disabled while unresolved paths remain and explains why. Skip and Abort remain deliberate actions with explicit consequences.
- Conflict listing, content reads, and side resolution run on workers. The content test tolerates a blocked reader while frames render and adopts content when it arrives; side resolution starts and completes through the shared operation worker.
- Resolving a path reloads the list and selects the next unresolved path predictably; a pure selection test covers keep-found, keep-missing, and empty lists.
- At narrow widths the Base/Ours/Theirs panes become side tabs with one bounded scroll region; the path list keeps its own bounded scroll.
- Native wide/narrow interaction review remains pending for this increment.
