# Coherent interaction proposal

Purpose: reduce competing interaction patterns and give gg's views a shared navigation, reading, action, and recovery model.

Status: design proposal, 2026-10-25. This revises the direction of the [flow review](user-flows.md), not the shipped behavior in [architecture](architecture.md). Explore the [coherence prototype](ui-coherence.html). The earlier 20-screen mockup pack remains the baseline; the new prototype demonstrates a smaller shared workspace with fictional data and local interactions.

## Main finding

The product does not need fewer Git capabilities. It needs fewer rules for finding and using them. The baseline describes operations thoroughly but often makes each operation a separate screen. The proposed model has two primary destinations, one file workspace, scoped command menus, and one operation area. Changing the comparison or presentation does not create another viewer.

## Consolidation decisions

| Current documented pattern | Proposed rule | What is removed or changed |
| --- | --- | --- |
| Local Changes / All Commits move from sidebar to toolbar at narrow widths | Keep **Local Changes / History** in one navigation strip at every width | Remove duplicate responsive navigation placements; All Commits becomes History |
| Clicking a branch pill inspects; clicking a sidebar local branch switches; double-clicking a pill switches | Selecting any ref inspects its target; **Switch to branch…** explicitly changes the checkout | Remove mutating single/double-click meanings from ref selection |
| Global Actions, Commit actions window, row menu, and ref menus expose actions | Give the selected object one **Commit actions**, **Branch actions**, or **File actions** menu beside its identity | Remove global target ambiguity and the separate command-list window; shortcuts/context menus invoke these same command definitions |
| Commit review, full-width reading, file history, blame, and staging grow their own headers | Use one file workspace with a source pair, file selector, presentation controls, and content | Remove separate Diff/Full-file viewers and a second history reader |
| Staging changes layout between two panes, tabs, and stacked Split | Keep Unstaged files above Staged files on the left, the selected comparison on the right, and commit/amend below the comparison | Both file groups stay visible; selecting an entry chooses the comparison without a duplicate Staged/Unstaged switch |
| Before/After navigates versions while Old/New selects content | **Older version / Newer version** moves in history; **Before / After** selects content | Remove overloaded directional terms |
| Full file, blame, and history can look like peer destinations | Diff/Full file is presentation; **Show attribution** is a Full-file overlay; **File history** changes the comparison source in place | Remove separate blame navigation and preserve the original comparison as the history return point |
| Whole-file conflict choices replace and stage; externally edited content has a different route | Preview a resolution, then use **Stage resolution** for every source | Separate choosing content from publishing it; no content selection silently stages |
| Continue/Skip/Abort, progress, and recovery appear in multiple places; rewrite keeps a blocking modal | Show one operation area beneath primary navigation; configuration closes on submission | Remove duplicate progress/recovery controls and the rewrite-specific progress dialog |
| Stash Apply and Pop are adjacent operations | One **Restore stash…** form, with **Remove stash after successful restore** | Remove parallel forms; retain the underlying Apply/Pop semantics and CLI commands |
| Backups and Reflog are separate windows with shared recovery actions | **Repository → Recovery**, with a source selector for Backups or Reflog | Remove duplicated recovery UI; retain distinct provenance, identifiers, and backend readers |
| Dialog-specific Back, Close, Cancel and return destinations vary | Back returns to the actual caller; Cancel dismisses an unsubmitted action; Close dismisses details | Remove hard-coded returns to History from nested inspection |

This is consolidation of UI rules. Staging and discarding, merge and rebase, or fetch and push remain distinct operations because they have different effects. Shortcuts and contextual access are useful aliases when target, preconditions, wording, confirmation, and outcome are identical.

## One stable shell

1. **Repository identity:** repository, worktree path, current branch, upstream snapshot. Display the actual checkout separately from the selected ref or commit.
2. **Application menus:** Repository (including network, worktrees, stashes, recovery), View (presentation preferences), Help. Move Stash management into Repository; move ref commands to the selected ref's actions. Ref browsing remains reachable at narrow widths through the same labeled ref selector.
3. **Primary navigation:** Local Changes and History, always in the same order and position. Stashes/Recovery are repository tools with one Back to their calling view, not extra permanent primary destinations.
4. **Operation area:** hidden when idle; otherwise one compact strip naming the operation, phase, target, and permitted next action. The same strip persists across browsing views.
5. **Workspace:** collection/list or selected-file workspace, with the same identity and actions alignment.
6. **Outcome line:** one concise result, Details if needed, and at most one next action. Repeated operation controls do not also appear here; an unresolved conflict stays in the operation area.

Wide layouts retain file selection to the left of the reader. Local Changes uses the requested three-pane arrangement: Unstaged above Staged on the left; changes on the right; commit/amend at the lower right. Other file views use a single list in that same left column. Narrow layouts move file selection above the reader; preserve both named Local Changes groups and keep commit/amend immediately below the diff. Larger lists can collapse by group without adding a second comparison switch. Avoid changing action order or replacing the user's task with a different screen at a breakpoint. Full-width reading is a layout preference that hides the list, not another navigation destination.

## One file workspace

| Control | Meaning | State changed |
| --- | --- | --- |
| Files | Select a path within the current comparison | Path and its remembered reading place |
| Comparison source | Commit parent, the selected Staged/Unstaged list entry, or file-history version | Exact Before/After endpoints; no duplicate staged-state switch in the reader |
| Diff / Full file | Present the same comparison | Presentation only |
| Before / After | Choose content in Full file | Inspection side only; hidden in unified Diff |
| Show attribution | Add commit provenance to Full file | Overlay only; a label explains unsupported/uncommitted attribution |
| Older version / Newer version | Move along content versions | Current file-history version and comparison edge |
| Previous change / Next change | Move within the current content | Reading place only |
| File actions | Export, copy path, scoped mutations where applicable | Explicit command with a named target |

The header always spells out both endpoints: parent → commit, HEAD → index, index → working file, or older version → selected version. In amend mode Staged is explicitly HEAD parent → index. No generic 'Before' label replaces the endpoint identity.

Each view states the file identity, exact endpoints, presentation, and its scoped commands once. The path appears in the reader header rather than in both a view identity strip and the reader. Low-frequency metadata (modes, object IDs, byte sizes) and commands (export, copy path) live in an on-demand **File details** disclosure or **File actions** menu instead of permanent rows. There is no second route to the same comparison, such as a separate **View diff** step.

Opening File history saves the entry comparison, file, side, presentation, attribution state, and line/range. A version strip appears above the same reader. Closing File history restores that saved context once; older/newer clicks do not add Back entries. A predecessor chooser is an interruption inside this source change and returns to the same reader. Diff is unavailable until the earlier edge is resolved; current Full file remains readable.

Line history uses this same version strip with a visible range anchor. Deleted ranges name the Before side. Attribution retains the existing read-only/history-bounds contracts; attribution of live uncommitted content requires its own specification and is not assumed here.

## One action grammar

Every command answers: **what object, what scope, what effect?**

- Commit actions stay next to the selected commit identity. Branch actions stay next to the selected branch identity. File actions stay next to the path. The current checkout remains independently visible.
- Promote only the frequent action beside the relevant content: Stage hunk at a hunk, Stage selected at a selected range, and the whole-file action in the file toolbar. These are different scopes of one command family, not interchangeable synonyms.
- Use Stage/Unstage for index changes, Discard for working changes, Delete for refs, Drop for stash entries, and Remove worktree for the directory-removal operation. The object is always in the confirmation. Do not use Drop as a second label for Discard or Delete.
- Use **Open commit** consistently from blame, stash, recovery, and reference inspection. It always opens the ordinary commit review with a return location.
- Confirmations share one structure: title/verb, exact object, source and destination, effect, recovery information when applicable, Cancel, and a specific submit label. Read-only navigation does not open confirmation dialogs.
- Context menus and keyboard bindings are generated from the same scoped commands as the visible action menu. A command has one readiness reason and one confirmation/outcome path regardless of how it was invoked.

Branch creation has one form with a starting point and an explicit **Switch after creation** choice. Creating from recovery defaults to staying in the current worktree. Context may prefill the form, but never changes what Create means without showing that choice.

## One lifecycle for operations

**Configure → validate → run → refresh → done**, with **needs attention** for conflicts and actionable errors. One operation area beneath navigation is hidden when idle and owns progress, cancellation, and supported continuation; configuration closes on submission, so a running rewrite keeps browsing available.

Configuration is specific to the Git operation. The lifecycle and placement are shared. Merge, rebase, rewrite, stash, and network operations use the same pending/attention/result presentation. Rewriting becomes non-blocking for browsing after submission; its old history remains explicitly labeled until replacement data is accepted. Incompatible mutations remain disabled. That is a proposed change to the existing blocking rewrite contract and needs native worker/state verification before implementation.

Continue/Skip/Abort appear only in the operation area and only where Git supports them. A merge has no Skip. A stash apply conflict has no sequencer Continue/Abort. Cancellation requests for cancellable network/export work remain distinct from aborting a Git sequencer. A cancelled process is followed by state inspection; it does not imply rollback.

If validation interrupts a requested action, retain an **intent**, not a queued mutation: target, chosen options, and caller. After the user resolves the prerequisite, offer the named action again and revalidate. Never automatically switch branches or start a rebase because a stash finished.

## Connected flows using the shared patterns

### Inspect a change

History → select commit → Open commit → choose file → Diff/Full file. Show attribution toggles in place. File history adds the version strip to the same reader. Close file history restores the original comparison. Back returns to History with the same commit selected. Opening the same commit from Recovery instead returns to Recovery.

### Commit a portion of the work

Local Changes → choose a file in the upper-left Unstaged list → select hunk → Stage hunk. Refresh both lists and the right-hand comparison together. A partially staged file appears in both groups, each entry selecting its corresponding comparison. A fully staged file leaves Unstaged; retain the just-reviewed path with an explicit empty comparison until another entry is selected. Selecting its lower-left Staged entry reviews the index in the same reader. The commit/amend form stays below the diff in the right column. New commit/Amend shares the form and keeps separate drafts; amend clearly identifies the old commit and changed base. Commit refreshes in place, retains remaining changes, and offers View commit. A hook failure preserves the form and draft and uses the normal outcome/details path.

### Resolve a conflict

Operation area → Resolve conflicts → Local Changes, Conflicts section → selected file. The file workspace identifies Base, current replay/branch content, incoming commit content, and a result preview. Select a source or refresh external edits, inspect the result, then Stage resolution. Choosing a source changes only the preview; Stage resolution explicitly writes the selected result and stages it. The next unresolved file is selected. Continue appears in the same operation area when all are resolved. This preview-first submission changes the current immediate choose-and-stage interaction and must retain stale-content checks and operation-aware identities.

### Restore a stash

Repository → Stashes → select entry → Restore stash. Options: Restore staged state; Remove stash after successful restore. A conflict keeps the entry, shows the normal attention area, and opens the same conflict file workspace. On resolution, acknowledge that stash recovery is complete without fabricating a Git Continue command. Deleting a stash is a separate scoped destructive command using the common confirmation layout.

### Recover a commit

Repository → Recovery → choose Backups or Reflog → select entry → Open commit in the usual reader. Back returns to that recovery source, the Reflog reference filter, and the selected entry. Create branch here opens the shared branch form with Switch after creation off. Reset opens the shared mode/target confirmation. A successful recovery does not erase the source entry or backup.

### Resume a blocked branch switch

History → select branch → Branch actions → Switch to main. Local changes block validation. Review changes or Restore/stash work through the existing workspace. Return to the saved switch intent, show its target again, and revalidate on submission. Clicking a branch or opening a commit never changes the checkout.

## Tradeoffs and rollout

The selected direction preserves simultaneous visibility of **file groups**, with one shared **diff reader**. The user requested Unstaged at upper left, Staged at lower left, changes on the right, and commit/amend at lower right. This gives an overview of both states without adding competing diff panes or a redundant reader switch. The left list selection is the single source of comparison context.

Adopt in small increments:

1. Standardize vocabulary, scoped commands, explicit branch switching, and return destinations. Preserve keyboard/context-menu accelerators as aliases.
2. Reuse the file workspace across review, Local Changes, file history, and attribution. Keep comparison identity and reading-place preservation testable across every source change.
3. Move conflict content into that workspace, introduce the explicit result-preview/stage boundary, and share the operation area including rewrite progress.
4. Consolidate repository management forms and recovery entry points. Then add M7 worktrees, submodules, LFS, and signing within these surfaces.

Before changing production UI, update the owning architecture and plan contracts for each increment. This proposal itself changes no production behavior and does not claim M7 implementation or native acceptance.

## Deferred elements

These controls appear in the design exploration but are intentionally not part of this delivery. They stay planned in [plan](plan.md) with honest extension points; no disabled or placeholder control pretends they work.

- **Onion skin**: a planned image presentation that overlays Before and After with an explicit opacity control; side-by-side and swipe are implemented, onion skin is not.
- **Commit and tag signing**: verification and signing configuration remain unimplemented.
- **Force push**: push stays non-force; no force-with-lease or force toggle is offered.
- **Restart-persistent drafts**: normal/amend drafts survive view and repository changes in one session, not application restarts.
- **A conflict panel inside Local Changes**: the shared preview/publication grammar lives in the dedicated resizable conflict workspace; folding it into the two-list panel remains a documented limitation rather than a hidden second route.
- **Cross-platform and full accessibility acceptance**: the recorded native evidence is Linux/X11 with llvmpipe; macOS, Windows, other display scales, screen readers, and exhaustive contrast remain unverified.

## Prototype review criteria

- The same file controls appear in the same order for commit review, staging, and file history.
- Local Changes keeps both file groups visible and places commit/amend below the right-hand diff. Selecting the upper/lower list sets index → worktree or HEAD → index respectively.
- Amend changes the staged base to HEAD parent, retains separate drafts, and supports a message-only edit.
- Changing presentation or comparison never changes the selected file accidentally.
- Before/After never moves between versions; Older/Newer never selects an inspection side.
- Attribution and file history preserve the reader instead of opening another content window.
- Stage/Unstage/Stage resolution have named scopes and one publication boundary.
- Back returns to the actual caller; source selection and presentation changes do not inflate the navigation stack.
- One operation area owns progress and continuation across views; no duplicate Continue buttons exist.
- Wide and narrow layouts expose the same commands; all controls remain keyboard-reachable.

The prototype covers these shared patterns with five starting scenarios. Branch switching, full command menus, networking, persistence across app restarts, and native Git execution remain specification-only in this exploration. The baseline pack retains the other feature examples.

Editable source: [ui-coherence.fragment.html](ui-coherence.fragment.html). Standalone preview: [ui-coherence.html](ui-coherence.html). Keep the fragment and generated preview in sync. Browser checks and captures: [verification](verification.md#coherence-proposal-and-shared-workspace--2026-10-25).

Implementation handoff: [subagent implementation and verification prompts](implementation-prompts.md), including dependency order, shared constraints, and independent acceptance checks.
